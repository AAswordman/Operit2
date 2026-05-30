// ignore_for_file: file_names

import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:webview_all/webview_all.dart';

class WorkspaceImagePreview extends StatelessWidget {
  const WorkspaceImagePreview({
    super.key,
    required this.bytes,
    required this.fileName,
  });

  final Uint8List bytes;
  final String fileName;

  @override
  Widget build(BuildContext context) {
    if (fileName.toLowerCase().endsWith('.svg')) {
      return WorkspaceSvgPreview(bytes: bytes);
    }
    return InteractiveViewer(
      minScale: 0.2,
      maxScale: 6,
      child: Center(child: Image.memory(bytes, fit: BoxFit.contain)),
    );
  }
}

class WorkspaceSvgPreview extends StatefulWidget {
  const WorkspaceSvgPreview({super.key, required this.bytes});

  final Uint8List bytes;

  @override
  State<WorkspaceSvgPreview> createState() => _WorkspaceSvgPreviewState();
}

class _WorkspaceSvgPreviewState extends State<WorkspaceSvgPreview> {
  late final WebViewController _controller;
  Future<void>? _loadFuture;

  @override
  void initState() {
    super.initState();
    _controller = WebViewController()
      ..setJavaScriptMode(JavaScriptMode.unrestricted)
      ..setBackgroundColor(Colors.transparent);
    _loadFuture = _loadSvg();
  }

  @override
  void didUpdateWidget(covariant WorkspaceSvgPreview oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(oldWidget.bytes, widget.bytes)) {
      setState(() {
        _loadFuture = _loadSvg();
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<void>(
      future: _loadFuture,
      builder: (context, snapshot) {
        if (snapshot.connectionState != ConnectionState.done) {
          return const Center(child: CircularProgressIndicator());
        }
        if (snapshot.hasError) {
          return Center(child: Text(snapshot.error.toString()));
        }
        return WebViewWidget(controller: _controller);
      },
    );
  }

  Future<void> _loadSvg() async {
    final dataUrl = 'data:image/svg+xml;base64,${base64Encode(widget.bytes)}';
    await _controller.loadHtmlString('''
<!doctype html>
<html>
<head>
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <style>
    html, body {
      margin: 0;
      width: 100%;
      height: 100%;
      background: transparent;
      overflow: hidden;
    }
    body {
      display: flex;
      align-items: center;
      justify-content: center;
    }
    img {
      max-width: 100%;
      max-height: 100%;
      object-fit: contain;
    }
  </style>
</head>
<body><img src="$dataUrl"></body>
</html>
''');
  }
}
