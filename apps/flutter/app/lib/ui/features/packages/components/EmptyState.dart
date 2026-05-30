// ignore_for_file: file_names

import 'package:flutter/material.dart';

class EmptyState extends StatelessWidget {
  const EmptyState({
    super.key,
    required this.icon,
    required this.title,
    required this.message,
    this.action,
    this.scrollable = true,
  });

  final IconData icon;
  final String title;
  final String message;
  final Widget? action;
  final bool scrollable;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final children = <Widget>[
      SizedBox(height: MediaQuery.sizeOf(context).height * 0.18),
      Icon(icon, size: 54, color: colorScheme.onSurfaceVariant),
      const SizedBox(height: 16),
      Center(
        child: Text(
          title,
          style: Theme.of(
            context,
          ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
        ),
      ),
      const SizedBox(height: 8),
      Padding(
        padding: const EdgeInsets.symmetric(horizontal: 32),
        child: Text(
          message,
          textAlign: TextAlign.center,
          style: Theme.of(
            context,
          ).textTheme.bodyMedium?.copyWith(color: colorScheme.onSurfaceVariant),
        ),
      ),
      if (action != null) ...<Widget>[
        const SizedBox(height: 16),
        Center(child: action),
      ],
    ];
    if (!scrollable) {
      return Column(children: children);
    }
    return ListView(
      physics: const AlwaysScrollableScrollPhysics(),
      children: children,
    );
  }
}
