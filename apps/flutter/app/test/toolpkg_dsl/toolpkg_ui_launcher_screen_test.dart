import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('renders compose dsl tree from toolpkg route', (tester) async {
    final bridge = _ToolPkgDslTestBridge();
    await tester.pumpWidget(_screen(bridge));

    await tester.pumpAndSettle();

    expect(find.text('Demo ToolPkg'), findsOneWidget);
    expect(find.text('Main route'), findsOneWidget);
    expect(find.text('Counter: 0'), findsOneWidget);
    expect(find.text('Increment'), findsOneWidget);
    expect(find.byType(Card), findsOneWidget);
    expect(find.byType(CustomPaint), findsAtLeastNWidgets(1));

    final scriptCall = bridge.calls.singleWhere(
      (request) => request.methodName == 'getToolPkgComposeDslScript',
    );
    expect(scriptCall.targetPath.key, 'permissions.packTool.packageManager');
    expect(scriptCall.args, isA<Map<String, Object?>>());
    final args = scriptCall.args as Map<String, Object?>;
    expect(args['containerPackageName'], 'demo_toolpkg');
    expect(args['uiModuleId'], 'main');

    final renderCall = bridge.calls.singleWhere(
      (request) => request.methodName == 'executeComposeDslScript',
    );
    expect(renderCall.targetPath.segments.take(4).toList(), <String>[
      'permissions',
      'packTool',
      'packageManager',
      'getToolPkgExecutionEngine',
    ]);
    expect(
      renderCall.targetPath.segments[4],
      startsWith(
        'toolpkg_compose_dsl:demo_toolpkg:main:screen:demo_toolpkg:main:',
      ),
    );
  });

  testWidgets('opens compose dsl module when plugin has no routes', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge();
    await tester.pumpWidget(
      _screen(bridge, plugin: _moduleOnlyPluginRuntime()),
    );
    await tester.pumpAndSettle();

    expect(find.text('Module ToolPkg'), findsOneWidget);
    expect(find.text('Toolbox module'), findsOneWidget);

    final scriptCall = bridge.calls.singleWhere(
      (request) => request.methodName == 'getToolPkgComposeDslScript',
    );
    final scriptArgs = scriptCall.args as Map<String, Object?>;
    expect(scriptArgs['containerPackageName'], 'module_only_toolpkg');
    expect(scriptArgs['uiModuleId'], 'toolbox');

    final renderCall = bridge.calls.singleWhere(
      (request) => request.methodName == 'executeComposeDslScript',
    );
    final renderArgs = renderCall.args as Map<String, Object?>;
    final runtimeOptions = renderArgs['runtimeOptions'] as Map<String, Object?>;
    expect(
      runtimeOptions['routeInstanceId'],
      'legacy:module_only_toolpkg:toolbox',
    );
    expect(
      runtimeOptions['executionContextKey'],
      startsWith(
        'toolpkg_compose_dsl:module_only_toolpkg:toolbox:legacy:module_only_toolpkg:toolbox:',
      ),
    );
    final moduleSpec = runtimeOptions['moduleSpec'] as Map<String, Object?>;
    expect(moduleSpec['id'], 'toolbox');
    expect(moduleSpec['routeId'], 'toolbox');
    expect(moduleSpec['runtime'], 'compose_dsl');
    expect(moduleSpec['screen'], 'ui/toolbox.js');
  });

  testWidgets('dispatches compose dsl action and renders returned tree', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge();
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    await tester.tap(find.text('Increment'));
    await tester.pumpAndSettle();

    expect(find.text('Counter: 1'), findsOneWidget);

    final actionCall = bridge.calls.singleWhere(
      (request) => request.methodName == 'dispatchComposeDslActionAsync',
    );
    final args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'increment');
    expect(args['payload'], isNull);
    expect(actionCall.targetPath.segments.take(4).toList(), <String>[
      'permissions',
      'packTool',
      'packageManager',
      'getToolPkgExecutionEngine',
    ]);
    expect(
      actionCall.targetPath.segments[4],
      startsWith(
        'toolpkg_compose_dsl:demo_toolpkg:main:screen:demo_toolpkg:main:',
      ),
    );

    final runtimeOptions = args['runtimeOptions'] as Map<String, Object?>;
    expect(runtimeOptions['routeInstanceId'], 'screen:demo_toolpkg:main');
    expect(runtimeOptions['state'], <String, Object?>{'count': 0});
    expect(
      runtimeOptions['moduleSpec'],
      containsPair('toolPkgId', 'demo_toolpkg'),
    );
  });

  testWidgets('renders row fillMaxWidth child with finite text constraints', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: _rowFillMaxWidthRenderResult,
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(find.text('Wide row text'), findsOneWidget);
    expect(find.text('Tail'), findsOneWidget);
  });

  testWidgets('renders row surface text with finite constraints', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _rowSurfaceRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(tester.takeException(), isNull);
    expect(find.text('开始/暂停'), findsOneWidget);
  });

  testWidgets('renders text field slots and preserves focused input state', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _textFieldRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Alias'), findsOneWidget);
    expect(find.text('Type alias'), findsOneWidget);
    expect(find.text('https://'), findsOneWidget);
    expect(find.text('.site'), findsOneWidget);
    expect(find.text('Visible to the model'), findsOneWidget);

    await tester.tap(find.byType(TextField));
    await tester.enterText(find.byType(TextField), 'operit');
    await tester.pumpAndSettle();

    final actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'dispatchComposeDslActionAsync',
    );
    final args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'alias_change');
    expect(args['payload'], 'operit');

    final field = tester.widget<TextField>(find.byType(TextField));
    expect(field.controller?.text, 'operit');
    expect(field.minLines, 2);
    expect(field.maxLines, 4);
    expect(field.style?.fontWeight, FontWeight.w600);
  });

  testWidgets(
    'renders text field input options and action gated enabled state',
    (tester) async {
      final bridge = _ToolPkgDslTestBridge(
        renderResult: _textFieldInputOptionsRenderResult,
      );
      await tester.pumpWidget(_screen(bridge));
      await tester.pumpAndSettle();

      final fields = tester
          .widgetList<TextField>(find.byType(TextField))
          .toList();
      expect(fields, hasLength(2));
      expect(fields.first.enabled, isTrue);
      expect(fields.first.keyboardType, TextInputType.emailAddress);
      expect(fields.first.textInputAction, TextInputAction.search);
      expect(fields.last.enabled, isFalse);

      await tester.enterText(find.byType(TextField).first, 'agent@example.com');
      await tester.pumpAndSettle();

      final actionCall = bridge.calls.lastWhere(
        (request) => request.methodName == 'dispatchComposeDslActionAsync',
      );
      final args = actionCall.args as Map<String, Object?>;
      expect(args['actionId'], 'email_change');
      expect(args['payload'], 'agent@example.com');
    },
  );

  testWidgets('renders selection controls with actions and switch colors', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: _selectionControlsRenderResult,
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('S'), findsOneWidget);
    final switchWidget = tester.widget<Switch>(find.byType(Switch));
    expect(switchWidget.onChanged, isNotNull);
    expect(
      switchWidget.thumbColor?.resolve(<WidgetState>{WidgetState.selected}),
      const Color(0xff4caf50),
    );
    expect(
      switchWidget.trackColor?.resolve(<WidgetState>{}),
      const Color(0xffffcc80),
    );

    await tester.tap(find.byType(Switch));
    await tester.pumpAndSettle();
    var actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'dispatchComposeDslActionAsync',
    );
    var args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'switch_change');
    expect(args['payload'], false);

    await tester.tap(find.byType(Checkbox).first);
    await tester.pumpAndSettle();
    actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'dispatchComposeDslActionAsync',
    );
    args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'checkbox_change');
    expect(args['payload'], true);

    final disabledCheckbox = tester.widget<Checkbox>(
      find.byType(Checkbox).last,
    );
    expect(disabledCheckbox.onChanged, isNull);

    await tester.tap(find.byWidgetPredicate((widget) => widget is Radio<bool>));
    await tester.pumpAndSettle();
    actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'dispatchComposeDslActionAsync',
    );
    args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'radio_select');
    expect(args['payload'], isNull);
  });

  testWidgets('renders progress colors and icon image semantics', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _progressImageResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    final linear = tester.widget<LinearProgressIndicator>(
      find.byType(LinearProgressIndicator).first,
    );
    expect(linear.value, 1);
    expect(linear.color, const Color(0xff00695c));
    expect(linear.backgroundColor, const Color(0xffb2dfdb));

    final circular = tester.widget<CircularProgressIndicator>(
      find.byType(CircularProgressIndicator).first,
    );
    expect(circular.value, 0);
    expect(circular.strokeWidth, 6);
    expect(circular.color, const Color(0xff5e35b1));
    expect(circular.backgroundColor, const Color(0xffd1c4e9));

    final icon = tester.widget<Icon>(find.byIcon(Icons.settings));
    expect(icon.size, 32);
    expect(icon.color, const Color(0xffff00ff));
    expect(
      tester
          .widget<Opacity>(
            find
                .ancestor(
                  of: find.byIcon(Icons.settings),
                  matching: find.byType(Opacity),
                )
                .first,
          )
          .opacity,
      moreOrLessEquals(0.5),
    );
    expect(find.bySemanticsLabel('Settings image'), findsOneWidget);
  });

  testWidgets('renders badge list item and snackbar slots', (tester) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: _badgeListSnackbarRenderResult,
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('7'), findsOneWidget);
    expect(find.text('Inbox'), findsOneWidget);
    expect(find.text('3'), findsOneWidget);
    expect(find.text('OVERLINE'), findsOneWidget);
    expect(find.text('Headline'), findsOneWidget);
    expect(find.text('Supporting copy'), findsOneWidget);
    expect(find.text('Lead'), findsOneWidget);
    expect(find.text('Trail'), findsOneWidget);
    expect(find.text('Snack content'), findsOneWidget);
    expect(find.text('Undo'), findsOneWidget);
    expect(find.text('Dismiss'), findsOneWidget);

    final badges = tester.widgetList<Badge>(find.byType(Badge)).toList();
    expect(badges.first.backgroundColor, const Color(0xffd32f2f));
    expect(badges.first.textColor, const Color(0xffffffff));

    await tester.tap(find.text('Undo'));
    await tester.pumpAndSettle();
    var actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'dispatchComposeDslActionAsync',
    );
    var args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'snackbar_undo');
    expect(args['payload'], isNull);

    await tester.tap(find.text('Dismiss'));
    await tester.pumpAndSettle();
    actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'dispatchComposeDslActionAsync',
    );
    args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'snackbar_dismiss');
    expect(args['payload'], isNull);
  });

  testWidgets('renders expanded dropdown menu content and selection action', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _dropdownRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Mode'), findsOneWidget);
    expect(find.text('Option A'), findsOneWidget);
    expect(find.text('Option B'), findsOneWidget);

    await tester.tap(find.text('Option B'));
    await tester.pumpAndSettle();

    final actionCall = bridge.calls.singleWhere(
      (request) => request.methodName == 'dispatchComposeDslActionAsync',
    );
    final args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'select_mode');
    expect(args['payload'], 1);
  });

  testWidgets('renders pull to refresh indicator slot while refreshing', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _refreshRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Refresh content'), findsOneWidget);
    expect(find.text('Refreshing now'), findsOneWidget);
  });

  testWidgets('reports canvas size changes through action payload', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _canvasSizeRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    final paintSizes = find
        .byType(CustomPaint)
        .evaluate()
        .map(
          (element) => tester.getSize(
            find.byElementPredicate((candidate) => candidate == element),
          ),
        )
        .toList(growable: false);
    final canvasSize = paintSizes.singleWhere(
      (size) => size.height == 48 && size.width > 100,
    );

    final actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'dispatchComposeDslActionAsync',
    );
    final args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'canvas_size');

    final payload = args['payload'] as Map<String, Object?>;
    expect(payload['width'], canvasSize.width);
    expect(payload['height'], canvasSize.height);
  });

  testWidgets('renders provide text style as inherited text context', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: _provideTextStyleRenderResult,
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Styled slot'), findsOneWidget);
    expect(
      find.byWidgetPredicate(
        (widget) =>
            widget is DefaultTextStyle &&
            widget.style.fontSize == 18 &&
            widget.style.fontWeight == FontWeight.w700,
      ),
      findsOneWidget,
    );
  });

  testWidgets('renders time picker dialog slots and dismiss request', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: _timePickerDialogRenderResult,
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Pick time'), findsOneWidget);
    expect(find.text('Clock content'), findsOneWidget);
    expect(find.text('Keyboard'), findsOneWidget);
    expect(find.text('Cancel'), findsOneWidget);
    expect(find.text('OK'), findsOneWidget);

    await tester.sendKeyEvent(LogicalKeyboardKey.escape);
    await tester.pumpAndSettle();

    final actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'dispatchComposeDslActionAsync',
    );
    final args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'dismiss_time_picker');
    expect(args['payload'], isNull);
  });

  testWidgets('renders scaffold slots and content color context', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _scaffoldRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Top tools'), findsOneWidget);
    expect(find.text('Scaffold body'), findsOneWidget);
    expect(find.text('Bottom tools'), findsOneWidget);
    expect(find.text('Snack zone'), findsOneWidget);
    expect(find.text('Add'), findsOneWidget);

    final bodyElement = tester.element(find.text('Scaffold body'));
    expect(
      DefaultTextStyle.of(bodyElement).style.color,
      const Color(0xffe91e63),
    );
  });

  testWidgets('renders card and surface colors borders and content slots', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: _cardSurfaceRenderResult,
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    final card = tester.widget<Card>(find.byType(Card).first);
    expect(card.color?.a, moreOrLessEquals(0.5));
    expect(card.color?.r, moreOrLessEquals(0x10 / 255));
    expect(card.color?.g, moreOrLessEquals(0x20 / 255));
    expect(card.color?.b, moreOrLessEquals(0x30 / 255));
    final cardShape = card.shape as RoundedRectangleBorder;
    expect(cardShape.side.width, 2);
    expect(cardShape.side.color.a, moreOrLessEquals(0.25));
    expect(cardShape.side.color.r, moreOrLessEquals(1));
    expect(find.text('Card content'), findsOneWidget);
    final cardTextStyle = tester.widget<DefaultTextStyle>(
      find
          .ancestor(
            of: find.text('Card content'),
            matching: find.byType(DefaultTextStyle),
          )
          .first,
    );
    expect(cardTextStyle.style.color, const Color(0xffffffff));

    final surfaceMaterial = tester.widget<Material>(
      find
          .ancestor(
            of: find.text('Surface content'),
            matching: find.byType(Material),
          )
          .first,
    );
    expect(surfaceMaterial.color?.a, moreOrLessEquals(0.75));
    expect(surfaceMaterial.color?.r, moreOrLessEquals(0xf5 / 255));
    expect(surfaceMaterial.elevation, 7);
    final surfaceTextStyle = tester.widget<DefaultTextStyle>(
      find
          .ancestor(
            of: find.text('Surface content'),
            matching: find.byType(DefaultTextStyle),
          )
          .first,
    );
    expect(surfaceTextStyle.style.color, const Color(0xff00695c));
    expect(
      find.ancestor(
        of: find.text('Surface content'),
        matching: find.byType(Opacity),
      ),
      findsNothing,
    );

    await tester.tap(find.text('Surface content'));
    await tester.pumpAndSettle();
    final actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'dispatchComposeDslActionAsync',
    );
    final args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'surface_click');
  });

  testWidgets('renders button colors slots disabled state and border', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: _buttonStyleRenderResult,
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    final filled = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, 'Slot Button'),
    );
    expect(
      filled.style?.backgroundColor?.resolve(<WidgetState>{}),
      const Color(0xff00695c),
    );
    expect(
      filled.style?.foregroundColor?.resolve(<WidgetState>{}),
      const Color(0xffffffff),
    );

    final disabled = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, 'Disabled Button'),
    );
    expect(disabled.onPressed, isNull);
    expect(
      disabled.style?.backgroundColor?.resolve(<WidgetState>{
        WidgetState.disabled,
      }),
      const Color(0xff78909c),
    );
    expect(
      disabled.style?.foregroundColor?.resolve(<WidgetState>{
        WidgetState.disabled,
      }),
      const Color(0xff263238),
    );

    final outlined = tester.widget<OutlinedButton>(
      find.widgetWithText(OutlinedButton, 'Outline'),
    );
    final side = outlined.style?.side?.resolve(<WidgetState>{});
    expect(side?.width, 3);
    expect(side?.color, const Color(0xffe91e63));

    await tester.tap(find.text('Slot Button'));
    await tester.pumpAndSettle();
    final actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'dispatchComposeDslActionAsync',
    );
    final args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'slot_button');
  });

  testWidgets('renders floating action button size variants and colors', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(
      renderResult: _fabVariantsRenderResult,
    );
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    final fabFinder = find.byType(FloatingActionButton);
    expect(fabFinder, findsNWidgets(3));

    expect(tester.getSize(fabFinder.at(0)), const Size(48, 48));
    expect(tester.getSize(fabFinder.at(1)), const Size(56, 56));
    expect(tester.getSize(fabFinder.at(2)), const Size(96, 96));

    final smallWidget = tester.widget<FloatingActionButton>(fabFinder.at(0));
    expect(smallWidget.backgroundColor, const Color(0xff4caf50));
    expect(smallWidget.foregroundColor, const Color(0xffffffff));
    expect(smallWidget.shape, isA<RoundedRectangleBorder>());
  });

  testWidgets('renders icon toggle selected state and checked change action', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _iconToggleRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Selected toggle'), findsOneWidget);
    expect(find.text('Unchecked toggle'), findsNothing);

    final selectedIconButtonFinder = find.byWidgetPredicate(
      (widget) => widget is IconButton && widget.isSelected == true,
    );
    expect(selectedIconButtonFinder, findsOneWidget);

    final selectedIconButton = tester.widget<IconButton>(
      selectedIconButtonFinder,
    );
    expect(selectedIconButton.selectedIcon, isNotNull);
    expect(
      selectedIconButton.style?.shape?.resolve(<WidgetState>{}),
      isA<RoundedRectangleBorder>(),
    );

    await tester.tap(find.text('Selected toggle'));
    await tester.pumpAndSettle();

    final actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'dispatchComposeDslActionAsync',
    );
    final args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'toggle_icon');
    expect(args['payload'], false);
  });

  testWidgets('renders navigation item slots and dispatches item actions', (
    tester,
  ) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _navigationRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Bar selected icon'), findsOneWidget);
    expect(find.text('Bar inactive icon'), findsOneWidget);
    expect(find.text('Bar badge'), findsOneWidget);
    expect(find.text('Rail header'), findsOneWidget);
    expect(find.text('Rail selected icon'), findsOneWidget);
    expect(find.text('Drawer icon'), findsOneWidget);
    expect(find.text('Drawer badge'), findsOneWidget);

    await tester.tap(find.text('Bar inactive'));
    await tester.pumpAndSettle();

    final barActionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'dispatchComposeDslActionAsync',
    );
    final barArgs = barActionCall.args as Map<String, Object?>;
    expect(barArgs['actionId'], 'bar_inactive');
    expect(barArgs['payload'], isNull);

    await tester.tap(find.text('Drawer item'));
    await tester.pumpAndSettle();

    final drawerActionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'dispatchComposeDslActionAsync',
    );
    final drawerArgs = drawerActionCall.args as Map<String, Object?>;
    expect(drawerArgs['actionId'], 'drawer_item');
    expect(drawerArgs['payload'], isNull);
  });

  testWidgets('renders tab row slots and tab action semantics', (tester) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _tabsRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Overview tab'), findsOneWidget);
    expect(find.text('Details tab'), findsOneWidget);
    expect(find.text('Leading icon'), findsOneWidget);
    expect(find.text('Leading text'), findsOneWidget);
    expect(find.text('Tab indicator'), findsOneWidget);
    expect(find.text('Tab divider'), findsOneWidget);

    final selectedTextElement = tester.element(find.text('Details tab'));
    expect(
      DefaultTextStyle.of(selectedTextElement).style.color,
      const Color(0xffe91e63),
    );

    await tester.tap(find.text('Details tab'));
    await tester.pumpAndSettle();

    final actionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'dispatchComposeDslActionAsync',
    );
    final args = actionCall.args as Map<String, Object?>;
    expect(args['actionId'], 'details_tab');
    expect(args['payload'], isNull);
  });

  testWidgets('renders chip family slots and actions', (tester) async {
    final bridge = _ToolPkgDslTestBridge(renderResult: _chipsRenderResult);
    await tester.pumpWidget(_screen(bridge));
    await tester.pumpAndSettle();

    expect(find.text('Assist leading'), findsOneWidget);
    expect(find.text('Assist label'), findsOneWidget);
    expect(find.text('Assist trailing'), findsOneWidget);
    expect(find.text('Filter leading'), findsOneWidget);
    expect(find.text('Filter label'), findsOneWidget);
    expect(find.text('Input avatar'), findsOneWidget);
    expect(find.text('Input label'), findsOneWidget);
    expect(find.text('Input dismiss'), findsOneWidget);
    expect(find.text('Suggestion icon'), findsOneWidget);
    expect(find.text('Suggestion label'), findsOneWidget);

    final assistChip = tester.widget<ActionChip>(find.byType(ActionChip).first);
    expect(assistChip.backgroundColor, const Color(0xffe3f2fd));
    expect(assistChip.labelStyle?.color, const Color(0xff0d47a1));
    expect(assistChip.shape, isA<RoundedRectangleBorder>());

    final filterChip = tester.widget<FilterChip>(find.byType(FilterChip));
    expect(filterChip.selected, isTrue);
    expect(filterChip.selectedColor, const Color(0xfffff59d));

    await tester.tap(find.byType(FilterChip));
    await tester.pumpAndSettle();

    final filterActionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'dispatchComposeDslActionAsync',
    );
    final filterArgs = filterActionCall.args as Map<String, Object?>;
    expect(filterArgs['actionId'], 'filter_chip');
    expect(filterArgs['payload'], isNull);

    await tester.tap(
      find.byWidgetPredicate(
        (widget) =>
            widget is Icon && widget.icon == Icons.close && widget.size == 18,
      ),
    );
    await tester.pumpAndSettle();

    final dismissActionCall = bridge.calls.lastWhere(
      (request) => request.methodName == 'dispatchComposeDslActionAsync',
    );
    final dismissArgs = dismissActionCall.args as Map<String, Object?>;
    expect(dismissArgs['actionId'], 'dismiss_input_chip');
    expect(dismissArgs['payload'], isNull);
  });
}

Widget _screen(
  _ToolPkgDslTestBridge bridge, {
  core_proxy.ToolPkgContainerRuntime? plugin,
}) {
  return MaterialApp(
    home: ToolPkgUiLauncherScreen(
      clients: GeneratedCoreProxyClients(bridge),
      plugin: plugin ?? _pluginRuntime(),
    ),
  );
}

core_proxy.ToolPkgContainerRuntime _pluginRuntime() {
  return const core_proxy.ToolPkgContainerRuntime(
    packageName: 'demo_toolpkg',
    displayName: core_proxy.LocalizedText(
      values: <String, String>{'default': 'Demo ToolPkg'},
    ),
    description: core_proxy.LocalizedText(
      values: <String, String>{'default': 'DSL test package'},
    ),
    version: '1.0.0',
    author: <String>['Operit'],
    mainEntry: 'dist/main.js',
    sourceType: core_proxy.ToolPkgSourceType.externalValue,
    sourcePath: 'test',
    subpackages: <core_proxy.ToolPkgSubpackageRuntime>[],
    resources: <core_proxy.ToolPkgResourceRuntime>[],
    wasmModules: <core_proxy.ToolPkgWasmModuleRuntime>[],
    workflowTemplates: <core_proxy.ToolPkgWorkflowTemplateRuntime>[],
    workspaceTemplates: <core_proxy.ToolPkgWorkspaceTemplateRuntime>[],
    uiModules: <core_proxy.ToolPkgUiModuleRuntime>[
      core_proxy.ToolPkgUiModuleRuntime(
        id: 'main',
        runtime: 'compose_dsl',
        screen: 'ui/main.js',
        title: core_proxy.LocalizedText(
          values: <String, String>{'default': 'Main route'},
        ),
        keepAlive: true,
      ),
    ],
    uiRoutes: <core_proxy.ToolPkgUiRouteRuntime>[
      core_proxy.ToolPkgUiRouteRuntime(
        id: 'main',
        routeId: 'main',
        runtime: 'compose_dsl',
        screen: 'ui/main.js',
        title: core_proxy.LocalizedText(
          values: <String, String>{'default': 'Main route'},
        ),
        keepAlive: true,
      ),
    ],
    navigationEntries: <core_proxy.ToolPkgNavigationEntryRuntime>[],
    desktopWidgets: <core_proxy.ToolPkgDesktopWidgetRuntime>[],
    appLifecycleHooks: <core_proxy.ToolPkgAppLifecycleHookRuntime>[],
    messageProcessingPlugins: <core_proxy.ToolPkgFunctionHookRuntime>[],
    xmlRenderPlugins: <core_proxy.ToolPkgTagFunctionHookRuntime>[],
    inputMenuTogglePlugins: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatInputHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatViewHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    hostEventHooks: <core_proxy.ToolPkgHostEventHookRuntime>[],
    toolLifecycleHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptInputHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptHistoryHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptEstimateHistoryHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    systemPromptComposeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    toolPromptComposeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptFinalizeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptEstimateFinalizeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    summaryGenerateHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    aiProviders: <core_proxy.ToolPkgAiProviderRuntime>[],
  );
}

core_proxy.ToolPkgContainerRuntime _moduleOnlyPluginRuntime() {
  return const core_proxy.ToolPkgContainerRuntime(
    packageName: 'module_only_toolpkg',
    displayName: core_proxy.LocalizedText(
      values: <String, String>{'default': 'Module ToolPkg'},
    ),
    description: core_proxy.LocalizedText(
      values: <String, String>{'default': 'Module only package'},
    ),
    version: '1.0.0',
    author: <String>['Operit'],
    mainEntry: 'dist/main.js',
    sourceType: core_proxy.ToolPkgSourceType.externalValue,
    sourcePath: 'test',
    subpackages: <core_proxy.ToolPkgSubpackageRuntime>[],
    resources: <core_proxy.ToolPkgResourceRuntime>[],
    wasmModules: <core_proxy.ToolPkgWasmModuleRuntime>[],
    workflowTemplates: <core_proxy.ToolPkgWorkflowTemplateRuntime>[],
    workspaceTemplates: <core_proxy.ToolPkgWorkspaceTemplateRuntime>[],
    uiModules: <core_proxy.ToolPkgUiModuleRuntime>[
      core_proxy.ToolPkgUiModuleRuntime(
        id: 'toolbox',
        runtime: 'compose_dsl',
        screen: 'ui/toolbox.js',
        title: core_proxy.LocalizedText(
          values: <String, String>{'default': 'Toolbox module'},
        ),
        keepAlive: true,
      ),
    ],
    uiRoutes: <core_proxy.ToolPkgUiRouteRuntime>[],
    navigationEntries: <core_proxy.ToolPkgNavigationEntryRuntime>[],
    desktopWidgets: <core_proxy.ToolPkgDesktopWidgetRuntime>[],
    appLifecycleHooks: <core_proxy.ToolPkgAppLifecycleHookRuntime>[],
    messageProcessingPlugins: <core_proxy.ToolPkgFunctionHookRuntime>[],
    xmlRenderPlugins: <core_proxy.ToolPkgTagFunctionHookRuntime>[],
    inputMenuTogglePlugins: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatInputHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    chatViewHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    hostEventHooks: <core_proxy.ToolPkgHostEventHookRuntime>[],
    toolLifecycleHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptInputHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptHistoryHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptEstimateHistoryHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    systemPromptComposeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    toolPromptComposeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptFinalizeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    promptEstimateFinalizeHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    summaryGenerateHooks: <core_proxy.ToolPkgFunctionHookRuntime>[],
    aiProviders: <core_proxy.ToolPkgAiProviderRuntime>[],
  );
}

class _ToolPkgDslTestBridge extends OperitRuntimeBridge {
  _ToolPkgDslTestBridge({String Function(int count)? renderResult})
    : _renderResult = renderResult ?? _counterRenderResult;

  final List<CoreCallRequest> calls = <CoreCallRequest>[];
  final String Function(int count) _renderResult;
  var _count = 0;

  @override
  Future<Object?> call(CoreCallRequest request) async {
    calls.add(request);
    switch (request.methodName) {
      case 'getToolPkgComposeDslScript':
        return 'export default function render() {}';
      case 'getToolPkgComposeDslScreenPath':
        final args = request.args as Map<String, Object?>;
        return args['uiModuleId'] == 'toolbox' ? 'ui/toolbox.js' : 'ui/main.js';
      case 'executeComposeDslScript':
        _count = 0;
        return _renderResult(_count);
    }
    throw StateError('unexpected core call: ${request.methodName}');
  }

  /// Rejects push streams because this test bridge only models DSL calls and watches.
  @override
  Future<CorePushSink> push(CorePushRequest request) async {
    throw StateError('unexpected core push: ${request.methodName}');
  }

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) async {
    return CoreEvent(
      requestId: request.requestId,
      targetPath: request.targetPath,
      propertyName: request.propertyName,
      kind: 'Snapshot',
      value: null,
    );
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    if (request.propertyName != 'dispatchComposeDslActionAsync') {
      throw StateError('unexpected core watch: ${request.propertyName}');
    }
    final args = request.args as Map<String, Object?>;
    calls.add(
      CoreCallRequest(
        requestId: request.requestId,
        targetPath: request.targetPath,
        methodName: request.propertyName,
        args: request.args,
      ),
    );
    if (args['actionId'] == 'increment') {
      _count += 1;
    }
    return Stream<CoreEvent>.fromIterable(<CoreEvent>[
      CoreEvent(
        requestId: request.requestId,
        targetPath: request.targetPath,
        propertyName: request.propertyName,
        kind: 'Changed',
        value: jsonEncode(<String, Object?>{
          'phase': 'final',
          'result': _renderResult(_count),
        }),
      ),
      CoreEvent(
        requestId: request.requestId,
        targetPath: request.targetPath,
        propertyName: request.propertyName,
        kind: 'Completed',
        value: jsonEncode(<String, Object?>{'phase': 'complete'}),
      ),
    ]);
  }
}

String _counterRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Column',
      children: <Map<String, Object?>>[
        _node(
          'Card',
          children: <Map<String, Object?>>[
            _node('Text', props: <String, Object?>{'text': 'Counter: $count'}),
            _node(
              'Button',
              props: <String, Object?>{
                'text': 'Increment',
                'onClick': <String, Object?>{'__actionId': 'increment'},
              },
            ),
          ],
        ),
        _node(
          'Canvas',
          props: <String, Object?>{
            'width': 80,
            'height': 48,
            'commands': <Map<String, Object?>>[
              <String, Object?>{
                'type': 'drawRect',
                'x': 0,
                'y': 0,
                'width': 80,
                'height': 48,
                'color': '#2196f3',
              },
            ],
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _rowFillMaxWidthRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Row',
      children: <Map<String, Object?>>[
        _node(
          'Box',
          props: <String, Object?>{
            'fillMaxWidth': true,
            'padding': <String, Object?>{'horizontal': 12, 'vertical': 8},
          },
          children: <Map<String, Object?>>[
            _node('Text', props: <String, Object?>{'text': 'Wide row text'}),
          ],
        ),
        _node('Text', props: <String, Object?>{'text': 'Tail'}),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _rowSurfaceRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Row',
      children: <Map<String, Object?>>[
        _node(
          'Surface',
          props: <String, Object?>{
            'containerColor': '#5F6368EE',
            'contentColor': '#F7F7F7',
            'shape': <String, Object?>{'type': 'pill'},
            'padding': <String, Object?>{'horizontal': 22, 'vertical': 14},
          },
          children: <Map<String, Object?>>[
            _node(
              'Text',
              props: <String, Object?>{'text': '开始/暂停', 'style': 'labelLarge'},
            ),
          ],
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _textFieldRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'TextField',
      props: <String, Object?>{
        'key': 'alias-field',
        'value': 'initial',
        'minLines': 2,
        'maxLines': 4,
        'style': <String, Object?>{
          'fontSize': 18,
          'fontWeight': 'semibold',
          'color': '#1b5e20',
        },
        'onValueChange': <String, Object?>{'__actionId': 'alias_change'},
      },
      slots: <String, List<Map<String, Object?>>>{
        'label': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Alias'}),
        ],
        'placeholder': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Type alias'}),
        ],
        'prefix': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'https://'}),
        ],
        'suffix': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': '.site'}),
        ],
        'supportingText': <Map<String, Object?>>[
          _node(
            'Text',
            props: <String, Object?>{'text': 'Visible to the model'},
          ),
        ],
      },
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _textFieldInputOptionsRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Column',
      children: <Map<String, Object?>>[
        _node(
          'TextField',
          props: <String, Object?>{
            'value': 'agent',
            'keyboardType': 'email',
            'keyboardAction': 'search',
            'onValueChange': <String, Object?>{'__actionId': 'email_change'},
          },
        ),
        _node(
          'TextField',
          props: <String, Object?>{'value': 'locked', 'keyboardType': 'number'},
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _selectionControlsRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Row',
      children: <Map<String, Object?>>[
        _node(
          'Switch',
          props: <String, Object?>{
            'checked': true,
            'checkedThumbColor': '#4caf50',
            'checkedTrackColor': '#a5d6a7',
            'uncheckedThumbColor': '#ff9800',
            'uncheckedTrackColor': '#ffcc80',
            'onCheckedChange': <String, Object?>{'__actionId': 'switch_change'},
          },
          slots: <String, List<Map<String, Object?>>>{
            'thumbContent': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'S'}),
            ],
          },
        ),
        _node(
          'Checkbox',
          props: <String, Object?>{
            'checked': false,
            'onCheckedChange': <String, Object?>{
              '__actionId': 'checkbox_change',
            },
          },
        ),
        _node('Checkbox', props: <String, Object?>{'checked': true}),
        _node(
          'RadioButton',
          props: <String, Object?>{
            'selected': false,
            'onClick': <String, Object?>{'__actionId': 'radio_select'},
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _progressImageResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Column',
      children: <Map<String, Object?>>[
        _node(
          'LinearProgressIndicator',
          props: <String, Object?>{
            'progress': 1.25,
            'color': '#00695c',
            'trackColor': '#b2dfdb',
          },
        ),
        _node(
          'CircularProgressIndicator',
          props: <String, Object?>{
            'progress': -0.25,
            'strokeWidth': 6,
            'color': '#5e35b1',
            'trackColor': '#d1c4e9',
          },
        ),
        _node(
          'Image',
          props: <String, Object?>{
            'name': 'settings',
            'tint': '#ff00ff',
            'size': 32,
            'alpha': 0.5,
            'contentDescription': 'Settings image',
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _badgeListSnackbarRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Column',
      children: <Map<String, Object?>>[
        _node(
          'Badge',
          props: <String, Object?>{
            'containerColor': '#d32f2f',
            'contentColor': '#ffffff',
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': '7'}),
            ],
          },
        ),
        _node(
          'BadgedBox',
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Inbox'}),
            ],
            'badge': <Map<String, Object?>>[
              _node(
                'Badge',
                slots: <String, List<Map<String, Object?>>>{
                  'content': <Map<String, Object?>>[
                    _node('Text', props: <String, Object?>{'text': '3'}),
                  ],
                },
              ),
            ],
          },
        ),
        _node(
          'ListItem',
          props: <String, Object?>{'shadowElevation': 2},
          slots: <String, List<Map<String, Object?>>>{
            'overlineContent': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'OVERLINE'}),
            ],
            'headlineContent': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Headline'}),
            ],
            'supportingContent': <Map<String, Object?>>[
              _node(
                'Text',
                props: <String, Object?>{'text': 'Supporting copy'},
              ),
            ],
            'leadingContent': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Lead'}),
            ],
            'trailingContent': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Trail'}),
            ],
          },
        ),
        _node(
          'Snackbar',
          props: <String, Object?>{
            'actionOnNewLine': true,
            'shape': 8,
            'containerColor': '#263238',
            'contentColor': '#ffffff',
            'actionContentColor': '#80deea',
            'dismissActionContentColor': '#ffab91',
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Snack content'}),
            ],
            'action': <Map<String, Object?>>[
              _node(
                'TextButton',
                props: <String, Object?>{
                  'text': 'Undo',
                  'onClick': <String, Object?>{'__actionId': 'snackbar_undo'},
                },
              ),
            ],
            'dismissAction': <Map<String, Object?>>[
              _node(
                'TextButton',
                props: <String, Object?>{
                  'text': 'Dismiss',
                  'onClick': <String, Object?>{
                    '__actionId': 'snackbar_dismiss',
                  },
                },
              ),
            ],
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _dropdownRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'DropdownMenu',
      props: <String, Object?>{
        'expanded': true,
        'label': 'Mode',
        'text': 'Choose',
        'onClick': <String, Object?>{'__actionId': 'select_mode'},
      },
      slots: <String, List<Map<String, Object?>>>{
        'content': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Option A'}),
          _node('Text', props: <String, Object?>{'text': 'Option B'}),
        ],
      },
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _refreshRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'PullToRefreshBox',
      props: <String, Object?>{
        'isRefreshing': true,
        'contentAlignment': 'topCenter',
        'onRefresh': <String, Object?>{'__actionId': 'refresh'},
      },
      slots: <String, List<Map<String, Object?>>>{
        'content': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Refresh content'}),
        ],
        'indicator': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Refreshing now'}),
        ],
      },
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _canvasSizeRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Canvas',
      props: <String, Object?>{
        'width': 80,
        'height': 48,
        'onSizeChanged': <String, Object?>{'__actionId': 'canvas_size'},
        'commands': <Map<String, Object?>>[
          <String, Object?>{
            'type': 'drawLine',
            'x1': 0,
            'y1': 0,
            'x2': 80,
            'y2': 48,
            'color': '#f44336',
          },
        ],
      },
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _provideTextStyleRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'ProvideTextStyle',
      props: <String, Object?>{
        'style': 'bodyMedium',
        'fontSize': 18,
        'fontWeight': 'bold',
      },
      slots: <String, List<Map<String, Object?>>>{
        'content': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Styled slot'}),
        ],
      },
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _timePickerDialogRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'TimePickerDialog',
      props: <String, Object?>{
        'containerColor': '#ffffff',
        'shape': 12,
        'onDismissRequest': <String, Object?>{
          '__actionId': 'dismiss_time_picker',
        },
      },
      slots: <String, List<Map<String, Object?>>>{
        'title': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Pick time'}),
        ],
        'content': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Clock content'}),
        ],
        'modeToggleButton': <Map<String, Object?>>[
          _node(
            'TextButton',
            props: <String, Object?>{
              'text': 'Keyboard',
              'onClick': <String, Object?>{'__actionId': 'mode_toggle'},
            },
          ),
        ],
        'dismissButton': <Map<String, Object?>>[
          _node(
            'TextButton',
            props: <String, Object?>{
              'text': 'Cancel',
              'onClick': <String, Object?>{'__actionId': 'cancel_time'},
            },
          ),
        ],
        'confirmButton': <Map<String, Object?>>[
          _node(
            'Button',
            props: <String, Object?>{
              'text': 'OK',
              'onClick': <String, Object?>{'__actionId': 'confirm_time'},
            },
          ),
        ],
      },
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _scaffoldRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Scaffold',
      props: <String, Object?>{
        'containerColor': '#ffffff',
        'contentColor': '#e91e63',
      },
      slots: <String, List<Map<String, Object?>>>{
        'topBar': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Top tools'}),
        ],
        'content': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Scaffold body'}),
        ],
        'bottomBar': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Bottom tools'}),
        ],
        'snackbarHost': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Snack zone'}),
        ],
        'floatingActionButton': <Map<String, Object?>>[
          _node(
            'ExtendedFloatingActionButton',
            props: <String, Object?>{
              'onClick': <String, Object?>{'__actionId': 'fab_add'},
            },
            slots: <String, List<Map<String, Object?>>>{
              'content': <Map<String, Object?>>[
                _node('Text', props: <String, Object?>{'text': 'Add'}),
              ],
            },
          ),
        ],
      },
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _cardSurfaceRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Column',
      children: <Map<String, Object?>>[
        _node(
          'Card',
          props: <String, Object?>{
            'containerColor': '#102030',
            'containerAlpha': 0.5,
            'contentColor': '#ffffff',
            'contentPadding': <String, Object?>{'all': 8},
            'shape': 10,
            'border': <String, Object?>{
              'width': 2,
              'color': '#ff0000',
              'alpha': 0.25,
            },
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Card content'}),
            ],
          },
        ),
        _node(
          'Surface',
          props: <String, Object?>{
            'color': '#f5f5f5',
            'alpha': 0.75,
            'contentColor': '#00695c',
            'shadowElevation': 7,
            'shape': 6,
            'contentPadding': <String, Object?>{'all': 4},
            'onClick': <String, Object?>{'__actionId': 'surface_click'},
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node(
                'Text',
                props: <String, Object?>{'text': 'Surface content'},
              ),
            ],
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _buttonStyleRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Column',
      children: <Map<String, Object?>>[
        _node(
          'Button',
          props: <String, Object?>{
            'containerColor': '#00695c',
            'contentColor': '#ffffff',
            'disabledContainerColor': '#78909c',
            'disabledContentColor': '#263238',
            'contentPadding': <String, Object?>{
              'horizontal': 18,
              'vertical': 7,
            },
            'onClick': <String, Object?>{'__actionId': 'slot_button'},
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Slot Button'}),
            ],
          },
        ),
        _node(
          'Button',
          props: <String, Object?>{
            'text': 'Disabled Button',
            'enabled': false,
            'containerColor': '#00695c',
            'contentColor': '#ffffff',
            'disabledContainerColor': '#78909c',
            'disabledContentColor': '#263238',
          },
        ),
        _node(
          'OutlinedButton',
          props: <String, Object?>{
            'text': 'Outline',
            'contentColor': '#e91e63',
            'shape': 9,
            'border': <String, Object?>{'width': 3, 'color': '#e91e63'},
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _fabVariantsRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Row',
      children: <Map<String, Object?>>[
        _node(
          'SmallFloatingActionButton',
          props: <String, Object?>{
            'shape': 10,
            'containerColor': '#4caf50',
            'contentColor': '#ffffff',
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node('Icon', props: <String, Object?>{'name': 'add'}),
            ],
          },
        ),
        _node(
          'FloatingActionButton',
          props: <String, Object?>{
            'containerColor': '#2196f3',
            'contentColor': '#ffffff',
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node('Icon', props: <String, Object?>{'name': 'edit'}),
            ],
          },
        ),
        _node(
          'LargeFloatingActionButton',
          props: <String, Object?>{
            'shape': 14,
            'containerColor': '#ff9800',
            'contentColor': '#000000',
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node('Icon', props: <String, Object?>{'name': 'star'}),
            ],
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _iconToggleRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Row',
      children: <Map<String, Object?>>[
        _node(
          'FilledIconToggleButton',
          props: <String, Object?>{
            'checked': true,
            'shape': 18,
            'onCheckedChange': <String, Object?>{'__actionId': 'toggle_icon'},
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node(
                'Text',
                props: <String, Object?>{'text': 'Unchecked toggle'},
              ),
            ],
            'selectedIcon': <Map<String, Object?>>[
              _node(
                'Text',
                props: <String, Object?>{'text': 'Selected toggle'},
              ),
            ],
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _navigationRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Column',
      children: <Map<String, Object?>>[
        _node(
          'NavigationBar',
          props: <String, Object?>{
            'containerColor': '#fafafa',
            'tonalElevation': 3,
          },
          slots: <String, List<Map<String, Object?>>>{
            'content': <Map<String, Object?>>[
              _node(
                'NavigationBarItem',
                props: <String, Object?>{
                  'selected': true,
                  'onClick': <String, Object?>{'__actionId': 'bar_selected'},
                },
                slots: <String, List<Map<String, Object?>>>{
                  'icon': <Map<String, Object?>>[
                    _node(
                      'Text',
                      props: <String, Object?>{'text': 'Bar plain icon'},
                    ),
                  ],
                  'selectedIcon': <Map<String, Object?>>[
                    _node(
                      'Text',
                      props: <String, Object?>{'text': 'Bar selected icon'},
                    ),
                  ],
                  'label': <Map<String, Object?>>[
                    _node('Text', props: <String, Object?>{'text': 'Bar home'}),
                  ],
                  'badge': <Map<String, Object?>>[
                    _node(
                      'Text',
                      props: <String, Object?>{'text': 'Bar badge'},
                    ),
                  ],
                },
              ),
              _node(
                'NavigationBarItem',
                props: <String, Object?>{
                  'selected': false,
                  'onClick': <String, Object?>{'__actionId': 'bar_inactive'},
                },
                slots: <String, List<Map<String, Object?>>>{
                  'icon': <Map<String, Object?>>[
                    _node(
                      'Text',
                      props: <String, Object?>{'text': 'Bar inactive icon'},
                    ),
                  ],
                  'label': <Map<String, Object?>>[
                    _node(
                      'Text',
                      props: <String, Object?>{'text': 'Bar inactive'},
                    ),
                  ],
                },
              ),
            ],
          },
        ),
        _node(
          'NavigationRail',
          slots: <String, List<Map<String, Object?>>>{
            'header': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Rail header'}),
            ],
            'content': <Map<String, Object?>>[
              _node(
                'NavigationRailItem',
                props: <String, Object?>{
                  'selected': true,
                  'alwaysShowLabel': true,
                  'onClick': <String, Object?>{'__actionId': 'rail_selected'},
                },
                slots: <String, List<Map<String, Object?>>>{
                  'icon': <Map<String, Object?>>[
                    _node(
                      'Text',
                      props: <String, Object?>{'text': 'Rail plain icon'},
                    ),
                  ],
                  'selectedIcon': <Map<String, Object?>>[
                    _node(
                      'Text',
                      props: <String, Object?>{'text': 'Rail selected icon'},
                    ),
                  ],
                  'label': <Map<String, Object?>>[
                    _node(
                      'Text',
                      props: <String, Object?>{'text': 'Rail item'},
                    ),
                  ],
                },
              ),
            ],
          },
        ),
        _node(
          'NavigationDrawerItem',
          props: <String, Object?>{
            'selected': true,
            'shape': 12,
            'onClick': <String, Object?>{'__actionId': 'drawer_item'},
          },
          slots: <String, List<Map<String, Object?>>>{
            'icon': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Drawer icon'}),
            ],
            'label': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Drawer item'}),
            ],
            'badge': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Drawer badge'}),
            ],
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _tabsRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'PrimaryScrollableTabRow',
      props: <String, Object?>{
        'selectedTabIndex': 1,
        'containerColor': '#ffffff',
        'contentColor': '#111111',
        'edgePadding': 4,
      },
      slots: <String, List<Map<String, Object?>>>{
        'tabs': <Map<String, Object?>>[
          _node(
            'Tab',
            props: <String, Object?>{
              'selected': false,
              'unselectedContentColor': '#607d8b',
              'onClick': <String, Object?>{'__actionId': 'overview_tab'},
            },
            slots: <String, List<Map<String, Object?>>>{
              'content': <Map<String, Object?>>[
                _node('Text', props: <String, Object?>{'text': 'Overview tab'}),
              ],
            },
          ),
          _node(
            'Tab',
            props: <String, Object?>{
              'selected': true,
              'selectedContentColor': '#e91e63',
              'onClick': <String, Object?>{'__actionId': 'details_tab'},
            },
            slots: <String, List<Map<String, Object?>>>{
              'content': <Map<String, Object?>>[
                _node('Text', props: <String, Object?>{'text': 'Details tab'}),
              ],
            },
          ),
          _node(
            'LeadingIconTab',
            props: <String, Object?>{
              'selected': false,
              'enabled': false,
              'unselectedContentColor': '#9e9e9e',
              'onClick': <String, Object?>{'__actionId': 'leading_tab'},
            },
            slots: <String, List<Map<String, Object?>>>{
              'icon': <Map<String, Object?>>[
                _node('Text', props: <String, Object?>{'text': 'Leading icon'}),
              ],
              'text': <Map<String, Object?>>[
                _node('Text', props: <String, Object?>{'text': 'Leading text'}),
              ],
            },
          ),
        ],
        'indicator': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Tab indicator'}),
        ],
        'divider': <Map<String, Object?>>[
          _node('Text', props: <String, Object?>{'text': 'Tab divider'}),
        ],
      },
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

String _chipsRenderResult(int count) {
  return jsonEncode(<String, Object?>{
    'success': true,
    'tree': _node(
      'Column',
      children: <Map<String, Object?>>[
        _node(
          'AssistChip',
          props: <String, Object?>{
            'shape': 10,
            'containerColor': '#e3f2fd',
            'contentColor': '#0d47a1',
            'onClick': <String, Object?>{'__actionId': 'assist_chip'},
          },
          slots: <String, List<Map<String, Object?>>>{
            'leadingIcon': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Assist leading'}),
            ],
            'label': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Assist label'}),
            ],
            'trailingIcon': <Map<String, Object?>>[
              _node(
                'Text',
                props: <String, Object?>{'text': 'Assist trailing'},
              ),
            ],
          },
        ),
        _node(
          'FilterChip',
          props: <String, Object?>{
            'selected': true,
            'selectedContainerColor': '#fff59d',
            'onClick': <String, Object?>{'__actionId': 'filter_chip'},
          },
          slots: <String, List<Map<String, Object?>>>{
            'leadingIcon': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Filter leading'}),
            ],
            'label': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Filter label'}),
            ],
          },
        ),
        _node(
          'InputChip',
          props: <String, Object?>{
            'selected': true,
            'onDismiss': <String, Object?>{'__actionId': 'dismiss_input_chip'},
            'onClick': <String, Object?>{'__actionId': 'input_chip'},
          },
          slots: <String, List<Map<String, Object?>>>{
            'avatar': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Input avatar'}),
            ],
            'label': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Input label'}),
            ],
            'trailingIcon': <Map<String, Object?>>[
              _node('Text', props: <String, Object?>{'text': 'Input dismiss'}),
            ],
          },
        ),
        _node(
          'ElevatedSuggestionChip',
          props: <String, Object?>{
            'onClick': <String, Object?>{'__actionId': 'suggestion_chip'},
          },
          slots: <String, List<Map<String, Object?>>>{
            'icon': <Map<String, Object?>>[
              _node(
                'Text',
                props: <String, Object?>{'text': 'Suggestion icon'},
              ),
            ],
            'label': <Map<String, Object?>>[
              _node(
                'Text',
                props: <String, Object?>{'text': 'Suggestion label'},
              ),
            ],
          },
        ),
      ],
    ),
    'state': <String, Object?>{'count': count},
    'memo': <String, Object?>{'route': 'main'},
  });
}

Map<String, Object?> _node(
  String type, {
  Map<String, Object?> props = const <String, Object?>{},
  List<Map<String, Object?>> children = const <Map<String, Object?>>[],
  Map<String, List<Map<String, Object?>>> slots =
      const <String, List<Map<String, Object?>>>{},
}) {
  return <String, Object?>{
    'type': type,
    'props': props,
    'children': children,
    'slots': slots,
  };
}
