// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/ChatResponsePresentation.dart';

/// Retains completed display copies across viewport mounts without changing canonical messages.
class MessageTransformCache {
  /// Observes one invalidation subscription for the entire displayed history window.
  MessageTransformCache(this.clients) {
    _subscription = clients.chatRuntimeHolderMain
        .messageTransformRevisionFlow()
        .listen(
          (revision) {
            final initial = _revision == null;
            _revision = revision;
            if (initial) {
              for (final entry in _entries.values) {
                entry.revision = revision;
              }
              return;
            }
            for (final entry in _entries.values) {
              if (entry.mounts > 0) _enqueue(entry);
            }
          },
          onError: (Object error, StackTrace stack) {
            for (final entry in _entries.values) {
              if (entry.mounts == 0) continue;
              entry.snapshot.value = AsyncSnapshot.withError(
                ConnectionState.done,
                error,
                stack,
              );
            }
          },
        );
  }

  final GeneratedCoreProxyClients clients;
  final _entries = <int, _MessageProjection>{};
  late final StreamSubscription<int> _subscription;
  String? _chatId;
  int? _revision;
  bool _disposed = false;
  final _pending = <_MessageProjection>{};
  bool _dispatchScheduled = false;

  /// Changes conversation ownership without projecting any unmounted messages.
  void bindChat(String? chatId) {
    if (_chatId == chatId) return;
    for (final entry in _entries.values) {
      if (entry.mounts == 0) entry.snapshot.dispose();
    }
    _entries.clear();
    _pending.clear();
    _chatId = chatId;
  }

  /// Acquires one mounted row and reuses only its exact source and rule revision.
  _MessageProjection _retain(
    String chatId,
    core_proxy.ChatMessage message,
    int? depthAnchor,
  ) {
    bindChat(chatId);
    final existing = _entries.remove(message.timestamp);
    final unchanged =
        existing != null &&
        identical(existing.source, message) &&
        existing.depthAnchor == depthAnchor;
    final entry = unchanged
        ? existing
        : _MessageProjection(
            message,
            existing?.snapshot.value.data,
            depthAnchor,
          );
    if (!unchanged && existing != null && existing.mounts == 0) {
      _pending.remove(existing);
      existing.snapshot.dispose();
    }
    entry.mounts++;
    _entries[message.timestamp] = entry;
    if (!unchanged || entry.revision != _revision || entry.generation == 0) {
      _enqueue(entry);
    }
    _trim();
    return entry;
  }

  /// Releases viewport ownership before queued work can prepare an offscreen row.
  void _release(_MessageProjection entry) {
    entry.mounts--;
    if (entry.mounts != 0) return;
    _pending.remove(entry);
    if (_entries[entry.source.timestamp] != entry) entry.snapshot.dispose();
    _trim();
  }

  /// Bounds retained projections while keeping all currently mounted rows owned.
  void _trim() {
    for (final timestamp in _entries.keys.toList()) {
      if (_entries.length <= 256) break;
      final entry = _entries[timestamp]!;
      if (entry.mounts != 0) continue;
      _entries.remove(timestamp);
      _pending.remove(entry);
      entry.snapshot.dispose();
    }
  }

  /// Batches only rows acquired by this layout, excluding rows unmounted before dispatch.
  void _enqueue(_MessageProjection entry) {
    if (usesLiveStream(entry.source)) {
      unawaited(_project([entry]));
      return;
    }
    _pending.add(entry);
    if (_dispatchScheduled) return;
    _dispatchScheduled = true;
    scheduleMicrotask(() {
      _dispatchScheduled = false;
      if (_disposed) return;
      final entries = _pending.where((entry) => entry.mounts > 0).toList();
      _pending.clear();
      unawaited(_project(entries));
    });
  }

  /// Keeps live output on its original incremental stream until the completed display copy is ready.
  static bool usesLiveStream(core_proxy.ChatMessage message) =>
      message.contentStream != null;

  /// Projects changed rows in one request and rejects superseded chats or source revisions.
  Future<void> _project(List<_MessageProjection> entries) async {
    final pending = <(_MessageProjection, int)>[];
    for (final entry in entries) {
      entry.revision = _revision;
      final generation = ++entry.generation;
      final hasText = entry.source.parts.any(
        (part) =>
            part.kind == core_proxy.MessagePartKind.markdown ||
            part.kind == core_proxy.MessagePartKind.thinking,
      );
      if (usesLiveStream(entry.source) ||
          (!hasText &&
              !entry.source.pluginExtensions.containsKey(
                responsePresentationExtension,
              ))) {
        entry.snapshot.value = AsyncSnapshot.withData(
          ConnectionState.done,
          entry.source,
        );
      } else {
        pending.add((entry, generation));
      }
    }
    if (pending.isEmpty) return;
    final chatId = _chatId!;

    /// Publishes only entries that still belong to this exact request.
    void publish(int index, AsyncSnapshot<core_proxy.ChatMessage> snapshot) {
      final (entry, generation) = pending[index];
      if (!_disposed &&
          _chatId == chatId &&
          _entries[entry.source.timestamp] == entry &&
          entry.generation == generation) {
        entry.snapshot.value = snapshot;
      }
    }

    try {
      final projected = await clients.chatRuntimeHolderMain
          .projectMessagesForDisplay(
            chatId: chatId,
            messages: pending.map((item) => item.$1.source).toList(),
          );
      if (projected.length != pending.length ||
          Iterable<int>.generate(pending.length).any(
            (index) =>
                projected[index].timestamp !=
                pending[index].$1.source.timestamp,
          )) {
        throw StateError(
          'Display projection does not match the requested message order',
        );
      }
      for (var index = 0; index < projected.length; index++) {
        publish(
          index,
          AsyncSnapshot.withData(ConnectionState.done, projected[index]),
        );
      }
    } catch (error, stack) {
      for (var index = 0; index < pending.length; index++) {
        publish(
          index,
          AsyncSnapshot.withError(ConnectionState.done, error, stack),
        );
      }
    }
  }

  /// Releases the owning window's subscription; outstanding requests cannot publish after disposal.
  void dispose() {
    _disposed = true;
    unawaited(_subscription.cancel());
    _pending.clear();
    for (final entry in _entries.values) {
      if (entry.mounts == 0) entry.snapshot.dispose();
    }
    _entries.clear();
  }
}

/// Holds a source revision and its last completed display copy during reprojection.
class _MessageProjection {
  /// Keeps an already displayed projection while its replacement is calculated.
  _MessageProjection(
    this.source,
    core_proxy.ChatMessage? previous,
    this.depthAnchor,
  ) : snapshot = ValueNotifier(
        previous == null
            ? const AsyncSnapshot<core_proxy.ChatMessage>.waiting()
            : AsyncSnapshot.withData(ConnectionState.waiting, previous),
      );
  final core_proxy.ChatMessage source;
  final int? depthAnchor;
  final ValueNotifier<AsyncSnapshot<core_proxy.ChatMessage>> snapshot;
  int generation = 0;
  int mounts = 0;
  int? revision;
}

/// Projects plugin display text while edit and copy actions retain their canonical message.
class TransformedChatMessage extends StatefulWidget {
  /// Binds a row to its exact source and an optional window-owned projection cache.
  const TransformedChatMessage({
    super.key,
    required this.message,
    required this.chatId,
    required this.clients,
    required this.depthAnchor,
    required this.builder,
    this.cache,
  });
  final core_proxy.ChatMessage message;
  final String chatId;
  final int? depthAnchor;
  final GeneratedCoreProxyClients clients;
  final MessageTransformCache? cache;
  final Widget Function(core_proxy.ChatMessage message) builder;

  /// Creates a row-local cache only when no history-window cache was supplied.
  @override
  State<TransformedChatMessage> createState() => _TransformedChatMessageState();
}

class _TransformedChatMessageState extends State<TransformedChatMessage> {
  late MessageTransformCache _cache;
  late _MessageProjection _entry;

  /// Binds the supplied window cache or owns an isolated single-message cache.
  void _bind() {
    _cache = widget.cache ?? MessageTransformCache(widget.clients);
    _entry = _cache._retain(widget.chatId, widget.message, widget.depthAnchor);
  }

  /// Initializes projection ownership before rendering the first row.
  @override
  void initState() {
    super.initState();
    _bind();
  }

  /// Updates standalone projections without disrupting shared history-window ownership.
  @override
  void didUpdateWidget(covariant TransformedChatMessage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.clients.bridge != widget.clients.bridge ||
        oldWidget.cache != widget.cache) {
      _cache._release(_entry);
      if (oldWidget.cache == null) _cache.dispose();
      _bind();
    } else if (oldWidget.message != widget.message ||
        oldWidget.chatId != widget.chatId ||
        oldWidget.depthAnchor != widget.depthAnchor) {
      _cache._release(_entry);
      _entry = _cache._retain(
        widget.chatId,
        widget.message,
        widget.depthAnchor,
      );
    }
  }

  /// Releases only caches owned by this row.
  @override
  void dispose() {
    _cache._release(_entry);
    if (widget.cache == null) _cache.dispose();
    super.dispose();
  }

  /// Renders completed projections and explicit failures while retaining valid prior display copies.
  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<AsyncSnapshot<core_proxy.ChatMessage>>(
      valueListenable: _entry.snapshot,
      builder: (context, snapshot, _) {
        if (snapshot.hasError) {
          return Text(
            AppLocalizations.of(
              context,
            )!.messageTransformFailed(snapshot.error.toString()),
          );
        }
        if (!snapshot.hasData) {
          // Keep pending rows measurable so lazy layout does not mount the entire history.
          return const SizedBox(height: 64);
        }
        return widget.builder(snapshot.requireData);
      },
    );
  }
}
