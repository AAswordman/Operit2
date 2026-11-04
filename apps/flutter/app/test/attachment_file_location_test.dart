import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/ui/features/chat/components/attachments/AttachmentViewerDialog.dart';
import 'package:operit2/ui/features/chat/components/style/input/common/ChatAttachmentImagePreview.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  test(
    'attachment previews use the same VFS reader for workspace, output and historical files',
    () async {
      for (final path in [
        '/operitapp/workspaces/project/.operit/attachments/id/photo.png',
        '/operitapp/outputs/family/.operit/attachments/id/photo.png',
        '/app/data/data/chat_attachments/old/id/photo.png',
      ]) {
        final bridge = _FileBridge();
        expect(
          await readChatAttachmentFileBytes(
            path,
            sourceNodeId: 'origin',
            clients: GeneratedCoreProxyClients(bridge),
          ),
          [0, 1, 255],
        );
        expect(bridge.paths, [path]);
      }
    },
  );

  test(
    'missing synchronized attachments use their recorded source without reinterpreting native paths',
    () async {
      final bridge = _FileBridge(missingLocal: true);
      const path = '/operitapp/outputs/family/.operit/attachments/id/photo.png';
      expect(
        await readChatAttachmentFileBytes(
          path,
          sourceNodeId: 'origin',
          clients: GeneratedCoreProxyClients(bridge),
        ),
        [0, 1, 255],
      );
      expect(bridge.paths, [path, 'origin:$path']);
      bridge.paths.clear();
      expect(
        await readChatAttachmentFileBytes(
          'origin:$path',
          sourceNodeId: 'origin',
          clients: GeneratedCoreProxyClients(bridge),
        ),
        [0, 1, 255],
      );
      expect(bridge.paths, ['origin:$path']);
    },
  );

  testWidgets('file preview displays and copies the stable source location', (
    tester,
  ) async {
    String? copied;
    tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
      SystemChannels.platform,
      (call) async {
        if (call.method == 'Clipboard.setData') {
          copied = (call.arguments as Map)['text'] as String;
        }
        return null;
      },
    );
    addTearDown(
      () => tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        null,
      ),
    );
    const location =
        'origin:/operitapp/outputs/family/.operit/attachments/id/report.pdf';
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: AttachmentViewerDialog(
            visible: true,
            attachment: const ChatAttachment(
              id: '/operitapp/outputs/family/.operit/attachments/id/report.pdf',
              path:
                  '/operitapp/outputs/family/.operit/attachments/id/report.pdf',
              nodeId: 'origin',
              filename: 'report.pdf',
              mimeType: 'application/pdf',
            ),
            onDismiss: () {},
          ),
        ),
      ),
    );
    expect(find.text('保存位置：$location'), findsOneWidget);
    await tester.tap(find.byTooltip('复制文件位置'));
    await tester.pump();
    expect(copied, location);
    expect(tester.takeException(), isNull);
  });

  test('media pool identities are not reported as storage locations', () {
    expect(
      const ChatAttachment(
        id: 'pool-id',
        filename: 'image',
        mimeType: 'image/png',
        mediaPoolType: 'image',
      ).fileLocation,
      isNull,
    );
    expect(
      const ChatAttachment(
        id: 'pool-id',
        filename: 'image',
        mimeType: 'image/png',
        mediaPoolType: 'image',
        path: '/operitapp/outputs/family/photo.png',
        nodeId: 'origin',
      ).fileLocation,
      'origin:/operitapp/outputs/family/photo.png',
    );
  });
}

class _FileBridge extends OperitRuntimeBridge {
  _FileBridge({this.missingLocal = false});
  final bool missingLocal;
  final paths = <String>[];

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    expect(request.methodName, 'executeTool');
    final tool = (request.args as Map)['tool'] as Map;
    expect(tool['name'], 'read_file_binary');
    final path =
        ((tool['parameters'] as List).single as Map)['value'] as String;
    paths.add(path);
    if (missingLocal && path.startsWith('/')) {
      throw StateError('not synchronized');
    }
    return encodeCoreLink([
      0,
      {
        'toolName': 'read_file_binary',
        'success': true,
        'error': null,
        'result': {
          '__type': 'BinaryFileContentData',
          'path': path,
          'contentBase64': base64Encode([0, 1, 255]),
          'size': 3,
        },
      },
    ]);
  }

  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}
