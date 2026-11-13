import '../../apps/flutter/app/lib/ui/features/packages/market/PluginCreationIntent.dart';

/// Rejects a contract violation without depending on enabled runtime assertions.
void require(bool condition, String message) {
  if (!condition) {
    throw StateError(message);
  }
}

/// Verifies all draft variants preserve requirements and use the same host workflow.
void main() {
  const packageId = 'com.operit.contract_test';
  const requirement = '  实现一个按需运行的工具  ';
  const intents = <PluginCreationIntent>[
    FreshPluginCreationIntent(requirement: requirement),
    ContinuePluginCreationIntent(
      runtimePackageId: packageId,
      requirement: requirement,
    ),
    MergePluginCreationIntent(
      runtimePackageId: packageId,
      requirement: requirement,
    ),
  ];
  final drafts = intents.map((intent) => intent.toPrompt()).toList();
  final expectedShared = <String>{
    'Operit2 新建 ToolPkg 显式声明 api_version 为 2.0.0；其对 1.0.0 和 1.0.1 的加载支持并不完整。Operit1 完整支持 1.0.0 和 1.0.1，这两个版本主要面向 Android，是旧版 API 形式。',
    '2.0.0 作者须注重跨平台兼容性。基本所有公共接口都兼容多平台，常规功能使用统一接口即可；遇到平台特异接口时，明确适用范围，并考虑、验证其他平台的安装、界面和核心功能。',
    '先阅读 PackageBuilder/references/PLUGIN_CREATION_WORKFLOW.md，使用随 Skill 携带的当前版本类型；按需通过 VFS 自行查阅当前已安装插件。',
    '需要操作包、Skill、MCP、日志或模型时，调用 operit_editor 包的 operit_editor 工具，传入 args 字符串数组实际执行命令；args: [] 获取命令帮助，不使用自然语言 query。',
    '出现错误停止并定位。交付源码路径、成品路径、安装状态与已验证结果；未测试或未发布的部分明确说明。',
  };
  for (final draft in drafts) {
    final lines = draft.split('\n');
    require(
      lines.last == requirement.trim(),
      'The requirement must be trimmed.',
    );
    require(
      lines[lines.length - 2] == '需求:',
      'The requirement marker is missing.',
    );
    require(
      expectedShared.difference(lines.toSet()).isEmpty,
      'The shared host-aware workflow instructions are missing.',
    );
    for (final path in <String>[
      '/operitapp/data/extensions/device/packages/',
      '/operitapp/data/extensions/space/packages/',
      '/operitapp/data/cache/toolpkg/',
    ]) {
      require(draft.contains(path), 'Plugin discovery VFS path missing: $path');
    }
    require(
      !RegExp(r'/sdcard|手机下载|开发目录固定为|debug_run_sandbox_script').hasMatch(draft),
      'The draft still assumes an obsolete path or execution tool.',
    );
  }
  const identityRule = '当前沙盒包 id 是 $packageId。包 id 和插件名字都必须沿用，不要改名，也不要新起包。';
  require(
    drafts[0].split('\n').contains('先确定新的沙盒包 id，后续不要改名。'),
    'Fresh draft must request a stable identity.',
  );
  for (final draft in drafts.skip(1)) {
    require(
      draft.split('\n').contains(identityRule),
      'Existing plugin identity must be preserved.',
    );
    require(
      RegExp(r'原始项目并核验源码与构建配置').hasMatch(draft),
      'Existing projects must be verified before editing.',
    );
  }
  print(
    'Plugin creation draft contracts passed for all three intent variants.',
  );
}
