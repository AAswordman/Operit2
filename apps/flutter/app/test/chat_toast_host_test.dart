import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/chat/components/ChatToastHost.dart';

/// Verifies that wrapped short toast messages retain every visible line.
void main() {
  testWidgets('wrapped toast messages are not constrained to one line', (
    tester,
  ) async {
    const message = '请先在设置中添加并选中一个语音识别配置，然后再开始语音输入。';

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: Center(
            child: SizedBox(
              width: 280,
              child: ChatToastHost(message: message, onDismiss: () {}),
            ),
          ),
        ),
      ),
    );

    expect(tester.getSize(find.text(message)).height, greaterThan(28));
  });
}
