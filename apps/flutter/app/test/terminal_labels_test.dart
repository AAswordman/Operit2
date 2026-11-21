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
      'terminalBuiltinMacNative': 'macOS 本机 Bash 终端',
      'terminalBuiltinMacSandbox': 'macOS 沙箱 Bash 终端',
      'terminalBuiltinAndroidLinux': 'Android PRoot Linux Bash 终端',
      'terminalBuiltinAndroidSystem': 'Android 系统 Shell 终端',
      'terminalBuiltinLinux': 'Linux 本机 Bash 终端',
      'terminalBuiltinPosix': 'POSIX 本机 Bash 终端',
      'terminalBuiltinPrivileged': '系统特权 sh 终端',
      'terminalBuiltinPowerShellDownload': 'PowerShell 7 终端',
      'terminalBuiltinPowerShell': 'Windows 内置 PowerShell 7 终端',
      'terminalBuiltinWindowsBash': 'Windows 内置 Bash 终端',
      'terminalBuiltinArm64': 'Linux ARM64 模拟终端',
      'terminalBuiltinIosSystem': 'iOS 系统 sh 终端',
      'terminalBuiltinOhosLinux': 'OpenHarmony QEMU Linux 模拟终端',
      'terminalBuiltinOhosSystem': 'OpenHarmony 系统 sh 终端',
      'terminalBuiltinBrowser': '浏览器 Linux 虚拟机终端',
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
