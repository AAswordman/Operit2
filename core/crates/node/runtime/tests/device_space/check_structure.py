"""Check test registration and Rust syntax without compiling or running behavior tests."""
from pathlib import Path
import json
import re
import subprocess


# Validate the explicit inventory, include wiring, persistence paths and parser acceptance.
def main():
    """Fail on disconnected test modules, inventory drift or invalid Rust syntax."""
    root = Path(__file__).resolve().parent
    crate = root.parent.parent
    inventory = json.loads((root / "coverage.json").read_text(encoding="utf-8"))
    pattern = re.compile(r"#\[(?:tokio::)?test(?:\([^\]]*\))?\]\s*(?:async\s+)?fn\s+(\w+)")
    declared = {suite["file"]: suite["cases"] for suite in inventory["suites"]}
    actual = {}
    source = {}
    for path in sorted(root.glob("*.rs")):
        text = path.read_text(encoding="utf-8")
        source[path.name] = text
        cases = pattern.findall(text)
        if cases:
            actual[path.name] = cases
        subprocess.run(["rustfmt", "--edition", "2021", "--emit", "stdout", str(path)],
                       check=True, stdout=subprocess.DEVNULL)
    assert actual == declared, "coverage.json must enumerate the exact current test cases"
    names = [case for cases in actual.values() for case in cases]
    assert len(names) == len(set(names)), "duplicate test names obscure coverage"
    owners = {
        "mod.rs": crate / "src/CoreNodeRouter.rs",
        "join_state_machine.rs": crate / "src/peer/space_join.rs",
        "facade_state.rs": crate / "src/RuntimeRemoteLinkService.rs",
    }
    for file in source:
        owner = owners[file] if file in owners else root / "mod.rs"
        assert f"/tests/device_space/{file}" in owner.read_text(encoding="utf-8"), f"unwired module: {file}"
    assert not (crate / "src/router_space_join_tests.rs").exists(), "legacy test file must not duplicate the new suite"
    protocol = (crate / "src/peer/space_join.rs").read_text(encoding="utf-8")
    layout = (crate.parent.parent / "foundation/util/src/RuntimeStorageLayout.rs").read_text(encoding="utf-8")
    for runtime_name, fixture_name in [("INBOUND", "INBOUND_RECORDS"), ("OUTBOUND", "OUTBOUND_RECORDS"),
                                        ("INBOX", "REVIEW_RECORDS"), ("RESULTS", "RESULT_RECORDS")]:
        symbol = re.search(rf'const {runtime_name}: &str = (\w+)_PATH;', protocol)[1]
        runtime_path = re.search(rf'pub const {symbol}: RuntimeStoragePathDefinition =\s*RuntimeStoragePathDefinition::exact\("([^"]+)"', layout)[1]
        fixture_path = re.search(rf'const {fixture_name}: &str = "([^"]+)"', source["fixtures.rs"])[1]
        assert runtime_path == fixture_path, f"fixture path drift: {runtime_name}"
    for file, text in source.items():
        assert not re.search(r"#\[ignore(?:\(|\])", text), f"ignored contract: {file}"
    print(f"Structure and Rust parsing OK: {len(source)} files, {len(names)} registered tests.")
    print("This check does NOT compile, execute or prove the behavior of these tests.")


if __name__ == "__main__":
    main()
