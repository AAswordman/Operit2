// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../../../core/web_access/FlutterWebAccessServer.dart';
import '../../../../core/web_access/WebAccessConfig.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';

class WebAccessSettingsPanel extends StatefulWidget {
  const WebAccessSettingsPanel({super.key});

  @override
  State<WebAccessSettingsPanel> createState() => _WebAccessSettingsPanelState();
}

class _WebAccessSettingsPanelState extends State<WebAccessSettingsPanel> {
  final TextEditingController _bindAddressController = TextEditingController();
  final TextEditingController _tokenController = TextEditingController();
  WebAccessConfig? _config;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    _load();
  }

  @override
  void dispose() {
    _bindAddressController.dispose();
    _tokenController.dispose();
    super.dispose();
  }

  Future<void> _load() async {
    final config = await WebAccessConfigStore.read();
    if (!mounted) {
      return;
    }
    setState(() {
      _config = config;
      _bindAddressController.text = config.bindAddress;
      _tokenController.text = config.token;
    });
  }

  Future<void> _setEnabled(bool enabled) async {
    final l10n = AppLocalizations.of(context)!;
    final config = _config;
    if (config == null || _busy) {
      return;
    }
    if (!_bindAddressLooksValid(_bindAddressController.text)) {
      _showMessage(l10n.settingsWebAccessInvalidBindAddress);
      return;
    }
    setState(() => _busy = true);
    try {
      final next = config.copyWith(
        enabled: enabled,
        bindAddress: _bindAddressController.text.trim(),
        token: _tokenController.text,
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      );
      await WebAccessConfigStore.write(next);
      if (enabled) {
        await FlutterWebAccessServer.instance.start(next);
      } else {
        await FlutterWebAccessServer.instance.stop();
      }
      if (!mounted) {
        return;
      }
      setState(() => _config = next);
      _showMessage(l10n.settingsWebAccessSaved);
    } catch (error) {
      if (mounted) {
        _showMessage(
          enabled
              ? l10n.settingsWebAccessStartFailed(error.toString())
              : l10n.settingsWebAccessStopFailed(error.toString()),
        );
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _save() async {
    final l10n = AppLocalizations.of(context)!;
    final config = _config;
    if (config == null || _busy) {
      return;
    }
    if (!_bindAddressLooksValid(_bindAddressController.text)) {
      _showMessage(l10n.settingsWebAccessInvalidBindAddress);
      return;
    }
    setState(() => _busy = true);
    try {
      final next = config.copyWith(
        bindAddress: _bindAddressController.text.trim(),
        token: _tokenController.text,
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      );
      await WebAccessConfigStore.write(next);
      if (next.enabled) {
        await FlutterWebAccessServer.instance.start(next);
      }
      if (!mounted) {
        return;
      }
      setState(() => _config = next);
      _showMessage(l10n.settingsWebAccessSaved);
    } catch (error) {
      if (mounted) {
        _showMessage(l10n.settingsWebAccessStartFailed(error.toString()));
      }
    } finally {
      if (mounted) {
        setState(() => _busy = false);
      }
    }
  }

  Future<void> _rotateToken() async {
    final config = _config;
    if (config == null) {
      return;
    }
    final token = WebAccessToken.generate();
    _tokenController.text = token;
    final next = config.copyWith(
      token: token,
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
    await WebAccessConfigStore.write(next);
    if (next.enabled) {
      await FlutterWebAccessServer.instance.start(next);
    }
    if (!mounted) {
      return;
    }
    setState(() => _config = next);
    _showMessage(AppLocalizations.of(context)!.settingsWebAccessSaved);
  }

  Future<void> _copyToken() async {
    await Clipboard.setData(ClipboardData(text: _tokenController.text));
    if (mounted) {
      _showMessage(AppLocalizations.of(context)!.settingsWebAccessTokenCopied);
    }
  }

  Future<void> _copyUrl(String url) async {
    await Clipboard.setData(ClipboardData(text: url));
    if (mounted) {
      _showMessage(AppLocalizations.of(context)!.settingsWebAccessUrlCopied);
    }
  }

  Future<void> _openUrl(String url) async {
    await launchUrl(Uri.parse(url), mode: LaunchMode.externalApplication);
  }

  void _showMessage(String message) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(content: Text(message)),
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final config = _config;
    if (config == null) {
      return const Center(child: CircularProgressIndicator());
    }
    final running = FlutterWebAccessServer.instance.isRunning;
    final bindAddress = _bindAddressController.text;
    final runningUrl = FlutterWebAccessServer.instance.baseUrl;
    final url = running && runningUrl != null
        ? runningUrl
        : (_bindAddressLooksValid(bindAddress)
              ? _baseUrlForBindAddress(bindAddress)
              : l10n.settingsWebAccessInvalidBindAddress);
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
      children: <Widget>[
        _SectionCard(
          title: l10n.settingsWebAccessService,
          children: <Widget>[
            Text(
              l10n.settingsWebAccessServiceDescription,
              style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
            ),
            const SizedBox(height: 10),
            SwitchListTile(
              contentPadding: EdgeInsets.zero,
              dense: true,
              visualDensity: VisualDensity.compact,
              title: Text(l10n.settingsWebAccessEnable),
              subtitle: Text(
                running
                    ? l10n.settingsWebAccessRunning
                    : l10n.settingsWebAccessStopped,
              ),
              value: config.enabled,
              onChanged: _busy ? null : _setEnabled,
            ),
            const SizedBox(height: 8),
            TextField(
              controller: _bindAddressController,
              decoration: InputDecoration(
                labelText: l10n.settingsWebAccessBindAddress,
                border: const OutlineInputBorder(),
                isDense: true,
              ),
              onSubmitted: (_) => _save(),
            ),
            const SizedBox(height: 10),
            TextField(
              controller: _tokenController,
              decoration: InputDecoration(
                labelText: l10n.settingsWebAccessToken,
                border: const OutlineInputBorder(),
                isDense: true,
                suffixIcon: IconButton(
                  tooltip: l10n.settingsWebAccessCopyToken,
                  icon: const Icon(Icons.content_copy_outlined),
                  onPressed: _copyToken,
                ),
              ),
            ),
            const SizedBox(height: 10),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                FilledButton.icon(
                  style: SettingsControlStyles.sectionFilledButton(),
                  onPressed: _busy ? null : _save,
                  icon: const Icon(Icons.save_outlined, size: 18),
                  label: Text(l10n.save),
                ),
                TextButton.icon(
                  style: SettingsControlStyles.sectionTextButton(),
                  onPressed: _busy ? null : _rotateToken,
                  icon: const Icon(Icons.autorenew_outlined, size: 18),
                  label: Text(l10n.settingsWebAccessRotateToken),
                ),
              ],
            ),
          ],
        ),
        _SectionCard(
          title: l10n.settingsWebAccessAccessUrl,
          children: <Widget>[
            SelectableText(
              url,
              style: Theme.of(context).textTheme.bodyMedium,
            ),
            const SizedBox(height: 10),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                TextButton.icon(
                  style: SettingsControlStyles.sectionTextButton(),
                  onPressed: () => _copyUrl(url),
                  icon: const Icon(Icons.content_copy_outlined, size: 18),
                  label: Text(l10n.settingsWebAccessCopyUrl),
                ),
                TextButton.icon(
                  style: SettingsControlStyles.sectionTextButton(),
                  onPressed: running ? () => _openUrl(url) : null,
                  icon: const Icon(Icons.open_in_browser_outlined, size: 18),
                  label: Text(l10n.settingsWebAccessOpenUrl),
                ),
              ],
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
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        borderRadius: BorderRadius.circular(12),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(14, 12, 14, 10),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(
                title,
                style: SettingsControlStyles.sectionTitleTextStyle(context),
              ),
              const SizedBox(height: 8),
              ...children,
            ],
          ),
        ),
      ),
    );
  }
}

bool _bindAddressLooksValid(String value) {
  final trimmed = value.trim();
  final index = trimmed.lastIndexOf(':');
  if (index <= 0 || index == trimmed.length - 1) {
    return false;
  }
  return int.tryParse(trimmed.substring(index + 1)) != null;
}

String _baseUrlForBindAddress(String bindAddress) {
  final trimmed = bindAddress.trim();
  final index = trimmed.lastIndexOf(':');
  final host = trimmed.substring(0, index);
  final port = trimmed.substring(index + 1);
  final displayHost = switch (host) {
    '0.0.0.0' => '127.0.0.1',
    '::' => '127.0.0.1',
    _ => host,
  };
  return 'http://$displayHost:$port';
}
