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
  void Function()? _disposeBridgeHandler;

  @override
  void initState() {
    super.initState();
    _disposeBridgeHandler = widget.bridge.registerHandler(_handleRequest);
  }

  @override
  void dispose() {
    _disposeBridgeHandler?.call();
    super.dispose();
  }

  Future<WebVisitResponse> _handleRequest(WebVisitRequest request) async {
    try {
      return await _registry.visitWeb(request);
    } catch (error, stackTrace) {
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: error,
          stack: stackTrace,
          library: 'workspace web visit host',
          context: ErrorDescription('executing visit_web'),
        ),
      );
      return WebVisitResponse(
        requestId: request.requestId,
        success: false,
        error: error.toString(),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    return widget.child;
  }
}
