// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../../../l10n/generated/app_localizations.dart';
import '../../../common/OperitLogoMark.dart';
import '../../../theme/OperitGlassSurface.dart';

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
    final l10n = AppLocalizations.of(context)!;
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
              child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  const _AboutHeader(),
                  const SizedBox(height: 24),
                  _AboutSection(
                    title: 'Operit2',
                    child: Text(l10n.aboutDescription),
                  ),
                  const SizedBox(height: 12),
                  const _ProjectLinksSection(),
                  const SizedBox(height: 12),
                  const _SupportSection(),
                  const SizedBox(height: 24),
                  const _CopyrightNotice(),
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
    final l10n = AppLocalizations.of(context)!;
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
          l10n.aboutVersion(_appVersion),
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
    final l10n = AppLocalizations.of(context)!;
    return _AboutSection(
      title: l10n.aboutProjectSection,
      child: Column(
        children: <Widget>[
          _AboutActionRow(
            icon: Icons.code_outlined,
            title: l10n.aboutSourceTitle,
            subtitle: 'github.com/AAswordman/Operit2',
            onTap: () => _launchExternalUri(_projectUri),
          ),
          const Divider(height: 1),
          _AboutActionRow(
            icon: Icons.menu_book_outlined,
            title: l10n.aboutDocumentationTitle,
            subtitle: l10n.aboutDocumentationSubtitle,
            onTap: () => _launchExternalUri(_documentationUri),
          ),
          const Divider(height: 1),
          _AboutActionRow(
            icon: Icons.description_outlined,
            title: l10n.aboutOpenSourceLicenses,
            subtitle: l10n.aboutOpenSourceLicensesSubtitle,
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
    final l10n = AppLocalizations.of(context)!;
    return _AboutSection(
      title: l10n.aboutContactSection,
      child: _AboutActionRow(
        icon: Icons.alternate_email_outlined,
        title: l10n.aboutMaintainer,
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
    final l10n = AppLocalizations.of(context)!;
    return Text(
      l10n.aboutCopyright,
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
    return OperitGlassSurface(
      color: colorScheme.surfaceContainerLow,
      borderRadius: BorderRadius.circular(8),
      border: Border.all(
        color: colorScheme.outlineVariant.withValues(alpha: 0.18),
      ),
      material: true,
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
  final l10n = AppLocalizations.of(context)!;
  return showDialog<void>(
    context: context,
    builder: (dialogContext) {
      return AlertDialog(
        title: Text(l10n.aboutOpenSourceLicenses),
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
            child: Text(l10n.close),
          ),
        ],
      );
    },
  );
}
