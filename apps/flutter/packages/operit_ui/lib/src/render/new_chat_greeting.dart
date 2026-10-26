import 'package:flutter/material.dart';

/// Settled new-chat greeting paint shared by the client animation and Studio.
/// The caller owns its stage constraints and animation timing.
class ThemeNewChatGreeting extends StatelessWidget {
  const ThemeNewChatGreeting({
    super.key,
    required this.greeting,
    required this.subtitle,
    required this.caretVisible,
    required this.subtitleVisible,
  });

  final String greeting;
  final String subtitle;
  final bool caretVisible;
  final bool subtitleVisible;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Text(
              greeting,
              style: textTheme.headlineSmall!.copyWith(
                color: scheme.primary,
                fontWeight: FontWeight.w600,
                letterSpacing: 2,
              ),
            ),
            const SizedBox(width: 3),
            Opacity(
              opacity: caretVisible ? 1 : 0,
              child: Container(width: 2, height: 26, color: scheme.primary),
            ),
          ],
        ),
        const SizedBox(height: 10),
        Opacity(
          opacity: subtitleVisible ? 1 : 0,
          child: Text(
            subtitle,
            style: textTheme.bodySmall!.copyWith(
              color: scheme.onSurfaceVariant,
            ),
          ),
        ),
      ],
    );
  }
}
