from __future__ import annotations

import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent
RELEASE_SCRIPT = SCRIPT_DIR / "release.py"


@dataclass(frozen=True)
class Choice:
    key: str
    title: str
    args: tuple[str, ...]


def choose(title: str, choices: list[Choice]) -> Choice:
    print()
    print(title)
    for index, choice in enumerate(choices, start=1):
        print(f"  {index}. {choice.title}")

    while True:
        raw = input("选择序号: ").strip()
        if raw.isdecimal():
            index = int(raw)
            if 1 <= index <= len(choices):
                return choices[index - 1]
        print("输入无效，请重新选择。")


def build_command() -> list[str]:
    target = choose(
        "这次要处理什么？",
        [
            Choice("cli", "CLI/TUI", ("--scope", "cli")),
            Choice("app", "App", ("--scope", "app")),
            Choice("full", "全量：App + CLI/TUI", ("--scope", "full")),
            Choice("check", "只检查版本和脚本入口", ("--scope", "none", "--build-only", "--no-wsl")),
        ],
    )

    command = [sys.executable, str(RELEASE_SCRIPT), *target.args]

    if target.key != "check":
        mode = choose(
            "执行方式？",
            [
                Choice("build", "只构建检查", ("--build-only",)),
                Choice("publish", "发布到 GitHub Release", ()),
                Choice("draft", "发布到 GitHub Draft", ("--draft",)),
            ],
        )
        command.extend(mode.args)

        linux = choose(
            "Linux WSL 构建？",
            [
                Choice("no_wsl", "关闭 WSL Linux 构建", ("--no-wsl",)),
                Choice("wsl", "启用 WSL Linux 构建", ()),
            ],
        )
        command.extend(linux.args)

    return command


def main() -> int:
    if not RELEASE_SCRIPT.is_file():
        print(f"发布脚本不存在: {RELEASE_SCRIPT}", file=sys.stderr)
        return 1

    command = build_command()

    print()
    print("即将执行:")
    print(" ".join(f'"{part}"' if " " in part else part for part in command))

    confirm = choose(
        "确认执行？",
        [
            Choice("run", "执行", ()),
            Choice("cancel", "取消", ()),
        ],
    )
    if confirm.key == "cancel":
        print("已取消。")
        return 0

    completed = subprocess.run(command, cwd=REPO_ROOT)
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
