// ignore_for_file: file_names
import 'dart:async';
import 'package:file_selector/file_selector.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:mobile_scanner/mobile_scanner.dart';
import 'package:qr_flutter/qr_flutter.dart';

import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../core/proxy/generated/CoreProxyModels.g.dart' as generated;
import '../../core/runtime/PeerQrImage.dart';
import '../../l10n/generated/app_localizations.dart';
import 'components/OperitDialog.dart';

enum PeerQrMode { scan, show }

/// Returns a validated, explicitly confirmed code. Pairing remains owned by the caller.
Future<String?> showPeerQrCodeDialog(
  BuildContext context, {
  required GeneratedCoreProxyClients clients,
  PeerQrMode mode = PeerQrMode.scan,
  String? onlyNodeId,
}) => showDialog<String>(
  context: context,
  builder: (_) => PeerQrCodeDialog(
    clients: clients,
    initialMode: mode,
    onlyNodeId: onlyNodeId,
  ),
);

class PeerQrCodeDialog extends StatefulWidget {
  const PeerQrCodeDialog({
    super.key,
    required this.clients,
    this.initialMode = PeerQrMode.scan,
    this.onlyNodeId,
  });
  final GeneratedCoreProxyClients clients;
  final PeerQrMode initialMode;
  final String? onlyNodeId;
  @override
  State<PeerQrCodeDialog> createState() => _PeerQrCodeDialogState();
}

class _PeerQrCodeDialogState extends State<PeerQrCodeDialog> {
  late PeerQrMode _mode = widget.initialMode;
  final _paste = TextEditingController();
  String? _ownCode, _candidateCode, _error, _lastScan;
  generated.PeerConnectionInfo? _candidate;
  bool _busy = false, _copied = false;
  int _generation = 0;

  bool get _cameraSupported =>
      kIsWeb ||
      const ['android', 'iOS', 'macOS'].contains(defaultTargetPlatform.name);

  @override
  void initState() {
    super.initState();
    if (_mode == PeerQrMode.show) unawaited(_loadCode());
  }

  @override
  void dispose() {
    _generation++;
    _paste.dispose();
    super.dispose();
  }

  void _switch(PeerQrMode mode) {
    if (mode == _mode) return;
    _generation++;
    setState(() {
      _mode = mode;
      _busy = false;
      _error = null;
      _lastScan = null;
    });
    if (mode == PeerQrMode.show) unawaited(_loadCode());
  }

  Future<void> _loadCode() async {
    final generation = ++_generation;
    setState(() {
      _busy = true;
      _error = null;
      _ownCode = null;
      _copied = false;
    });
    try {
      final code = await widget.clients.server.runtimeRemoteLinkService
          .peerConnectionCode();
      if (mounted && generation == _generation) setState(() => _ownCode = code);
    } catch (error) {
      if (mounted && generation == _generation) {
        setState(() => _error = error.toString());
      }
    } finally {
      if (mounted && generation == _generation) setState(() => _busy = false);
    }
  }

  Future<void> _inspect(String value) async {
    if (_busy || _candidate != null || value == _lastScan) return;
    final l10n = AppLocalizations.of(context)!;
    final generation = ++_generation;
    setState(() {
      _busy = true;
      _error = null;
      _lastScan = value;
    });
    try {
      // Validation, version handling and self-connection checks belong to the runtime.
      final info = await widget.clients.server.runtimeRemoteLinkService
          .inspectPeerConnectionCode(code: value);
      if (!mounted || generation != _generation) return;
      if (widget.onlyNodeId != null && info.nodeId != widget.onlyNodeId) {
        setState(() => _error = l10n.peerQrWrongDevice);
      } else {
        setState(() {
          _candidate = info;
          _candidateCode = value;
        });
      }
    } catch (error) {
      if (mounted && generation == _generation) {
        setState(() => _error = error.toString());
      }
    } finally {
      if (mounted && generation == _generation) setState(() => _busy = false);
    }
  }

  Future<void> _importImage() async {
    if (_busy) return;
    final l10n = AppLocalizations.of(context)!;
    final generation = ++_generation;
    setState(() {
      _busy = true;
      _error = null;
    });
    String? value;
    try {
      final file = await openFile(
        acceptedTypeGroups: [
          const XTypeGroup(
            label: 'QR image',
            extensions: ['png', 'jpg', 'jpeg', 'webp'],
            mimeTypes: ['image/png', 'image/jpeg', 'image/webp'],
            uniformTypeIdentifiers: [
              'public.png',
              'public.jpeg',
              'org.webmproject.webp',
            ],
          ),
        ],
      );
      if (file == null) return;
      if (await file.length() > 16 * 1024 * 1024) {
        throw FormatException(l10n.peerQrImageInvalid);
      }
      value = await compute(readPeerQrImage, await file.readAsBytes());
      if (value == null) throw FormatException(l10n.peerQrImageInvalid);
    } catch (error) {
      if (mounted && generation == _generation) {
        setState(() => _error = error.toString());
      }
    } finally {
      if (mounted && generation == _generation) setState(() => _busy = false);
    }
    if (value != null && mounted && generation == _generation) {
      _lastScan = null;
      await _inspect(value);
    }
  }

  Widget _status(IconData icon, String text) {
    final colors = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.all(24),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Icon(icon, size: 36, color: colors.onSurfaceVariant),
          const SizedBox(height: 12),
          Text(text, textAlign: TextAlign.center),
        ],
      ),
    );
  }

  Widget _showCode() {
    final l10n = AppLocalizations.of(context)!;
    final code = _ownCode;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(l10n.peerQrShowHint, textAlign: TextAlign.center),
        const SizedBox(height: 20),
        if (_busy)
          const SizedBox(
            height: 240,
            child: Center(child: CircularProgressIndicator()),
          )
        else if (code != null)
          LayoutBuilder(
            builder: (context, constraints) {
              final size = constraints.maxWidth.clamp(0.0, 280.0);
              return Container(
                decoration: BoxDecoration(
                  color: Colors.white,
                  borderRadius: BorderRadius.circular(20),
                ),
                padding: const EdgeInsets.all(20),
                child: QrImageView(
                  data: code,
                  version: QrVersions.auto,
                  size: (size - 40).clamp(0.0, 240.0),
                  padding: EdgeInsets.zero,
                  backgroundColor: Colors.white,
                  eyeStyle: const QrEyeStyle(
                    eyeShape: QrEyeShape.square,
                    color: Colors.black,
                  ),
                  dataModuleStyle: const QrDataModuleStyle(
                    dataModuleShape: QrDataModuleShape.square,
                    color: Colors.black,
                  ),
                  semanticsLabel: l10n.peerQrShow,
                ),
              );
            },
          )
        else
          _status(Icons.wifi_off_rounded, l10n.peerQrListenerRequired),
        const SizedBox(height: 16),
        Text(
          l10n.peerQrTrustHint,
          textAlign: TextAlign.center,
          style: Theme.of(context).textTheme.bodySmall?.copyWith(
            color: Theme.of(context).colorScheme.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: 12),
        Wrap(
          alignment: WrapAlignment.center,
          spacing: 8,
          runSpacing: 8,
          children: [
            TextButton.icon(
              onPressed: code == null
                  ? null
                  : () async {
                      await Clipboard.setData(ClipboardData(text: code));
                      if (mounted) setState(() => _copied = true);
                    },
              icon: Icon(
                _copied ? Icons.check_rounded : Icons.copy_rounded,
                size: 18,
              ),
              label: Text(_copied ? l10n.peerQrCopied : l10n.peerQrCopy),
            ),
            TextButton.icon(
              onPressed: _busy ? null : _loadCode,
              icon: const Icon(Icons.refresh_rounded, size: 18),
              label: Text(l10n.refresh),
            ),
          ],
        ),
      ],
    );
  }

  Widget _scanCode() {
    final l10n = AppLocalizations.of(context)!;
    final candidate = _candidate;
    if (candidate != null) {
      return Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          Container(
            padding: const EdgeInsets.all(18),
            decoration: BoxDecoration(
              color: Theme.of(context).colorScheme.primaryContainer,
              shape: BoxShape.circle,
            ),
            child: Icon(
              Icons.devices_rounded,
              size: 36,
              color: Theme.of(context).colorScheme.onPrimaryContainer,
            ),
          ),
          const SizedBox(height: 18),
          Text(
            candidate.displayName.isEmpty
                ? l10n.peerQrDevice
                : candidate.displayName,
            textAlign: TextAlign.center,
            style: Theme.of(context).textTheme.titleLarge,
          ),
          const SizedBox(height: 8),
          SelectableText(
            candidate.nodeId,
            textAlign: TextAlign.center,
            style: Theme.of(context).textTheme.bodySmall,
          ),
          const SizedBox(height: 16),
          ...candidate.addresses.map(
            (address) => Padding(
              padding: const EdgeInsets.only(bottom: 4),
              child: Text(
                address,
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ),
          ),
          const SizedBox(height: 16),
          Text(l10n.peerQrTrustHint, textAlign: TextAlign.center),
          TextButton(
            onPressed: () => setState(() {
              _candidate = null;
              _candidateCode = null;
              _lastScan = null;
              _error = null;
            }),
            child: Text(l10n.peerQrScanAgain),
          ),
        ],
      );
    }
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(l10n.peerQrScanHint, textAlign: TextAlign.center),
        const SizedBox(height: 16),
        ClipRRect(
          borderRadius: BorderRadius.circular(20),
          child: _cameraSupported
              ? AspectRatio(
                  aspectRatio: 1.25,
                  child: _PeerCamera(onCode: _inspect),
                )
              : ColoredBox(
                  color: Theme.of(context).colorScheme.surfaceContainerLow,
                  child: _status(
                    Icons.qr_code_scanner_rounded,
                    l10n.peerQrCameraUnavailable,
                  ),
                ),
        ),
        const SizedBox(height: 12),
        OutlinedButton.icon(
          onPressed: _busy ? null : _importImage,
          icon: const Icon(Icons.image_outlined, size: 18),
          label: Text(l10n.peerQrImport),
        ),
        const SizedBox(height: 12),
        TextField(
          controller: _paste,
          minLines: 1,
          maxLines: 3,
          maxLength: 4096,
          enabled: !_busy,
          decoration: InputDecoration(
            labelText: l10n.peerQrPaste,
            counterText: '',
            border: const OutlineInputBorder(),
            suffixIcon: IconButton(
              tooltip: l10n.peerQrRead,
              onPressed: _busy
                  ? null
                  : () {
                      _lastScan = null;
                      unawaited(_inspect(_paste.text));
                    },
              icon: const Icon(Icons.arrow_forward_rounded),
            ),
          ),
          onSubmitted: _busy
              ? null
              : (text) {
                  _lastScan = null;
                  unawaited(_inspect(text));
                },
        ),
        if (_busy)
          const Padding(
            padding: EdgeInsets.only(top: 12),
            child: LinearProgressIndicator(),
          ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colors = Theme.of(context).colorScheme;
    return OperitDialogScaffold(
      title: l10n.peerQrTitle,
      maxWidth: 440,
      maxHeight: MediaQuery.sizeOf(context).height * .85,
      expandContent: false,
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(l10n.cancel),
        ),
        if (_mode == PeerQrMode.scan && _candidate != null)
          FilledButton.icon(
            onPressed: _busy
                ? null
                : () => Navigator.pop(context, _candidateCode),
            icon: const Icon(Icons.link_rounded, size: 18),
            label: Text(l10n.settingsRuntimeStartPairing),
          ),
      ],
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            LayoutBuilder(
              builder: (context, constraints) {
                if (constraints.maxWidth < 330 ||
                    MediaQuery.textScalerOf(context).scale(1) > 1.4) {
                  return Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    alignment: WrapAlignment.center,
                    children: [
                      ChoiceChip(
                        label: Text(l10n.peerQrScan),
                        avatar: const Icon(
                          Icons.qr_code_scanner_rounded,
                          size: 18,
                        ),
                        selected: _mode == PeerQrMode.scan,
                        onSelected: (_) => _switch(PeerQrMode.scan),
                      ),
                      ChoiceChip(
                        label: Text(l10n.peerQrShow),
                        avatar: const Icon(Icons.qr_code_rounded, size: 18),
                        selected: _mode == PeerQrMode.show,
                        onSelected: (_) => _switch(PeerQrMode.show),
                      ),
                    ],
                  );
                }
                return SizedBox(
                  width: double.infinity,
                  child: SegmentedButton<PeerQrMode>(
                    segments: [
                      ButtonSegment(
                        value: PeerQrMode.scan,
                        icon: const Icon(Icons.qr_code_scanner_rounded),
                        label: Text(l10n.peerQrScan),
                      ),
                      ButtonSegment(
                        value: PeerQrMode.show,
                        icon: const Icon(Icons.qr_code_rounded),
                        label: Text(l10n.peerQrShow),
                      ),
                    ],
                    selected: {_mode},
                    onSelectionChanged: (values) => _switch(values.single),
                    showSelectedIcon: false,
                  ),
                );
              },
            ),
            const SizedBox(height: 24),
            if (_mode == PeerQrMode.show) _showCode() else _scanCode(),
            if (_error != null)
              Padding(
                padding: const EdgeInsets.only(top: 16),
                child: Container(
                  width: double.infinity,
                  padding: const EdgeInsets.all(14),
                  decoration: BoxDecoration(
                    color: colors.errorContainer,
                    borderRadius: BorderRadius.circular(12),
                  ),
                  child: Text(
                    _error!,
                    style: TextStyle(color: colors.onErrorContainer),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }
}

class _PeerCamera extends StatefulWidget {
  const _PeerCamera({required this.onCode});
  final Future<void> Function(String) onCode;
  @override
  State<_PeerCamera> createState() => _PeerCameraState();
}

class _PeerCameraState extends State<_PeerCamera> with WidgetsBindingObserver {
  final _controller = MobileScannerController(
    autoStart: false,
    formats: const [BarcodeFormat.qrCode],
    detectionSpeed: DetectionSpeed.noDuplicates,
  );
  Future<void> _cameraWork = Future.value();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _scheduleCamera(true);
    });
  }

  void _scheduleCamera(bool start) {
    _cameraWork = _cameraWork.then((_) async {
      if (start && !mounted) return;
      try {
        if (start) {
          await _controller.start();
        } else {
          await _controller.stop();
        }
      } on MobileScannerException {
        /* The scanner's error state is rendered below. */
      }
    });
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    _scheduleCamera(state == AppLifecycleState.resumed);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    unawaited(_cameraWork.then((_) => _controller.dispose()));
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return MobileScanner(
      controller: _controller,
      onDetect: (capture) {
        final values = capture.barcodes
            .map((barcode) => barcode.rawValue)
            .whereType<String>()
            .where((value) => value.isNotEmpty)
            .toSet();
        if (values.length == 1) unawaited(widget.onCode(values.single));
      },
      errorBuilder: (context, error) => ColoredBox(
        color: Theme.of(context).colorScheme.surfaceContainerLow,
        child: Center(
          child: SingleChildScrollView(
            padding: const EdgeInsets.all(20),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Icon(Icons.no_photography_outlined, size: 32),
                const SizedBox(height: 12),
                Text(
                  error.errorCode.name == 'permissionDenied'
                      ? l10n.peerQrCameraPermission
                      : l10n.peerQrCameraUnavailable,
                  textAlign: TextAlign.center,
                ),
                TextButton(
                  onPressed: () => _scheduleCamera(true),
                  child: Text(l10n.retry),
                ),
              ],
            ),
          ),
        ),
      ),
      overlayBuilder: (_, _) => IgnorePointer(
        child: Center(
          child: Container(
            width: 190,
            height: 190,
            decoration: BoxDecoration(
              border: Border.all(
                color: Colors.white.withValues(alpha: .85),
                width: 2,
              ),
              borderRadius: BorderRadius.circular(20),
            ),
          ),
        ),
      ),
    );
  }
}
