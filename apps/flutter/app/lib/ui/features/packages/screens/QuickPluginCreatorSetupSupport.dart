// ignore_for_file: file_names

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';

class QuickPluginCreatorSetupResult {
  const QuickPluginCreatorSetupResult({
    required this.success,
    required this.skillName,
    required this.packageResult,
    required this.error,
  });

  final bool success;
  final String skillName;
  final String packageResult;
  final String? error;
}

Future<QuickPluginCreatorSetupResult> runQuickPluginCreatorSetup(
  GeneratedCoreProxyClients clients,
) async {
  try {
    final skill = await clients.skillRepository
        .ensureQuickPluginCreatorSkillVisible();
    final packageResult = await clients.permissionsPackToolPackageManager
        .enablePackage(packageName: 'operit_editor');
    return QuickPluginCreatorSetupResult(
      success: true,
      skillName: skill.name,
      packageResult: packageResult,
      error: null,
    );
  } catch (error) {
    return QuickPluginCreatorSetupResult(
      success: false,
      skillName: '',
      packageResult: '',
      error: error.toString(),
    );
  }
}
