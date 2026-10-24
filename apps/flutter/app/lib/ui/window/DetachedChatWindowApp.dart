// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../core/logging/ClientLogger.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../features/chat/screens/AIChatScreen.dart';
import '../features/chat/viewmodel/ChatViewModel.dart';
import '../theme/OperitTheme.dart';
import 'OperitWindowArguments.dart';

class DetachedChatWindowApp extends StatefulWidget {
  const DetachedChatWindowApp({super.key, required this.arguments});

  final DetachedChatWindowArguments arguments;

  @override
  State<DetachedChatWindowApp> createState() => _DetachedChatWindowAppState();
}

class _DetachedChatWindowAppState extends State<DetachedChatWindowApp> {
  late final GeneratedChatRuntimeHolderMainCoreProxy _chatCore =
      const GeneratedCoreProxyClients(
        ProxyCoreRuntimeBridge(),
      ).chatRuntimeHolderMainForSlot(widget.arguments.slotId);
  late final ChatViewModel _chatViewModel = ChatViewModel(chat: _chatCore);
  bool _ready = false;
  Object? _error;

  @override
  void initState() {
    super.initState();
    _bindChat();
  }

  @override
  void dispose() {
    super.dispose();
  }

  Future<void> _bindChat() async {
    ClientLogger.v(
      'bind detached chat start slotId=${widget.arguments.slotId} chatId=${widget.arguments.chatId}',
      tag: 'DetachedChatWindow',
      verboseLevel: VerboseLevel.level4,
    );
    try {
      await _chatCore.switchChatLocal(chatId: widget.arguments.chatId);
      if (!mounted) {
        return;
      }
      ClientLogger.v(
        'bind detached chat done slotId=${widget.arguments.slotId} chatId=${widget.arguments.chatId}',
        tag: 'DetachedChatWindow',
        verboseLevel: VerboseLevel.level4,
      );
      setState(() {
        _ready = true;
      });
    } catch (error, stackTrace) {
      ClientLogger.e(
        'bind detached chat failed slotId=${widget.arguments.slotId} chatId=${widget.arguments.chatId}',
        tag: 'DetachedChatWindow',
        error: error,
        stackTrace: stackTrace,
      );
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
      initialThemePreferenceSnapshot: widget.arguments.themePreferenceSnapshot,
      initialThemeIsReady: true,
      hostInteractionHostsEnabled: false,
      unconfiguredChildEnabled: true,
      child: Scaffold(
        // Desktop windows do not need to reserve space for an on-screen
        // keyboard; large macOS view insets would otherwise collapse chat to
        // the app-bar height while the editor owns focus.
        resizeToAvoidBottomInset: false,
        appBar: AppBar(
          leading: const SizedBox.shrink(),
          leadingWidth: 48,
          titleSpacing: 8,
          title: Text(widget.arguments.title),
        ),
        body: SizedBox.expand(child: _body()),
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
    return AIChatEmbed(viewModel: _chatViewModel);
  }
}
