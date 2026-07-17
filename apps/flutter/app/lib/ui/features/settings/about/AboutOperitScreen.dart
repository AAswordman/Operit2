// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../../common/OperitLogoMark.dart';

const String _appVersion = '2.0.0+5';
final Uri _projectUri = Uri.parse('https://github.com/AAswordman/Operit2');
final Uri _documentationUri = Uri.parse(
  'https://github.com/AAswordman/Operit2#readme',
);
final Uri _contactUri = Uri.parse('mailto:aaswordsman@foxmail.com');

class AboutOperitScreen extends StatelessWidget {
  const AboutOperitScreen({super.key});

  /// Builds the Operit2 project information screen.
  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        return SingleChildScrollView(
          padding: const EdgeInsets.all(20),
          child: Center(
            child: ConstrainedBox(
              constraints: BoxConstraints(
                maxWidth: 720,
                minHeight: constraints.maxHeight - 40,
              ),
              child: const Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  _AboutHeader(),
                  SizedBox(height: 24),
                  _AboutSection(
                    title: 'Operit2',
                    child: Text(
                      '面向终端与桌面的 AI 工作台，提供聊天会话、工作区、工具、插件、MCP、远程连接和 Web 访问。',
                    ),
                  ),
                  SizedBox(height: 12),
                  _ProjectLinksSection(),
                  SizedBox(height: 12),
                  _SupportSection(),
                  SizedBox(height: 24),
                  _CopyrightNotice(),
                ],
              ),
            ),
          ),
        );
      },
    );
  }
}

class _AboutHeader extends StatelessWidget {
  const _AboutHeader();

  /// Builds the application identity header.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Column(
      children: <Widget>[
        DecoratedBox(
          decoration: BoxDecoration(
            color: colorScheme.primaryContainer,
            shape: BoxShape.circle,
          ),
          child: Padding(
            padding: const EdgeInsets.all(20),
            child: OperitLogoMark(size: 76, color: colorScheme.primary),
          ),
        ),
        const SizedBox(height: 14),
        Text(
          'Operit2',
          style: Theme.of(
            context,
          ).textTheme.headlineSmall?.copyWith(fontWeight: FontWeight.w800),
        ),
        const SizedBox(height: 4),
        Text(
          '版本 $_appVersion',
          style: Theme.of(
            context,
          ).textTheme.bodyMedium?.copyWith(color: colorScheme.onSurfaceVariant),
        ),
      ],
    );
  }
}

class _ProjectLinksSection extends StatelessWidget {
  const _ProjectLinksSection();

  /// Builds the project and license actions.
  @override
  Widget build(BuildContext context) {
    return _AboutSection(
      title: '项目',
      child: Column(
        children: <Widget>[
          _AboutActionRow(
            icon: Icons.code_outlined,
            title: '项目源码',
            subtitle: 'github.com/AAswordman/Operit2',
            onTap: () => _launchExternalUri(_projectUri),
          ),
          const Divider(height: 1),
          _AboutActionRow(
            icon: Icons.menu_book_outlined,
            title: '使用文档',
            subtitle: 'README 与命令行说明',
            onTap: () => _launchExternalUri(_documentationUri),
          ),
          const Divider(height: 1),
          _AboutActionRow(
            icon: Icons.description_outlined,
            title: '开源许可证',
            subtitle: 'Operit2 使用 AGPL-3.0',
            onTap: () => _showOpenSourceLicenses(context),
          ),
        ],
      ),
    );
  }
}

class _SupportSection extends StatelessWidget {
  const _SupportSection();

  /// Builds the maintainer contact action.
  @override
  Widget build(BuildContext context) {
    return _AboutSection(
      title: '联系',
      child: _AboutActionRow(
        icon: Icons.alternate_email_outlined,
        title: '开发者 AAswordman',
        subtitle: 'aaswordsman@foxmail.com',
        onTap: () => _launchExternalUri(_contactUri),
      ),
    );
  }
}

class _CopyrightNotice extends StatelessWidget {
  const _CopyrightNotice();

  /// Builds the project copyright notice.
  @override
  Widget build(BuildContext context) {
    return Text(
      '© 2025 - 2026 Operit. 保留所有权利。',
      textAlign: TextAlign.center,
      style: Theme.of(context).textTheme.bodySmall?.copyWith(
        color: Theme.of(context).colorScheme.onSurfaceVariant,
      ),
    );
  }
}

class _AboutSection extends StatelessWidget {
  const _AboutSection({required this.title, required this.child});

  final String title;
  final Widget child;

  /// Builds one grouped area of the about screen.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Card(
      margin: EdgeInsets.zero,
      elevation: 0,
      color: colorScheme.surfaceContainerLow,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            Text(
              title,
              style: Theme.of(
                context,
              ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w800),
            ),
            const SizedBox(height: 10),
            child,
          ],
        ),
      ),
    );
  }
}

class _AboutActionRow extends StatelessWidget {
  const _AboutActionRow({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.onTap,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final Future<void> Function() onTap;

  /// Builds one external about-page action.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return ListTile(
      contentPadding: EdgeInsets.zero,
      leading: Icon(icon, color: colorScheme.primary),
      title: Text(title),
      subtitle: Text(subtitle),
      trailing: const Icon(Icons.open_in_new_outlined, size: 20),
      onTap: onTap,
    );
  }
}

class _OpenSourceComponent {
  const _OpenSourceComponent({
    required this.name,
    required this.license,
    required this.description,
  });

  final String name;
  final String license;
  final String description;
}

const List<_OpenSourceComponent> _openSourceComponents = <_OpenSourceComponent>[
  _OpenSourceComponent(
    name: 'Operit2',
    license: 'AGPL-3.0',
    description: 'AI workspace application and runtime.',
  ),
  _OpenSourceComponent(
    name: 'Flutter',
    license: 'BSD-3-Clause',
    description: 'Cross-platform application framework.',
  ),
  _OpenSourceComponent(
    name: 'xterm',
    license: 'MIT',
    description: 'Embedded terminal widget.',
  ),
  _OpenSourceComponent(
    name: 'flutter_math_fork',
    license: 'Apache-2.0',
    description: 'LaTeX and mathematical expression rendering.',
  ),
  _OpenSourceComponent(
    name: 'liquid_glass_widgets',
    license: 'MIT',
    description: 'Glass surface widgets used by the app theme.',
  ),
  _OpenSourceComponent(
    name: 'webview_all',
    license: 'MIT',
    description: 'Cross-platform WebView integration.',
  ),
];

/// Opens an external URI using the platform handler.
Future<void> _launchExternalUri(Uri uri) async {
  final launched = await launchUrl(uri, mode: LaunchMode.externalApplication);
  if (!launched) {
    throw StateError('Unable to open $uri');
  }
}

/// Opens the bundled component license list.
Future<void> _showOpenSourceLicenses(BuildContext context) {
  return showDialog<void>(
    context: context,
    builder: (dialogContext) {
      return AlertDialog(
        title: const Text('开源许可证'),
        content: SizedBox(
          width: 520,
          child: ListView.separated(
            shrinkWrap: true,
            itemCount: _openSourceComponents.length,
            separatorBuilder: (_, _) => const Divider(height: 1),
            itemBuilder: (context, index) {
              final component = _openSourceComponents[index];
              return ListTile(
                contentPadding: EdgeInsets.zero,
                title: Text(component.name),
                subtitle: Text(component.description),
                trailing: Text(
                  component.license,
                  style: Theme.of(context).textTheme.labelMedium?.copyWith(
                    color: Theme.of(context).colorScheme.primary,
                    fontWeight: FontWeight.w700,
                  ),
                ),
              );
            },
          ),
        ),
        actions: <Widget>[
          TextButton(
            onPressed: () => Navigator.of(dialogContext).pop(),
            child: const Text('关闭'),
          ),
        ],
      );
    },
  );
}
