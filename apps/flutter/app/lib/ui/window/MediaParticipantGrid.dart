// ignore_for_file: file_names
import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../features/packages/screens/ComposeDslImageSource.dart';

@immutable
class MediaStageParticipant {
  const MediaStageParticipant({
    required this.id,
    required this.title,
    this.imageUri,
  });

  final String id;
  final String title;
  final String? imageUri;
}

/// Keeps the complete roster visible, with equal tile areas even in partial rows.
class MediaParticipantGrid extends StatelessWidget {
  const MediaParticipantGrid({
    super.key,
    required this.participants,
    this.activeParticipantId,
  });

  final List<MediaStageParticipant> participants;
  final String? activeParticipantId;

  @override
  Widget build(BuildContext context) {
    if (participants.isEmpty) return const SizedBox.shrink();
    final columns = math.sqrt(participants.length).ceil();
    final rows = (participants.length / columns).ceil();
    final counts = List.generate(
      rows,
      (row) =>
          participants.length ~/ rows +
          (row < participants.length % rows ? 1 : 0),
    );
    final rowMembers = List.generate(rows, (row) {
      final offset =
          row * (participants.length ~/ rows) +
          math.min(row, participants.length % rows).toInt();
      return participants.sublist(offset, offset + counts[row]);
    });
    return LayoutBuilder(
      builder: (context, constraints) {
        if (constraints.maxWidth <= 0 || constraints.maxHeight <= 0) {
          return const SizedBox.shrink();
        }
        final gap = math.min(
          6.0,
          constraints.biggest.shortestSide / (columns * 4),
        );
        final widths = [
          for (final count in counts)
            (constraints.maxWidth - gap * (count - 1)) / count,
        ];
        final availableHeight = constraints.maxHeight - gap * (rows - 1);
        // Tile width differs in partial rows; inverse widths preserve equal areas.
        final area =
            availableHeight /
            widths.fold<double>(0, (sum, width) => sum + 1 / width);
        return Column(
          children: [
            for (var row = 0; row < rows; row++) ...[
              if (row > 0) SizedBox(height: gap),
              SizedBox(
                height: area / widths[row],
                child: Row(
                  children: [
                    for (var column = 0; column < counts[row]; column++) ...[
                      if (column > 0) SizedBox(width: gap),
                      Expanded(
                        child: _ParticipantTile(
                          key: ValueKey(
                            'media-participant-${rowMembers[row][column].id}',
                          ),
                          participant: rowMembers[row][column],
                          active:
                              rowMembers[row][column].id == activeParticipantId,
                        ),
                      ),
                    ],
                  ],
                ),
              ),
            ],
          ],
        );
      },
    );
  }
}

class _ParticipantTile extends StatefulWidget {
  const _ParticipantTile({
    super.key,
    required this.participant,
    required this.active,
  });
  final MediaStageParticipant participant;
  final bool active;

  @override
  State<_ParticipantTile> createState() => _ParticipantTileState();
}

class _ParticipantTileState extends State<_ParticipantTile> {
  static const _clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
  Future<ImageProvider>? _image;

  @override
  void initState() {
    super.initState();
    _resolveImage();
  }

  @override
  void didUpdateWidget(covariant _ParticipantTile oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.participant.imageUri != widget.participant.imageUri) {
      _resolveImage();
    }
  }

  void _resolveImage() {
    final uri = widget.participant.imageUri;
    _image = uri == null || uri.isEmpty
        ? null
        : resolveComposeDslImage(
            uri,
            vfsOwner: this,
            readVfsBytes: (path) => readComposeDslVfsBytes(_clients, path),
          );
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final title = widget.participant.title;
    return Semantics(
      label: title,
      selected: widget.active,
      child: Container(
        decoration: BoxDecoration(
          color: widget.active
              ? colors.primaryContainer
              : colors.surfaceContainerHigh,
          borderRadius: BorderRadius.circular(14),
          border: Border.all(
            color: widget.active ? colors.primary : colors.outlineVariant,
            width: 2,
          ),
        ),
        padding: const EdgeInsets.all(4),
        child: LayoutBuilder(
          builder: (context, constraints) {
            final showTitle =
                constraints.maxHeight >= 30 && constraints.maxWidth >= 24;
            final size = math.max(
              0.0,
              math.min(
                constraints.maxWidth,
                constraints.maxHeight - (showTitle ? 18 : 0),
              ),
            );
            final fallback = Center(
              child: Text(
                title.isEmpty ? '?' : title.characters.first,
                style: TextStyle(
                  fontSize: size * 0.38,
                  color: colors.onPrimaryContainer,
                ),
              ),
            );
            return Column(
              children: [
                Expanded(
                  child: Center(
                    child: SizedBox.square(
                      dimension: size,
                      child: ClipOval(
                        child: ColoredBox(
                          color: colors.primaryContainer,
                          child: FutureBuilder<ImageProvider>(
                            future: _image,
                            builder: (context, snapshot) =>
                                snapshot.data == null
                                ? fallback
                                : Image(
                                    image: snapshot.data!,
                                    fit: BoxFit.cover,
                                    errorBuilder: (context, error, stack) =>
                                        fallback,
                                  ),
                          ),
                        ),
                      ),
                    ),
                  ),
                ),
                if (showTitle)
                  SizedBox(
                    height: 18,
                    child: Row(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        if (widget.active)
                          Icon(
                            Icons.graphic_eq,
                            size: 12,
                            color: colors.primary,
                          ),
                        Flexible(
                          child: Text(
                            title,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: TextStyle(
                              fontSize: 11,
                              color: widget.active
                                  ? colors.onPrimaryContainer
                                  : colors.onSurface,
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),
              ],
            );
          },
        ),
      ),
    );
  }
}
