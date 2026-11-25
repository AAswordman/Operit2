"""Prepare a pinned MessagePack overlay for the ESP32 flash budget.

Array and binary sequence decoding share one SeqAccess type and visitor call.
This prevents two recursive decoder instantiations per value type under LTO.
Wire markers, depth limits, u8 visitor semantics and consumption checks remain
identical to rmp-serde 1.3.1. Cargo's global cache is never modified.
"""
from pathlib import Path
import hashlib
import io
import os
import tarfile
import urllib.request

ROOT = Path(__file__).resolve().parents[4]
VERSION = "1.3.1"
SHA256 = "72f81bee8c8ef9b577d1681a70ebbc962c232461e397b22c208c43c04b67a155"
CRATE = f"rmp-serde-{VERSION}"
DEST = ROOT / "edge/boards/esp32/.embuild"
MARKER = "operit-shared-sequence-v1"
SOURCE = "src/decode.rs"


def replace_once(text, old, new):
    if text.count(old) != 1:
        raise RuntimeError("Unexpected pinned MessagePack source; manual review required")
    return text.replace(old, new, 1)


def patch_source(text):
    text = replace_once(text, "use serde::de::value::SeqDeserializer;\n", "")
    text = replace_once(text, """                    let mut seq = SeqAccess::new(self, len);
                    let res = visitor.visit_seq(&mut seq)?;
                    match seq.left {
                        0 => Ok(res),
                        excess => Err(Error::LengthMismatch(len - excess)),
                    }""", """                    visit_sequence(SeqAccess::new(self, len), visitor, true)""")
    text = replace_once(text, "visitor.visit_seq(SeqDeserializer::new(buf.iter().copied()))",
                        "visit_sequence(SeqAccess::<R, C>::from_bytes(buf), visitor, false)")
    text = replace_once(text, """struct SeqAccess<'a, R, C> {
    de: &'a mut Deserializer<R, C>,
    left: u32,
}""", """struct SeqAccess<'a, R, C> {
    de: Option<&'a mut Deserializer<R, C>>,
    bytes: &'a [u8],
    left: u32,
}

// The visitor and its recursive decoder are instantiated once for both carriers.
#[inline(never)]
fn visit_sequence<'de, 'a, R: ReadSlice<'de> + 'a, C: SerializerConfig, V: Visitor<'de>>(
    mut sequence: SeqAccess<'a, R, C>, visitor: V, require_end: bool,
) -> Result<V::Value, Error> {
    let length = sequence.left;
    let result = visitor.visit_seq(&mut sequence)?;
    if require_end && sequence.left != 0 {
        return Err(Error::LengthMismatch(length - sequence.left));
    }
    Ok(result)
}""")
    text = replace_once(text, """        SeqAccess { de, left: len }
    }
}""", """        SeqAccess { de: Some(de), bytes: &[], left: len }
    }
    fn from_bytes(bytes: &'a [u8]) -> Self {
        Self { de: None, bytes, left: bytes.len() as u32 }
    }
}""")
    text = replace_once(text, """            Ok(Some(seed.deserialize(&mut *self.de)?))""", """            match self.de.as_deref_mut() {
                Some(de) => seed.deserialize(de).map(Some),
                None => {
                    let index = self.bytes.len() - self.left as usize - 1;
                    seed.deserialize(serde::de::value::U8Deserializer::<Error>::new(self.bytes[index])).map(Some)
                }
            }""")
    # Each field type shares its recursive decoder across message visitors.
    # The upstream inline hints otherwise duplicate those bodies under Fat LTO.
    head, tail = text.split("struct SeqAccess<'a, R, C>", 1)
    for method in ("next_element_seed", "next_key_seed", "next_value_seed"):
        tail = replace_once(tail, f"#[inline]\n    fn {method}", f"#[inline(never)]\n    fn {method}")
    return head + "struct SeqAccess<'a, R, C>" + tail


def pinned_archive():
    cargo = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    cached = list((cargo / "registry/cache").glob(f"*/{CRATE}.crate"))
    if cached:
        data = cached[0].read_bytes()
    else:
        with urllib.request.urlopen(f"https://static.crates.io/crates/rmp-serde/{CRATE}.crate", timeout=60) as response:
            data = response.read()
    if hashlib.sha256(data).hexdigest() != SHA256:
        raise RuntimeError("MessagePack archive checksum mismatch")
    return data


def prepare():
    data = pinned_archive()
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        expected = patch_source(archive.extractfile(f"{CRATE}/{SOURCE}").read().decode("utf-8"))
    crate = DEST / CRATE
    marker = crate / MARKER
    if not marker.exists():
        DEST.mkdir(parents=True, exist_ok=True)
        with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
            for member in archive.getmembers():
                if not (DEST / member.name).resolve().is_relative_to(crate.resolve()) or not (member.isfile() or member.isdir()):
                    raise RuntimeError(f"Unsafe archive entry: {member.name}")
            archive.extractall(DEST, filter="data")
        (crate / SOURCE).write_text(expected, encoding="utf-8")
        marker.write_text(SHA256 + "\n", encoding="utf-8")
    if marker.read_text(encoding="utf-8").strip() != SHA256 or (crate / SOURCE).read_text(encoding="utf-8") != expected:
        raise RuntimeError("MessagePack overlay differs from the pinned patch")
    print(f"ESP32 MessagePack {VERSION}: verified pinned source and shared sequence decoding")


if __name__ == "__main__":
    prepare()
