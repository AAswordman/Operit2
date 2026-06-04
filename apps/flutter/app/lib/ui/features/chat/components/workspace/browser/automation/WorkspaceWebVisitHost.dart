// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';
import 'package:operit2/core/web_visit/WebVisitBridge.dart';
import 'package:operit2/core/web_visit/WebVisitModels.dart';

import 'WorkspaceWebVisitSessionRegistry.dart';

class WorkspaceWebVisitHost extends StatefulWidget {
  const WorkspaceWebVisitHost({
    super.key,
    required this.child,
    this.bridge = const WebVisitBridge(),
  });

  final Widget child;
  final WebVisitBridge bridge;

  @override
  State<WorkspaceWebVisitHost> createState() => _WorkspaceWebVisitHostState();
}

class _WorkspaceWebVisitHostState extends State<WorkspaceWebVisitHost> {
  final WorkspaceWebVisitSessionRegistry _registry =
      WorkspaceWebVisitSessionRegistry.instance;
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
          library: 'workspace web visit host',
          context: ErrorDescription('polling web visit request'),
        ),
      );
    } finally {
      _polling = false;
    }
  }

  Future<void> _handleRequest(WebVisitRequest request) async {
    try {
      final response = await _registry.visitWeb(request);
      await widget.bridge.handleResult(response);
    } catch (error, stackTrace) {
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: error,
          stack: stackTrace,
          library: 'workspace web visit host',
          context: ErrorDescription('executing visit_web'),
        ),
      );
      await widget.bridge.handleResult(
        WebVisitResponse(
          requestId: request.requestId,
          success: false,
          error: error.toString(),
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return widget.child;
  }
}
