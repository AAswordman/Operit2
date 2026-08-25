// ignore_for_file: file_names

sealed class PluginCreationIntent {
  const PluginCreationIntent({required this.requirement});

  final String requirement;

  String toPrompt();
}

class FreshPluginCreationIntent extends PluginCreationIntent {
  const FreshPluginCreationIntent({required super.requirement});

  @override
  String toPrompt() {
    return _buildCreationPrompt(
      taskLine:
          'PackageBuilder skill と operit_editor パッケージを使い、新しいサンドボックスパッケージを開発してください。',
      packageRuleLine: '最初に新しいサンドボックスパッケージ ID を決め、その後は名前を変更しないでください。',
      devDirectoryLine:
          '開発ディレクトリは Download/Operit/dev_package/決定したID に固定します。開発、インストール、テストはすべてここで完了してください。',
      requirement: requirement,
    );
  }
}

class ContinuePluginCreationIntent extends PluginCreationIntent {
  const ContinuePluginCreationIntent({
    required this.runtimePackageId,
    required super.requirement,
  });

  final String runtimePackageId;

  @override
  String toPrompt() {
    return _buildCreationPrompt(
      taskLine:
          'PackageBuilder skill と operit_editor パッケージを使い、サンドボックスパッケージ $runtimePackageId の場所を探し、このバージョンを基に開発とテストを続けてください。',
      packageRuleLine:
          '現在のサンドボックスパッケージ ID は $runtimePackageId です。パッケージ ID とプラグイン名は必ず引き継ぎ、改名や新規パッケージ作成はしないでください。',
      devDirectoryLine:
          '開発ディレクトリは Download/Operit/dev_package/$runtimePackageId に固定します。開発、インストール、テストはすべてここで完了してください。',
      requirement: requirement,
    );
  }
}

class MergePluginCreationIntent extends PluginCreationIntent {
  const MergePluginCreationIntent({
    required this.runtimePackageId,
    required super.requirement,
  });

  final String runtimePackageId;

  @override
  String toPrompt() {
    return _buildCreationPrompt(
      taskLine:
          'PackageBuilder skill と operit_editor パッケージを使い、サンドボックスパッケージ $runtimePackageId の場所を探し、このバージョンを基に統合開発とテストを行ってください。',
      packageRuleLine:
          '現在のサンドボックスパッケージ ID は $runtimePackageId です。パッケージ ID とプラグイン名は必ず引き継ぎ、改名や新規パッケージ作成はしないでください。',
      devDirectoryLine:
          '開発ディレクトリは Download/Operit/dev_package/$runtimePackageId に固定します。開発、インストール、テストはすべてここで完了してください。',
      requirement: requirement,
    );
  }
}

String _buildCreationPrompt({
  required String taskLine,
  required String packageRuleLine,
  required String devDirectoryLine,
  required String requirement,
}) {
  return <String>[
    taskLine,
    'PackageBuilder/types にある現在のバージョンの型定義を使用してください。',
    'パッケージ、Skill、MCP、ログ、モデルを操作する必要がある場合は、operit_editor パッケージの説明を読んでから execute_cli_command を呼び出してください。',
    devDirectoryLine,
    packageRuleLine,
    'PackageBuilder/types を Download/Operit/dev_package/types にコピーし、各パッケージのディレクトリからは ../types を参照してください。',
    'ターミナルで開発を行い、ts と js を記述して最終的な js をコンパイルしてください。tsconfig は examples を参考にしてください。',
    '再開発しやすいよう、パッケージには ts 部分と tsconfig を含めてください。',
    '要件:',
    requirement.trim(),
  ].join('\n');
}
