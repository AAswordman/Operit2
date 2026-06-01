// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';
import '../tabs/WorkspaceBrowserTabModels.dart';

class WorkspaceBrowserUrlBar extends StatefulWidget {
  const WorkspaceBrowserUrlBar({
    super.key,
    required this.tab,
    required this.isBookmarked,
    required this.onSubmitted,
    required this.onToggleBookmark,
    required this.onBack,
    required this.onForward,
    required this.onRefreshOrStop,
    required this.onOpenMenu,
    this.menuButtonKey,
  });

  final WorkspaceBrowserTabState tab;
  final bool isBookmarked;
  final ValueChanged<String> onSubmitted;
  final VoidCallback onToggleBookmark;
  final VoidCallback onBack;
  final VoidCallback onForward;
  final VoidCallback onRefreshOrStop;
  final VoidCallback onOpenMenu;
  final Key? menuButtonKey;

  @override
  State<WorkspaceBrowserUrlBar> createState() => _WorkspaceBrowserUrlBarState();
}

class _WorkspaceBrowserUrlBarState extends State<WorkspaceBrowserUrlBar> {
  final FocusNode _focusNode = FocusNode();
  bool _isEditing = false;

  @override
  void dispose() {
    _focusNode.dispose();
    super.dispose();
  }

  void _startEditing() {
    setState(() => _isEditing = true);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted) {
        return;
      }
      _focusNode.requestFocus();
      final controller = widget.tab.addressController.controller;
      controller.selection = TextSelection(
        baseOffset: 0,
        extentOffset: controller.text.length,
      );
    });
  }

  void _submit() {
    final text = widget.tab.addressController.text;
    setState(() => _isEditing = false);
    widget.onSubmitted(text);
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final l10n = AppLocalizations.of(context)!;
    final tab = widget.tab;
    final urlText = tab.url.trim().isEmpty
        ? (tab.title.trim().isEmpty ? 'about:blank' : tab.title)
        : tab.url;
    return Material(
      color: theme.colorScheme.surface,
      elevation: 4,
      shadowColor: theme.colorScheme.shadow.withValues(alpha: 0.12),
      child: SafeArea(
        top: false,
        bottom: false,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(12, 8, 12, 8),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Row(
                children: <Widget>[
                  if (!_isEditing) ...<Widget>[
                    _ToolbarIconButton(
                      tooltip: l10n.back,
                      icon: Icons.arrow_back,
                      onPressed: tab.canGoBack ? widget.onBack : null,
                    ),
                    _ToolbarIconButton(
                      tooltip: l10n.forward,
                      icon: Icons.arrow_forward,
                      onPressed: tab.canGoForward ? widget.onForward : null,
                    ),
                    _ToolbarIconButton(
                      tooltip: tab.isLoading ? l10n.stop : l10n.refresh,
                      icon: tab.isLoading ? Icons.close : Icons.refresh,
                      onPressed: widget.onRefreshOrStop,
                    ),
                    const SizedBox(width: 6),
                  ],
                  Expanded(
                    child: _AddressSurface(
                      isEditing: _isEditing,
                      child: _isEditing
                          ? Row(
                              children: <Widget>[
                                const SizedBox(width: 11),
                                Icon(
                                  tab.url.startsWith('https://')
                                      ? Icons.lock
                                      : Icons.language,
                                  size: 18,
                                  color: theme.colorScheme.onSurfaceVariant,
                                ),
                                const SizedBox(width: 7),
                                Expanded(
                                  child: TextField(
                                    controller:
                                        tab.addressController.controller,
                                    focusNode: _focusNode,
                                    minLines: 1,
                                    maxLines: 1,
                                    textInputAction: TextInputAction.go,
                                    onSubmitted: (_) => _submit(),
                                    decoration: const InputDecoration(
                                      isDense: true,
                                      border: InputBorder.none,
                                      contentPadding: EdgeInsets.symmetric(
                                        vertical: 7,
                                      ),
                                    ),
                                    style: theme.textTheme.bodyMedium?.copyWith(
                                      fontWeight: FontWeight.w600,
                                    ),
                                  ),
                                ),
                                IconButton(
                                  tooltip: l10n.open,
                                  onPressed: _submit,
                                  icon: const Icon(Icons.check, size: 20),
                                  visualDensity: VisualDensity.compact,
                                  constraints: const BoxConstraints.tightFor(
                                    width: 30,
                                    height: 30,
                                  ),
                                  padding: EdgeInsets.zero,
                                ),
                              ],
                            )
                          : InkWell(
                              borderRadius: BorderRadius.circular(20),
                              onTap: _startEditing,
                              child: Padding(
                                padding: const EdgeInsets.symmetric(
                                  horizontal: 11,
                                  vertical: 6,
                                ),
                                child: Row(
                                  children: <Widget>[
                                    Icon(
                                      tab.url.startsWith('https://')
                                          ? Icons.lock
                                          : Icons.language,
                                      size: 18,
                                      color: theme.colorScheme.onSurfaceVariant,
                                    ),
                                    const SizedBox(width: 7),
                                    Expanded(
                                      child: Text(
                                        urlText,
                                        maxLines: 1,
                                        overflow: TextOverflow.ellipsis,
                                        style: theme.textTheme.bodyMedium
                                            ?.copyWith(
                                              fontWeight: FontWeight.w600,
                                            ),
                                      ),
                                    ),
                                    IconButton(
                                      tooltip: widget.isBookmarked
                                          ? l10n.removeBookmark
                                          : l10n.addBookmark,
                                      onPressed: widget.onToggleBookmark,
                                      icon: Icon(
                                        widget.isBookmarked
                                            ? Icons.star
                                            : Icons.star_border,
                                        size: 20,
                                      ),
                                      color: widget.isBookmarked
                                          ? theme.colorScheme.primary
                                          : theme.colorScheme.onSurfaceVariant,
                                      visualDensity: VisualDensity.compact,
                                      constraints:
                                          const BoxConstraints.tightFor(
                                            width: 30,
                                            height: 30,
                                          ),
                                      padding: EdgeInsets.zero,
                                    ),
                                  ],
                                ),
                              ),
                            ),
                    ),
                  ),
                  if (!_isEditing)
                    _ToolbarIconButton(
                      key: widget.menuButtonKey,
                      tooltip: l10n.menu,
                      icon: Icons.more_horiz,
                      onPressed: widget.onOpenMenu,
                    ),
                ],
              ),
              if (tab.isLoading)
                Align(
                  alignment: Alignment.centerLeft,
                  child: Container(
                    width: _isEditing ? double.infinity : 72,
                    height: 2,
                    margin: const EdgeInsets.only(top: 6),
                    decoration: BoxDecoration(
                      color: theme.colorScheme.primary,
                      borderRadius: BorderRadius.circular(999),
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

class _AddressSurface extends StatelessWidget {
  const _AddressSurface({required this.isEditing, required this.child});

  final bool isEditing;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    if (isEditing) {
      return ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 36),
        child: DecoratedBox(
          decoration: BoxDecoration(
            border: Border(
              bottom: BorderSide(color: theme.colorScheme.outlineVariant),
            ),
          ),
          child: child,
        ),
      );
    }
    return Material(
      color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.86),
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(18),
        side: BorderSide(
          color: isEditing
              ? theme.colorScheme.outlineVariant.withValues(alpha: 0.9)
              : theme.colorScheme.outlineVariant.withValues(alpha: 0.55),
        ),
      ),
      clipBehavior: Clip.antiAlias,
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 36),
        child: child,
      ),
    );
  }
}

class _ToolbarIconButton extends StatelessWidget {
  const _ToolbarIconButton({
    super.key,
    required this.tooltip,
    required this.icon,
    required this.onPressed,
  });

  final String tooltip;
  final IconData icon;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    return IconButton(
      tooltip: tooltip,
      onPressed: onPressed,
      icon: Icon(icon, size: 20),
      padding: EdgeInsets.zero,
      visualDensity: VisualDensity.compact,
      constraints: const BoxConstraints.tightFor(width: 34, height: 34),
    );
  }
}
