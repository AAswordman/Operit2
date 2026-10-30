// Frozen pre-plugin UI oracle; never imported by production.
// ignore_for_file: file_names

import 'package:flutter/material.dart';

import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/theme/OperitGlassSurface.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';
import 'WorkspaceDiffContent.dart';
import 'WorkspaceDiffReview.dart';
import 'package:operit2/ui/features/chat/components/part/FileDiffDisplay.dart';
import 'package:operit2/ui/features/chat/components/part/ToolDisplayComponents.dart'
    show fileDiffsForResult, decodeToolMetadata, toolDisplayFromAttributes;

/// Collects the file diffs applied by one completed assistant turn.
WorkspaceDiffReview reviewForMessage(ChatUiMessage message) {
  final merged = <String, FileDiff>{};
  final order = <String>[];
  for (final part in message.parts) {
    if (part.kind != core_proxy.MessagePartKind.toolResult) {
      continue;
    }
    for (final diff in fileDiffsForResult(
      part.content,
      decodeToolMetadata(part.attributes['result_data']),
      display: toolDisplayFromAttributes(part.attributes),
    )) {
      final previous = merged[diff.path];
      if (previous == null) {
        order.add(diff.path);
        merged[diff.path] = diff;
      } else {
        merged[diff.path] = FileDiff(
          path: diff.path,
          details: diff.details,
          diffContent: '${previous.diffContent}\n${diff.diffContent}',
        );
      }
    }
  }
  return WorkspaceDiffReview(
    files: <WorkspaceDiffFile>[
      for (final path in order)
        WorkspaceDiffFile(
          path: path,
          diffContent: merged[path]!.diffContent,
          additions: _countDiffLines(merged[path]!.diffContent, added: true),
          deletions: _countDiffLines(merged[path]!.diffContent, added: false),
        ),
    ],
  );
}

/// Collects the file diffs applied across all assistant messages in a conversation.
WorkspaceDiffReview reviewForMessages(Iterable<ChatUiMessage> messages) {
  final merged = <String, FileDiff>{};
  final order = <String>[];
  for (final message in messages) {
    if (message.sender != 'ai') {
      continue;
    }
    for (final part in message.parts) {
      if (part.kind != core_proxy.MessagePartKind.toolResult) {
        continue;
      }
      for (final diff in fileDiffsForResult(
        part.content,
        decodeToolMetadata(part.attributes['result_data']),
        display: toolDisplayFromAttributes(part.attributes),
      )) {
        final previous = merged[diff.path];
        if (previous == null) {
          order.add(diff.path);
          merged[diff.path] = diff;
        } else {
          merged[diff.path] = FileDiff(
            path: diff.path,
            details: diff.details,
            diffContent: '${previous.diffContent}\n${diff.diffContent}',
          );
        }
      }
    }
  }
  return WorkspaceDiffReview(
    files: <WorkspaceDiffFile>[
      for (final path in order)
        WorkspaceDiffFile(
          path: path,
          diffContent: merged[path]!.diffContent,
          additions: _countDiffLines(merged[path]!.diffContent, added: true),
          deletions: _countDiffLines(merged[path]!.diffContent, added: false),
        ),
    ],
  );
}

/// Finds the most recent assistant message review containing file diffs.
WorkspaceDiffReview? latestReviewInMessages(Iterable<ChatUiMessage> messages) {
  for (final message in messages.toList().reversed) {
    if (message.sender == 'ai') {
      final review = reviewForMessage(message);
      if (review.files.isNotEmpty) {
        return review;
      }
    }
  }
  return null;
}

int _countDiffLines(String diffContent, {required bool added}) {
  final prefix = added ? '+' : '-';
  final skipped = added ? '+++' : '---';
  var count = 0;
  for (final line in diffContent.split('\n')) {
    if (line.startsWith(prefix) && !line.startsWith(skipped)) {
      count += 1;
    }
  }
  return count;
}

/// Shows the files changed by one finished assistant turn.
class TurnDiffSummary extends StatefulWidget {
  const TurnDiffSummary({
    super.key,
    required this.review,
    required this.onUndo,
    this.onOpenReview,
  });

  final WorkspaceDiffReview review;
  final VoidCallback onUndo;
  final ValueChanged<WorkspaceDiffReview>? onOpenReview;

  @override
  State<TurnDiffSummary> createState() => _TurnDiffSummaryState();
}

class _TurnDiffSummaryState extends State<TurnDiffSummary> {
  bool _filesExpanded = true;
  final ScrollController _scrollController = ScrollController();

  @override
  void dispose() {
    _scrollController.dispose();
    super.dispose();
  }

  void _openSingleFileDiff(WorkspaceDiffFile file) {
    widget.onOpenReview?.call(
      WorkspaceDiffReview(
        files: <WorkspaceDiffFile>[
          file,
          ...widget.review.files.where((item) => item.path != file.path),
        ],
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final review = widget.review;

    final cardBorderColor = colorScheme.outline.withValues(alpha: 0.25);
    final dividerLineColor = colorScheme.outline.withValues(alpha: 0.15);

    return Padding(
      padding: const EdgeInsets.fromLTRB(12, 6, 12, 4),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerLowest,
        borderRadius: BorderRadius.circular(10),
        border: Border.all(color: cardBorderColor, width: 0.8),
        layer: OperitGlassSurfaceLayer.card,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: <Widget>[
            // Integrated Header
            Material(
              color: Colors.transparent,
              child: InkWell(
                onTap: () => setState(() => _filesExpanded = !_filesExpanded),
                borderRadius: BorderRadius.vertical(
                  top: const Radius.circular(10),
                  bottom: Radius.circular(_filesExpanded ? 0 : 10),
                ),
                child: Padding(
                  padding: const EdgeInsets.symmetric(
                    horizontal: 10,
                    vertical: 7,
                  ),
                  child: Row(
                    children: <Widget>[
                      // Left icon container
                      Container(
                        padding: const EdgeInsets.all(4.5),
                        decoration: BoxDecoration(
                          color: colorScheme.primary.withValues(alpha: 0.12),
                          borderRadius: BorderRadius.circular(6),
                        ),
                        child: Icon(
                          Icons.difference_outlined,
                          size: 14,
                          color: colorScheme.primary,
                        ),
                      ),
                      const SizedBox(width: 8),
                      // Title
                      Text(
                        l10n.toolEditedFileCount(review.files.length),
                        style: theme.textTheme.labelMedium?.copyWith(
                          fontWeight: FontWeight.w600,
                          color: colorScheme.onSurface,
                          letterSpacing: 0.1,
                        ),
                      ),
                      const SizedBox(width: 8),
                      // Stats Capsule
                      _StatCapsule(
                        additions: review.additions,
                        deletions: review.deletions,
                      ),
                      const Spacer(),
                      // Undo action button
                      TextButton.icon(
                        onPressed: widget.onUndo,
                        icon: const Icon(Icons.undo_rounded, size: 14),
                        label: Text(
                          l10n.workspaceUndo,
                          style: const TextStyle(fontSize: 11.5),
                        ),
                        style: TextButton.styleFrom(
                          foregroundColor: colorScheme.onSurfaceVariant
                              .withValues(alpha: 0.8),
                          padding: const EdgeInsets.symmetric(
                            horizontal: 8,
                            vertical: 4,
                          ),
                          minimumSize: Size.zero,
                          tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                          visualDensity: VisualDensity.compact,
                        ),
                      ),
                      const SizedBox(width: 4),
                      // View Changes action pill
                      Material(
                        color: Colors.transparent,
                        child: InkWell(
                          onTap: () {
                            widget.onOpenReview?.call(review);
                          },
                          borderRadius: BorderRadius.circular(16),
                          child: Container(
                            padding: const EdgeInsets.symmetric(
                              horizontal: 9,
                              vertical: 3.5,
                            ),
                            decoration: BoxDecoration(
                              color: colorScheme.primary.withValues(
                                alpha: 0.12,
                              ),
                              borderRadius: BorderRadius.circular(16),
                              border: Border.all(
                                color: colorScheme.primary.withValues(
                                  alpha: 0.35,
                                ),
                                width: 0.8,
                              ),
                            ),
                            child: Row(
                              mainAxisSize: MainAxisSize.min,
                              children: <Widget>[
                                Text(
                                  l10n.toolViewChanges,
                                  style: TextStyle(
                                    fontSize: 11,
                                    fontWeight: FontWeight.w500,
                                    color: colorScheme.primary,
                                  ),
                                ),
                                const SizedBox(width: 2),
                                Icon(
                                  Icons.arrow_forward_rounded,
                                  size: 11,
                                  color: colorScheme.primary,
                                ),
                              ],
                            ),
                          ),
                        ),
                      ),
                      const SizedBox(width: 4),
                      // Collapse/Expand chevron indicator
                      Padding(
                        padding: const EdgeInsets.all(2),
                        child: AnimatedRotation(
                          turns: _filesExpanded ? 0.5 : 0.0,
                          duration: const Duration(milliseconds: 200),
                          curve: Curves.easeOutCubic,
                          child: Icon(
                            Icons.keyboard_arrow_down,
                            size: 16,
                            color: colorScheme.onSurfaceVariant.withValues(
                              alpha: 0.75,
                            ),
                          ),
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ),

            // Collapsible File List Body
            AnimatedSize(
              duration: const Duration(milliseconds: 220),
              curve: Curves.easeOutCubic,
              alignment: Alignment.topCenter,
              child: _filesExpanded
                  ? Column(
                      mainAxisSize: MainAxisSize.min,
                      crossAxisAlignment: CrossAxisAlignment.stretch,
                      children: <Widget>[
                        Divider(
                          height: 1,
                          thickness: 0.8,
                          color: dividerLineColor,
                        ),
                        Container(
                          color: colorScheme.surfaceContainerHighest.withValues(
                            alpha: 0.18,
                          ),
                          child: ConstrainedBox(
                            constraints: const BoxConstraints(maxHeight: 240),
                            child: Scrollbar(
                              controller: _scrollController,
                              thumbVisibility: review.files.length > 5,
                              child: SingleChildScrollView(
                                controller: _scrollController,
                                padding: const EdgeInsets.symmetric(
                                  horizontal: 4,
                                  vertical: 4,
                                ),
                                child: Column(
                                  mainAxisSize: MainAxisSize.min,
                                  children: <Widget>[
                                    for (final file in review.files)
                                      _buildFileRow(context, file, colorScheme),
                                  ],
                                ),
                              ),
                            ),
                          ),
                        ),
                      ],
                    )
                  : const SizedBox.shrink(),
            ),
          ],
        ),
      ),
    );
  }

  Widget _buildFileRow(
    BuildContext context,
    WorkspaceDiffFile file,
    ColorScheme colorScheme,
  ) {
    return Material(
      color: Colors.transparent,
      child: InkWell(
        onTap: () => _openSingleFileDiff(file),
        borderRadius: BorderRadius.circular(6),
        child: Tooltip(
          message: AppLocalizations.of(
            context,
          )!.toolViewFileDiff(file.fileName),
          waitDuration: const Duration(milliseconds: 400),
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 5),
            child: Row(
              children: <Widget>[
                // IDE file format icon
                buildWorkspaceFileIcon(file.fileName, size: 14),
                const SizedBox(width: 8),
                // Filename in front, directory path behind
                Expanded(
                  child: Text.rich(
                    TextSpan(
                      children: <InlineSpan>[
                        TextSpan(
                          text: file.fileName,
                          style: TextStyle(
                            fontSize: 12,
                            fontWeight: FontWeight.w600,
                            color: colorScheme.onSurface,
                          ),
                        ),
                        if (file.directoryPath.isNotEmpty) ...<InlineSpan>[
                          const TextSpan(text: '  '),
                          TextSpan(
                            text: file.directoryPath,
                            style: TextStyle(
                              fontSize: 11,
                              fontFamily: 'monospace',
                              color: colorScheme.onSurfaceVariant.withValues(
                                alpha: 0.65,
                              ),
                            ),
                          ),
                        ],
                      ],
                    ),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),
                const SizedBox(width: 8),
                // File diff line count stats
                _StatLine(additions: file.additions, deletions: file.deletions),
                const SizedBox(width: 2),
                Icon(
                  Icons.chevron_right_rounded,
                  size: 13,
                  color: colorScheme.onSurfaceVariant.withValues(alpha: 0.35),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _StatCapsule extends StatelessWidget {
  const _StatCapsule({required this.additions, required this.deletions});

  final int additions;
  final int deletions;

  @override
  Widget build(BuildContext context) {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
          decoration: BoxDecoration(
            color: const Color(0xFF3FB950).withValues(alpha: 0.12),
            borderRadius: BorderRadius.circular(4),
            border: Border.all(
              color: const Color(0xFF3FB950).withValues(alpha: 0.28),
              width: 0.8,
            ),
          ),
          child: Text(
            '+$additions',
            style: const TextStyle(
              color: Color(0xFF3FB950),
              fontWeight: FontWeight.w600,
              fontSize: 10.5,
              fontFamily: 'monospace',
              height: 1.1,
            ),
          ),
        ),
        const SizedBox(width: 4),
        Container(
          padding: const EdgeInsets.symmetric(horizontal: 5, vertical: 1),
          decoration: BoxDecoration(
            color: const Color(0xFFF85149).withValues(alpha: 0.12),
            borderRadius: BorderRadius.circular(4),
            border: Border.all(
              color: const Color(0xFFF85149).withValues(alpha: 0.28),
              width: 0.8,
            ),
          ),
          child: Text(
            '-$deletions',
            style: const TextStyle(
              color: Color(0xFFF85149),
              fontWeight: FontWeight.w600,
              fontSize: 10.5,
              fontFamily: 'monospace',
              height: 1.1,
            ),
          ),
        ),
      ],
    );
  }
}

class _StatLine extends StatelessWidget {
  const _StatLine({required this.additions, required this.deletions});

  final int additions;
  final int deletions;

  @override
  Widget build(BuildContext context) {
    return Text.rich(
      TextSpan(
        children: <InlineSpan>[
          TextSpan(
            text: '+$additions',
            style: const TextStyle(
              color: Color(0xFF3FB950),
              fontWeight: FontWeight.w600,
              fontSize: 11,
              fontFamily: 'monospace',
            ),
          ),
          const TextSpan(text: '  '),
          TextSpan(
            text: '-$deletions',
            style: const TextStyle(
              color: Color(0xFFF85149),
              fontWeight: FontWeight.w600,
              fontSize: 11,
              fontFamily: 'monospace',
            ),
          ),
        ],
      ),
    );
  }
}
