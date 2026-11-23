// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../../../l10n/generated/app_localizations.dart';
import '../viewmodel/ChatViewModel.dart';
import 'ChatMessageLocatorContent.dart';

/// Searches persisted history independently of the transcript's display window.
class ChatMessageLocatorPanel extends StatefulWidget {
  const ChatMessageLocatorPanel({
    super.key,
    required this.loadEntries,
    required this.onToggleFavorite,
    required this.onSelected,
    required this.onClose,
    this.activeTimestamp,
    this.autofocus = false,
    this.initialFavoritesOnly = false,
  });

  final Future<List<ChatMessageLocatorPreview>> Function(String query)
  loadEntries;
  final Future<void> Function(int timestamp, bool favorite) onToggleFavorite;
  final ValueChanged<int> onSelected;
  final VoidCallback onClose;
  final int? activeTimestamp;
  final bool autofocus;
  final bool initialFavoritesOnly;

  @override
  State<ChatMessageLocatorPanel> createState() =>
      _ChatMessageLocatorPanelState();
}

class _ChatMessageLocatorPanelState extends State<ChatMessageLocatorPanel> {
  final _search = TextEditingController();
  final _favoriteOverrides = <int, bool>{};
  final _savingFavorites = <int>{};
  List<ChatMessageLocatorPreview> _entries = const [];
  Timer? _debounce;
  int _generation = 0;
  bool _favoritesOnly = false;
  bool _loading = true;
  bool _loadFailed = false;
  bool _favoriteFailed = false;

  @override
  void initState() {
    super.initState();
    _favoritesOnly = widget.initialFavoritesOnly;
    _loadEntries();
  }

  @override
  void dispose() {
    _debounce?.cancel();
    _search.dispose();
    super.dispose();
  }

  bool _isFavorite(ChatMessageLocatorPreview entry) =>
      _favoriteOverrides[entry.timestamp] ?? entry.isFavorite;

  void _queryChanged(String _) {
    _debounce?.cancel();
    // Invalidate the in-flight request before the debounce starts.
    _generation++;
    setState(() {
      _loading = true;
      _loadFailed = false;
    });
    _debounce = Timer(const Duration(milliseconds: 220), _loadEntries);
  }

  Future<void> _loadEntries() async {
    _debounce?.cancel();
    final generation = ++_generation;
    setState(() {
      _loading = true;
      _loadFailed = false;
    });
    try {
      final entries = await widget.loadEntries(_search.text.trim());
      if (!mounted || generation != _generation) return;
      setState(() {
        _entries = entries;
        _loading = false;
      });
    } catch (_) {
      if (!mounted || generation != _generation) return;
      setState(() {
        _loading = false;
        _loadFailed = true;
      });
    }
  }

  Future<void> _toggleFavorite(ChatMessageLocatorPreview entry) async {
    final timestamp = entry.timestamp;
    if (_savingFavorites.contains(timestamp)) return;
    final favorite = !_isFavorite(entry);
    setState(() {
      _savingFavorites.add(timestamp);
      _favoriteFailed = false;
    });
    try {
      await widget.onToggleFavorite(timestamp, favorite);
      if (!mounted) return;
      setState(() => _favoriteOverrides[timestamp] = favorite);
    } catch (_) {
      if (!mounted) return;
      setState(() => _favoriteFailed = true);
    } finally {
      if (mounted) setState(() => _savingFavorites.remove(timestamp));
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final entries = _favoritesOnly
        ? _entries.where(_isFavorite).toList(growable: false)
        : _entries;
    return LayoutBuilder(
      builder: (context, constraints) {
        final panel = Material(
          color: colors.surface,
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(20, 12, 8, 8),
                child: Row(
                  children: [
                    Icon(
                      Icons.manage_search_rounded,
                      size: 21,
                      color: colors.primary,
                    ),
                    const SizedBox(width: 10),
                    Expanded(
                      child: Text(
                        l10n.messageLocatorTitle,
                        style: theme.textTheme.titleSmall?.copyWith(
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                    ),
                    IconButton(
                      tooltip: l10n.close,
                      onPressed: widget.onClose,
                      icon: const Icon(Icons.close_rounded, size: 19),
                    ),
                  ],
                ),
              ),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: TextField(
                  key: const ValueKey('message-locator-search'),
                  controller: _search,
                  autofocus: widget.autofocus,
                  textInputAction: TextInputAction.search,
                  onChanged: _queryChanged,
                  onSubmitted: (_) => _loadEntries(),
                  decoration: InputDecoration(
                    hintText: l10n.messageLocatorSearchHint,
                    hintStyle: theme.textTheme.bodyMedium?.copyWith(
                      color: colors.onSurfaceVariant,
                    ),
                    prefixIcon: const Icon(Icons.search_rounded, size: 20),
                    suffixIcon: _search.text.isEmpty
                        ? null
                        : IconButton(
                            tooltip: l10n.clearSearch,
                            onPressed: () {
                              _search.clear();
                              _queryChanged('');
                            },
                            icon: const Icon(Icons.close_rounded, size: 18),
                          ),
                    filled: true,
                    fillColor: colors.surfaceContainerHighest.withValues(
                      alpha: 0.55,
                    ),
                    isDense: true,
                    border: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(12),
                      borderSide: BorderSide.none,
                    ),
                    enabledBorder: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(12),
                      borderSide: BorderSide.none,
                    ),
                    focusedBorder: OutlineInputBorder(
                      borderRadius: BorderRadius.circular(12),
                      borderSide: BorderSide(
                        color: colors.primary.withValues(alpha: 0.5),
                      ),
                    ),
                  ),
                ),
              ),
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 12, 16, 10),
                child: Row(
                  children: [
                    _filter(l10n.messageLocatorAll, false),
                    const SizedBox(width: 6),
                    _filter(l10n.messageLocatorFavorites, true),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        _loading || _loadFailed
                            ? ''
                            : l10n.messageLocatorResultCount(entries.length),
                        textAlign: TextAlign.end,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.labelSmall?.copyWith(
                          color: colors.onSurfaceVariant,
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              if (_favoriteFailed)
                Padding(
                  padding: const EdgeInsets.fromLTRB(20, 0, 20, 8),
                  child: Text(
                    l10n.messageLocatorFavoriteFailed,
                    style: theme.textTheme.bodySmall?.copyWith(
                      color: colors.error,
                    ),
                  ),
                ),
              Divider(
                height: 1,
                color: colors.outlineVariant.withValues(alpha: 0.45),
              ),
              Expanded(
                child: _loading
                    ? const Center(
                        child: SizedBox(
                          width: 22,
                          height: 22,
                          child: CircularProgressIndicator(strokeWidth: 2),
                        ),
                      )
                    : _loadFailed
                    ? _empty(
                        Icons.cloud_off_rounded,
                        l10n.messageLocatorLoadFailed,
                        action: TextButton.icon(
                          onPressed: _loadEntries,
                          icon: const Icon(Icons.refresh_rounded, size: 17),
                          label: Text(l10n.retry),
                        ),
                      )
                    : entries.isEmpty
                    ? _empty(
                        _favoritesOnly
                            ? Icons.star_outline_rounded
                            : Icons.search_off_rounded,
                        _favoritesOnly && _search.text.trim().isEmpty
                            ? l10n.messageLocatorNoFavorites
                            : l10n.messageLocatorNoMatches,
                      )
                    : ListView.separated(
                        key: const ValueKey('message-locator-results'),
                        padding: const EdgeInsets.symmetric(vertical: 6),
                        itemCount: entries.length,
                        separatorBuilder: (_, _) => const SizedBox(height: 2),
                        itemBuilder: (context, index) => _entry(entries[index]),
                      ),
              ),
            ],
          ),
        );
        // Keep controls reachable when a landscape keyboard leaves little room.
        if (constraints.maxHeight < 300) {
          return SingleChildScrollView(
            child: SizedBox(height: 300, child: panel),
          );
        }
        return panel;
      },
    );
  }

  Widget _filter(String label, bool favorites) {
    final colors = Theme.of(context).colorScheme;
    final selected = _favoritesOnly == favorites;
    return ChoiceChip(
      label: Text(label),
      avatar: favorites
          ? Icon(
              Icons.star_rounded,
              size: 15,
              color: selected
                  ? colors.onSecondaryContainer
                  : colors.onSurfaceVariant,
            )
          : null,
      selected: selected,
      showCheckmark: false,
      onSelected: (_) => setState(() => _favoritesOnly = favorites),
      side: BorderSide.none,
      backgroundColor: Colors.transparent,
      visualDensity: VisualDensity.compact,
      labelStyle: TextStyle(
        fontSize: 12,
        color: selected ? colors.onSecondaryContainer : colors.onSurfaceVariant,
      ),
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(8)),
    );
  }

  Widget _empty(IconData icon, String label, {Widget? action}) {
    final theme = Theme.of(context);
    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(
              icon,
              size: 30,
              color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.55),
            ),
            const SizedBox(height: 12),
            Text(
              label,
              textAlign: TextAlign.center,
              style: theme.textTheme.bodySmall?.copyWith(
                color: theme.colorScheme.onSurfaceVariant,
              ),
            ),
            if (action != null) ...[const SizedBox(height: 8), action],
          ],
        ),
      ),
    );
  }

  Widget _entry(ChatMessageLocatorPreview entry) {
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final title = entry.displayName.trim().isEmpty
        ? senderLabel(context, entry.sender)
        : entry.displayName.trim();
    final favorite = _isFavorite(entry);
    final saving = _savingFavorites.contains(entry.timestamp);
    final active = entry.timestamp == widget.activeTimestamp;
    final time = DateTime.fromMillisecondsSinceEpoch(entry.timestamp);
    final localizations = MaterialLocalizations.of(context);
    final timeLabel =
        '${localizations.formatShortDate(time)} ${localizations.formatTimeOfDay(TimeOfDay.fromDateTime(time), alwaysUse24HourFormat: MediaQuery.alwaysUse24HourFormatOf(context))}';
    final preview = buildMessagePreview(l10n, entry, _search.text);
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 8),
      child: Material(
        color: active
            ? colors.primary.withValues(alpha: 0.06)
            : Colors.transparent,
        borderRadius: BorderRadius.circular(10),
        child: Row(
          children: [
            Expanded(
              child: InkWell(
                key: ValueKey('message-locator-entry-${entry.timestamp}'),
                borderRadius: BorderRadius.circular(10),
                onTap: () => widget.onSelected(entry.timestamp),
                child: Padding(
                  padding: const EdgeInsets.fromLTRB(12, 12, 0, 12),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          if (active) ...[
                            Icon(Icons.circle, size: 5, color: colors.primary),
                            const SizedBox(width: 6),
                          ],
                          Expanded(
                            child: Text(
                              title,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: theme.textTheme.labelMedium?.copyWith(
                                fontWeight: FontWeight.w600,
                                color: entry.sender == 'user'
                                    ? colors.primary
                                    : colors.onSurface,
                              ),
                            ),
                          ),
                          const SizedBox(width: 8),
                          Flexible(
                            child: Text(
                              timeLabel,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              textAlign: TextAlign.end,
                              style: theme.textTheme.labelSmall?.copyWith(
                                fontSize: 10,
                                color: colors.onSurfaceVariant,
                              ),
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 5),
                      Text.rich(
                        _highlight(preview),
                        maxLines: 2,
                        overflow: TextOverflow.ellipsis,
                        style: theme.textTheme.bodySmall?.copyWith(
                          height: 1.5,
                          color: colors.onSurfaceVariant,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),
            IconButton(
              key: ValueKey('message-locator-favorite-${entry.timestamp}'),
              tooltip: favorite
                  ? l10n.messageLocatorRemoveFavorite
                  : l10n.messageLocatorAddFavorite,
              onPressed: saving ? null : () => _toggleFavorite(entry),
              icon: saving
                  ? const SizedBox(
                      width: 15,
                      height: 15,
                      child: CircularProgressIndicator(strokeWidth: 1.5),
                    )
                  : Icon(
                      favorite
                          ? Icons.star_rounded
                          : Icons.star_outline_rounded,
                      size: 19,
                      color: favorite
                          ? colors.primary
                          : colors.onSurfaceVariant.withValues(alpha: 0.6),
                    ),
            ),
          ],
        ),
      ),
    );
  }

  TextSpan _highlight(String text) {
    final query = _search.text.trim().toLowerCase();
    if (query.isEmpty) return TextSpan(text: text);
    final normalized = text.toLowerCase();
    final spans = <InlineSpan>[];
    var start = 0;
    while (start < text.length) {
      final match = normalized.indexOf(query, start);
      if (match < 0) {
        spans.add(TextSpan(text: text.substring(start)));
        break;
      }
      if (match > start) {
        spans.add(TextSpan(text: text.substring(start, match)));
      }
      final end = match + query.length;
      spans.add(
        TextSpan(
          text: text.substring(match, end),
          style: TextStyle(
            color: Theme.of(context).colorScheme.primary,
            backgroundColor: Theme.of(
              context,
            ).colorScheme.primary.withValues(alpha: 0.1),
            fontWeight: FontWeight.w600,
          ),
        ),
      );
      start = end;
    }
    return TextSpan(children: spans);
  }
}
