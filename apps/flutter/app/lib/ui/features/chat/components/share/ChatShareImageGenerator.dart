// ignore_for_file: file_names

import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:path_provider/path_provider.dart';

import '../../../../common/OperitLogoMark.dart';
import '../../viewmodel/ChatViewModel.dart';
import '../style/cursor/CursorStyleChatMessage.dart';

class ChatShareImageGenerator {
  ChatShareImageGenerator._();

  static Future<File> generate({
    required BuildContext context,
    required List<ChatUiMessage> messages,
  }) async {
    final boundaryKey = GlobalKey();
    final overlay = Overlay.of(context);
    late final OverlayEntry entry;
    entry = OverlayEntry(
      builder: (context) {
        return Positioned(
          left: 0,
          top: 0,
          child: IgnorePointer(
            child: Opacity(
              opacity: 0.01,
              child: RepaintBoundary(
                key: boundaryKey,
                child: ChatShareImageSurface(messages: messages),
              ),
            ),
          ),
        );
      },
    );

    overlay.insert(entry);
    await WidgetsBinding.instance.endOfFrame;
    await WidgetsBinding.instance.endOfFrame;

    final boundary =
        boundaryKey.currentContext!.findRenderObject()!
            as RenderRepaintBoundary;
    final image = await boundary.toImage(pixelRatio: 2);
    final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
    image.dispose();
    entry.remove();

    final directory = await getTemporaryDirectory();
    final file = File(
      '${directory.path}${Platform.pathSeparator}operit_share_${DateTime.now().millisecondsSinceEpoch}.png',
    );
    final pngBytes = bytes!.buffer.asUint8List();
    await file.writeAsBytes(pngBytes, flush: true);
    return file;
  }
}

class ChatShareImageSurface extends StatelessWidget {
  const ChatShareImageSurface({super.key, required this.messages});

  final List<ChatUiMessage> messages;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return MediaQuery(
      data: MediaQuery.of(
        context,
      ).copyWith(size: const Size(420, 1), textScaler: TextScaler.noScaling),
      child: Material(
        color: colorScheme.surface,
        child: ConstrainedBox(
          constraints: const BoxConstraints.tightFor(width: 420),
          child: DecoratedBox(
            decoration: BoxDecoration(color: colorScheme.surface),
            child: Padding(
              padding: const EdgeInsets.fromLTRB(16, 18, 16, 18),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  Row(
                    mainAxisAlignment: MainAxisAlignment.center,
                    crossAxisAlignment: CrossAxisAlignment.center,
                    children: <Widget>[
                      const OperitLogoMark(size: 24, contentScale: 0.84),
                      const SizedBox(width: 8),
                      Text(
                        'Operit',
                        style: theme.textTheme.titleMedium?.copyWith(
                          fontWeight: FontWeight.w700,
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(height: 14),
                  for (var index = 0; index < messages.length; index++) ...[
                    CursorStyleChatMessage(
                      message: messages[index],
                      isStreaming: false,
                    ),
                    if (index != messages.length - 1) const SizedBox(height: 8),
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
