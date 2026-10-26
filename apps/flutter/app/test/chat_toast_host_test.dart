import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/AppToastController.dart';
import 'package:operit2/ui/common/AppToastHost.dart';

/// Verifies that wrapped notice messages retain every visible line.
void main() {
  setUp(AppToastController.instance.reset);

  tearDown(AppToastController.instance.reset);

  testWidgets('wrapped notice messages are not constrained to one line', (
    tester,
  ) async {
    const message = '请先在设置中添加并选中一个语音识别配置，然后再开始语音输入。';

    await tester.pumpWidget(
      const MaterialApp(
        home: AppToastHost(child: ColoredBox(color: Colors.white)),
      ),
    );
    AppToastController.instance.show(message);
    await tester.pump();

    expect(tester.getSize(find.text(message)).height, greaterThan(28));
    AppToastController.instance.reset();
  });
}
