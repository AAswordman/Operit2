// ignore_for_file: file_names

import 'dart:async';
import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../viewmodel/ChatMessageLookupRequests.dart';
import 'ChatMessageLocatorPanel.dart';
import 'ChatScrollNavigator.dart';

/// Opens lookup on demand without adding a persistent control to the chat.
class ChatMessageLocatorHost extends StatefulWidget {
  const ChatMessageLocatorHost({
    super.key,
    required this.currentChatId,
    required this.navigation,
    required this.loadEntries,
    required this.onToggleFavorite,
    required this.child,
    this.acceptsSidebarRequests = true,
  });

  final String? currentChatId;
  final ChatMessageNavigationController navigation;
  final LoadMessageLocatorEntries loadEntries;
  final ToggleFavoriteMessage onToggleFavorite;
  final Widget child;
  final bool acceptsSidebarRequests;

  @override
  State<ChatMessageLocatorHost> createState() => _ChatMessageLocatorHostState();
}

class _ChatMessageLocatorHostState extends State<ChatMessageLocatorHost> {
  Route<int>? _route;
  bool _isOpen = false;
  int _generation = 0;

  @override
  void initState() {
    super.initState();
    widget.navigation.openRequests.addListener(_openFromKeyboard);
    ChatMessageLookupRequests.requests.addListener(_checkSidebarRequest);
    _checkSidebarRequest();
  }

  @override
  void didUpdateWidget(ChatMessageLocatorHost oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.navigation != widget.navigation) {
      oldWidget.navigation.openRequests.removeListener(_openFromKeyboard);
      widget.navigation.openRequests.addListener(_openFromKeyboard);
    }
    if (oldWidget.currentChatId != widget.currentChatId ||
        (oldWidget.acceptsSidebarRequests && !widget.acceptsSidebarRequests)) {
      _dismiss();
    }
    _checkSidebarRequest();
  }

  @override
  void dispose() {
    widget.navigation.openRequests.removeListener(_openFromKeyboard);
    ChatMessageLookupRequests.requests.removeListener(_checkSidebarRequest);
    _dismiss();
    super.dispose();
  }

  void _dismiss() {
    _generation++;
    _isOpen = false;
    final route = _route;
    _route = null;
    if (route == null) return;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (route.isActive) route.navigator?.removeRoute(route);
    });
  }

  void _checkSidebarRequest() {
    final request = ChatMessageLookupRequests.requests.value;
    if (!widget.acceptsSidebarRequests ||
        request == null ||
        request.chatId != widget.currentChatId) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted ||
          !widget.acceptsSidebarRequests ||
          request.chatId != widget.currentChatId ||
          !ChatMessageLookupRequests.consume(request)) {
        return;
      }
      if (_isOpen) _dismiss();
      unawaited(_open(favoritesOnly: request.favoritesOnly));
    });
    WidgetsBinding.instance.ensureVisualUpdate();
  }

  void _openFromKeyboard() => unawaited(_open());

  Future<void> _open({bool favoritesOnly = false}) async {
    final chatId = widget.currentChatId;
    if (_isOpen || chatId == null) return;
    final generation = ++_generation;
    _isOpen = true;
    final navigator = Navigator.of(context, rootNavigator: true);
    Widget panel(BuildContext context, {required bool autofocus}) {
      return ChatMessageLocatorPanel(
        initialFavoritesOnly: favoritesOnly,
        autofocus: autofocus,
        loadEntries: (query) => widget.loadEntries(chatId, query),
        onToggleFavorite: (timestamp, favorite) async {
          if (!mounted || widget.currentChatId != chatId) return;
          await widget.onToggleFavorite(timestamp, favorite);
        },
        onClose: () => Navigator.of(context).pop(),
        onSelected: (timestamp) => Navigator.of(context).pop(timestamp),
      );
    }

    try {
      final int? timestamp;
      if (MediaQuery.sizeOf(context).width < 600) {
        timestamp = await showModalBottomSheet<int>(
          context: context,
          useRootNavigator: true,
          isScrollControlled: true,
          useSafeArea: true,
          showDragHandle: true,
          builder: (context) {
            _route = ModalRoute.of<int>(context);
            final media = MediaQuery.of(context);
            return Padding(
              padding: EdgeInsets.only(bottom: media.viewInsets.bottom),
              child: SizedBox(
                width: double.infinity,
                height: math.min(
                  560,
                  math.max(
                    120,
                    (media.size.height - media.viewInsets.bottom) * 0.8,
                  ),
                ),
                child: panel(context, autofocus: false),
              ),
            );
          },
        );
      } else {
        final route = DialogRoute<int>(
          context: context,
          builder: (context) => Dialog(
            key: const ValueKey('message-locator-dialog'),
            insetPadding: const EdgeInsets.all(24),
            clipBehavior: Clip.antiAlias,
            child: SizedBox(
              width: 560,
              height: math.min(560, MediaQuery.sizeOf(context).height - 48),
              child: panel(context, autofocus: !favoritesOnly),
            ),
          ),
        );
        _route = route;
        timestamp = await navigator.push(route);
      }
      if (mounted &&
          generation == _generation &&
          widget.currentChatId == chatId &&
          timestamp != null) {
        widget.navigation.jumpTo(chatId, timestamp);
      }
    } finally {
      if (generation == _generation) {
        _route = null;
        _isOpen = false;
      }
    }
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
