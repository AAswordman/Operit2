"""Prepare board-local size hints for the pinned Serde derive generator.

Generated visitors share field-name lookup and use size-oriented inlining hints.
Schemas, field checks and formats stay unchanged. Other products keep upstream Serde.
"""
from pathlib import Path
import hashlib
import io
import os
import re
import tarfile
import tempfile
import urllib.request

ROOT = Path(__file__).resolve().parents[4]
VERSION = "1.0.229"
SHA256 = "e7a5d71263a5a7d47b41f6b3f06ba276f10cc18b0931f1799f710578e2309348"
CRATE = f"serde_derive-{VERSION}"
DEST = ROOT / "edge/boards/esp32/.embuild"
OVERLAY = f"serde_derive-tables-{VERSION}"
MARKER = "operit-derive-tables-v1"


def patch_source(text):
    text = text.replace("#[inline]", "#[inline(never)]")
    text = re.sub(r"(?m)^(\s*)((?:#vis )?fn (?:serialize|deserialize)<__)",
                  r"\1#[inline(never)]\n\1\2", text)
    text = re.sub(r"(?m)^(\s*)(fn (?:visit_seq|visit_map|visit_enum|visit_newtype_struct|visit_some|expecting)(?:<|\())",
                  r"\1#[inline(never)]\n\1\2", text)
    if "fn deserialize_identifier(" in text:
        text = share_field_lookup(text)
    return text


def share_field_lookup(text):
    start = text.index("    let str_mapping = deserialized_fields.iter().map(")
    end = text.index("\n    let expecting = ", start)
    text = text[:start] + r'''    let field_names: Vec<_> = deserialized_fields.iter().enumerate().flat_map(|(index, field)| {
        field.aliases.iter().map(move |alias| {
            let name = Literal::byte_string(alias.value.as_bytes());
            quote! { (#name, #index) }
        })
    }).collect();
    let mappings: Vec<_> = deserialized_fields.iter().enumerate().map(|(index, field)| {
        let ident = &field.ident;
        quote! { #index => _serde::#private::Ok(#this_value::#ident), }
    }).collect();
    let lookup = quote! {
        #[inline(never)]
        fn __lookup(value: &[u8], fields: &[(&[u8], usize)]) -> usize {
            for &(name, index) in fields { if name == value { return index; } }
            usize::MAX
        }
    };
''' + text[end:]
    for kind, value in [("str", "__value.as_bytes()"), ("bytes", "__value")]:
        text = text.replace(f"        let {kind}_mapping = {kind}_mapping.clone();\n", "")
        pattern = r"match __value \{\n(\s*)#\(#" + kind + r"_mapping\)\*"
        text, count = re.subn(pattern, lambda match: "#lookup\n" + match[1]
                             + "match __lookup(" + value + ", &[#(#field_names),*]) {\n"
                             + match[1] + "#(#mappings)*", text)
        if count != 2:
            raise RuntimeError("Unexpected pinned Serde field lookup; manual review required")
    return text


def pinned_archive():
    cargo = Path(os.environ.get("CARGO_HOME", Path.home() / ".cargo"))
    cached = list((cargo / "registry/cache").glob(f"*/{CRATE}.crate"))
    if cached:
        data = cached[0].read_bytes()
    else:
        with urllib.request.urlopen(f"https://static.crates.io/crates/serde_derive/{CRATE}.crate", timeout=60) as response:
            data = response.read()
    if hashlib.sha256(data).hexdigest() != SHA256:
        raise RuntimeError("Serde derive archive checksum mismatch")
    return data


def expected_files(data):
    expected = {}
    with tarfile.open(fileobj=io.BytesIO(data), mode="r:gz") as archive:
        for member in archive.getmembers():
            prefix = CRATE + "/"
            if not member.name.startswith(prefix) or not (member.isfile() or member.isdir()):
                raise RuntimeError(f"Unsafe archive entry: {member.name}")
            relative = member.name[len(prefix):]
            parts = Path(relative).parts
            if not parts or Path(relative).is_absolute() or ".." in parts:
                raise RuntimeError(f"Unsafe archive entry: {member.name}")
            if not member.isfile():
                continue
            if relative in expected:
                raise RuntimeError(f"Duplicate archive entry: {member.name}")
            content = archive.extractfile(member).read()
            if relative.startswith("src/") and relative.endswith(".rs"):
                content = patch_source(content.decode("utf-8")).encode("utf-8")
            expected[relative] = content
    if "Cargo.toml" not in expected or "src/lib.rs" not in expected:
        raise RuntimeError("Pinned Serde derive crate files missing")
    return expected


def prepare():
    expected = expected_files(pinned_archive())
    crate = DEST / OVERLAY
    if not crate.exists():
        DEST.mkdir(parents=True, exist_ok=True)
        # Publish only a complete overlay; an interrupted preparation is retryable.
        with tempfile.TemporaryDirectory(prefix="serde-prepare-", dir=DEST) as temporary:
            staging = Path(temporary) / OVERLAY
            staging.mkdir()
            for relative, content in expected.items():
                target = staging / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(content)
            (staging / MARKER).write_text(SHA256 + "\n", encoding="utf-8")
            staging.rename(crate)
    marker = crate / MARKER
    if marker.exists() and marker.read_text(encoding="utf-8").strip() != SHA256:
        raise RuntimeError("Serde derive overlay checksum marker mismatch")
    for relative, content in expected.items():
        target = crate / relative
        actual = target.read_bytes() if target.is_file() else None
        # Python's earlier Windows preparation used CRLF for source files.
        if actual is not None and relative.endswith(".rs"):
            actual = actual.replace(b"\r\n", b"\n")
        if actual != content:
            raise RuntimeError(f"Serde derive overlay differs from pinned patch: {relative}")
    if not marker.exists():
        marker.write_text(SHA256 + "\n", encoding="utf-8")
    print(f"ESP32 Serde derive {VERSION}: verified pinned source, shared field lookup and size hints")


if __name__ == "__main__":
    prepare()
