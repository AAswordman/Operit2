#!/usr/bin/env python3
"""Verify the actual waifu dependency using the installed CLI's current identity."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import subprocess


# Runs one CLI operation without creating or replacing the user's persistent identity.
def command(binary: Path, *args: str) -> dict:
    done = subprocess.run([str(binary), 'cli', *args, '--json'], capture_output=True, timeout=150)
    if done.returncode != 0:
        raise RuntimeError(done.stdout.decode('utf-8') + done.stderr.decode('utf-8'))
    return json.loads(done.stdout.decode('utf-8'))


# Unwraps a real tool result while preserving the original execution error.
def tool(binary: Path, name: str, params: dict) -> dict:
    result = command(binary, 'tool', 'exec', name, json.dumps(params, ensure_ascii=False))
    if not result['success']:
        raise RuntimeError(result['error'])
    return json.loads(result['result']['value'])


# Exercises dependency loading, real model output, structured segmentation and native resource extraction.
def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=Path(os.environ['LOCALAPPDATA']) / 'Programs/Operit2/bin/operit2.exe')
    parser.add_argument('--chat', required=True, help='Explicit existing conversation used to persist the preview turn')
    parser.add_argument('--profile', default='default')
    args = parser.parse_args()
    for package in ['com.operit.waifu', 'com.operit.qqbot_bundle']:
        info = command(args.binary, 'plugin', 'show', package)
        assert info['enabled'], info
    settings = tool(args.binary, 'waifu:waifu_settings_get', {'profile': args.profile})
    assert settings['enableEmoticons'], 'Enable emoticons in the selected profile before testing image segments'
    result = tool(args.binary, 'qqbot:qqbot_waifu_preview', {
        'chat_id': args.chat,
        'profile': args.profile,
        'message': '请只回复三句话，每句话以句号结尾，分别说你已上线、你支持逐句回复、今天见到我很开心。第三句话后添加 <emotion>happy</emotion>。不要调用工具。',
    })
    text = [item for item in result['segments'] if item['segment']['kind'] == 'text']
    images = [item for item in result['segments'] if item['segment']['kind'] == 'image']
    assert len(text) >= 3, result
    assert any(item['segment']['emotion'] == 'happy' for item in images), result
    assert text[0]['elapsedMs'] < result['completedAfterMs'], result
    print(json.dumps(result, ensure_ascii=True, indent=2))
    print('PASS: installed CLI, declared waifu dependency, real model, incremental text and image resource')


if __name__ == '__main__':
    main()
