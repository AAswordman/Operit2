"""Compare the original UI and pinned uGUI primitive adapter on a desktop host."""
import json
from pathlib import Path
import shutil
import statistics
import tempfile

from common import BOARD, BUILD, HERE, ROOT, compile_c, prepare, run, write_json


def main():
    library = prepare("ugui")
    BUILD.mkdir(parents=True, exist_ok=True)
    output = Path(tempfile.mkdtemp(prefix="ugui-", dir=BUILD))
    source = output / "edge/boards/esp32/ui_port"
    source.mkdir(parents=True)
    shutil.copyfile(BOARD / "ui_port/operit_mini_ui.c", source / "operit_mini_ui.c")
    # Apply only to an isolated source copy. Production sources stay untouched.
    run(["git", "apply", f"--directory={output.relative_to(ROOT).as_posix()}",
         HERE / "ugui-ui.patch"], cwd=ROOT)
    variants = {
        "baseline": [BOARD / "ui_port/operit_mini_ui.c"],
        "ugui": [source / "operit_mini_ui.c", library / "ugui.c"],
    }
    for name, sources in variants.items():
        (output / name).mkdir()
        compile_c(output / f"{name}.exe", [*sources, HERE / "primitive-harness.c"],
                  [BOARD / "ui_port", library], ["-D_Static_assert(x,msg)="])
    timings = {name: [] for name in variants}
    for iteration in range(7):
        # Alternate ordering to reduce warmup/order bias.
        for name in (list(variants) if iteration % 2 == 0 else list(reversed(variants))):
            metrics = json.loads(run([output / f"{name}.exe", output / name]))
            if metrics["benchmarkFlushes"] != 12000 or metrics["drawBytes"] != 1280:
                raise RuntimeError(f"Unexpected rendering workload: {metrics}")
            timings[name].append(metrics["cpuSeconds"])
    comparisons = []
    frames = sorted((output / "baseline").glob("*.rgb565"))
    if len(frames) != 12:
        raise RuntimeError(f"Expected 12 frames, got {len(frames)}")
    for original in frames:
        actual = (output / "ugui" / original.name).read_bytes()
        expected = original.read_bytes()
        if len(expected) != 320 * 240 * 2 or len(actual) != len(expected):
            raise RuntimeError("Invalid full-frame size")
        difference = sum(expected[i:i+2] != actual[i:i+2] for i in range(0, len(actual), 2))
        states = []
        for name in variants:
            state = json.loads((output / name / f"{original.stem}.json").read_text(encoding="utf-8"))
            state.pop("staticBytes", None)
            states.append(state)
        comparisons.append({"scenario": original.stem, "pixelDifferences": difference,
                            "stateEqual": states[0] == states[1]})
    write_json(output / "pixel-comparison.json", comparisons)
    write_json(output / "host-benchmark.json", timings)
    print(json.dumps({"output": str(output), "scenarios": len(comparisons),
                      "desktopMedianSeconds": {k: statistics.median(v) for k, v in timings.items()}}))
    if any(item["pixelDifferences"] or not item["stateEqual"] for item in comparisons):
        raise RuntimeError("Pixel or interaction state mismatch; inspect output")


if __name__ == "__main__":
    main()
