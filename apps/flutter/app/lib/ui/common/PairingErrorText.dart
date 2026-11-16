// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../core/link/CoreLinkProtocol.dart';
import '../../l10n/generated/app_localizations.dart';

/// Shows a pairing error, adding firewall guidance for connection timeouts.
class PairingErrorText extends StatelessWidget {
  const PairingErrorText({super.key, required this.error, this.style});

  final Object error;
  final TextStyle? style;

  @override
  Widget build(BuildContext context) {
    final base = style ?? DefaultTextStyle.of(context).style;
    final err = error;
    final text = err.toString();
    if (err is! CoreLinkError || !err.isPairingTimeout) {
      return Text(text, style: base);
    }
    final fontSize =
        base.fontSize ?? DefaultTextStyle.of(context).style.fontSize ?? 14;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: [
        Text(AppLocalizations.of(context)!.peerPairTimeoutHint, style: base),
        const SizedBox(height: 6),
        Text(text, style: base.copyWith(fontSize: fontSize * 0.85)),
      ],
    );
  }
}
