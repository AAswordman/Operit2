// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';
import '../chrome/WorkspaceBrowserPopupWidgets.dart';
import 'WorkspaceBrowserHistoryStore.dart';

class WorkspaceBrowserHistorySheet extends StatefulWidget {
  const WorkspaceBrowserHistorySheet({
    super.key,
    required this.store,
    required this.onOpen,
    required this.onChanged,
  });

  final WorkspaceBrowserHistoryStore store;
  final ValueChanged<String> onOpen;
  final VoidCallback onChanged;

  @override
  State<WorkspaceBrowserHistorySheet> createState() =>
      _WorkspaceBrowserHistorySheetState();
}

class _WorkspaceBrowserHistorySheetState
    extends State<WorkspaceBrowserHistorySheet> {
  String _query = '';

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final items = widget.store.search(_query);
    final materialL10n = MaterialLocalizations.of(context);
    return WorkspaceBrowserPopupBody(
      children: <Widget>[
        WorkspaceBrowserPopupHeader(
          title: l10n.history,
          trailing: IconButton(
            tooltip: l10n.clear,
            onPressed: items.isEmpty
                ? null
                : () {
                    widget.store.clear();
                    widget.onChanged();
                    setState(() {});
                  },
            icon: const Icon(Icons.delete_sweep_outlined, size: 18),
            visualDensity: VisualDensity.compact,
            constraints: const BoxConstraints.tightFor(width: 32, height: 32),
            padding: EdgeInsets.zero,
          ),
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(12, 0, 12, 6),
          child: SizedBox(
            height: 34,
            child: TextField(
              onChanged: (value) => setState(() => _query = value),
              style: theme.textTheme.bodySmall,
              textAlignVertical: TextAlignVertical.center,
              decoration: InputDecoration(
                prefixIcon: const Icon(Icons.search, size: 17),
                prefixIconConstraints: const BoxConstraints(
                  minWidth: 32,
                  minHeight: 32,
                ),
                hintText: l10n.searchHistory,
                isDense: true,
                contentPadding: const EdgeInsets.symmetric(
                  horizontal: 10,
                  vertical: 8,
                ),
                border: OutlineInputBorder(
                  borderRadius: BorderRadius.circular(8),
                ),
              ),
            ),
          ),
        ),
        if (items.isEmpty)
          WorkspaceBrowserPopupEmpty(icon: Icons.history, text: l10n.history)
        else
          for (final item in items)
            WorkspaceBrowserPopupRow(
              icon: Icons.history,
              title: item.title,
              subtitle: item.url,
              detail:
                  '${materialL10n.formatMediumDate(item.visitedAt)} '
                  '${materialL10n.formatTimeOfDay(TimeOfDay.fromDateTime(item.visitedAt))}',
              onTap: () => widget.onOpen(item.url),
            ),
      ],
    );
  }
}
