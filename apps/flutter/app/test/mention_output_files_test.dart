import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/chat/components/style/input/common/MentionSuggestionPanel.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

void main() {
  testWidgets('unbound chat lists and selects its output files', (tester) async {
    final model = _ChatModel();
    String? selected;
    await tester.pumpWidget(OperitTheme(
      initialThemePreferenceSnapshot: UserPreferencesManager.defaultThemePreferenceSnapshot,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: Scaffold(body: MentionSuggestionPanel(
        viewModel: model,
        chatId: 'chat-a',
        searchQuery: 'report',
        triggerChar: '@',
        hasBoundWorkspace: false,
        onPackageSelected: (_) {},
        onFileSelected: (path) => selected = path,
      )),
    ));
    await tester.pumpAndSettle();
    expect(model.queries, ['report']);
    final file = find.text('report.md', findRichText: true);
    expect(file, findsOneWidget);
    await tester.tap(file);
    expect(selected, 'report.md');
  });
}

class _ChatModel extends ChatViewModel {
  _ChatModel() : super(bridge: _Bridge());
  final queries = <String>[];

  @override
  Future<List<WorkspaceFileEntry>> listMentionWorkspaceSuggestions(String query) async {
    queries.add(query);
    return const [WorkspaceFileEntry(
      name: 'report.md', path: '/operitapp/outputs/chat/report.md',
      relativePath: 'report.md', isDirectory: false, size: 12, lastModified: '',
    )];
  }
}

class _Bridge extends OperitRuntimeBridge {
  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}
