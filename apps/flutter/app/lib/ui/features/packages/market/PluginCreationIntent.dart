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
      taskLine: '请你使用 PackageBuilder skill 和 operit_editor 包，开发新的沙盒包。',
      packageRuleLine: '先确定新的沙盒包 id，后续不要改名。',
      devDirectoryLine:
          '开发目录固定为 手机下载/Operit/dev_package/你确定的id。开发、安装和测试都只在这里完成。',
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
          '请你使用 PackageBuilder skill 和 operit_editor 包，查找沙盒包 $runtimePackageId 的位置，在此版本基础上继续开发并测试。',
      packageRuleLine:
          '当前沙盒包 id 是 $runtimePackageId。包 id 和插件名字都必须沿用，不要改名，也不要新起包。',
      devDirectoryLine:
          '开发目录固定为 手机下载/Operit/dev_package/$runtimePackageId。开发、安装和测试都只在这里完成。',
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
          '请你使用 PackageBuilder skill 和 operit_editor 包，查找沙盒包 $runtimePackageId 的位置，在此版本基础上做合并开发并测试。',
      packageRuleLine:
          '当前沙盒包 id 是 $runtimePackageId。包 id 和插件名字都必须沿用，不要改名，也不要新起包。',
      devDirectoryLine:
          '开发目录固定为 手机下载/Operit/dev_package/$runtimePackageId。开发、安装和测试都只在这里完成。',
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
    '使用 PackageBuilder/types 中的当前版本类型定义。',
    '需要操作包、Skill、MCP、日志或模型时，读取 operit_editor 包说明后调用 execute_cli_command。',
    devDirectoryLine,
    packageRuleLine,
    '把 PackageBuilder/types 复制到 手机下载/Operit/dev_package/types，具体包目录通过 ../types 引用。',
    '用终端完成开发，编写 ts 和 js，编译出最终 js。tsconfig 参考 examples。',
    '为了方便二次开发，打包需要把 ts 部分和 tsconfig 打包进去。',
    '需求:',
    requirement.trim(),
  ].join('\n');
}
