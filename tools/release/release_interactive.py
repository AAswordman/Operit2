from __future__ import annotations

import subprocess
import sys
from dataclasses import dataclass
from enum import Enum
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent
RELEASE_SCRIPT = SCRIPT_DIR / "build_release.py"


class ValueEnum(str, Enum):
    # Returns the raw enum value for command-line interpolation.
    def __str__(self):
        return self.value


class ChoiceKey(ValueEnum):
    CLI = "cli"
    APP = "app"
    FULL = "full"
    CHECK = "check"
    HOST = "host"
    ALL = "all"
    WSL = "wsl"
    NO_WSL = "no_wsl"
    RUN = "run"
    CANCEL = "cancel"


@dataclass(frozen=True)
class Choice:
    key: ChoiceKey
    title: str
    args: tuple[str, ...]


# Prompts the user to select one interactive choice.
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


# Builds the release command selected by the interactive prompts.
def build_command() -> list[str]:
    target = choose(
        "这次要处理什么？",
        [
            Choice(ChoiceKey.CLI, "CLI/TUI", ("--scope", "cli")),
            Choice(ChoiceKey.APP, "App（Android + OpenHarmony + 当前桌面平台）", ("--scope", "app")),
            Choice(ChoiceKey.FULL, "全量：App（Android + OpenHarmony + 当前桌面平台）+ CLI/TUI", ("--scope", "full")),
            Choice(ChoiceKey.CHECK, "只检查版本和脚本入口", ("--scope", "none", "--no-wsl")),
        ],
    )

    command = [sys.executable, str(RELEASE_SCRIPT), *target.args]

    if target.key in (ChoiceKey.CLI, ChoiceKey.FULL):
        arches = choose(
            "CLI 构建架构？",
            [
                Choice(ChoiceKey.HOST, "当前主机架构 (x86_64/aarch64)", ("--cli-arches", "host")),
                Choice(ChoiceKey.ALL, "全量桌面架构 (x86_64 + aarch64 for Windows/Linux/macOS)", ("--cli-arches", "all")),
            ],
        )
        command.extend(arches.args)

    if target.key != ChoiceKey.CHECK:
        linux = choose(
            "Linux WSL 构建？",
            [
                Choice(ChoiceKey.WSL, "启用 WSL Linux 构建", ()),
                Choice(ChoiceKey.NO_WSL, "关闭 WSL Linux 构建", ("--no-wsl",)),
            ],
        )
        command.extend(linux.args)

    return command


# Runs the selected release command after user confirmation.
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
            Choice(ChoiceKey.RUN, "执行", ()),
            Choice(ChoiceKey.CANCEL, "取消", ()),
        ],
    )
    if confirm.key == ChoiceKey.CANCEL:
        print("已取消。")
        return 0

    completed = subprocess.run(command, cwd=REPO_ROOT)
    return completed.returncode


if __name__ == "__main__":
    raise SystemExit(main())
