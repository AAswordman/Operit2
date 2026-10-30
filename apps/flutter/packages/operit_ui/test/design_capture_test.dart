import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';

List<Map<String, Object?>> nodes(Map<String, Object?> document) {
  final result = <Map<String, Object?>>[];
  void visit(Map<String, Object?> node) {
    result.add(node);
    for (final child in node['children'] as List? ?? []) {
      visit(child as Map<String, Object?>);
    }
  }

  for (final scene in document['scenes'] as List) {
    visit(scene as Map<String, Object?>);
  }
  return result;
}

void main() {
  testWidgets(
    'captures nested text and shapes with owner, clipping and no hidden content',
    (tester) async {
      final key = GlobalKey();
      await tester.pumpWidget(
        MaterialApp(
          home: RepaintBoundary(
            key: key,
            child: Scaffold(
              body: Column(
                children: [
                  OperitDesignSource(
                    source: const {
                      'kind': 'plugin-compose',
                      'package': 'demo',
                      'composeId': 'root:4',
                    },
                    child: Container(
                      width: 320,
                      height: 120,
                      padding: const EdgeInsets.all(12),
                      decoration: BoxDecoration(
                        color: const Color(0xff336699),
                        borderRadius: BorderRadius.circular(16),
                      ),
                      child: const Text.rich(
                        TextSpan(
                          children: [
                            TextSpan(
                              text: 'Deep plugin ',
                              style: TextStyle(
                                color: Colors.white,
                                fontSize: 18,
                              ),
                            ),
                            TextSpan(
                              text: 'UI',
                              style: TextStyle(
                                color: Colors.yellow,
                                fontWeight: FontWeight.bold,
                              ),
                            ),
                          ],
                        ),
                      ),
                    ),
                  ),
                  const Offstage(child: Text('SECRET OFFSTAGE')),
                  const Opacity(opacity: 0, child: Text('SECRET TRANSPARENT')),
                  const SizedBox(
                    height: 40,
                    child: ClipRect(
                      child: SingleChildScrollView(
                        child: Column(
                          children: [
                            Text('Visible tool result'),
                            SizedBox(height: 80),
                            Text('HIDDEN SCROLLED'),
                          ],
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      );
      await tester.pump();
      final document = await tester.runAsync(
        () => captureOperitDesign(
          key.currentContext!.findRenderObject()! as RenderRepaintBoundary,
          name: 'Deep UI',
        ),
      );
      final all = nodes(document!);
      final text = all.singleWhere((n) => n['text'] == 'Deep plugin UI');
      expect((text['source'] as Map)['package'], 'demo');
      expect((text['source'] as Map)['composeId'], 'root:4');
      expect((text['runs'] as List).length, 2);
      expect(
        all.any((n) => n['fill'] == '#336699FF' && n['radius'] == 16),
        isTrue,
      );
      expect(all.any((n) => n['text'] == 'Visible tool result'), isTrue);
      expect(all.any((n) => n['text'].toString().contains('SECRET')), isFalse);
      expect(all.any((n) => n['text'] == 'HIDDEN SCROLLED'), isFalse);
      expect((document['assets'] as List).isNotEmpty, isTrue);
      expect((document['scenes'] as List).length, 2);
      final file = File('.dart_tool/design-capture-test.operit-design.json');
      file.parent.createSync(recursive: true);
      file.writeAsStringSync(jsonEncode(document));
    },
  );

  testWidgets(
    'captures editable input safely and marks custom paint honestly',
    (tester) async {
      final key = GlobalKey();
      await tester.pumpWidget(
        MaterialApp(
          home: RepaintBoundary(
            key: key,
            child: Scaffold(
              body: Column(
                children: [
                  const TextField(
                    obscureText: true,
                    decoration: InputDecoration(labelText: 'Password'),
                  ),
                  CustomPaint(
                    size: const Size(60, 40),
                    painter: _ChartPainter(),
                  ),
                ],
              ),
            ),
          ),
        ),
      );
      await tester.enterText(find.byType(TextField), 'do-not-export-me');
      await tester.pump();
      final document = await tester.runAsync(
        () => captureOperitDesign(
          key.currentContext!.findRenderObject()! as RenderRepaintBoundary,
          includeReference: false,
        ),
      );
      final all = nodes(document!);
      expect(all.any((n) => n['text'] == '••••••'), isTrue);
      expect(jsonEncode(document).contains('do-not-export-me'), isFalse);
      expect(
        all.any(
          (n) => (n['source'] as Map?)?['rasterReason'] == 'custom-paint',
        ),
        isTrue,
      );
      expect((document['scenes'] as List).length, 1);
    },
  );
}

class _ChartPainter extends CustomPainter {
  @override
  void paint(Canvas canvas, Size size) =>
      canvas.drawCircle(const Offset(20, 20), 12, Paint()..color = Colors.blue);
  @override
  bool shouldRepaint(covariant CustomPainter oldDelegate) => false;
}
