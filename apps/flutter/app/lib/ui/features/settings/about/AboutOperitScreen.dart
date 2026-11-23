// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:flutter/services.dart';
import 'package:package_info_plus/package_info_plus.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../../common/OperitLogoMark.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../../../../core/update/AppUpdateController.dart';
import '../../../../core/update/AppUpdateModels.dart';
import '../../update/AppUpdateBadge.dart';
import '../../update/AppUpdateDialog.dart';

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
                  _AboutHeader(),
                  SizedBox(height: 24),
                  _AboutSection(title: l10n.aboutUpdateSection, child: _AboutUpdateAction()),
                  SizedBox(height: 12),
                  _AboutSection(
                    title: 'Operit2',
                    child: Text(
                      l10n.aboutAppDescription,
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

class _AboutHeader extends StatefulWidget {
  /// Creates the application identity header.
  const _AboutHeader();

  /// Creates the state that reads installed application metadata.
  @override
  State<_AboutHeader> createState() => _AboutHeaderState();
}

class _AboutHeaderState extends State<_AboutHeader> {
  late final Future<String> _appVersion;

  /// Starts one metadata read for this header instance.
  @override
  void initState() {
    super.initState();
    _appVersion = _readAppVersion();
  }

  /// Reads the version and build number from the installed application package.
  Future<String> _readAppVersion() async {
    final packageInfo = await PackageInfo.fromPlatform();
    if (packageInfo.version.trim().isEmpty ||
        packageInfo.buildNumber.trim().isEmpty) {
      throw StateError('Application package version or build number is empty.');
    }
    return '${packageInfo.version}+${packageInfo.buildNumber}';
  }

  /// Builds the application identity header.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
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
        FutureBuilder<String>(
          future: _appVersion,
          builder: (context, snapshot) {
            final style = Theme.of(context).textTheme.bodyMedium;
            if (snapshot.hasError) {
              return Text(
                l10n.aboutReadVersionFailed(snapshot.error.toString()),
                style: style?.copyWith(color: colorScheme.error),
                textAlign: TextAlign.center,
              );
            }
            if (snapshot.connectionState != ConnectionState.done) {
              return Text(
                l10n.aboutReadingVersion,
                style: style?.copyWith(color: colorScheme.onSurfaceVariant),
              );
            }
            return Text(
              l10n.aboutVersion(snapshot.requireData),
              style: style?.copyWith(color: colorScheme.onSurfaceVariant),
            );
          },
        ),
      ],
    );
  }
}

/// Presents update discovery and progress as an ordinary settings row.
class _AboutUpdateAction extends StatelessWidget {
  /// Creates the About-page update entry.
  const _AboutUpdateAction();

  /// Checks explicitly when needed and opens the shared update flow.
  Future<void> _openUpdates(
    BuildContext context,
    AppUpdateController controller,
  ) async {
    if (!controller.visible) await controller.check(force: true);
    if (!context.mounted) return;
  final l10n = AppLocalizations.of(context)!;
    if (controller.visible) {
      await showAppUpdateDialog(context, controller: controller);
    } else if (controller.error == null) {
      ScaffoldMessenger.of(
        context,
      ).showSnackBar(SnackBar(content: Text(l10n.aboutUpToDate)));
    }
  }

  /// Uses the same controller and dialog as the sidebar logo.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final controller =
        AppUpdateScope.maybeOf(context) ?? AppUpdateController.instance;
    return ListenableBuilder(
      listenable: controller,
      builder: (context, _) {
        final colors = Theme.of(context).colorScheme;
        final checking = controller.phase == AppUpdatePhase.checking;
        final subtitle = checking
            ? l10n.aboutCheckingUpdates
            : controller.visible
            ? controller.busy
                  ? l10n.aboutUpdatingTo(controller.release!.version)
                  : l10n.appUpdateAvailableVersion(controller.release!.version)
            : controller.error != null
            ? l10n.aboutRetryCheck
            : l10n.aboutCheckHint;
        return ListTile(
          contentPadding: EdgeInsets.zero,
          leading: Icon(Icons.system_update_alt, color: colors.primary),
          title: Text(l10n.extensionsCheckUpdates),
          subtitle: Text(subtitle),
          trailing: checking
              ? const SizedBox.square(
                  dimension: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : Icon(Icons.chevron_right, color: colors.onSurfaceVariant),
          onTap: checking ? null : () => _openUpdates(context, controller),
        );
      },
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
      title: l10n.aboutProject,
      child: Column(
        children: <Widget>[
          _AboutActionRow(
            icon: Icons.code_outlined,
            title: l10n.aboutSource,
            subtitle: 'github.com/AAswordman/Operit2',
            onTap: () => _launchExternalUri(context, _projectUri),
          ),
          const Divider(height: 1),
          _AboutActionRow(
            icon: Icons.menu_book_outlined,
            title: l10n.aboutDocs,
            subtitle: l10n.aboutDocsSubtitle,
            onTap: () => _launchExternalUri(context, _documentationUri),
          ),
          const Divider(height: 1),
          _AboutActionRow(
            icon: Icons.description_outlined,
            title: l10n.aboutLicenses,
            subtitle: l10n.aboutLicenseSubtitle,
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
      title: l10n.aboutContact,
      child: _AboutActionRow(
        icon: Icons.alternate_email_outlined,
        title: l10n.aboutDeveloper,
        subtitle: 'aaswordsman@foxmail.com',
        onTap: () => _launchExternalUri(context, _contactUri),
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
///
/// When no activity can handle the URI (for example no browser or mail app),
/// copies the address to the clipboard and notifies the user instead of
/// surfacing an unhandled platform exception.
Future<void> _launchExternalUri(BuildContext context, Uri uri) async {
  final l10n = AppLocalizations.of(context)!;
  try {
    final launched = await launchUrl(uri, mode: LaunchMode.externalApplication);
    if (!launched) {
      throw StateError('Unable to open $uri');
    }
  } catch (_) {
    await Clipboard.setData(ClipboardData(text: uri.toString()));
    if (context.mounted) {
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text(l10n.aboutLinkCopied(uri.toString()))),
      );
    }
  }
}

/// Opens the bundled component license list.
Future<void> _showOpenSourceLicenses(BuildContext context) {
  final l10n = AppLocalizations.of(context)!;
  return showDialog<void>(
    context: context,
    builder: (dialogContext) {
      return AlertDialog(
        title: Text(l10n.aboutLicenses),
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
