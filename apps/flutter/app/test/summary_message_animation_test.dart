import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/style/SummaryMessageComposable.dart';

import 'support/markdown_event_fixture.dart';

void main() {
  for (final brightness in Brightness.values) {
    for (final width in [280.0, 640.0]) {
      for (final locale in [const Locale('zh'), const Locale('en')]) {
        testWidgets(
          'summary contracts without fading ($brightness, $width, $locale)',
          (tester) async {
            await tester.pumpWidget(
              MaterialApp(
                locale: locale,
                localizationsDelegates: AppLocalizations.localizationsDelegates,
                supportedLocales: AppLocalizations.supportedLocales,
                theme: ThemeData(brightness: brightness),
                home: Scaffold(
                  body: Align(
                    alignment: Alignment.topCenter,
                    child: ConstrainedBox(
                      constraints: BoxConstraints(maxWidth: width),
                      child: Column(
                        mainAxisSize: MainAxisSize.min,
                        crossAxisAlignment: CrossAxisAlignment.start,
                        children: [
                          SummaryMessageComposable(
                            message: _message,
                            onEdit: (_) {},
                            onDelete: () {},
                            splitMarkdownContent: splitMarkdownEventFixture,
                          ),
                          const SizedBox(
                            key: ValueKey('following-message'),
                            width: 100,
                            height: 40,
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
            );
            await tester.pumpAndSettle();

            final summary = find.byType(SummaryMessageComposable);
            final vessel = find.byKey(const ValueKey('summary-vessel'));
            final toggle = find.byKey(const ValueKey('summary-toggle'));
            final collapsedSize = tester.getSize(vessel);
            final collapsedTop = tester.getTopLeft(summary).dy;
            final originalShell = tester.element(vessel);

            void expectOpaque() {
              expect(
                find.descendant(of: summary, matching: find.byType(Opacity)),
                findsNothing,
              );
              expect(
                find.descendant(
                  of: summary,
                  matching: find.byType(FadeTransition),
                ),
                findsNothing,
              );
              expect(tester.element(vessel), same(originalShell));
              expect(tester.getTopLeft(summary).dy, collapsedTop);
              expect(tester.getSize(summary).width, width);
              expect(
                tester
                    .getTopLeft(find.byKey(const ValueKey('following-message')))
                    .dy,
                closeTo(tester.getBottomLeft(summary).dy, 0.01),
              );
              expect(
                tester.getCenter(vessel).dx,
                closeTo(tester.getCenter(summary).dx, 0.01),
              );
              final divider = find
                  .descendant(of: summary, matching: find.byType(Divider))
                  .first;
              expect(tester.getSize(divider).width, width);
              expect(tester.takeException(), isNull);
            }

            await tester.tap(toggle);
            await tester.pump();
            expect(tester.getSize(vessel), collapsedSize);
            await tester.pump(const Duration(milliseconds: 64));
            final openingSize = tester.getSize(vessel);
            expect(openingSize.height, greaterThan(collapsedSize.height));
            expect(
              openingSize.width,
              greaterThanOrEqualTo(collapsedSize.width),
            );
            expectOpaque();
            await tester.pump(const Duration(milliseconds: 136));
            await tester.pumpAndSettle();
            final expandedSize = tester.getSize(vessel);
            expect(openingSize.height, lessThan(expandedSize.height));
            expect(expandedSize.width, closeTo(width - 20, 0.01));
            expectOpaque();

            final actionIcons = [
              Icons.content_copy_rounded,
              Icons.edit_outlined,
              Icons.delete_outline,
              Icons.keyboard_arrow_up,
            ].map(find.byIcon).toList();
            expect(
              tester.getTopRight(vessel).dx -
                  tester.getTopRight(actionIcons.last).dx,
              closeTo(14, 0.01),
            );
            for (var i = 1; i < actionIcons.length; i++) {
              expect(
                tester.getCenter(actionIcons[i]).dx -
                    tester.getCenter(actionIcons[i - 1]).dx,
                closeTo(27, 0.01),
              );
              expect(
                tester.getCenter(actionIcons[i]).dy,
                closeTo(tester.getCenter(actionIcons.first).dy, 0.01),
              );
            }

            await tester.tap(toggle);
            await tester.pump();
            expect(tester.getSize(vessel), expandedSize);
            await tester.pump(const Duration(milliseconds: 48));
            final closingSize = tester.getSize(vessel);
            expect(
              closingSize.height,
              inExclusiveRange(collapsedSize.height, expandedSize.height),
            );
            // The contraction should be fast immediately, then ease into the pill.
            expect(
              expandedSize.height - closingSize.height,
              greaterThan((expandedSize.height - collapsedSize.height) / 2),
            );
            if (collapsedSize.width < expandedSize.width) {
              expect(
                closingSize.width,
                inExclusiveRange(collapsedSize.width, expandedSize.width),
              );
            }
            expectOpaque();
            await tester.pump(const Duration(milliseconds: 96));
            final beforeEnd = tester.getSize(vessel);
            await tester.pump(const Duration(milliseconds: 16));
            final afterEnd = tester.getSize(vessel);
            expect(afterEnd.width, closeTo(collapsedSize.width, 0.01));
            expect(afterEnd.height, closeTo(collapsedSize.height, 0.01));
            expect(beforeEnd.height, closeTo(afterEnd.height, 1));
            expect(beforeEnd.width, closeTo(afterEnd.width, 1));
            expectOpaque();
            await tester.pumpAndSettle();
          },
        );
      }
    }
  }
}

final _message = ChatMessage(
  pluginExtensions: const {},
  sender: 'ai',
  parts: const [
    MessagePart(
      partId: 'summary',
      sequence: 0,
      kind: MessagePartKind.markdown,
      content: 'Summary content.',
      toolCallId: null,
      toolName: null,
      attributes: {},
    ),
  ],
  timestamp: 1,
  roleName: 'assistant',
  selectedVariantIndex: 0,
  variantCount: 1,
  provider: 'test',
  modelName: 'test',
  inputTokens: 8300,
  outputTokens: 2800,
  cachedInputTokens: 0,
  sentAt: 0,
  outputDurationMs: 0,
  waitDurationMs: 7000,
  completedAt: 1,
  displayMode: ChatMessageDisplayMode.normal,
  isFavorite: false,
  contentStream: null,
);
