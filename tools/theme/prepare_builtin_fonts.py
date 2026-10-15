"""Prepare pinned static theme font data, not an application build.

Uses a hash-pinned pure-Python FontTools wheel in the ignored project cache.
No pip install, SDK mutation, or system-font installation. Existing asset bytes
are never overwritten with different content.
"""
from __future__ import annotations

import hashlib
import io
import json
from pathlib import Path
import sys
import urllib.request

ROOT = Path(__file__).resolve().parents[2]
PACKAGE = ROOT / "apps/flutter/packages/operit_ui"
FONTTOOLS_VERSION = "4.61.1"
FONTTOOLS_URL = (
    "https://files.pythonhosted.org/packages/c7/4e/"
    "ce75a57ff3aebf6fc1f4e9d508b8e5810618a33d900ad6c19eb30b290b97/"
    "fonttools-4.61.1-py3-none-any.whl"
)
FONTTOOLS_SHA256 = "17d2bf5d541add43822bcf0c43d7d847b160c9bb01d15d5007d84e2217aaa371"
NOTO_COMMIT = "f8d157532fbfaeda587e826d4cd5b21a49186f7c"
CASCADIA_LICENSE_COMMIT = "56bcca3f2c1e4cb19458954f0e2bb4635960df91"
NOTO_URL = (
    f"https://raw.githubusercontent.com/notofonts/noto-cjk/{NOTO_COMMIT}/"
    "Sans/Variable/TTF/Subset/NotoSansSC-VF.ttf"
)
NOTO_LICENSE_URL = (
    f"https://raw.githubusercontent.com/notofonts/noto-cjk/{NOTO_COMMIT}/Sans/LICENSE"
)
CASCADIA_LICENSE_URL = (
    "https://raw.githubusercontent.com/microsoft/cascadia-code/"
    f"{CASCADIA_LICENSE_COMMIT}/LICENSE"
)
MAX_BYTES = 32 * 1024 * 1024


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def fetch(url: str, limit: int) -> bytes:
    request = urllib.request.Request(url, headers={"User-Agent": "Operit2-theme-font-assets"})
    with urllib.request.urlopen(request, timeout=90) as response:
        content = response.read(limit + 1)
    if not content or len(content) > limit:
        raise ValueError(f"Source exceeds byte budget: {url}")
    return content


def write_new_or_identical(path: Path, data: bytes) -> None:
    if path.exists():
        if path.read_bytes() != data:
            raise ValueError(f"Refusing to overwrite different bytes: {path}")
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(data)


def prepare_font(font_id: str, source: bytes, license_text: bytes, source_uri: str,
                 license_uri: str, family: str) -> dict:
    from fontTools.ttLib import TTFont
    from fontTools.varLib.instancer import instantiateVariableFont

    if source[:4] != b"\x00\x01\x00\x00":
        raise ValueError(f"Source is not TrueType: {font_id}")
    font = TTFont(io.BytesIO(source), recalcTimestamp=False)
    if "fvar" not in font or "glyf" not in font:
        raise ValueError(f"Expected variable TrueType source: {font_id}")
    source_version = font["name"].getDebugName(5)
    copyright_text = font["name"].getDebugName(0)
    axes = {axis.axisTag: axis.defaultValue for axis in font["fvar"].axes}
    if "wght" not in axes:
        raise ValueError(f"Expected explicit weight axis: {font_id}")
    axes["wght"] = 400
    print(f"Preparing {font_id}, axes={axes}", flush=True)
    font = instantiateVariableFont(font, axes, inplace=True)
    forbidden = {"fvar", "gvar", "avar", "cvar", "HVAR", "MVAR", "CFF ", "CFF2",
                 "COLR", "CPAL", "CBDT", "CBLC", "sbix", "SVG "}
    if forbidden.intersection(font.keys()):
        raise ValueError(f"Output is not static monochrome TrueType: {font_id}")
    if not {"head", "hhea", "hmtx", "maxp", "loca", "glyf", "cmap"}.issubset(font.keys()):
        raise ValueError(f"Output has missing tables: {font_id}")
    if len(font.keys()) > 64 or font["maxp"].numGlyphs > 65535:
        raise ValueError(f"Output exceeds v1 table/glyph budget: {font_id}")
    # Derivative family names avoid using original reserved font names.
    names = font["name"]
    rewritten_ids = {1, 2, 3, 4, 6, 16, 17}
    names.names = [record for record in names.names if record.nameID not in rewritten_ids]
    new_names = {
        1: family, 2: "Regular", 3: f"{family};{FONTTOOLS_VERSION};{sha(source)}",
        4: f"{family} Regular", 6: family.replace(" ", "") + "-Regular",
        16: family, 17: "Regular",
    }
    for name_id, text in new_names.items():
        names.setName(text, name_id, 3, 1, 0x409)
    font["OS/2"].usWeightClass = 400
    output = io.BytesIO()
    font.save(output, reorderTables=True)
    font.close()
    data = output.getvalue()
    if len(data) > MAX_BYTES:
        raise ValueError(f"Output exceeds font byte budget: {font_id}")
    license_string = license_text.decode("utf-8")
    if "SIL OPEN FONT LICENSE" not in license_string.upper():
        raise ValueError(f"Expected complete OFL license: {font_id}")
    bundled_license = (
        f"Operit static derivative: {family}\nOriginal font: {font_id}\n"
        f"Original copyright: {copyright_text}\nSource: {source_uri}\n"
        f"License source: {license_uri}\nFontTools: {FONTTOOLS_VERSION}\n"
        f"Static axes: {json.dumps(axes, sort_keys=True)}\n\n"
        + license_string
    ).encode("utf-8")
    asset = f"assets/fonts/{font_id}.ttf"
    license_asset = f"assets/licenses/{font_id}.txt"
    write_new_or_identical(PACKAGE / asset, data)
    write_new_or_identical(PACKAGE / license_asset, bundled_license)
    print(f"Wrote {font_id}: {len(data)} bytes, SHA-256 {sha(data)}", flush=True)
    return {
        "id": font_id, "asset": asset, "bytes": len(data), "sha256": sha(data),
        "license": "OFL-1.1", "licenseAsset": license_asset,
        "licenseSha256": sha(bundled_license), "source": source_uri,
        "sourceSha256": sha(source), "sourceFontVersion": source_version,
        "recipe": {"fontTools": FONTTOOLS_VERSION, "axes": axes, "family": family},
    }


def main() -> None:
    print("Fetching hash-pinned FontTools wheel into project-local cache", flush=True)
    wheel = fetch(FONTTOOLS_URL, 2 * 1024 * 1024)
    if sha(wheel) != FONTTOOLS_SHA256:
        raise ValueError("FontTools wheel hash mismatch")
    wheel_path = ROOT / ".venv/theme-font-tools/fonttools-4.61.1-py3-none-any.whl"
    write_new_or_identical(wheel_path, wheel)
    sys.path.insert(0, str(wheel_path))
    print("Fetching pinned Noto source and OFL license text", flush=True)
    noto_source = fetch(NOTO_URL, MAX_BYTES)
    noto_license = fetch(NOTO_LICENSE_URL, 128 * 1024)
    cascadia_source = (ROOT / "apps/flutter/app/assets/fonts/CascadiaMono.ttf").read_bytes()
    cascadia_license = fetch(CASCADIA_LICENSE_URL, 128 * 1024)
    entries = [
        prepare_font("noto-sans-sc", noto_source, noto_license, NOTO_URL,
                     NOTO_LICENSE_URL, "Operit Theme Sans SC"),
        prepare_font("cascadia-mono", cascadia_source, cascadia_license,
                     "repository:apps/flutter/app/assets/fonts/CascadiaMono.ttf",
                     CASCADIA_LICENSE_URL, "Operit Theme Mono"),
    ]
    catalog = {"formatVersion": 1, "fonts": entries}
    catalog_bytes = (json.dumps(catalog, ensure_ascii=False, indent=2) + "\n").encode("utf-8")
    write_new_or_identical(PACKAGE / "assets/fonts/catalog.v1.json", catalog_bytes)
    print("Prepared pinned catalog; original app font and SDK are unchanged", flush=True)


if __name__ == "__main__":
    main()
