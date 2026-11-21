import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/l10n/generated/app_localizations_en.dart';
import 'package:operit2/l10n/generated/app_localizations_zh.dart';
import 'package:operit2/ui/common/TerminalLabels.dart';

void main() {
  final zh = AppLocalizationsZh();
  final en = AppLocalizationsEn();

  test('all host keys translate independently of Core description text', () {
    const labels = {
      'terminalBuiltinMacNative': 'macOS 本机 Bash 终端（支持作业控制）',
      'terminalBuiltinMacSandbox': 'macOS 沙箱 Bash 终端（无控制终端）',
      'terminalBuiltinAndroidLinux': 'Android Linux Bash 终端（PRoot 环境）',
      'terminalBuiltinAndroidSystem': 'Android 系统命令行终端',
      'terminalBuiltinLinux': 'Linux Bash 终端',
      'terminalBuiltinPosix': 'POSIX Bash 终端',
      'terminalBuiltinPrivileged': '系统 /bin/sh 终端（需要特权权限）',
      'terminalBuiltinPowerShellDownload': 'PowerShell 7 终端（首次使用时下载）',
      'terminalBuiltinPowerShell': '内置 PowerShell 7 终端',
      'terminalBuiltinWindowsBash': 'Windows 内置 Bash 终端',
      'terminalBuiltinArm64': 'ARM64 Linux 终端（使用已配置的根文件系统）',
      'terminalBuiltinIosSystem': 'iOS 系统 /bin/sh 终端（需要越狱或其他特权权限）',
      'terminalBuiltinOhosLinux': 'OpenHarmony Alpine Linux 终端（QEMU 虚拟环境）',
      'terminalBuiltinOhosSystem': 'OpenHarmony 系统 /bin/sh 终端',
      'terminalBuiltinBrowser':
          '浏览器本地 Buildroot Linux 虚拟机（含 BusyBox、Node 和 Python）',
    };
    for (final entry in labels.entries) {
      final profile = TerminalProfile(
        id: 'any-host-id',
        name: 'Completely rewritten Core description',
        nameKey: entry.key,
        terminal: 'native',
        terminalType: 'bash',
        options: const {},
      );
      final runtime = RuntimeTerminalTypeInfo(
        terminal: 'profile:any-host-id',
        terminalType: 'bash',
        available: true,
        description: 'Another version of the description',
        nameKey: entry.key,
      );
      expect(TerminalLabels.profileName(zh, profile), entry.value);
      expect(TerminalLabels.runtimeName(zh, runtime), entry.value);
      expect(TerminalLabels.profileName(en, profile), isNot(entry.value));
    }
  });

  test('custom names and unknown keys use literal fallback text', () {
    for (final key in [null, 'newer.host.label']) {
      final profile = TerminalProfile(
        id: 'builtin_0',
        name: 'macOS native bash login PTY with job control',
        nameKey: key,
        terminal: 'native',
        terminalType: 'bash',
        options: const {},
      );
      expect(TerminalLabels.profileName(zh, profile), profile.name);
      expect(TerminalLabels.profileName(en, profile), profile.name);
    }
  });

  test(
    'implementation labels retain interpreter identities and unknown hosts',
    () {
      expect(TerminalLabels.implementation(zh, 'native', 'zsh'), '本机终端 / Zsh');
      expect(
        TerminalLabels.implementation(en, 'native', 'zsh'),
        'Native terminal / Zsh',
      );
      expect(
        TerminalLabels.implementation(zh, 'custom', 'fish'),
        'custom / fish',
      );
    },
  );
}
