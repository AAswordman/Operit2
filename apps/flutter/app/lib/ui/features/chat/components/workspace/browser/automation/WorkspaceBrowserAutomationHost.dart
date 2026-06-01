// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:operit2/core/browser/BrowserAutomationBridge.dart';
import 'package:operit2/core/browser/BrowserAutomationModels.dart';

import 'WorkspaceBrowserAutomationController.dart';
import 'WorkspaceBrowserSessionRegistry.dart';

class WorkspaceBrowserAutomationHost extends StatefulWidget {
  const WorkspaceBrowserAutomationHost({
    super.key,
    required this.child,
    this.bridge = const BrowserAutomationBridge(),
  });

  final Widget child;
  final BrowserAutomationBridge bridge;

  @override
  State<WorkspaceBrowserAutomationHost> createState() =>
      _WorkspaceBrowserAutomationHostState();
}

class _WorkspaceBrowserAutomationHostState
    extends State<WorkspaceBrowserAutomationHost> {
  final WorkspaceBrowserSessionRegistry _registry =
      WorkspaceBrowserSessionRegistry.instance;
  Timer? _pollTimer;
  bool _polling = false;

  @override
  void initState() {
    super.initState();
    _pollTimer = Timer.periodic(
      const Duration(milliseconds: 120),
      (_) => _pollRequest(),
    );
    _pollRequest();
  }

  @override
  void dispose() {
    _pollTimer?.cancel();
    super.dispose();
  }

  Future<void> _pollRequest() async {
    if (_polling) {
      return;
    }
    _polling = true;
    try {
      final request = await widget.bridge.nextRequest();
      if (request != null) {
        await _handleRequest(request);
      }
    } catch (error, stackTrace) {
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: error,
          stack: stackTrace,
          library: 'workspace browser automation host',
          context: ErrorDescription('polling browser automation request'),
        ),
      );
    } finally {
      _polling = false;
    }
  }

  Future<void> _handleRequest(BrowserAutomationRequest request) async {
    try {
      final result = await _execute(request);
      await widget.bridge.handleResult(
        BrowserAutomationResponse(
          requestId: request.requestId,
          success: true,
          result: result,
        ),
      );
    } catch (error, stackTrace) {
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: error,
          stack: stackTrace,
          library: 'workspace browser automation host',
          context: ErrorDescription('executing ${request.toolName}'),
        ),
      );
      await widget.bridge.handleResult(
        BrowserAutomationResponse(
          requestId: request.requestId,
          success: false,
          result: '',
          error: error.toString(),
        ),
      );
    }
  }

  Future<String> _execute(BrowserAutomationRequest request) async {
    final params = request.parameters;
    switch (request.toolName) {
      case 'browser_navigate':
        final url = _required(params, 'url');
        if (_registry.activeControllerForChat(request.chatId) == null) {
          _registry.openBrowserTab(request.chatId, url: url);
          await _registry.waitForChatSession(
            request.chatId,
            timeout: const Duration(seconds: 30),
          );
          return jsonEncode(<String, Object?>{
            'chat_id': request.chatId,
            'url': url,
          });
        }
        _registry.navigate(request.chatId, url);
        return jsonEncode(<String, Object?>{
          'chat_id': request.chatId,
          'url': url,
        });
      case 'browser_navigate_back':
        _registry.navigateBack(request.chatId);
        return 'OK';
      case 'browser_close':
        _registry.closeActiveTab(request.chatId);
        return 'OK';
      case 'browser_close_all':
        _registry.closeAllTabs(request.chatId);
        return 'OK';
      case 'browser_tabs':
        return _handleTabs(request);
      case 'browser_snapshot':
        final controller = _controller(request);
        return _stringify(await controller.snapshot());
      case 'browser_console_messages':
        final controller = _controller(request);
        return controller.consoleMessages(level: _optional(params, 'level'));
      case 'browser_network_requests':
        final controller = _controller(request);
        return controller.networkRequests();
      case 'browser_click':
        final controller = _controller(request);
        await controller.click(_target(params));
        return 'OK';
      case 'browser_type':
        final controller = _controller(request);
        await controller.type(_target(params), _required(params, 'text'));
        if (_boolParam(params, 'submit')) {
          await controller.pressKey('Enter');
        }
        return 'OK';
      case 'browser_hover':
        final controller = _controller(request);
        await controller.hover(_target(params));
        return 'OK';
      case 'browser_drag':
        final controller = _controller(request);
        await controller.drag(
          _required(params, 'startRef'),
          _required(params, 'endRef'),
        );
        return 'OK';
      case 'browser_fill_form':
        final controller = _controller(request);
        await controller.fillForm(_formFields(_required(params, 'fields')));
        return 'OK';
      case 'browser_press_key':
        final controller = _controller(request);
        await controller.pressKey(_required(params, 'key'));
        return 'OK';
      case 'browser_select_option':
        final controller = _controller(request);
        await controller.selectOption(
          _required(params, 'ref'),
          _stringList(_required(params, 'values')),
        );
        return 'OK';
      case 'browser_evaluate':
        final controller = _controller(request);
        return _stringify(
          await controller.evaluateFunction(
            _required(params, 'function'),
            selector: _optional(params, 'ref'),
          ),
        );
      case 'browser_run_code':
        final controller = _controller(request);
        return _stringify(await controller.runCode(_required(params, 'code')));
      case 'browser_wait_for':
        return _handleWaitFor(request);
      case 'browser_file_upload':
      case 'browser_handle_dialog':
      case 'browser_resize':
      case 'browser_take_screenshot':
        throw StateError(
          '${request.toolName} is not supported by Workspace Browser',
        );
    }
    throw StateError('Unknown browser automation tool: ${request.toolName}');
  }

  Future<String> _handleTabs(BrowserAutomationRequest request) async {
    final params = request.parameters;
    final action = _required(params, 'action');
    switch (action) {
      case 'list':
        return jsonEncode(_registry.listTabs(request.chatId));
      case 'create':
        final url = _required(params, 'url');
        _registry.openBrowserTab(request.chatId, url: url);
        await _registry.waitForChatSession(
          request.chatId,
          timeout: const Duration(seconds: 30),
        );
        return jsonEncode(_registry.listTabs(request.chatId));
      case 'select':
        final index = _intParam(params, 'index');
        final tabs = _registry.listTabs(request.chatId);
        final tab = tabs[index];
        _registry.selectTab(request.chatId, tab['sessionId'] as String);
        return jsonEncode(tab);
      case 'close':
        final indexText = _optional(params, 'index');
        if (indexText == null) {
          _registry.closeActiveTab(request.chatId);
          return 'OK';
        }
        final index = int.parse(indexText);
        final tabs = _registry.listTabs(request.chatId);
        final tab = tabs[index];
        _registry.closeTab(request.chatId, tab['sessionId'] as String);
        return 'OK';
    }
    throw StateError('Unsupported browser tab action: $action');
  }

  Future<String> _handleWaitFor(BrowserAutomationRequest request) async {
    final params = request.parameters;
    final timeText = _optional(params, 'time');
    if (timeText != null) {
      final milliseconds = (double.parse(timeText) * 1000).round();
      await Future<void>.delayed(Duration(milliseconds: milliseconds));
      return 'OK';
    }
    final controller = _controller(request);
    final text = _optional(params, 'text');
    if (text != null) {
      return _stringify(await controller.waitForText(text));
    }
    final textGone = _optional(params, 'textGone');
    if (textGone != null) {
      return _stringify(await controller.waitForTextGone(textGone));
    }
    throw StateError('time, text, or textGone is required');
  }

  WorkspaceBrowserAutomationController _controller(
    BrowserAutomationRequest request,
  ) {
    final controller = _registry.activeControllerForChat(request.chatId);
    if (controller == null) {
      throw StateError(
        'No active browser session for chat_id ${request.chatId}',
      );
    }
    return controller;
  }

  String _required(Map<String, String> params, String name) {
    final value = params[name]?.trim();
    if (value == null || value.isEmpty) {
      throw StateError('Missing parameter: $name');
    }
    return value;
  }

  String? _optional(Map<String, String> params, String name) {
    final value = params[name]?.trim();
    if (value == null || value.isEmpty) {
      return null;
    }
    return value;
  }

  String _target(Map<String, String> params) {
    final ref = _optional(params, 'ref');
    if (ref != null) {
      return ref;
    }
    final selector = _optional(params, 'selector');
    if (selector != null) {
      return selector;
    }
    throw StateError('ref or selector is required');
  }

  int _intParam(Map<String, String> params, String name) {
    return int.parse(_required(params, name));
  }

  bool _boolParam(Map<String, String> params, String name) {
    final value = _optional(params, name);
    return value == 'true' || value == '1';
  }

  List<String> _stringList(String raw) {
    final value = jsonDecode(raw) as List<Object?>;
    return value.map((item) => item as String).toList(growable: false);
  }

  Map<String, String> _formFields(String raw) {
    final value = jsonDecode(raw) as List<Object?>;
    return <String, String>{
      for (final item in value.cast<Map<String, Object?>>())
        _fieldTarget(item): _fieldValue(item),
    };
  }

  String _fieldTarget(Map<String, Object?> field) {
    final ref = field['ref'];
    if (ref is String && ref.trim().isNotEmpty) {
      return ref.trim();
    }
    final selector = field['selector'];
    if (selector is String && selector.trim().isNotEmpty) {
      return selector.trim();
    }
    throw StateError('Form field is missing ref or selector');
  }

  String _fieldValue(Map<String, Object?> field) {
    final value = field['value'];
    if (value is String) {
      return value;
    }
    return jsonEncode(value);
  }

  String _stringify(Object? value) {
    if (value is String) {
      return value;
    }
    return jsonEncode(value);
  }

  @override
  Widget build(BuildContext context) {
    return widget.child;
  }
}
