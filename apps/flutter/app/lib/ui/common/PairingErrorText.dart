// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../l10n/generated/app_localizations.dart';

/// Shows a pairing error, adding firewall guidance for connection timeouts.
class PairingErrorText extends StatelessWidget {
  const PairingErrorText({super.key, required this.error, this.style});

  final String error;
  final TextStyle? style;

  @override
  Widget build(BuildContext context) {
    final base = style ?? DefaultTextStyle.of(context).style;
    final isTimeout = error.contains('PAIRING_TIMEOUT') ||
        error.contains('PEER_CONNECT_TIMEOUT');
    if (!isTimeout) return Text(error, style: base);
    final fontSize =
        base.fontSize ?? DefaultTextStyle.of(context).style.fontSize ?? 14;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(AppLocalizations.of(context)!.peerPairTimeoutHint, style: base),
        const SizedBox(height: 6),
        Text(error, style: base.copyWith(fontSize: fontSize * 0.85)),
      ],
    );
  }
}