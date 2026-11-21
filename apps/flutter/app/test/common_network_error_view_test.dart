import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import 'package:operit2/ui/common/components/CommonNetworkErrorView.dart';

/// Verifies summaries used by model configuration error surfaces.
void main() {
  test('structured HTTP status controls the explanation and retains raw text', () {
    const original =
        'list_models request failed: 403 {"message":"blocked","trace":"abc"}';
    final summary = NetworkErrorSummary.fromDetails(
      const core_proxy.CoreProxyErrorDetails(
        errorType: 'ModelConfigError',
        message: original,
        httpStatus: 403,
        remoteMessage: 'blocked',
      ),
      null,
    );
    expect(summary.title, 'HTTP 403：请求被服务器风控拒绝');
    expect(summary.detail, original);
  });

  testWidgets(
    'text-only API errors show the explanation and selectable original',
    (tester) async {
      const original =
          'list_models request failed: 402 {"message":"insufficient balance"}';
      await tester.pumpWidget(
        const MaterialApp(
          home: Scaffold(body: CommonNetworkErrorView(errorText: original)),
        ),
      );
      expect(find.text('HTTP 402：API 欠费'), findsOneWidget);
      expect(
        find.widgetWithText(SelectableText, '原始错误：\n$original'),
        findsOneWidget,
      );
    },
  );

  test('structured status is authoritative when the text has a different code', () {
    final summary = NetworkErrorSummary.fromDetails(
      const core_proxy.CoreProxyErrorDetails(
        errorType: 'ModelConfigError',
        message: 'request failed: 401: original detail',
        httpStatus: 403,
      ),
      null,
    );
    expect(summary.title, 'HTTP 403：请求被服务器风控拒绝');
    expect(summary.detail, 'request failed: 401: original detail');
  });

  test('model duplicates are described with the model and provider names', () {
    final summary = NetworkErrorSummary.fromDetails(
      const core_proxy.CoreProxyErrorDetails(
        errorType: 'ModelConfigError',
        message: 'model already exists: provider-id:gpt-5.6-sol',
        variant: 'ModelAlreadyExists',
        fields: <String, Object?>{
          'providerId': 'provider-id',
          'providerName': 'OpenAI',
          'modelId': 'gpt-5.6-sol',
        },
      ),
      null,
    );

    expect(summary.title, '模型已存在');
    expect(summary.message, '模型“gpt-5.6-sol”已添加到供应商“OpenAI”。');
    expect(summary.detail, isNull);
  });
}
