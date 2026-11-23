// ignore_for_file: file_names

import '../../core/proxy/generated/CoreProxyModels.g.dart';
import '../../l10n/generated/app_localizations.dart';

/// Localizes host-owned terminal labels without rewriting persisted user names.
abstract final class TerminalLabels {
  static String profileName(AppLocalizations l10n, TerminalProfile profile) =>
      _name(l10n, profile.nameKey, profile.name, profile.terminalType);

  static String runtimeName(
    AppLocalizations l10n,
    RuntimeTerminalTypeInfo terminal,
  ) => _name(
    l10n,
    terminal.nameKey,
    terminal.description,
    terminal.terminalType,
  );

  /// Protocol identifiers remain stable; only their visible captions change.
  static String implementation(
    AppLocalizations l10n,
    String terminal,
    String terminalType,
  ) {
    final label = switch (terminal) {
      'native' => l10n.terminalImplementationNative,
      'proot' => l10n.terminalImplementationProot,
      'chroot' => l10n.terminalImplementationChroot,
      'termux' => l10n.terminalImplementationTermux,
      'android-system' => l10n.terminalImplementationAndroidSystem,
      'qemu-vroot' => l10n.terminalImplementationQemu,
      'arm64' => l10n.terminalImplementationArm64,
      'shell' => l10n.terminalImplementationSystem,
      'v86' => l10n.terminalImplementationBrowser,
      _ => terminal,
    };
    return '$label / ${interpreter(l10n, terminalType)}';
  }

  static String interpreter(AppLocalizations l10n, String terminalType) =>
      switch (terminalType) {
        'bash' => 'Bash',
        'zsh' => 'Zsh',
        'powershell' => 'PowerShell',
        'shell' => l10n.terminalInterpreterShell,
        _ => terminalType,
      };

  /// Only stable protocol keys select translations. Unknown keys and custom
  /// names use Core's fallback text, so newer hosts work with older clients.
  static String _name(
    AppLocalizations l10n,
    String? key,
    String fallback,
    String terminalType,
  ) => switch (key) {
    'terminalBuiltinMacNative' => l10n.terminalBuiltinMacNative(
      interpreter(l10n, terminalType),
    ),
    'terminalBuiltinMacSandbox' => l10n.terminalBuiltinMacSandbox(
      interpreter(l10n, terminalType),
    ),
    'terminalBuiltinAndroidLinux' => l10n.terminalBuiltinAndroidLinux,
    'terminalBuiltinAndroidSystem' => l10n.terminalBuiltinAndroidSystem,
    'terminalBuiltinLinux' => l10n.terminalBuiltinLinux,
    'terminalBuiltinPosix' => l10n.terminalBuiltinPosix,
    'terminalBuiltinPrivileged' => l10n.terminalBuiltinPrivileged,
    'terminalBuiltinPowerShellDownload' =>
      l10n.terminalBuiltinPowerShellDownload,
    'terminalBuiltinPowerShell' => l10n.terminalBuiltinPowerShell,
    'terminalBuiltinWindowsBash' => l10n.terminalBuiltinWindowsBash,
    'terminalBuiltinArm64' => l10n.terminalBuiltinArm64,
    'terminalBuiltinIosSystem' => l10n.terminalBuiltinIosSystem,
    'terminalBuiltinOhosLinux' => l10n.terminalBuiltinOhosLinux,
    'terminalBuiltinOhosSystem' => l10n.terminalBuiltinOhosSystem,
    'terminalBuiltinBrowser' => l10n.terminalBuiltinBrowser,
    _ => fallback,
  };
}
