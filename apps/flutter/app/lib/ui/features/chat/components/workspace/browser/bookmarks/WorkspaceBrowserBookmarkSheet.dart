// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';
import '../chrome/WorkspaceBrowserPopupWidgets.dart';
import 'WorkspaceBrowserBookmarkStore.dart';

class WorkspaceBrowserBookmarkSheet extends StatelessWidget {
  const WorkspaceBrowserBookmarkSheet({
    super.key,
    required this.store,
    required this.onOpen,
    required this.onChanged,
  });

  final WorkspaceBrowserBookmarkStore store;
  final ValueChanged<String> onOpen;
  final VoidCallback onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final items = store.items;
    return WorkspaceBrowserPopupBody(
      children: <Widget>[
        WorkspaceBrowserPopupHeader(title: l10n.bookmarks),
        if (items.isEmpty)
          WorkspaceBrowserPopupEmpty(
            icon: Icons.bookmark,
            text: l10n.noBookmarks,
          )
        else
          for (final item in items)
            WorkspaceBrowserPopupRow(
              icon: Icons.bookmark,
              title: item.title,
              subtitle: item.url,
              onTap: () => onOpen(item.url),
              trailing: IconButton(
                tooltip: l10n.delete,
                onPressed: () {
                  store.remove(item.url);
                  onChanged();
                },
                icon: const Icon(Icons.close, size: 17),
                visualDensity: VisualDensity.compact,
                constraints: const BoxConstraints.tightFor(
                  width: 30,
                  height: 30,
                ),
                padding: EdgeInsets.zero,
              ),
            ),
      ],
    );
  }
}
