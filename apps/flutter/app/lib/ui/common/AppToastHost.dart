// ignore_for_file: file_names

import 'dart:async';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';

import '../../core/host/AppToastController.dart';
import '../../l10n/generated/app_localizations.dart';
import 'OperitLogoMark.dart';

/// Presents the process-wide notice stack at the top-right of the window.
class AppToastHost extends StatefulWidget {
  /// Creates the corner-notice overlay around [child].
  const AppToastHost({super.key, required this.child});

  final Widget child;

  /// Creates the state that tracks entrance and exit of each notice.
  @override
  State<AppToastHost> createState() => _AppToastHostState();
}

class _NoticeEntry {
  /// Creates a tracked copy of one published notice.
  _NoticeEntry(this.notice);

  AppNotice notice;
  bool leaving = false;
}

class _AppToastHostState extends State<AppToastHost> {
  final List<_NoticeEntry> _entries = <_NoticeEntry>[];
  final Map<int, Timer> _exitTimers = <int, Timer>{};
  bool _syncScheduled = false;

  /// Starts mirroring the process-wide notice stack.
  @override
  void initState() {
    super.initState();
    AppToastController.instance.notices.addListener(_sync);
    _apply(rebuild: false);
  }

  /// Stops mirroring notices and releases exit timers.
  @override
  void dispose() {
    AppToastController.instance.notices.removeListener(_sync);
    for (final timer in _exitTimers.values) {
      timer.cancel();
    }
    super.dispose();
  }

  /// Mirrors changes after build and disposal have released the widget tree.
  void _sync() {
    if (!mounted || _syncScheduled) {
      return;
    }
    if (SchedulerBinding.instance.schedulerPhase ==
        SchedulerPhase.persistentCallbacks) {
      _syncScheduled = true;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        _syncScheduled = false;
        if (mounted) {
          _apply(rebuild: true);
        }
      });
      return;
    }
    _apply(rebuild: true);
  }

  /// Copies the controller stack and marks removed notices as leaving.
  void _apply({required bool rebuild}) {
    final current = AppToastController.instance.notices.value;
    final currentIds = <int>{for (final notice in current) notice.id};
    for (final entry in _entries) {
      if (!currentIds.contains(entry.notice.id) && !entry.leaving) {
        entry.leaving = true;
        _exitTimers[entry.notice.id] = Timer(
          const Duration(milliseconds: 240),
          () => _drop(entry.notice.id),
        );
      }
    }
    final known = <int>{for (final entry in _entries) entry.notice.id};
    for (final notice in current.reversed) {
      if (known.contains(notice.id)) {
        continue;
      }
      _entries.insert(0, _NoticeEntry(notice));
    }
    for (final notice in current) {
      for (final entry in _entries) {
        if (entry.notice.id == notice.id) {
          entry.notice = notice;
        }
      }
    }
    _entries.sort((left, right) {
      final leftIndex = _stackIndex(current, left.notice.id);
      final rightIndex = _stackIndex(current, right.notice.id);
      return leftIndex.compareTo(rightIndex);
    });
    if (rebuild && mounted) {
      setState(() {});
    }
  }

  /// Removes one notice after its exit animation.
  void _drop(int id) {
    _exitTimers.remove(id)?.cancel();
    if (!mounted) {
      return;
    }
    setState(() {
      _entries.removeWhere((entry) => entry.notice.id == id);
    });
  }

  /// Builds the window content with the top-right notice column above it.
  @override
  Widget build(BuildContext context) {
    // Keep the application's parent unchanged across notice entry and exit.
    // Reparenting it here disposes background media and the shared glass scene.
    return Stack(
      fit: StackFit.expand,
      children: <Widget>[
        widget.child,
        // This host is mounted above the navigator. The column carries its
        // own overlay so close-button tooltips have an ancestor to attach to.
        if (_entries.isNotEmpty)
          Positioned.fill(child: Overlay.wrap(child: _noticeColumn(context))),
      ],
    );
  }

  /// Places the live notice column at the top-right of the window.
  Widget _noticeColumn(BuildContext context) {
    final media = MediaQuery.of(context);
    final width = _noticeWidth(media);
    return Stack(
      children: <Widget>[
        Positioned(
          top: media.padding.top + 12,
          right: media.padding.right + 12,
          width: width,
          child: ConstrainedBox(
            constraints: BoxConstraints(maxHeight: _stackHeight(media)),
            child: ListView(
              shrinkWrap: true,
              primary: false,
              padding: const EdgeInsets.fromLTRB(16, 0, 0, 16),
              clipBehavior: Clip.none,
              children: <Widget>[
                for (final entry in _entries)
                  Padding(
                    padding: const EdgeInsets.only(bottom: 10),
                    child: _MacNoticeTile(
                      key: ValueKey<int>(entry.notice.id),
                      notice: entry.notice,
                      leaving: entry.leaving,
                      onDismiss: () =>
                          AppToastController.instance.dismiss(entry.notice.id),
                    ),
                  ),
              ],
            ),
          ),
        ),
      ],
    );
  }
}

/// Finds a notice position, keeping exiting notices after the live stack.
int _stackIndex(List<AppNotice> current, int id) {
  for (var index = 0; index < current.length; index++) {
    if (current[index].id == id) {
      return index;
    }
  }
  return current.length + id;
}

/// Resolves the notice width inside the window.
double _noticeWidth(MediaQueryData media) {
  final available = media.size.width - media.padding.horizontal - 24;
  return available < 344 ? available : 344;
}

/// Resolves the vertical room available to the notice stack.
double _stackHeight(MediaQueryData media) {
  return media.size.height - media.padding.vertical - 24;
}

class _MacNoticeTile extends StatefulWidget {
  /// Creates one animated corner notice.
  const _MacNoticeTile({
    super.key,
    required this.notice,
    required this.leaving,
    required this.onDismiss,
  });

  final AppNotice notice;
  final bool leaving;
  final VoidCallback onDismiss;

  /// Creates the state that plays the entrance slide.
  @override
  State<_MacNoticeTile> createState() => _MacNoticeTileState();
}

class _MacNoticeTileState extends State<_MacNoticeTile> {
  bool _entered = false;
  bool _expanded = false;
  bool _hovered = false;

  /// Keeps a notice alive while its full message is being read.
  void _setExpanded(bool expanded) {
    setState(() => _expanded = expanded);
    _updateLifetime();
  }

  void _updateLifetime() {
    final controller = AppToastController.instance;
    if (_expanded || _hovered) {
      controller.pause(widget.notice.id);
    } else {
      controller.resume(widget.notice.id);
    }
  }

  /// Schedules the slide-in on the frame after insertion.
  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        setState(() => _entered = true);
      }
    });
  }

  /// Slides the notice in from the right and back out when it leaves.
  @override
  Widget build(BuildContext context) {
    final visible = _entered && !widget.leaving;
    return MouseRegion(
      onEnter: (_) {
        _hovered = true;
        _updateLifetime();
      },
      onExit: (_) {
        _hovered = false;
        _updateLifetime();
      },
      child: AnimatedSlide(
        offset: visible ? Offset.zero : const Offset(1.08, 0),
        duration: const Duration(milliseconds: 240),
        curve: Curves.easeOutCubic,
        child: AnimatedOpacity(
          opacity: visible ? 1 : 0,
          duration: const Duration(milliseconds: 180),
          child: IgnorePointer(
            ignoring: !visible,
            child: _MacNoticeCard(
              notice: widget.notice,
              onDismiss: widget.onDismiss,
              expanded: _expanded,
              onExpandedChanged: _setExpanded,
            ),
          ),
        ),
      ),
    );
  }
}

class _MacNoticeCard extends StatelessWidget {
  /// Creates the frosted card for one notice.
  const _MacNoticeCard({
    required this.notice,
    required this.onDismiss,
    required this.expanded,
    required this.onExpandedChanged,
  });

  final AppNotice notice;
  final VoidCallback onDismiss;
  final bool expanded;
  final ValueChanged<bool> onExpandedChanged;

  /// Builds the macOS-style notice card.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final brightness = theme.brightness;
    final onSurface = theme.colorScheme.onSurface;
    final fill = brightness == Brightness.dark
        ? const Color(0xF22A2A2C)
        : const Color(0xF2F6F6F7);
    final border = brightness == Brightness.dark
        ? const Color(0x33FFFFFF)
        : const Color(0x66FFFFFF);
    final iconWell = brightness == Brightness.dark
        ? const Color(0xFF1C1C1E)
        : Colors.white;
    final bodyStyle = theme.textTheme.bodyMedium?.copyWith(
      color: onSurface,
      fontSize: 13,
      height: 1.3,
    );
    return DecoratedBox(
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(16),
        boxShadow: const <BoxShadow>[
          BoxShadow(
            color: Color(0x33000000),
            blurRadius: 28,
            offset: Offset(0, 12),
          ),
          BoxShadow(
            color: Color(0x14000000),
            blurRadius: 3,
            offset: Offset(0, 1),
          ),
        ],
      ),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(16),
        child: BackdropFilter(
          filter: ImageFilter.blur(sigmaX: 22, sigmaY: 22),
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: fill,
              borderRadius: BorderRadius.circular(16),
              border: Border.all(color: border),
            ),
            child: Padding(
              padding: const EdgeInsets.fromLTRB(12, 10, 8, 10),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  DecoratedBox(
                    decoration: BoxDecoration(
                      color: iconWell,
                      borderRadius: BorderRadius.circular(9),
                    ),
                    child: const SizedBox.square(
                      dimension: 36,
                      child: Center(child: OperitLogoMark(size: 22)),
                    ),
                  ),
                  const SizedBox(width: 10),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text(
                          'Operit',
                          style: theme.textTheme.labelMedium?.copyWith(
                            color: onSurface.withValues(alpha: 0.62),
                            fontSize: 12,
                            fontWeight: FontWeight.w600,
                            height: 1.2,
                          ),
                        ),
                        const SizedBox(height: 2),
                        _NoticeBody(
                          notice: notice,
                          style: bodyStyle,
                          expanded: expanded,
                          onExpandedChanged: onExpandedChanged,
                        ),
                        if (notice.actionLabel != null)
                          TextButton(
                            onPressed: () {
                              notice.onAction!.call();
                              onDismiss();
                            },
                            style: TextButton.styleFrom(
                              visualDensity: VisualDensity.compact,
                              padding: const EdgeInsets.symmetric(
                                horizontal: 8,
                              ),
                              minimumSize: const Size(0, 32),
                              tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                            ),
                            child: Text(notice.actionLabel!),
                          ),
                      ],
                    ),
                  ),
                  IconButton(
                    onPressed: onDismiss,
                    icon: const Icon(Icons.close),
                    iconSize: 14,
                    visualDensity: VisualDensity.compact,
                    constraints: const BoxConstraints.tightFor(
                      width: 22,
                      height: 22,
                    ),
                    padding: EdgeInsets.zero,
                    tooltip: MaterialLocalizations.of(
                      context,
                    ).closeButtonTooltip,
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _NoticeBody extends StatelessWidget {
  const _NoticeBody({
    required this.notice,
    required this.style,
    required this.expanded,
    required this.onExpandedChanged,
  });

  final AppNotice notice;
  final TextStyle? style;
  final bool expanded;
  final ValueChanged<bool> onExpandedChanged;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final painter = TextPainter(
          text: TextSpan(text: notice.text, style: style),
          textDirection: Directionality.of(context),
          textScaler: MediaQuery.textScalerOf(context),
        )..layout(maxWidth: constraints.maxWidth);
        final needsExpansion = painter.height > _maxBodyHeight;
        painter.dispose();
        final text = notice.selectable || expanded
            ? SelectableText(notice.text, style: style)
            : Text(notice.text, style: style);
        if (!needsExpansion) {
          return text;
        }
        final l10n = AppLocalizations.of(context);
        final useChinese = Localizations.localeOf(context).languageCode == 'zh';
        final label = expanded
            ? l10n?.noticeCollapse ?? (useChinese ? '收起' : 'Collapse')
            : l10n?.noticeExpand ?? (useChinese ? '展开全文' : 'Expand');
        final expandedHeight = (MediaQuery.sizeOf(context).height * 0.5).clamp(
          _maxBodyHeight,
          420.0,
        );
        return Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            if (expanded)
              ConstrainedBox(
                constraints: BoxConstraints(maxHeight: expandedHeight),
                child: SingleChildScrollView(child: text),
              )
            else
              ClipRect(
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxHeight: _maxBodyHeight),
                  child: Text(
                    notice.text,
                    style: style,
                    maxLines: 6,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
              ),
            TextButton.icon(
              onPressed: () => onExpandedChanged(!expanded),
              icon: Icon(expanded ? Icons.expand_less : Icons.expand_more),
              label: Text(label),
              style: TextButton.styleFrom(
                visualDensity: VisualDensity.compact,
                padding: const EdgeInsets.symmetric(horizontal: 4),
                minimumSize: const Size(0, 32),
                tapTargetSize: MaterialTapTargetSize.shrinkWrap,
              ),
            ),
          ],
        );
      },
    );
  }
}

const double _maxBodyHeight = 140;
