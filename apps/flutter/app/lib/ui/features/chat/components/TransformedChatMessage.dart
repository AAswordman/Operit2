// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/ChatResponsePresentation.dart';

/// Retains completed display copies across viewport mounts without changing canonical messages.
class MessageTransformCache extends ChangeNotifier {
  /// Observes one invalidation subscription for the entire displayed history window.
  MessageTransformCache(this.clients) {
    _subscription = clients.chatRuntimeHolderMain
        .messageTransformRevisionFlow()
        .listen(
          (revision) {
            final initial = _revision == null;
            _revision = revision;
            if (initial) return;
            unawaited(_project(_entries.values.toList()));
          },
          onError: (Object error, StackTrace stack) {
            for (final entry in _entries.values) {
              entry.snapshot.value = AsyncSnapshot.withError(
                ConnectionState.done,
                error,
                stack,
              );
            }
            _updateHistoryReadiness();
          },
        );
  }

  final GeneratedCoreProxyClients clients;
  final _entries = <int, _MessageProjection>{};
  late final StreamSubscription<int> _subscription;
  String? _chatId;
  int? _revision;
  bool _disposed = false;
  Set<int> _windowTimestamps = <int>{};
  bool _historyReady = false;
  bool _preparingHistory = false;

  /// Keeps initial history behind one transcript-level loader until its display copies settle.
  bool get isPreparingHistory => _preparingHistory;

  /// Prepares the whole visible history window so scrolling reuses already projected rows.
  void prepare(
    String chatId,
    List<core_proxy.ChatMessage> messages,
    int? depthAnchor,
  ) {
    if (_chatId != chatId) {
      _entries.clear();
      _historyReady = false;
    }
    _chatId = chatId;
    final retained = messages.map((message) => message.timestamp).toSet();
    _windowTimestamps = retained;
    // Keep a bounded set of previous window entries so returning rows retain measured content.
    for (final timestamp in _entries.keys.toList()) {
      if (_entries.length <= retained.length + 256) break;
      if (!retained.contains(timestamp)) _entries.remove(timestamp);
    }
    final changed = <_MessageProjection>[];
    for (final message in messages) {
      final existing = _entries[message.timestamp];
      if (existing != null &&
          identical(existing.source, message) &&
          existing.depthAnchor == depthAnchor) {
        continue;
      }
      final entry = _MessageProjection(
        message,
        existing?.snapshot.value.data,
        depthAnchor,
      );
      _entries[message.timestamp] = entry;
      changed.add(entry);
    }
    unawaited(_project(changed));
    _updateHistoryReadiness();
  }

  /// Opens initial history together and keeps subsequent reprojections in the mounted transcript.
  void _updateHistoryReadiness() {
    if (_disposed || _historyReady) return;
    final preparing = _windowTimestamps.any((timestamp) {
      final snapshot = _entries[timestamp]!.snapshot.value;
      return !snapshot.hasData && !snapshot.hasError;
    });
    if (!preparing && _windowTimestamps.isNotEmpty) _historyReady = true;
    if (_preparingHistory == preparing) return;
    _preparingHistory = preparing;
    notifyListeners();
  }

  /// Keeps live output on its original incremental stream until the completed display copy is ready.
  static bool usesLiveStream(core_proxy.ChatMessage message) =>
      message.contentStream != null;

  /// Projects changed rows in one request and rejects superseded chats or source revisions.
  Future<void> _project(List<_MessageProjection> entries) async {
    final pending = <(_MessageProjection, int)>[];
    for (final entry in entries) {
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
    _updateHistoryReadiness();
  }

  /// Releases the owning window's subscription; outstanding requests cannot publish after disposal.
  @override
  void dispose() {
    _disposed = true;
    unawaited(_subscription.cancel());
    _entries.clear();
    super.dispose();
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

  /// Binds the supplied window cache or owns an isolated single-message cache.
  void _bind() {
    _cache = widget.cache ?? MessageTransformCache(widget.clients);
    if (widget.cache == null) {
      _cache.prepare(widget.chatId, [widget.message], widget.depthAnchor);
    }
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
      if (oldWidget.cache == null) _cache.dispose();
      _bind();
    } else if (widget.cache == null) {
      _cache.prepare(widget.chatId, [widget.message], widget.depthAnchor);
    }
  }

  /// Releases only caches owned by this row.
  @override
  void dispose() {
    if (widget.cache == null) _cache.dispose();
    super.dispose();
  }

  /// Renders completed projections and explicit failures while retaining valid prior display copies.
  @override
  Widget build(BuildContext context) {
    final entry = _cache._entries[widget.message.timestamp]!;
    return ValueListenableBuilder<AsyncSnapshot<core_proxy.ChatMessage>>(
      valueListenable: entry.snapshot,
      builder: (context, snapshot, _) {
        if (snapshot.hasError) {
          return Text(
            AppLocalizations.of(
              context,
            )!.messageTransformFailed(snapshot.error.toString()),
          );
        }
        if (!snapshot.hasData) {
          return const SizedBox.shrink();
        }
        return widget.builder(snapshot.requireData);
      },
    );
  }
}
