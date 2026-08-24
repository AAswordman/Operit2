import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/components/CommonNetworkErrorView.dart';

/// Verifies summaries used by model configuration error surfaces.
void main() {
  test('model duplicates are described with the model and provider names', () {
    final l10n = lookupAppLocalizations(const Locale('ja'));
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
      l10n,
      suppressChineseOnlyDetail: true,
    );

    expect(summary.title, '追加済みのモデルです');
    expect(summary.message, 'モデル「gpt-5.6-sol」はAIサービス「OpenAI」へ追加済みです。');
    expect(summary.detail, isNull);
  });

  testWidgets('locale controls Chinese-only raw error visibility', (
    tester,
  ) async {
    Future<void> pumpError(Locale locale) {
      return tester.pumpWidget(
        MaterialApp(
          locale: locale,
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: const Scaffold(
            body: CommonNetworkErrorView(errorText: '连接远程服务失败'),
          ),
        ),
      );
    }

    await pumpError(const Locale('ja'));
    expect(find.text('モデル設定を完了できませんでした'), findsOneWidget);
    expect(find.text('连接远程服务失败'), findsNothing);

    await pumpError(const Locale('en'));
    expect(find.text('Model setup failed'), findsOneWidget);
    expect(find.text('连接远程服务失败'), findsOneWidget);

    await pumpError(const Locale('zh'));
    expect(find.text('模型配置失败'), findsOneWidget);
    expect(find.text('连接远程服务失败'), findsOneWidget);
  });
}
