import 'dart:convert';
import 'dart:io';

import 'package:desktop_multi_window/desktop_multi_window.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:operit2/core/bridge/PlatformCoreProxy.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/host/ComposeWebViewControllerBridge.dart';
import 'package:operit2/core/host/RuntimeHostInteractionSubscriber.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceFileTree.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceHomeContent.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceTopBarButton.dart';
import 'package:operit2/ui/features/chat/components/workspace/file_preview/WorkspaceDocumentPreviewParsers.dart';
import 'package:operit2/ui/features/chat/screens/AIChatScreen.dart';
import 'package:operit2/ui/main/MainLayoutController.dart';
import 'package:operit2/ui/main/TopBarController.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:path_provider/path_provider.dart';

import 'office_chat_fixture.dart';

void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  binding.framePolicy = LiveTestWidgetsFlutterBindingFramePolicy.fullyLive;

  testWidgets(
    'chat edits the open workspace DOCX and the workspace saves it',
    (tester) async {
      if (Platform.isMacOS) {
        // WKWebView does not paint pages while the test window is hidden.
        await (await WindowController.fromCurrentEngine()).show();
      }
      const fixturePath = String.fromEnvironment('OPERIT_OFFICE_FIXTURE');
      final fixture = await File(fixturePath).readAsBytes();
      final support = await getApplicationSupportDirectory();
      final root = await Directory(
        '${support.path}/office-test',
      ).create(recursive: true);
      final directory = await root.createTemp('run-');
      await platformCoreProxy.setRuntimeStorageRoots(
        '${directory.path}/runtime',
        '${directory.path}/workspace',
      );
      await ClientLogger.initialize();
      await LiquidGlassWidgets.initialize();
      final unregister = const ComposeWebViewControllerBridge().registerHandler(
        ComposeDslWebViewHostRegistry.handleControllerCommand,
      );
      addTearDown(unregister);
      RuntimeHostInteractionSubscriber.install();
      addTearDown(RuntimeHostInteractionSubscriber.uninstall);

      final clients = GeneratedCoreProxyClients(const ProxyCoreRuntimeBridge());
      final manager = clients.application.packageManager();
      const owner = 'com.operit.office';
      final imported = await manager.importBundledExternalToolPkgContainer(
        containerPackageName: owner,
      );
      expect(imported, startsWith('Successfully imported package: $owner'));
      final enabled = await manager.enableToolPkgContainer(
        containerPackageName: owner,
      );
      final plugin = await manager.getToolPkgContainerRuntime(
        containerPackageName: owner,
      );
      final issues = await manager.getToolPkgLoadIssues();
      expect(
        plugin,
        isNotNull,
        reason:
            '$imported\n$enabled\n${issues.map((issue) => issue.toJson()).join('\n')}',
      );
      expect(plugin!.subpackages.single.enabledByDefault, isTrue);
      expect(
        await manager.isPackageEnabled(
          packageName: plugin.subpackages.single.packageName,
        ),
        isTrue,
      );
      final chat = clients.chatRuntimeHolderMain;
      final chatId = await chat.createNewChat(
        setAsCurrentChat: true,
        sourceChatId: null,
        parentChatId: null,
        workspaceId: null,
        input: const {},
      );
      await chat.createAndBindWorkspace(chatId: chatId, name: 'Office test');
      final workspace = clients.servicesWorkspaceService;
      final mounts = await workspace.listWorkspaceFiles(
        chatId: chatId,
        relativePath: '',
      );
      final relativePath = '${mounts.single.relativePath}/Smoke.docx';
      await workspace.writeWorkspaceFileBytes(
        chatId: chatId,
        relativePath: relativePath,
        base64Content: base64Encode(fixture),
      );
      expect(
        (await workspace.listWorkspaceFiles(
          chatId: chatId,
          relativePath: mounts.single.relativePath,
        )).map((entry) => entry.relativePath),
        contains(relativePath),
      );
      final provider = await OfficeChatFixture.start(
        clients,
        plugin.subpackages.single.packageName,
      );
      addTearDown(() => provider.server.close(force: true));
      final layout = MainLayoutController();
      final topBar = TopBarController();
      addTearDown(layout.dispose);
      addTearDown(topBar.dispose);
      await tester.pumpWidget(
        OperitTheme(
          initialThemePreferenceSnapshot:
              UserPreferencesManager.defaultThemePreferenceSnapshot,
          initialThemeIsReady: false,
          unconfiguredChildEnabled: true,
          hostInteractionHostsEnabled: false,
          child: MaterialApp(
            locale: const Locale('en'),
            supportedLocales: AppLocalizations.supportedLocales,
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            home: MainLayoutScope(
              controller: layout,
              child: TopBarScope(
                controller: topBar,
                child: Scaffold(
                  appBar: AppBar(
                    actions: [
                      ListenableBuilder(
                        listenable: topBar,
                        builder: (context, _) =>
                            Row(children: topBar.actions?.call(context) ?? []),
                      ),
                    ],
                  ),
                  body: ListenableBuilder(
                    listenable: layout,
                    builder: (context, child) =>
                        layout.decorate(context, child!),
                    child: const AIChatScreen(),
                  ),
                ),
              ),
            ),
          ),
        ),
      );

      Future<void> waitFor(bool Function() ready, String description) async {
        final deadline = Stopwatch()..start();
        while (!ready() && deadline.elapsed < const Duration(seconds: 30)) {
          final error = tester.takeException() ?? provider.failure;
          if (error != null) throw error;
          await tester.pump(const Duration(milliseconds: 100));
        }
        expect(
          ready(),
          isTrue,
          reason: '$description; provider steps: ${provider.steps}',
        );
      }

      final workspaceButton = find.byType(WorkspaceTopBarButton);
      await waitFor(
        () => workspaceButton.evaluate().isNotEmpty,
        'Workspace toggle did not appear',
      );
      await tester.tap(workspaceButton);
      final home = find.byType(WorkspaceHomeContent);
      final tree = find.byType(WorkspaceFileTree);
      final file = find.descendant(of: tree, matching: find.text('Smoke.docx'));
      final mountPath = find.descendant(
        of: home,
        matching: find.byTooltip(mounts.single.path),
      );
      final mount = find.ancestor(
        of: mountPath,
        matching: find.byType(InkWell),
      );
      await waitFor(
        () => mount.evaluate().isNotEmpty,
        'Mounted directory did not appear in the workspace overview',
      );
      await tester.ensureVisible(mount.first);
      await waitFor(
        () => mount.first.hitTestable().evaluate().isNotEmpty,
        'Mounted directory is not clickable',
      );
      await tester.tap(mount.first.hitTestable());
      await waitFor(
        () => tree.evaluate().isNotEmpty,
        'Mounted directory did not open a file tree',
      );
      await waitFor(
        () => file.evaluate().isNotEmpty,
        'Workspace document was not listed',
      );
      await tester.ensureVisible(file);
      await waitFor(
        () => file.hitTestable().evaluate().isNotEmpty,
        'Workspace document is not clickable',
      );
      await tester.tap(file.hitTestable());

      final openWith = find.byTooltip('Open with');
      await waitFor(
        () => openWith.hitTestable().evaluate().isNotEmpty,
        'File tab did not expose its Open with button',
      );
      await tester.tap(openWith.hitTestable());
      final chooser = find.byType(SimpleDialog);
      await waitFor(
        () => chooser.evaluate().isNotEmpty,
        'Open with button did not open the chooser',
      );
      expect(
        find.descendant(of: chooser, matching: find.text('Built-in preview')),
        findsOneWidget,
      );
      final currentOpener = find.ancestor(
        of: find.descendant(
          of: chooser,
          matching: find.byIcon(Icons.radio_button_checked),
        ),
        matching: find.byType(SimpleDialogOption),
      );
      expect(
        find.descendant(
          of: currentOpener,
          matching: find.text('Office documents'),
        ),
        findsOneWidget,
      );
      await tester.tap(currentOpener);
      await waitFor(
        () => chooser.evaluate().isEmpty,
        'Open with chooser did not close',
      );

      final bindPattern = RegExp(
        r'event=controller_bind context=(\S+) .*?key=office\b',
      );
      String? contextKey;
      final bindDeadline = Stopwatch()..start();
      do {
        contextKey = bindPattern
            .allMatches(await ClientLogger.readText())
            .lastOrNull
            ?.group(1);
        if (contextKey != null) break;
        await tester.pump(const Duration(milliseconds: 100));
      } while (bindDeadline.elapsed < const Duration(seconds: 20));
      expect(contextKey, isNotNull, reason: 'Office WebView never bound');

      Future<dynamic> evaluate(String script) async {
        final reply =
            jsonDecode(
                  await ComposeDslWebViewHostRegistry.handleControllerCommand(
                    jsonEncode({
                      'executionContextKey': contextKey,
                      'key': 'office',
                      'command': 'evaluateJavascript',
                      'payload': {'script': script},
                    }),
                  ),
                )
                as Map;
        if (reply['success'] != true) throw StateError('$reply');
        return reply['data'];
      }

      Future<Map> waitForEditor(
        bool Function(Map) ready, {
        int seconds = 10,
      }) async {
        final deadline = Stopwatch()..start();
        Map last;
        do {
          last =
              await evaluate('''(() => {
                    const root = document.getElementById('editor');
                    const rect = root?.getBoundingClientRect();
                    window.officeFrameCount ??= 0;
                    if (!window.officeFrameProbe) {
                      window.officeFrameProbe = true;
                      requestAnimationFrame(() => window.officeFrameCount++);
                    }
                    return {url: location.href,
                      ready: Boolean(window.OfficeEditor && !document.getElementById('quote')?.disabled),
                      visibility: document.visibilityState, frames: window.officeFrameCount,
                      status: document.getElementById('status')?.textContent,
                      error: document.getElementById('error')?.textContent,
                      viewport: [innerWidth, innerHeight], size: [rect?.width, rect?.height],
                      pages: root?.querySelectorAll('.superdoc-page').length,
                      layoutError: root?.querySelector('.presentation-editor__layout-error')?.textContent,
                      text: root?.innerText ?? ''};
                  })()''')
                  as Map;
          if (ready(last)) return last;
          await tester.pump(const Duration(milliseconds: 100));
        } while (deadline.elapsed < Duration(seconds: seconds));
        return last;
      }

      final original = await waitForEditor(
        (data) =>
            data['ready'] == true && '${data['text']}'.contains(officeOriginal),
      );
      expect(original['ready'], isTrue, reason: '$original');
      final input = find.byType(EditableText).hitTestable().first;
      await waitFor(
        () => input.evaluate().isNotEmpty,
        'Chat composer did not appear',
      );
      await tester.enterText(input, officePrompt);
      final send = find.byTooltip('Send');
      await waitFor(
        () => send.evaluate().isNotEmpty,
        'Chat send button did not appear',
      );
      await tester.tap(send.first);
      await waitFor(
        () => provider.steps.contains('done'),
        'Chat did not complete the Office tool call',
      );
      expect(provider.steps, ['use_package', 'list', 'read', 'edit', 'done']);

      Future<String> fileText() async => workspaceDocxPreviewText(
        base64Decode(
          (await workspace.readWorkspaceFileBytes(
            chatId: chatId,
            relativePath: relativePath,
          )).base64Content,
        ),
      );
      expect(await fileText(), contains(officeOriginal));
      final updated = await waitForEditor(
        (data) => '${data['text']}'.contains(officeEdited),
      );
      expect(
        original['text'],
        contains(officeOriginal),
        reason: 'Initial render: $original',
      );
      expect(
        updated['text'],
        contains(officeEdited),
        reason: 'Updated render: $updated',
      );
      expect(updated['pages'], greaterThan(0));

      expect(
        await evaluate('''(() => {
        const paragraph = [...document.querySelectorAll('#editor .superdoc-page p')]
          .find(node => node.textContent === ${jsonEncode(officeEdited)});
        if (!paragraph) return false;
        const node = paragraph.getBoundingClientRect();
        paragraph.dispatchEvent(new MouseEvent('mousedown', {bubbles: true,
          clientX: node.left + 1, clientY: node.top + node.height / 2}));
        paragraph.dispatchEvent(new MouseEvent('mouseup', {bubbles: true,
          clientX: node.left + 1, clientY: node.top + node.height / 2}));
        const active = document.activeElement;
        if (!active?.isContentEditable) return false;
        active.dispatchEvent(new InputEvent('beforeinput', {
          inputType: 'insertText', data: 'Manual: ', bubbles: true, cancelable: true,
        }));
        return true;
      })()'''),
        isTrue,
      );
      final manual = await waitForEditor(
        (data) => '${data['text']}'.contains('Manual: '),
      );
      expect(manual['text'], contains('Manual: '));
      await evaluate("document.getElementById('save').click(); true");
      final saveDeadline = Stopwatch()..start();
      String saved;
      do {
        saved = await fileText();
        if (saved.contains('Manual: ')) break;
        await tester.pump(const Duration(milliseconds: 100));
      } while (saveDeadline.elapsed < const Duration(seconds: 10));
      expect(saved, contains('Manual: '));
      expect(saved, contains(officeEdited));
      await tester.pumpWidget(const SizedBox.shrink());
    },
    timeout: const Timeout(Duration(minutes: 3)),
  );
}
