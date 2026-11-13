import 'dart:async';
import 'dart:typed_data';

import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/features/packages/dialogs/SkillImportDialog.dart';

void main() {
  testWidgets('ZIP import applies the chosen scope to every new skill', (
    tester,
  ) async {
    final bridge = _SkillImportBridge(['alpha', 'beta']);
    final result = await _importZip(tester, bridge);

    expect(result?.message, bridge.message);
    expect(bridge.scopedSkills, {'alpha': 'space', 'beta': 'space'});
    expect(bridge.scopedSkills, isNot(contains('existing')));
    expect(find.byType(SkillImportDialog), findsNothing);
  });

  testWidgets('partial ZIP success preserves the import report', (
    tester,
  ) async {
    final bridge = _SkillImportBridge(['alpha'])
      ..message = 'Imported 1 skills; failed: 1\nSkill beta already exists';
    final result = await _importZip(tester, bridge);

    expect(result?.message, bridge.message);
    expect(bridge.scopedSkills, {'alpha': 'space'});
  });

  testWidgets('a ZIP with no imported skills stays open and shows the error', (
    tester,
  ) async {
    final bridge = _SkillImportBridge([])
      ..message = 'No SKILL.md found in the imported zip';
    final result = await _importZip(tester, bridge);

    expect(result, isNull);
    expect(bridge.scopedSkills, isEmpty);
    expect(find.byType(SkillImportDialog), findsOneWidget);
    expect(find.textContaining(bridge.message), findsOneWidget);
  });
}

Future<SkillImportResult?> _importZip(
  WidgetTester tester,
  _SkillImportBridge bridge,
) async {
  final originalSelector = FileSelectorPlatform.instance;
  FileSelectorPlatform.instance = _ZipSelector();
  addTearDown(() => FileSelectorPlatform.instance = originalSelector);
  SkillImportResult? result;
  await tester.pumpWidget(
    MaterialApp(
      home: Scaffold(
        body: Builder(
          builder: (context) => TextButton(
            onPressed: () async {
              result = await showDialog<SkillImportResult>(
                context: context,
                builder: (_) => SkillImportDialog(
                  clients: GeneratedCoreProxyClients(bridge),
                ),
              );
            },
            child: const Text('Open import'),
          ),
        ),
      ),
    ),
  );
  await tester.tap(find.text('Open import'));
  await tester.pumpAndSettle();
  await tester.tap(find.text('ZIP'));
  await tester.pumpAndSettle();
  await tester.tap(find.text('选择 ZIP 文件'));
  await tester.pumpAndSettle();
  await tester.tap(find.text('导入'));
  await tester.pumpAndSettle();
  await tester.tap(find.text('设备空间'));
  await tester.pumpAndSettle();
  return result;
}

class _ZipSelector extends FileSelectorPlatform {
  @override
  Future<XFile?> openFile({
    List<XTypeGroup>? acceptedTypeGroups,
    String? initialDirectory,
    String? confirmButtonText,
  }) async => XFile('/tmp/skills.zip');
}

class _SkillImportBridge extends OperitRuntimeBridge {
  _SkillImportBridge(this.newSkills);

  final List<String> newSkills;
  final Map<String, String> scopedSkills = {};
  bool imported = false;
  String message = 'Imported 2 skills; failed: 0';

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    final Object? result;
    switch (request.methodName) {
      case 'getAvailableSkillPackages':
        result = {
          for (final id in ['existing', if (imported) ...newSkills])
            id: SkillPackage(
              name: id,
              description: '',
              directory: '/operitapp/data/extensions/device/skills/$id',
              skillFile:
                  '/operitapp/data/extensions/device/skills/$id/SKILL.md',
            ).toJson(),
        };
      case 'importSkillFromZip':
        expect((request.args as Map)['zipFile'], '/tmp/skills.zip');
        imported = true;
        result = message;
      case 'setExtensionScope':
        final args = request.args as Map;
        expect(args['kind'], 'skill');
        scopedSkills[args['id'] as String] = args['scope'] as String;
        result = null;
      default:
        throw UnsupportedError('Unexpected Core call: ${request.methodName}');
    }
    return encodeCoreLink(<Object?>[0, result]);
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      const Stream.empty();
}
