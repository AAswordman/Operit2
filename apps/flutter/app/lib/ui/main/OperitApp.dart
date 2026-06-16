// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../core/web_access/FlutterWebAccessServer.dart';
import '../../l10n/generated/app_localizations.dart';
import '../theme/OperitTheme.dart';
import 'screens/OperitMainScreen.dart';

class OperitApp extends StatelessWidget {
  const OperitApp({super.key, this.startupWebAccessError});

  final String? startupWebAccessError;

  @override
  Widget build(BuildContext context) {
    return OperitTheme(
      child: _AppDialogHost(
        startupWebAccessError: startupWebAccessError,
        child: const OperitMainScreen(),
      ),
    );
  }
}

class _AppDialogHost extends StatefulWidget {
  const _AppDialogHost({
    required this.startupWebAccessError,
    required this.child,
  });

  final String? startupWebAccessError;
  final Widget child;

  @override
  State<_AppDialogHost> createState() => _AppDialogHostState();
}

class _AppDialogHostState extends State<_AppDialogHost> {
  bool _shownStartupWebAccessError = false;
  String _shownPairingId = '';

  @override
  void initState() {
    super.initState();
    FlutterWebAccessServer.instance.addListener(_onWebAccessChanged);
  }

  @override
  void dispose() {
    FlutterWebAccessServer.instance.removeListener(_onWebAccessChanged);
    super.dispose();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _showStartupWebAccessError();
  }

  void _showStartupWebAccessError() {
    final error = widget.startupWebAccessError;
    if (_shownStartupWebAccessError || error == null) {
      return;
    }
    _shownStartupWebAccessError = true;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) {
        return;
      }
      final l10n = AppLocalizations.of(context)!;
      showDialog<void>(
        context: context,
        builder: (context) {
          return AlertDialog(
            title: Text(l10n.settingsWebAccessService),
            content: SingleChildScrollView(
              child: SelectableText(l10n.settingsWebAccessStartFailed(error)),
            ),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: Text(l10n.ok),
              ),
            ],
          );
        },
      );
    });
  }

  void _onWebAccessChanged() {
    final record = FlutterWebAccessServer.instance.lastPairingCode;
    if (record == null || record.pairingId == _shownPairingId) {
      return;
    }
    _shownPairingId = record.pairingId;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) {
        return;
      }
      final l10n = AppLocalizations.of(context)!;
      showDialog<void>(
        context: context,
        builder: (context) {
          return AlertDialog(
            title: Text(l10n.settingsWebAccessPairingRequest),
            content: SelectableText(
              l10n.settingsWebAccessPairingRequestMessage(
                record.pairingCode,
                record.clientDeviceId,
              ),
            ),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: Text(l10n.ok),
              ),
            ],
          );
        },
      );
    });
  }

  @override
  Widget build(BuildContext context) {
    return widget.child;
  }
}
