import 'support/native_sidebar_bridge.dart';

import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/chat/components/style/input/agent/AgentChatInputSection.dart';
import 'package:operit2/ui/features/chat/components/style/input/classic/ClassicChatInputSection.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';
import 'package:operit2/ui/main/MainLayoutController.dart';
import 'package:operit2/ui/main/TopBarController.dart';
import 'package:operit2/ui/main/components/AppContent.dart';
import 'package:operit2/ui/main/components/DrawerConversationState.dart';
import 'package:operit2/ui/main/layout/PhoneLayout.dart';
import 'package:operit2/ui/main/layout/TabletLayout.dart';
import 'package:operit2/ui/main/navigation/AppNavigationModels.dart';
import 'package:operit2/ui/main/screens/OperitScreens.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Checks real composers in the shared host, including already-safe pages.
void main() {
  setUp(ClientLogger.initialize);
  test('chat routes own the bottom system inset', () {
    expect(const AiChatScreenRoute().handlesBottomSafeArea, isTrue);
  });
  for (final tablet in <bool>[false, true]) {
    for (final agent in <bool>[true, false]) {
      for (final floating in <bool>[false, true]) {
        for (final (handlesBottomSafeArea, transparent) in <(bool, bool)>[
          (false, false),
          (true, false),
          (true, true),
        ]) {
          testWidgets(
            '${tablet ? 'tablet' : 'phone'} ${agent ? 'agent' : 'classic'} '
            'composer avoids system bars and keyboard (floating: $floating, '
            'page owns inset: $handlesBottomSafeArea, glass: $transparent)',
            (tester) async {
              final width = tablet ? 900.0 : 400.0;
              tester.view.devicePixelRatio = 1;
              tester.view.physicalSize = Size(width, 800);
              tester.view.padding = const FakeViewPadding(top: 24, bottom: 48);
              tester.view.viewPadding = const FakeViewPadding(
                top: 24,
                bottom: 48,
              );
              addTearDown(tester.view.resetDevicePixelRatio);
              addTearDown(tester.view.resetPhysicalSize);
              addTearDown(tester.view.resetPadding);
              addTearDown(tester.view.resetViewPadding);
              addTearDown(tester.view.resetViewInsets);
              final controller = TextEditingController();
              final focus = FocusNode();
              final layout = MainLayoutController();
              final topBar = TopBarController();
              final drawerOpen = ValueNotifier<bool>(false);
              final conversations = ValueNotifier<DrawerConversationState>(
                const DrawerConversationState(loading: false),
              );
              final entry = RouteEntry(instanceId: 'chat', routeId: 'ai_chat');
              final router = AppRouterState(entry);
              addTearDown(controller.dispose);
              addTearDown(focus.dispose);
              addTearDown(layout.dispose);
              addTearDown(topBar.dispose);
              addTearDown(drawerOpen.dispose);
              addTearDown(conversations.dispose);
              addTearDown(router.dispose);
              final screen = _ComposerRoute(
                agent: agent,
                handlesBottomSafeArea: handlesBottomSafeArea,
                nestedSafeArea: floating && !handlesBottomSafeArea,
                controller: controller,
                focus: focus,
                viewModel: ChatViewModel(bridge: _PendingModelBridge()),
              );
              final content = AppContent(
                routerState: router,
                currentScreen: screen,
                currentRouteEntry: entry,
                currentRouteTitle: 'Chat',
                useTabletLayout: tablet,
                isTabletSidebarExpanded: false,
                canGoBack: false,
                enableNavigationAnimation: false,
                isNavigatingBack: false,
                topBarController: topBar,
                appBarEntries: const [],
                onGoBack: () {},
                onNavigationButtonPressed: () {},
                onAppBarEntrySelected: (_) {},
              );
              await tester.pumpWidget(
                OperitTheme(
                  initialThemePreferenceSnapshot:
                      ThemePreferenceSnapshot.fromJson({
                        ...UserPreferencesManager.defaultThemePreferenceSnapshot
                            .toJson(),
                        'inputStyle': agent ? 'agent' : 'classic',
                        'chatInputFloating': floating,
                        'transparentSurfaceEnabled': transparent,
                      }),
                  initialThemeIsReady: false,
                  unconfiguredChildEnabled: true,
                  hostInteractionHostsEnabled: false,
                  child: MainLayoutScope(
                    controller: layout,
                    child: Scaffold(
                      body: tablet
                          ? TabletLayout(
                              bridge: NativeSidebarBridge(),
                              content: content,
                              navigationEntries: const [
                                NavigationEntrySpec(
                                  entryId: 'main.ai_chat',
                                  routeId: 'ai_chat',
                                  surface: NavigationSurface.mainSidebarAi,
                                  title: 'Chat',
                                  icon: Icons.chat_outlined,
                                ),
                                NavigationEntrySpec(
                                  entryId: 'main.package_manager',
                                  routeId: 'package_manager',
                                  surface: NavigationSurface.mainSidebarSystem,
                                  title: 'Packages',
                                  icon: Icons.inventory_2_outlined,
                                ),
                                NavigationEntrySpec(
                                  entryId: 'main.settings',
                                  routeId: 'settings',
                                  surface: NavigationSurface.mainSidebarSystem,
                                  title: 'Settings',
                                  icon: Icons.settings_outlined,
                                ),
                              ],
                              pluginSidebarEntries: const [],
                              selectedRouteId: 'ai_chat',
                              drawerConversationState: conversations,
                              isTabletSidebarExpanded: false,
                              tabletSidebarWidth: 280,
                              collapsedTabletSidebarWidth: 56,
                              onNavigationEntrySelected: (_) {},
                              onConversationActivated: () {},
                            )
                          : PhoneLayout(
                              bridge: NativeSidebarBridge(),
                              content: content,
                              navigationEntries: const [
                                NavigationEntrySpec(
                                  entryId: 'main.ai_chat',
                                  routeId: 'ai_chat',
                                  surface: NavigationSurface.mainSidebarAi,
                                  title: 'Chat',
                                  icon: Icons.chat_outlined,
                                ),
                                NavigationEntrySpec(
                                  entryId: 'main.package_manager',
                                  routeId: 'package_manager',
                                  surface: NavigationSurface.mainSidebarSystem,
                                  title: 'Packages',
                                  icon: Icons.inventory_2_outlined,
                                ),
                                NavigationEntrySpec(
                                  entryId: 'main.settings',
                                  routeId: 'settings',
                                  surface: NavigationSurface.mainSidebarSystem,
                                  title: 'Settings',
                                  icon: Icons.settings_outlined,
                                ),
                              ],
                              pluginSidebarEntries: const [],
                              selectedRouteId: 'ai_chat',
                              drawerConversationState: conversations,
                              drawerWidth: 300,
                              drawerOpenState: drawerOpen,
                              enableNavigationAnimation: false,
                              onOpenDrawer: () => drawerOpen.value = true,
                              onCloseDrawer: () => drawerOpen.value = false,
                              onNavigationEntrySelected: (_) {},
                              onConversationActivated: () {},
                            ),
                    ),
                  ),
                ),
              );
              await tester.pumpAndSettle();
              final page = find.byKey(const ValueKey<String>('safe-area-page'));
              final composer = find.byType(
                agent ? AgentChatInputSection : ClassicChatInputSection,
              );
              final field = find.byKey(const ValueKey<String>('chat.input'));
              final messages = find.byKey(
                const ValueKey<String>('safe-area-message-content'),
              );
              final surface = find.byKey(
                const ValueKey<String>('chat.input.surface'),
              );
              final inputState = tester.state(composer);
              await tester.tap(field);
              await tester.enterText(field, 'draft survives inset changes');
              await tester.pumpAndSettle();
              final originalFieldRect = tester.getRect(field);
              final originalMessagesBottom = tester.getRect(messages).bottom;

              /// Checks surface placement and control safety during inset changes.
              void expectBottom(double inset, {double keyboard = 0}) {
                final pageBottom = handlesBottomSafeArea
                    ? 800 - keyboard
                    : 800 - inset;
                expect(tester.getRect(page).bottom, closeTo(pageBottom, 0.1));
                expect(
                  tester.getRect(composer).bottom,
                  closeTo(pageBottom, 0.1),
                );
                expect(
                  tester.getRect(surface).bottom,
                  closeTo(
                    handlesBottomSafeArea && !floating
                        ? 800 - keyboard
                        : 800 - inset - (floating ? 6 : 0),
                    0.1,
                  ),
                );
                expect(
                  tester.getRect(field).bottom,
                  closeTo(originalFieldRect.bottom + 48 - inset, 0.1),
                );
                expect(
                  tester.getRect(field).top,
                  closeTo(originalFieldRect.top + 48 - inset, 0.1),
                );
                expect(
                  tester.getRect(messages).bottom,
                  closeTo(originalMessagesBottom + 48 - inset, 0.1),
                );
                // The top bar still handles the status bar exactly once.
                expect(tester.getRect(page).top, closeTo(24 + 64, 0.1));
                expect(tester.state(composer), same(inputState));
                expect(controller.text, 'draft survives inset changes');
                expect(focus.hasFocus, isTrue);
                expect(tester.takeException(), isNull);
              }

              // Change navigation modes without remounting the page.
              for (final bottom in <double>[48, 24, 80, 0]) {
                tester.view.padding = FakeViewPadding(top: 24, bottom: bottom);
                tester.view.viewPadding = FakeViewPadding(
                  top: 24,
                  bottom: bottom,
                );
                await tester.pumpAndSettle();
                expectBottom(bottom);
              }
              tester.view.viewPadding = const FakeViewPadding(
                top: 24,
                bottom: 48,
              );
              // IME consumes the bottom system-bar padding: no double spacing.
              for (final keyboard in <double>[100, 200, 300, 200, 100]) {
                tester.view.padding = const FakeViewPadding(top: 24);
                tester.view.viewInsets = FakeViewPadding(bottom: keyboard);
                await tester.pumpAndSettle();
                expectBottom(keyboard, keyboard: keyboard);
              }
              tester.view.padding = const FakeViewPadding(top: 24, bottom: 48);
              tester.view.viewInsets = const FakeViewPadding();
              await tester.pumpAndSettle();
              expectBottom(48);
              // Landscape system controls can also occupy the sides.
              tester.view.padding = const FakeViewPadding(
                top: 24,
                left: 12,
                right: 18,
                bottom: 48,
              );
              tester.view.viewPadding = const FakeViewPadding(
                top: 24,
                left: 12,
                right: 18,
                bottom: 48,
              );
              await tester.pumpAndSettle();
              expectBottom(48);
              expect(
                tester.getRect(page).left,
                closeTo((tablet ? 56 : 0) + 12, 0.1),
              );
              expect(tester.getRect(page).right, closeTo(width - 18, 0.1));
              await tester.pumpWidget(const SizedBox.shrink());
            },
          );
        }
      }
    }
  }
}

class _ComposerRoute extends OperitScreen {
  /// Creates a composer probe with either shared or page-owned system insets.
  const _ComposerRoute({
    required super.handlesBottomSafeArea,
    required this.agent,
    required this.nestedSafeArea,
    required this.controller,
    required this.focus,
    required this.viewModel,
  }) : super(routeTypeName: 'ComposerProbe', keepAlive: true);
  final bool agent;
  final bool nestedSafeArea;
  final TextEditingController controller;
  final FocusNode focus;
  final ChatViewModel viewModel;

  /// Builds a real composer with optional nested safe-area protection.
  @override
  Widget build(BuildContext context) {
    final composer = agent
        ? AgentChatInputSection(
            controller: controller,
            focusNode: focus,
            isLoading: false,
            inputState: InputProcessingState.idle(),
            viewModel: viewModel,
            currentChatId: 'empty-chat',

            onSendMessage: () {},
            onQueueMessage: () {},
            onCancelMessage: () {},
            isSpeechRecording: false,
            isSpeechTranscribing: false,
            onSpeechInput: () {},
          )
        : ClassicChatInputSection(
            controller: controller,
            focusNode: focus,
            isLoading: false,
            inputState: InputProcessingState.idle(),
            viewModel: viewModel,
            currentChatId: 'empty-chat',

            onSendMessage: () {},
            onQueueMessage: () {},
            onCancelMessage: () {},
            isSpeechRecording: false,
            isSpeechTranscribing: false,
            onSpeechInput: () {},
          );
    return Column(
      key: const ValueKey<String>('safe-area-page'),
      children: <Widget>[
        const Expanded(
          child: SizedBox.expand(
            key: ValueKey<String>('safe-area-message-content'),
          ),
        ),
        // Some pages already avoid system bars: do not reserve space twice.
        if (nestedSafeArea) SafeArea(top: false, child: composer) else composer,
      ],
    );
  }
}

/// Leaves metadata pending instead of contacting a real runtime.
class _PendingModelBridge extends OperitRuntimeBridge {
  final _binding = Completer<Uint8List>();

  /// Keeps model metadata pending while layout tests exercise the composer.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) {
    if (request.methodName == 'getModelBindingForFunction') {
      return _binding.future;
    }
    throw StateError('Unexpected call: ${request.methodName}');
  }

  /// Rejects push requests that are unrelated to the composer layout.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  /// Rejects snapshots that are unrelated to the composer layout.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  /// Supplies no events while model metadata remains pending.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      const Stream<CoreEvent>.empty();
}
