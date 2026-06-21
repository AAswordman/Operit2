// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../core/link/CoreLinkProtocol.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../features/chat/screens/AIChatScreen.dart';
import '../features/chat/viewmodel/ChatViewModel.dart';
import '../main/MainLayoutController.dart';
import '../main/TopBarController.dart';
import '../theme/OperitTheme.dart';
import 'OperitWindowArguments.dart';

class DetachedChatWindowApp extends StatefulWidget {
  const DetachedChatWindowApp({super.key, required this.arguments});

  final DetachedChatWindowArguments arguments;

  @override
  State<DetachedChatWindowApp> createState() => _DetachedChatWindowAppState();
}

class _DetachedChatWindowAppState extends State<DetachedChatWindowApp> {
  late final ChatRuntimeSurface _surface = DetachedChatRuntimeSurface(
    widget.arguments.slotId,
  );
  late final GeneratedChatRuntimeHolderMainCoreProxy _chatCore =
      GeneratedChatRuntimeHolderMainCoreProxy(
        const ProxyCoreRuntimeBridge(),
        CoreObjectPath.parse(
          'chatRuntimeHolder.detached.${widget.arguments.slotId}',
        ),
      );
  late final TopBarController _topBarController = TopBarController();
  late final MainLayoutController _mainLayoutController =
      MainLayoutController();
  bool _ready = false;
  Object? _error;

  @override
  void initState() {
    super.initState();
    _bindChat();
  }

  @override
  void dispose() {
    _topBarController.dispose();
    _mainLayoutController.dispose();
    super.dispose();
  }

  Future<void> _bindChat() async {
    try {
      await _chatCore.switchChatLocal(chatId: widget.arguments.chatId);
      if (!mounted) {
        return;
      }
      setState(() {
        _ready = true;
      });
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _error = error;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return OperitTheme(
      hostInteractionHostsEnabled: false,
      child: MainLayoutScope(
        controller: _mainLayoutController,
        child: TopBarScope(
          controller: _topBarController,
          child: AnimatedBuilder(
            animation: Listenable.merge(<Listenable>[
              _topBarController,
              _mainLayoutController,
            ]),
            builder: (context, _) {
              return _mainLayoutController.decorate(
                context,
                Scaffold(
                  appBar: AppBar(
                    title: Text(widget.arguments.title),
                    actions: _topBarController.actions?.call(context),
                  ),
                  body: _body(),
                ),
              );
            },
          ),
        ),
      ),
    );
  }

  Widget _body() {
    final error = _error;
    if (error != null) {
      return Center(child: SelectableText(error.toString()));
    }
    if (!_ready) {
      return const Center(child: CircularProgressIndicator());
    }
    return AIChatScreen(runtimeSurface: _surface);
  }
}
