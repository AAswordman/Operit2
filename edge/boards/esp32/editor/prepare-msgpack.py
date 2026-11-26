"""Prepare a pinned MessagePack overlay for the ESP32 flash budget.

Array and binary sequence decoding share one SeqAccess type and visitor call.
This prevents two recursive decoder instantiations per value type under LTO.
Marker/length/number reads and UTF-8 validation are shared across visitors.
The common dispatcher stays out of line across typed entrypoints.
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
MARKER = "operit-shared-token-v4"
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
    # Keep the marker dispatcher shared across seq/tuple/struct entrypoints.
    text = replace_once(text, "    fn any_inner<V: Visitor<'de>>", "    #[inline(never)]\n    fn any_inner<V: Visitor<'de>>")
    # Each field type shares its recursive decoder across message visitors.
    # The upstream inline hints otherwise duplicate those bodies under Fat LTO.
    head, tail = text.split("struct SeqAccess<'a, R, C>", 1)
    for method in ("next_element_seed", "next_key_seed", "next_value_seed"):
        tail = replace_once(tail, f"#[inline]\n    fn {method}", f"#[inline(never)]\n    fn {method}")
    text = head + "struct SeqAccess<'a, R, C>" + tail
    return share_token_reader(text)


def share_token_reader(text):
    # Wire parsing depends on the reader, not on each message's Serde visitor.
    # Keep the exact numeric visit_* width and float behavior from upstream.
    start = text.index("#[inline(never)]\nfn any_num<")
    end = text.index("\nimpl<'de, R: ReadSlice<'de>, C: SerializerConfig> serde::Deserializer", start)
    replacement = r"""enum Number {
    Unit, Bool(bool), U8(u8), U16(u16), U32(u32), U64(u64),
    I8(i8), I16(i16), I32(i32), I64(i64), F32(f32), F64(f64),
}

#[inline(never)]
fn read_number<'de, R: ReadSlice<'de>>(rd: &mut R, marker: Marker) -> Result<Number, Error> {
    Ok(match marker {
        Marker::Null => Number::Unit,
        Marker::True | Marker::False => Number::Bool(marker == Marker::True),
        Marker::FixPos(value) => Number::U8(value),
        Marker::FixNeg(value) => Number::I8(value),
        Marker::U8 => Number::U8(rd.read_data_u8()?),
        Marker::U16 => Number::U16(rd.read_data_u16()?),
        Marker::U32 => Number::U32(rd.read_data_u32()?),
        Marker::U64 => Number::U64(rd.read_data_u64()?),
        Marker::I8 => Number::I8(rd.read_data_i8()?),
        Marker::I16 => Number::I16(rd.read_data_i16()?),
        Marker::I32 => Number::I32(rd.read_data_i32()?),
        Marker::I64 => Number::I64(rd.read_data_i64()?),
        Marker::F32 => Number::F32(rd.read_data_f32()?),
        Marker::F64 => Number::F64(rd.read_data_f64()?),
        other => return Err(Error::TypeMismatch(other)),
    })
}

#[inline(never)]
fn visit_number<'de, V: Visitor<'de>>(number: Number, visitor: V) -> Result<V::Value, Error> {
    match number {
        Number::Unit => visitor.visit_unit(),
        Number::Bool(value) => visitor.visit_bool(value),
        Number::U8(value) => visitor.visit_u8(value),
        Number::U16(value) => visitor.visit_u16(value),
        Number::U32(value) => visitor.visit_u32(value),
        Number::U64(value) => visitor.visit_u64(value),
        Number::I8(value) => visitor.visit_i8(value),
        Number::I16(value) => visitor.visit_i16(value),
        Number::I32(value) => visitor.visit_i32(value),
        Number::I64(value) => visitor.visit_i64(value),
        Number::F32(value) => visitor.visit_f32(value),
        Number::F64(value) => visitor.visit_f64(value),
    }
}

#[inline(never)]
fn any_num<'de, R: ReadSlice<'de>, V: Visitor<'de>>(rd: &mut R, visitor: V, marker: Marker) -> Result<V::Value, Error> {
    visit_number(read_number(rd, marker)?, visitor)
}

enum Token { Number(Number), String(u32), Array(u32), Map(u32), Binary(u32), Extension(u32) }

impl<'de, R: ReadSlice<'de>, C: SerializerConfig> Deserializer<R, C> {
    #[inline(never)]
    fn read_token(&mut self) -> Result<Token, Error> {
        let marker = self.take_or_read_marker()?;
        Ok(match marker {
            Marker::FixStr(len) => Token::String(len.into()),
            Marker::Str8 => Token::String(read_u8(&mut self.rd)?.into()),
            Marker::Str16 => Token::String(read_u16(&mut self.rd)?.into()),
            Marker::Str32 => Token::String(read_u32(&mut self.rd)?),
            Marker::FixArray(len) => Token::Array(len.into()),
            Marker::Array16 => Token::Array(read_u16(&mut self.rd)?.into()),
            Marker::Array32 => Token::Array(read_u32(&mut self.rd)?),
            Marker::FixMap(len) => Token::Map(len.into()),
            Marker::Map16 => Token::Map(read_u16(&mut self.rd)?.into()),
            Marker::Map32 => Token::Map(read_u32(&mut self.rd)?),
            Marker::Bin8 => Token::Binary(read_u8(&mut self.rd)?.into()),
            Marker::Bin16 => Token::Binary(read_u16(&mut self.rd)?.into()),
            Marker::Bin32 => Token::Binary(read_u32(&mut self.rd)?),
            Marker::FixExt1 | Marker::FixExt2 | Marker::FixExt4 | Marker::FixExt8 |
            Marker::FixExt16 | Marker::Ext8 | Marker::Ext16 | Marker::Ext32 => Token::Extension(ext_len(&mut self.rd, marker)?),
            other => Token::Number(read_number(&mut self.rd, other)?),
        })
    }

    #[inline(never)]
    fn any_inner<V: Visitor<'de>>(&mut self, visitor: V, allow_bytes: bool) -> Result<V::Value, Error> {
        match self.read_token()? {
            Token::Number(number) => visit_number(number, visitor),
            Token::String(len) => read_str_data(&mut self.rd, len, visitor),
            Token::Array(len) => depth_count!(self.depth, {
                visit_sequence(SeqAccess::new(self, len), visitor, true)
            }),
            Token::Map(len) => depth_count!(self.depth, {
                let mut seq = MapAccess::new(self, len);
                let result = visitor.visit_map(&mut seq)?;
                match seq.left {
                    0 => Ok(result),
                    excess => Err(Error::LengthMismatch(len - excess)),
                }
            }),
            Token::Binary(len) => match read_bin_data(&mut self.rd, len)? {
                Reference::Borrowed(buf) if allow_bytes => visitor.visit_borrowed_bytes(buf),
                Reference::Copied(buf) if allow_bytes => visitor.visit_bytes(buf),
                Reference::Borrowed(buf) | Reference::Copied(buf) => {
                    visit_sequence(SeqAccess::<R, C>::from_bytes(buf), visitor, false)
                },
            },
            Token::Extension(len) => depth_count!(self.depth,
                visitor.visit_newtype_struct(ExtDeserializer::new(self, len))),
        }
    }
}
"""
    text = text[:start] + replacement + text[end:]
    start = text.index("fn read_str_data<'de, V, R>")
    end = text.index("\nfn read_bin_data", start)
    return text[:start] + r"""enum Text<'de, 'a> {
    Borrowed(Result<&'de str, Utf8Error>, &'de [u8]),
    Copied(Result<&'a str, Utf8Error>, &'a [u8]),
}

#[inline(never)]
fn read_text<'a, 'de, R: ReadSlice<'de>>(rd: &'a mut R, len: u32) -> Result<Text<'de, 'a>, Error> {
    Ok(match read_bin_data(rd, len)? {
        Reference::Borrowed(buf) => Text::Borrowed(str::from_utf8(buf), buf),
        Reference::Copied(buf) => Text::Copied(str::from_utf8(buf), buf),
    })
}

#[inline(never)]
fn read_str_data<'de, V: Visitor<'de>, R: ReadSlice<'de>>(rd: &mut R, len: u32, visitor: V) -> Result<V::Value, Error> {
    match read_text(rd, len)? {
        Text::Borrowed(Ok(text), _) => visitor.visit_borrowed_str(text),
        Text::Copied(Ok(text), _) => visitor.visit_str(text),
        Text::Borrowed(Err(error), bytes) => visitor.visit_borrowed_bytes::<Error>(bytes)
            .map_err(|_| Error::Utf8Error(error)),
        Text::Copied(Err(error), bytes) => visitor.visit_bytes::<Error>(bytes)
            .map_err(|_| Error::Utf8Error(error)),
    }
}
""" + text[end:]


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
    print(f"ESP32 MessagePack {VERSION}: verified pinned source and shared token/sequence decoding")


if __name__ == "__main__":
    prepare()
