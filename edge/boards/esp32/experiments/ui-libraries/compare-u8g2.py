"""Export every existing glyph and validate U8g2's actual font decoder."""
import json
from pathlib import Path
import tempfile

from common import BOARD, BUILD, HERE, compile_c, prepare, run, write_json


def main():
    library = prepare("u8g2")
    BUILD.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="u8g2-", dir=BUILD))
    compile_c(output / "font-export.exe", [HERE / "font-export.c"], [BOARD / "ui_port"])
    run([output / "font-export.exe", output / "text.bdf", output / "digits.bdf"])
    compile_c(output / "bdfconv.exe", sorted((library / "bdfconv").glob("*.c")),
              [library / "bdfconv"])
    for name, mapping in (("text", "32-65535"), ("digits", "48-57")):
        run([output / "bdfconv.exe", "-b", "0", "-f", "1", "-m", mapping,
             "-n", f"operit_font_{name}", "-o", output / f"{name}-u8g2.c", output / f"{name}.bdf"])
    flags = ["-flto", "-fwhole-program", "-ffunction-sections", "-fdata-sections",
             "-fno-asynchronous-unwind-tables", "-Wl,--gc-sections",
             "-DU8G2_WITHOUT_FONT_ROTATION", "-DU8G2_WITHOUT_INTERSECTION",
             "-DU8G2_WITHOUT_CLIP_WINDOW_SUPPORT", "-DU8G2_USE_LARGE_FONTS"]
    compile_c(output / "u8g2-font-check.exe",
              [HERE / "u8g2-font-check.c", library / "u8g2_font.c"],
              [output, library, BOARD / "ui_port"], flags)
    text = run([output / "u8g2-font-check.exe"])
    metrics = json.loads(text.strip().splitlines()[-1])
    write_json(output / "font-check.json", metrics)
    print(json.dumps({"output": str(output), **metrics}))
    # This experiment retains the entire baseline character set and advance widths.
    if metrics["glyphs"] != 4168 or metrics["mismatches"] != 0:
        raise RuntimeError("Glyph coverage or decoded pixels changed")


if __name__ == "__main__":
    main()
