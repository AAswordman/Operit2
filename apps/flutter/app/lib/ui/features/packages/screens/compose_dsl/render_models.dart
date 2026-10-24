// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Exposes the structured action result contract for return-type regression tests.
@visibleForTesting
({Object? actionResult, Object? renderedActionResult, bool hasRenderResult})
parseComposeDslActionEventForTest(core_proxy.ToolPkgComposeDslEvent event) {
  if (event.error != null) throw StateError(event.error!);
  final update = event.update;
  if (update != null) {
    final store = _ComposeProgramStore()..apply(update);
    store.dispose();
  }
  return (
    actionResult: event.actionResult,
    renderedActionResult: update == null ? null : event.actionResult,
    hasRenderResult: update != null,
  );
}

class _ComposeDslRenderResult {
  /// Stores the root handle and the latest structured runtime metadata.
  const _ComposeDslRenderResult({
    required this.tree,
    required this.actionResult,
  });

  final _ComposeProgramNode tree;
  final Object? actionResult;
}

/// Decodes one queued Compose navigation request.
({String routeId, Map<String, Object?> args}) _composeNavigateCommand(
  Object? raw,
) {
  if (raw is! Map) {
    throw const FormatException('compose navigation command must be an object');
  }
  final map = _stringMap(raw);
  final routeId = _string(map['route']).trim();
  if (routeId.isEmpty) {
    throw StateError('compose navigate requires a route');
  }
  final argsRaw = map['args'];
  final args = argsRaw is Map ? _stringMap(argsRaw) : <String, Object?>{};
  return (routeId: routeId, args: args);
}

/// Canonical node names indexed by the Kotlin-compatible normalized token.
const _composeNodeTypes = <String, String>{
  'gradientrule': 'GradientRule',
  'loadingindicator': 'LoadingIndicator',
  'activitydots': 'ActivityDots',
  'draggable': 'Draggable',
  'dragtarget': 'DragTarget',
  'swipeactions': 'SwipeActions',
  'hoverregion': 'HoverRegion',
  'hoveronly': 'HoverOnly',
  'popupmenu': 'PopupMenu',
  'horizontalpager': 'HorizontalPager',
  'inputdecorator': 'InputDecorator',
  'outlinedtextfield': 'OutlinedTextField',
  'asyncimage': 'AsyncImage',
  'navigationbaritem': 'NavigationBarItem',
  'adaptivesidepanel': 'AdaptiveSidePanel',
  'aichat': 'AiChat',
  'alertdialog': 'AlertDialog',
  'assistchip': 'AssistChip',
  'badge': 'Badge',
  'badgedbox': 'BadgedBox',
  'basictext': 'BasicText',
  'box': 'Box',
  'boxwithconstraints': 'BoxWithConstraints',
  'button': 'Button',
  'canvas': 'Canvas',
  'card': 'Card',
  'checkbox': 'Checkbox',
  'circularprogressindicator': 'CircularProgressIndicator',
  'column': 'Column',
  'dialog': 'Dialog',
  'disableselection': 'DisableSelection',
  'dismissibledrawersheet': 'DismissibleDrawerSheet',
  'dismissiblenavigationdrawer': 'DismissibleNavigationDrawer',
  'divider': 'Divider',
  'dropdownmenu': 'DropdownMenu',
  'elevatedassistchip': 'ElevatedAssistChip',
  'elevatedbutton': 'ElevatedButton',
  'elevatedcard': 'ElevatedCard',
  'elevatedfilterchip': 'ElevatedFilterChip',
  'elevatedsuggestionchip': 'ElevatedSuggestionChip',
  'extendedfloatingactionbutton': 'ExtendedFloatingActionButton',
  'fillediconbutton': 'FilledIconButton',
  'filledicontogglebutton': 'FilledIconToggleButton',
  'filledtonalbutton': 'FilledTonalButton',
  'filledtonaliconbutton': 'FilledTonalIconButton',
  'filledtonalicontogglebutton': 'FilledTonalIconToggleButton',
  'filterchip': 'FilterChip',
  'floatingactionbutton': 'FloatingActionButton',
  'flowrow': 'FlowRow',
  'horizontaldivider': 'HorizontalDivider',
  'icon': 'Icon',
  'iconbutton': 'IconButton',
  'icontogglebutton': 'IconToggleButton',
  'image': 'Image',
  'inputchip': 'InputChip',
  'largefloatingactionbutton': 'LargeFloatingActionButton',
  'lazycolumn': 'LazyColumn',
  'lazyrow': 'LazyRow',
  'leadingicontab': 'LeadingIconTab',
  'linearprogressindicator': 'LinearProgressIndicator',
  'listitem': 'ListItem',
  'markdown': 'Markdown',
  'materialtheme': 'MaterialTheme',
  'modaldrawersheet': 'ModalDrawerSheet',
  'modalnavigationdrawer': 'ModalNavigationDrawer',
  'modalwidenavigationrail': 'ModalWideNavigationRail',
  'navigationbar': 'NavigationBar',
  'navigationdraweritem': 'NavigationDrawerItem',
  'navigationrail': 'NavigationRail',
  'navigationrailitem': 'NavigationRailItem',
  'outlinedbutton': 'OutlinedButton',
  'outlinedcard': 'OutlinedCard',
  'outlinediconbutton': 'OutlinedIconButton',
  'outlinedicontogglebutton': 'OutlinedIconToggleButton',
  'permanentdrawersheet': 'PermanentDrawerSheet',
  'permanentnavigationdrawer': 'PermanentNavigationDrawer',
  'primaryscrollabletabrow': 'PrimaryScrollableTabRow',
  'primarytabrow': 'PrimaryTabRow',
  'providetextstyle': 'ProvideTextStyle',
  'pulltorefreshbox': 'PullToRefreshBox',
  'radiobutton': 'RadioButton',
  'row': 'Row',
  'scaffold': 'Scaffold',
  'secondaryscrollabletabrow': 'SecondaryScrollableTabRow',
  'secondarytabrow': 'SecondaryTabRow',
  'selectioncontainer': 'SelectionContainer',
  'shortnavigationbar': 'ShortNavigationBar',
  'shortnavigationbaritem': 'ShortNavigationBarItem',
  'smallfloatingactionbutton': 'SmallFloatingActionButton',
  'snackbar': 'Snackbar',
  'snackbarhost': 'SnackbarHost',
  'spacer': 'Spacer',
  'suggestionchip': 'SuggestionChip',
  'surface': 'Surface',
  'switch': 'Switch',
  'tab': 'Tab',
  'text': 'Text',
  'textbutton': 'TextButton',
  'textfield': 'TextField',
  'timepickerdialog': 'TimePickerDialog',
  'verticaldivider': 'VerticalDivider',
  'verticaldraghandle': 'VerticalDragHandle',
  'webview': 'WebView',
  'widenavigationrail': 'WideNavigationRail',
  'widenavigationrailitem': 'WideNavigationRailItem',
};

class _NoUiView extends StatelessWidget {
  /// Creates the no ui view instance.
  const _NoUiView();

  /// Builds the widget for the current DSL state.
  @override
  Widget build(BuildContext context) {
    return const Center(child: Icon(Icons.extension_off_outlined, size: 42));
  }
}

/// Matches Kotlin Map/List value equality for JSON-backed Compose effect inputs.
bool _composeInputEquals(Object? left, Object? right) {
  if (identical(left, right)) return true;
  if (left is Map && right is Map) {
    if (left.length != right.length) return false;
    for (final entry in left.entries) {
      if (!right.containsKey(entry.key) ||
          !_composeInputEquals(entry.value, right[entry.key])) {
        return false;
      }
    }
    return true;
  }
  if (left is List && right is List) {
    if (left.length != right.length) return false;
    for (var index = 0; index < left.length; index++) {
      if (!_composeInputEquals(left[index], right[index])) return false;
    }
    return true;
  }
  return left == right;
}

/// Correlates a command's completion and side effects without one subscription per action.
class _ComposeDslPendingCommand {
  /// Records unfinished action ownership across host input updates.
  _ComposeDslPendingCommand(this.generation);
  final int generation;
  final Completer<void> completion = Completer<void>();
  Object? actionResult;
  final List<({String routeId, Map<String, Object?> args})> navigationCommands = [];
}
