// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../l10n/generated/app_localizations.dart';
import '../../../main/navigation/AppNavigationModels.dart';
import '../../../main/screens/OperitScreens.dart';
import '../../../main/screens/ScreenRouteRegistry.dart';
import '../../../theme/OperitGlassSurface.dart';

class WorkspaceSettingsPanel extends StatelessWidget {
  const WorkspaceSettingsPanel({super.key});

  void _openChat() {
    final entry = ScreenRouteRegistry.toEntry(
      screen: const AiChatScreenRoute(),
    );
    AppRouterGateway.navigate(
      routeId: entry.routeId,
      args: entry.args,
      source: entry.source,
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return ListView(
      padding: const EdgeInsets.fromLTRB(28, 24, 28, 36),
      children: <Widget>[
        _SectionCard(
          title: l10n.settingsWorkspaceCurrentDesign,
          children: <Widget>[
            _BodyText(l10n.settingsWorkspaceCurrentDesignDescription),
            _ActionLine(
              icon: Icons.chat_bubble_outline,
              title: l10n.settingsWorkspaceOpenChat,
              subtitle: l10n.settingsWorkspaceOpenChatDescription,
              onTap: _openChat,
            ),
          ],
        ),
        _SectionCard(
          title: l10n.settingsWorkspaceContains,
          children: <Widget>[
            _InfoLine(label: l10n.files, value: l10n.settingsWorkspacePerChat),
            _InfoLine(
              label: l10n.terminal,
              value: l10n.settingsWorkspaceGlobalSessions,
            ),
            _InfoLine(
              label: l10n.browser,
              value: l10n.settingsWorkspaceBrowserSessions,
            ),
          ],
        ),
      ],
    );
  }
}

class _SectionCard extends StatelessWidget {
  const _SectionCard({required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final radius = BorderRadius.circular(18);
    return Padding(
      padding: const EdgeInsets.only(bottom: 14),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        borderRadius: radius,
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(18, 16, 18, 14),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(title, style: const TextStyle(fontWeight: FontWeight.w800)),
              const SizedBox(height: 8),
              ...children,
            ],
          ),
        ),
      ),
    );
  }
}

class _BodyText extends StatelessWidget {
  const _BodyText(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Text(
        text,
        style: TextStyle(color: Theme.of(context).colorScheme.onSurfaceVariant),
      ),
    );
  }
}

class _InfoLine extends StatelessWidget {
  const _InfoLine({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 9),
      child: Row(
        children: <Widget>[
          Expanded(child: Text(label)),
          const SizedBox(width: 12),
          Flexible(
            child: Text(
              value,
              textAlign: TextAlign.end,
              style: TextStyle(color: colorScheme.onSurfaceVariant),
            ),
          ),
        ],
      ),
    );
  }
}

class _ActionLine extends StatelessWidget {
  const _ActionLine({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.onTap,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    return ListTile(
      contentPadding: EdgeInsets.zero,
      leading: Icon(icon),
      title: Text(title),
      subtitle: Text(subtitle),
      trailing: const Icon(Icons.chevron_right),
      onTap: onTap,
    );
  }
}
