import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/l10n/generated/app_localizations_en.dart';
import 'package:operit2/l10n/generated/app_localizations_zh.dart';
import 'package:operit2/ui/features/settings/runtime/SpaceAuditLabels.dart';

void main() {
  final zh = AppLocalizationsZh();
  test('all current Core audit commands have readable actions and targets', () {
    final cases = <String, ({String action, String target})>{
      'Bootstrap control; administrator Mac': (
        action: '初始化空间权限',
        target: 'Mac',
      ),
      'Define role Reader': (action: '新建身份', target: 'Reader'),
      'Set identity Auditor on tablet': (
        action: '设置设备身份',
        target: 'tablet · 审计员',
      ),
      'Reset identity on tablet to the default user': (
        action: '重置为默认身份',
        target: 'tablet',
      ),
      'Admit device tablet': (action: '接纳设备', target: 'tablet'),
      'Merge Space source-id; admit tablet, Mac': (
        action: '合并设备空间',
        target: 'tablet, Mac',
      ),
      'Remove device tablet': (action: '移除设备', target: 'tablet'),
      'Disconnect device tablet': (action: '断开设备', target: 'tablet'),
      'Update policy test to enabled': (
        action: '更新权限策略',
        target: 'test to enabled',
      ),
    };
    for (final entry in cases.entries) {
      expect(spaceAuditSummary(entry.key, zh), entry.value);
    }
  });

  test('unknown or malformed summaries are preserved without throwing', () {
    for (final summary in <String>[
      'Future control command',
      'Set identity User',
      'Reset identity on tablet',
      'Merge Space x',
    ]) {
      expect(spaceAuditSummary(summary, zh), (action: summary, target: ''));
    }
  });

  test('device and custom identity names are not translated or stripped', () {
    expect(spaceAuditSummary('Set identity 项目负责人 on Android · 小王', zh), (
      action: '设置设备身份',
      target: 'Android · 小王 · 项目负责人',
    ));
  });

  test(
    'known identity names disambiguate sentence separators in both names',
    () {
      expect(
        spaceAuditSummary(
          'Set identity Team on call on Tablet on desk',
          zh,
          identityNames: <String>['Team', 'Team on call'],
        ),
        (action: '设置设备身份', target: 'Tablet on desk · Team on call'),
      );
    },
  );

  test('English actions use the English locale', () {
    expect(
      spaceAuditSummary('Set identity Auditor on tablet', AppLocalizationsEn()),
      (action: 'Set device identity', target: 'tablet · Auditor'),
    );
  });
}
