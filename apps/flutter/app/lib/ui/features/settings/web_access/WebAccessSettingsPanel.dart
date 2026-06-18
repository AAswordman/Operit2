// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../../../core/link_host/LinkHostServer.dart';
import '../../../../core/link_host/LinkHostConfig.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../components/SettingsControlStyles.dart';

class WebAccessSettingsPanel extends StatefulWidget {
  const WebAccessSettingsPanel({super.key, this.embedded = false});

  final bool embedded;

  @override
  State<WebAccessSettingsPanel> createState() => _WebAccessSettingsPanelState();
}

class _WebAccessSettingsPanelState extends State<WebAccessSettingsPanel> {
  final TextEditingController _bindAddressController = TextEditingController();
  final TextEditingController _tokenController = TextEditingController();
  LinkHostConfig? _config;
  List<String> _pairingBaseUrls = <String>[];
  LinkHostPortMode _portMode = LinkHostPortMode.automatic;
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
    final config = await LinkHostConfigStore.read();
    final server = LinkHostServer.instance;
    final displayConfig = config.webAccessEnabled && server.isRunning
        ? server.currentConfig!
        : config;
    final pairingBaseUrls = _bindAddressLooksValid(displayConfig.bindAddress)
        ? await server.pairingBaseUrls(displayConfig)
        : <String>[];
    if (!mounted) {
      return;
    }
    setState(() {
      _config = config;
      _pairingBaseUrls = pairingBaseUrls;
      _portMode = config.portMode;
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
    if (!_bindAddressInputIsValid()) {
      _showMessage(l10n.settingsWebAccessInvalidBindAddress);
      return;
    }
    setState(() => _busy = true);
    try {
      final next = config.copyWith(
        webAccessEnabled: enabled,
        portMode: _portMode,
        bindAddress: _bindAddressForPortMode(),
        token: _tokenController.text,
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      );
      LinkHostConfig displayConfig = next;
      if (next.webAccessEnabled || next.discoveryEnabled) {
        await LinkHostServer.instance.start(next);
        displayConfig = LinkHostServer.instance.currentConfig!;
        await LinkHostConfigStore.write(next);
      } else {
        await LinkHostConfigStore.write(next);
        await LinkHostServer.instance.stop(updateConfig: false);
      }
      if (!mounted) {
        return;
      }
      final pairingBaseUrls = await LinkHostServer.instance.pairingBaseUrls(
        displayConfig,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _config = next;
        _pairingBaseUrls = pairingBaseUrls;
        _portMode = next.portMode;
        _bindAddressController.text = next.bindAddress;
      });
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
    if (!_bindAddressInputIsValid()) {
      _showMessage(l10n.settingsWebAccessInvalidBindAddress);
      return;
    }
    setState(() => _busy = true);
    try {
      final next = config.copyWith(
        portMode: _portMode,
        bindAddress: _bindAddressForPortMode(),
        token: _tokenController.text,
        updatedAt: DateTime.now().millisecondsSinceEpoch,
      );
      LinkHostConfig displayConfig = next;
      if (next.webAccessEnabled || next.discoveryEnabled) {
        await LinkHostServer.instance.start(next);
        displayConfig = LinkHostServer.instance.currentConfig!;
      }
      await LinkHostConfigStore.write(next);
      if (!mounted) {
        return;
      }
      final pairingBaseUrls = await LinkHostServer.instance.pairingBaseUrls(
        displayConfig,
      );
      if (!mounted) {
        return;
      }
      setState(() {
        _config = next;
        _pairingBaseUrls = pairingBaseUrls;
        _portMode = next.portMode;
        _bindAddressController.text = next.bindAddress;
      });
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
    final token = LinkHostToken.generate();
    _tokenController.text = token;
    final next = config.copyWith(
      token: token,
      portMode: _portMode,
      bindAddress: _bindAddressForPortMode(),
      updatedAt: DateTime.now().millisecondsSinceEpoch,
    );
    if (next.webAccessEnabled || next.discoveryEnabled) {
      await LinkHostServer.instance.start(next);
    }
    await LinkHostConfigStore.write(next);
    if (!mounted) {
      return;
    }
    setState(() {
      _config = next;
      _portMode = next.portMode;
      _bindAddressController.text = next.bindAddress;
    });
  }

  bool _bindAddressInputIsValid() {
    return _portMode == LinkHostPortMode.automatic ||
        _bindAddressLooksValid(_bindAddressController.text);
  }

  String _bindAddressForPortMode() {
    return _portMode == LinkHostPortMode.automatic
        ? LinkHostConfig.automaticBindAddress
        : _bindAddressController.text.trim();
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
    ScaffoldMessenger.of(
      context,
    ).showSnackBar(SnackBar(content: Text(message)));
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final config = _config;
    if (config == null) {
      return const Center(child: CircularProgressIndicator());
    }
    final server = LinkHostServer.instance;
    final running = config.webAccessEnabled && server.isRunning;
    final displayConfig = running ? server.currentConfig! : config;
    final bindAddress = running
        ? displayConfig.bindAddress
        : _bindAddressForPortMode();
    final runningUrl = server.baseUrl;
    final url = running && runningUrl != null
        ? runningUrl
        : (_bindAddressLooksValid(bindAddress)
              ? _baseUrlForBindAddress(bindAddress)
              : l10n.settingsWebAccessInvalidBindAddress);
    final bindAddressIsValid = _bindAddressLooksValid(bindAddress);
    final pairingAddressText = bindAddressIsValid
        ? (_bindAddressIsLoopback(bindAddress)
              ? l10n.settingsWebAccessPairingUrlLocalOnly
              : (_pairingBaseUrls.isEmpty
                    ? l10n.settingsWebAccessPairingUrlUnavailable
                    : null))
        : l10n.settingsWebAccessInvalidBindAddress;
    final children = <Widget>[
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
            value: config.webAccessEnabled,
            onChanged: _busy ? null : _setEnabled,
          ),
          const SizedBox(height: 8),
          Text(
            l10n.settingsWebAccessPortMode,
            style: Theme.of(context).textTheme.labelMedium,
          ),
          const SizedBox(height: 6),
          SizedBox(
            width: double.infinity,
            child: SegmentedButton<LinkHostPortMode>(
              segments: <ButtonSegment<LinkHostPortMode>>[
                ButtonSegment<LinkHostPortMode>(
                  value: LinkHostPortMode.automatic,
                  icon: const Icon(Icons.auto_mode_outlined),
                  label: Text(l10n.settingsWebAccessPortAutomatic),
                ),
                ButtonSegment<LinkHostPortMode>(
                  value: LinkHostPortMode.fixed,
                  icon: const Icon(Icons.push_pin_outlined),
                  label: Text(l10n.settingsWebAccessPortFixed),
                ),
              ],
              selected: <LinkHostPortMode>{_portMode},
              onSelectionChanged: _busy
                  ? null
                  : (selection) {
                      setState(() => _portMode = selection.first);
                    },
            ),
          ),
          const SizedBox(height: 6),
          Text(
            _portMode == LinkHostPortMode.automatic
                ? l10n.settingsWebAccessPortAutomaticDescription
                : l10n.settingsWebAccessPortFixedDescription,
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
              color: Theme.of(context).colorScheme.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 10),
          if (_portMode == LinkHostPortMode.fixed) ...<Widget>[
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
          ],
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
          _AddressRow(
            label: l10n.settingsWebAccessLocalUrl,
            value: url,
            onCopy: () => _copyUrl(url),
            onOpen: running ? () => _openUrl(url) : null,
          ),
          const SizedBox(height: 8),
          if (pairingAddressText != null)
            _AddressRow(
              label: l10n.settingsWebAccessPairingUrl,
              value: pairingAddressText,
            )
          else
            for (var index = 0; index < _pairingBaseUrls.length; index++)
              Padding(
                padding: EdgeInsets.only(top: index == 0 ? 0 : 8),
                child: _AddressRow(
                  label: index == 0 ? l10n.settingsWebAccessPairingUrl : '',
                  value: _pairingBaseUrls[index],
                  onCopy: () => _copyUrl(_pairingBaseUrls[index]),
                ),
              ),
        ],
      ),
    ];
    if (widget.embedded) {
      return Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: children,
      );
    }
    return ListView(
      padding: const EdgeInsets.fromLTRB(16, 12, 16, 20),
      children: children,
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
        material: true,
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

class _AddressRow extends StatelessWidget {
  const _AddressRow({
    required this.label,
    required this.value,
    this.onCopy,
    this.onOpen,
  });

  final String label;
  final String value;
  final VoidCallback? onCopy;
  final VoidCallback? onOpen;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Row(
      crossAxisAlignment: CrossAxisAlignment.center,
      children: <Widget>[
        SizedBox(
          width: 72,
          child: Text(
            label,
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
              color: colorScheme.onSurfaceVariant,
            ),
          ),
        ),
        Expanded(
          child: SelectableText(
            value,
            style: Theme.of(context).textTheme.bodyMedium,
          ),
        ),
        if (onCopy != null)
          IconButton(
            tooltip: AppLocalizations.of(context)!.settingsWebAccessCopyUrl,
            icon: const Icon(Icons.content_copy_outlined, size: 18),
            onPressed: onCopy,
          ),
        if (onOpen != null)
          IconButton(
            tooltip: AppLocalizations.of(context)!.settingsWebAccessOpenUrl,
            icon: const Icon(Icons.open_in_browser_outlined, size: 18),
            onPressed: onOpen,
          ),
      ],
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

bool _bindAddressIsLoopback(String bindAddress) {
  final trimmed = bindAddress.trim();
  final index = trimmed.lastIndexOf(':');
  final host = trimmed.substring(0, index);
  return host == '127.0.0.1' || host == 'localhost' || host == '::1';
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
