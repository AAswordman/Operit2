// ignore_for_file: file_names

sealed class PluginCreationIntent {
  /// Stores the requested plugin development requirement.
  const PluginCreationIntent({required this.requirement});

  final String requirement;

  /// Builds the chat draft for the requested plugin development task.
  String toPrompt();
}

class FreshPluginCreationIntent extends PluginCreationIntent {
  /// Creates a request to develop a new plugin with a stable identity.
  const FreshPluginCreationIntent({required super.requirement});

  /// Builds a creation draft using the current bundled development workflow.
  @override
  String toPrompt() {
    return _buildCreationPrompt(
      taskLine: '请你使用 Operit 插件开发套件（PackageBuilder）Skill 和 operit_editor 包，开发新的沙盒包。',
      packageRuleLine: '先确定新的沙盒包 id，后续不要改名。',
      requirement: requirement,
    );
  }
}

class ContinuePluginCreationIntent extends PluginCreationIntent {
  /// Creates a request to continue development of an existing plugin.
  const ContinuePluginCreationIntent({
    required this.runtimePackageId,
    required super.requirement,
  });

  final String runtimePackageId;

  /// Builds a creation draft using the current bundled development workflow.
  @override
  String toPrompt() {
    return _buildCreationPrompt(
      taskLine:
          '请你使用 Operit 插件开发套件（PackageBuilder）Skill 和 operit_editor 包，查找沙盒包 $runtimePackageId 的原始项目并核验源码与构建配置，在此版本基础上继续开发并测试。缺失源码时明确说明。',
      packageRuleLine:
          '当前沙盒包 id 是 $runtimePackageId。包 id 和插件名字都必须沿用，不要改名，也不要新起包。',
      requirement: requirement,
    );
  }
}

class MergePluginCreationIntent extends PluginCreationIntent {
  /// Creates a request to merge changes into an existing plugin project.
  const MergePluginCreationIntent({
    required this.runtimePackageId,
    required super.requirement,
  });

  final String runtimePackageId;

  /// Builds a creation draft using the current bundled development workflow.
  @override
  String toPrompt() {
    return _buildCreationPrompt(
      taskLine:
          '请你使用 Operit 插件开发套件（PackageBuilder）Skill 和 operit_editor 包，查找沙盒包 $runtimePackageId 的原始项目并核验源码与构建配置，在此版本基础上做合并开发并测试。缺失源码时明确说明。',
      packageRuleLine:
          '当前沙盒包 id 是 $runtimePackageId。包 id 和插件名字都必须沿用，不要改名，也不要新起包。',
      requirement: requirement,
    );
  }
}

/// Builds a host-aware development prompt without assuming platform paths.
String _buildCreationPrompt({
  required String taskLine,
  required String packageRuleLine,
  required String requirement,
}) {
  return <String>[
    taskLine,
    '先阅读 PackageBuilder/references/PLUGIN_CREATION_WORKFLOW.md，使用随 Skill 携带的当前版本类型；按需通过 VFS 自行查阅当前已安装插件。',
    '需要操作包、Skill、MCP、日志或模型时，调用 operit_editor 包的 operit_editor 工具，传入 args 字符串数组实际执行命令；args: [] 获取命令帮助，不使用自然语言 query。',
    '通过 ["skill", "show", "PackageBuilder"] 读取真实 Skill 目录，通过终端 host 信息和实际文件访问确定用户同意使用的开发根目录，不写死平台路径。',
    '参考插件时，自行列举 VFS 路径 /operitapp/data/extensions/device/packages/ 和 /operitapp/data/extensions/space/packages/，按需查看 /operitapp/data/cache/toolpkg/ 已有缓存；缓存须与安装版本核验，不批量解压 toolpkg。',
    packageRuleLine,
    'Operit2 新建 ToolPkg 显式声明 api_version 为 2.0.0；其对 1.0.0 和 1.0.1 的加载支持并不完整。Operit1 完整支持 1.0.0 和 1.0.1，这两个版本主要面向 Android，是旧版 API 形式。',
    '2.0.0 作者须注重跨平台兼容性。基本所有公共接口都兼容多平台，常规功能使用统一接口即可；遇到平台特异接口时，明确适用范围，并考虑、验证其他平台的安装、界面和核心功能。',
    '在开发根目录下以包 id 建立源码目录，将 Skill 的 types 完整复制到同级 types；包项目通过 ../types 引用。核对 VFS、终端与导入使用的实际路径。',
    '按实际可用终端和编译器开发 TypeScript 并输出 CommonJS JavaScript；确认构建结果，使用统一 host API，不写平台分支。',
    '保留并打包 TypeScript 源码、tsconfig 和最终 JavaScript。成品放在源码目录之外，ToolPkg 的 manifest 位于归档根目录。',
    '安装测试使用 package import/enable/list/show/exec 和 log package，核验真实启用状态；同 ID 更新按流程取得删除授权后重新导入。operit_editor 返回实际命令结果与错误，不把命令已调用当作操作成功。',
    '出现错误停止并定位。交付源码路径、成品路径、安装状态与已验证结果；未测试或未发布的部分明确说明。',
    '需求:',
    requirement.trim(),
  ].join('\n');
}
