from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import subprocess
import zipfile
from dataclasses import dataclass
from pathlib import Path

MANIFEST_FILENAMES = ("manifest.hjson", "manifest.json")
SYNCABLE_SUFFIXES = {".js", ".toolpkg"}


@dataclass(frozen=True)
class SyncPlanItem:
    mode: str
    source: Path
    destination_name: str


def _repo_root() -> Path:
    return Path(__file__).resolve().parents[2]


def _plugins_root() -> Path:
    return Path(__file__).resolve().parents[1]


def _find_manifest_file(folder: Path) -> Path | None:
    for file_name in MANIFEST_FILENAMES:
        manifest = folder / file_name
        if manifest.is_file():
            return manifest
    return None


def _collect_sync_plan(source_dir: Path) -> list[SyncPlanItem]:
    plans: list[SyncPlanItem] = []
    if not source_dir.is_dir():
        return plans

    for child in sorted(source_dir.iterdir(), key=lambda path: path.name.lower()):
        if child.name in {"types", "node_modules"}:
            continue
        if child.is_file() and child.suffix.lower() in SYNCABLE_SUFFIXES:
            plans.append(
                SyncPlanItem(
                    mode="copy",
                    source=child,
                    destination_name=child.name,
                )
            )
            continue
        if child.is_file() and child.suffix.lower() == ".ts" and not child.name.endswith(".d.ts"):
            plans.append(
                SyncPlanItem(
                    mode="compile-ts",
                    source=child,
                    destination_name=f"{child.stem}.js",
                )
            )
            continue
        if child.is_dir() and _find_manifest_file(child):
            plans.append(
                SyncPlanItem(
                    mode="pack",
                    source=child,
                    destination_name=f"{child.name}.toolpkg",
                )
            )
    return plans


def _run_checked_command(command: list[str], cwd: Path, *, dry_run: bool) -> None:
    command_text = subprocess.list2cmdline(command)
    if dry_run:
        print(f"DRY-RUN-CMD: (cd {cwd}) {command_text}")
        return
    print(f"RUN-CMD: (cd {cwd}) {command_text}")
    completed = subprocess.run(command, cwd=str(cwd))
    if completed.returncode != 0:
        raise RuntimeError(f"Command failed with exit code {completed.returncode}: {command_text}")


def _platform_command(executable: str) -> str:
    if os.name == "nt":
        return f"{executable}.cmd"
    return executable


def _iter_signature_files(paths: list[Path]) -> list[Path]:
    seen: set[Path] = set()
    files: list[Path] = []
    for path in paths:
        if not path.is_file() or path in seen:
            continue
        seen.add(path)
        files.append(path)
    files.sort(key=lambda path: path.as_posix().lower())
    return files


def _compute_paths_signature(base_dir: Path, paths: list[Path]) -> str:
    digest = hashlib.sha256()
    for file_path in _iter_signature_files(paths):
        digest.update(file_path.relative_to(base_dir).as_posix().encode("utf-8"))
        digest.update(b"\0")
        with file_path.open("rb") as handle:
            for chunk in iter(lambda: handle.read(1024 * 1024), b""):
                digest.update(chunk)
        digest.update(b"\0")
    return digest.hexdigest()


def _collect_prebuild_inputs(source_dir: Path, child_dir: Path) -> list[Path]:
    paths: list[Path] = []
    tsconfig = child_dir / "tsconfig.json"
    if tsconfig.is_file():
        paths.append(tsconfig)
    for file_path in child_dir.rglob("*"):
        if "node_modules" in file_path.parts:
            continue
        if file_path.is_file() and file_path.suffix.lower() in {".ts", ".d.ts"}:
            paths.append(file_path)
    types_dir = source_dir.parent / "types"
    if types_dir.is_dir():
        for file_path in types_dir.rglob("*"):
            if file_path.is_file() and file_path.suffix.lower() in {".ts", ".d.ts"}:
                paths.append(file_path)
    package_json = child_dir / "package.json"
    if package_json.is_file():
        paths.append(package_json)
        build_script = child_dir / "build.js"
        if build_script.is_file():
            paths.append(build_script)
    return paths


def _collect_root_prebuild_inputs(source_dir: Path) -> list[Path]:
    paths: list[Path] = []
    tsconfig = source_dir / "tsconfig.json"
    if tsconfig.is_file():
        paths.append(tsconfig)
    for file_path in source_dir.iterdir():
        if file_path.is_file() and file_path.suffix.lower() in {".ts", ".d.ts"}:
            paths.append(file_path)
    types_dir = source_dir.parent / "types"
    if types_dir.is_dir():
        for file_path in types_dir.rglob("*"):
            if file_path.is_file() and file_path.suffix.lower() in {".ts", ".d.ts"}:
                paths.append(file_path)
    return paths


def _load_state(path: Path) -> dict[str, str]:
    if not path.is_file():
        return {}
    data = json.loads(path.read_text(encoding="utf-8"))
    if not isinstance(data, dict):
        raise ValueError(f"State file must contain a JSON object: {path}")
    return {str(key): str(value) for key, value in data.items()}


def _save_state(path: Path, state: dict[str, str]) -> None:
    path.write_text(json.dumps(state, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")


def _prebuild_plans(repo_root: Path, source_dir: Path, plans: list[SyncPlanItem], *, dry_run: bool) -> None:
    state_file = source_dir / ".sync_state.json"
    state = _load_state(state_file)
    changed = False

    if any(plan.mode == "compile-ts" for plan in plans):
        tsconfig = source_dir / "tsconfig.json"
        if not tsconfig.is_file():
            raise ValueError(f"Missing tsconfig.json for TypeScript plugins: {source_dir}")
        signature = _compute_paths_signature(repo_root, _collect_root_prebuild_inputs(source_dir))
        key = "prebuild:."
        if state.get(key) == signature:
            print(f"SKIP-PREBUILD: {source_dir}")
        else:
            _run_checked_command([_platform_command("tsc"), "-p", str(tsconfig)], repo_root, dry_run=dry_run)
            state[key] = signature
            changed = True

    child_dirs = sorted(
        {plan.source for plan in plans if plan.source.is_dir()},
        key=lambda path: path.name.lower(),
    )
    for child_dir in child_dirs:
        tsconfig = child_dir / "tsconfig.json"
        if not tsconfig.is_file():
            continue
        signature = _compute_paths_signature(repo_root, _collect_prebuild_inputs(source_dir, child_dir))
        key = f"prebuild:{child_dir.relative_to(source_dir).as_posix()}"
        if state.get(key) == signature:
            print(f"SKIP-PREBUILD: {child_dir}")
        else:
            _run_checked_command([_platform_command("tsc"), "-p", str(tsconfig)], repo_root, dry_run=dry_run)
            state[key] = signature
            changed = True

        package_json = child_dir / "package.json"
        if package_json.is_file():
            _run_checked_command(["pnpm", "build"], child_dir, dry_run=dry_run)

    if changed and not dry_run:
        _save_state(state_file, state)


def _iter_files_for_pack(repo_root: Path, folder: Path) -> list[Path]:
    folder_rel = folder.relative_to(repo_root).as_posix()
    completed = subprocess.run(
        ["git", "ls-files", "-z", "--cached", "--others", "--exclude-standard", "--", folder_rel],
        cwd=str(repo_root),
        capture_output=True,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(f"git ls-files failed for: {folder_rel}")

    files: list[Path] = []
    seen: set[Path] = set()
    for raw_path in completed.stdout.split(b"\x00"):
        if not raw_path:
            continue
        file_path = repo_root / Path(raw_path.decode("utf-8"))
        if file_path.is_file() and file_path not in seen:
            seen.add(file_path)
            files.append(file_path)
    files.sort(key=lambda path: path.relative_to(folder).as_posix())
    return files


def _pack_toolpkg_folder(repo_root: Path, source_folder: Path, destination_file: Path) -> None:
    if _find_manifest_file(source_folder) is None:
        raise ValueError(f"Missing manifest.hjson or manifest.json: {source_folder}")
    destination_file.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(destination_file, mode="w", compression=zipfile.ZIP_DEFLATED) as archive:
        for file_path in _iter_files_for_pack(repo_root, source_folder):
            archive.write(file_path, file_path.relative_to(source_folder).as_posix())


def _delete_unplanned_outputs(output_dir: Path, planned_names: set[str], *, dry_run: bool) -> int:
    if not output_dir.is_dir():
        return 0
    deleted = 0
    for file_path in sorted(output_dir.iterdir(), key=lambda path: path.name.lower()):
        if not file_path.is_file() or file_path.suffix.lower() not in SYNCABLE_SUFFIXES:
            continue
        if file_path.name in planned_names:
            continue
        print(f"{'DRY-DELETE' if dry_run else 'DELETE'}: {file_path}")
        if not dry_run:
            file_path.unlink()
        deleted += 1
    return deleted


def _sync(source_dir: Path, output_dir: Path, *, dry_run: bool) -> tuple[int, int, int]:
    repo_root = _repo_root()
    plans = _collect_sync_plan(source_dir)
    _prebuild_plans(repo_root, source_dir, plans, dry_run=dry_run)

    if not dry_run:
        output_dir.mkdir(parents=True, exist_ok=True)

    planned_names = {plan.destination_name for plan in plans}
    deleted = _delete_unplanned_outputs(output_dir, planned_names, dry_run=dry_run)
    copied = 0
    packed = 0
    for plan in plans:
        destination = output_dir / plan.destination_name
        if plan.mode == "copy":
            print(f"{'DRY-COPY' if dry_run else 'COPY'}: {plan.source} -> {destination}")
            if not dry_run:
                shutil.copy2(plan.source, destination)
            copied += 1
        elif plan.mode == "compile-ts":
            compiled = source_dir.parent / ".out" / source_dir.name / f"{plan.source.stem}.js"
            print(f"{'DRY-COPY' if dry_run else 'COPY'}: {compiled} -> {destination}")
            if not dry_run:
                if not compiled.is_file():
                    raise FileNotFoundError(f"Compiled JavaScript not found: {compiled}")
                shutil.copy2(compiled, destination)
            copied += 1
        else:
            print(f"{'DRY-PACK' if dry_run else 'PACK'}: {plan.source} -> {destination}")
            if not dry_run:
                _pack_toolpkg_folder(repo_root, plan.source, destination)
            packed += 1
    return copied, packed, deleted


def main() -> int:
    plugins_root = _plugins_root()
    repo_root = _repo_root()
    parser = argparse.ArgumentParser(description="Sync Operit2 plugin package sources.")
    parser.add_argument(
        "--source",
        choices=("buildin", "examples", "all"),
        default="all",
    )
    parser.add_argument("--dry-run", action="store_true")
    parser.add_argument(
        "--buildin-output",
        default=str(repo_root / "core" / "crates" / "operit-runtime" / "assets" / "plugins" / "buildin"),
    )
    parser.add_argument(
        "--examples-output",
        default=str(plugins_root / ".out" / "examples"),
    )
    args = parser.parse_args()

    total_copied = 0
    total_packed = 0
    total_deleted = 0
    jobs: list[tuple[Path, Path]] = []
    if args.source in {"buildin", "all"}:
        jobs.append((plugins_root / "buildin", Path(args.buildin_output)))
    if args.source in {"examples", "all"}:
        jobs.append((plugins_root / "examples", Path(args.examples_output)))

    for source_dir, output_dir in jobs:
        copied, packed, deleted = _sync(source_dir, output_dir, dry_run=args.dry_run)
        total_copied += copied
        total_packed += packed
        total_deleted += deleted

    print(
        "Done. "
        f"source={args.source}, copied={total_copied}, packed={total_packed}, "
        f"deleted={total_deleted}, dry_run={bool(args.dry_run)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
