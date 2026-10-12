import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';

void main() {
  final expectedLabels = <String, List<String>>{
    'zh': <String>[
      '主题库',
      '全局主题 · 本节点个人外观',
      '全局主题 · 主题包 · 视觉设置只读',
      '主题包的配色、背景、气泡、字体与输入样式为只读。请在主题库显式选择个人外观后编辑。',
    ],
    'en': <String>[
      'Theme library',
      'Global theme · Personal appearance on this node',
      'Global theme · Theme package · Visual settings are read-only',
      "A theme package's colors, background, bubbles, fonts and input style "
          'are read-only. Explicitly select personal appearance in the theme '
          'library before editing.',
    ],
  };

  for (final entry in expectedLabels.entries) {
    test('appearance theme status labels are readable in ${entry.key}', () {
      final l10n = lookupAppLocalizations(Locale(entry.key));
      final labels = <String>[
        l10n.settingsAppearanceThemeLibraryTab,
        l10n.settingsAppearancePersonalThemeSummary,
        l10n.settingsAppearancePackageThemeSummary,
        l10n.settingsAppearancePackageVisualsReadOnly,
      ];

      expect(labels, entry.value);
      for (final label in labels) {
        expect(label, isNot(contains('???')));
        expect(label, isNot(contains('\uFFFD')));
      }
    });
  }
}
