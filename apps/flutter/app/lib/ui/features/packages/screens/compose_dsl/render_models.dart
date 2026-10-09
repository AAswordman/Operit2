// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Exposes the production action-event parser for return-type regression tests.
@visibleForTesting
({Object? actionResult, Object? renderedActionResult, bool hasRenderResult})
parseComposeDslActionEventForTest(String event) {
  final parsed = _ParsedComposeDslActionEvent.parse(event);
  return (
    actionResult: parsed.actionResult,
    renderedActionResult: parsed.renderResult?.actionResult,
    hasRenderResult: parsed.renderResult != null,
  );
}

class _ParsedComposeDslActionEvent {
  /// Resolves parsed compose dsl action event for the Compose DSL renderer.
  const _ParsedComposeDslActionEvent({
    required this.phase,
    required this.renderResult,
    required this.actionResult,
    required this.errorText,
    this.navigationCommands = const [],
  });

  final String? phase;
  final _ComposeDslRenderResult? renderResult;
  final Object? actionResult;
  final String? errorText;
  final List<({String routeId, Map<String, Object?> args})> navigationCommands;

  /// Parses one serialized DSL value.
  static _ParsedComposeDslActionEvent parse(String event) {
    final decoded = jsonDecode(event) as Map<String, Object?>;
    final phase = decoded['phase']?.toString().trim();
    if (phase == 'intermediate' || phase == 'final') {
      final raw = decoded['result'];
      if (raw is! String) {
        throw StateError('compose_dsl action result event missing result');
      }
      final result = _ComposeDslRenderResult.tryParse(raw);
      return _ParsedComposeDslActionEvent(
        phase: phase,
        renderResult: result,
        actionResult: _ComposeDslRenderResult.actionResultOf(raw),
        errorText: null,
        navigationCommands: _ComposeDslRenderResult.navigationCommandsOf(raw),
      );
    }
    if (phase == 'error') {
      return _ParsedComposeDslActionEvent(
        phase: phase,
        renderResult: null,
        actionResult: null,
        errorText: decoded['error']?.toString(),
      );
    }
    return _ParsedComposeDslActionEvent(
      phase: phase,
      renderResult: null,
      actionResult: null,
      errorText: null,
    );
  }
}

class _ComposeDslRenderResult {
  /// Reads navigation side effects independently of the action return value.
  static List<({String routeId, Map<String, Object?> args})>
  navigationCommandsOf(String? raw) {
    final commands = _rootObject(raw)?['navigationCommands'];
    if (commands == null) return const [];
    return (commands as List).map(_composeNavigateCommand).toList();
  }

  /// Creates the compose dsl render result instance.
  const _ComposeDslRenderResult({
    required this.tree,
    required this.state,
    required this.memo,
    required this.actionResult,
  });

  final _ComposeDslNode tree;
  final Map<String, Object?> state;
  final Map<String, Object?> memo;
  final Object? actionResult;

  /// Parses one serialized DSL value.
  static _ComposeDslRenderResult parse(String? raw) {
    final result = tryParse(raw);
    if (result != null) {
      return result;
    }
    throw FormatException(
      'compose_dsl result is invalid: ${_rawResultSummary(raw)}',
    );
  }

  /// Parses a supported serialized DSL representation.
  static _ComposeDslRenderResult? tryParse(String? raw) {
    final value = _rootObject(raw);
    if (value == null) {
      return null;
    }
    final success = value['success'];
    if (success == false) {
      throw Exception((value['message'] ?? 'compose_dsl failed').toString());
    }
    final tree = _ComposeDslNode.parse(value['tree']);
    if (tree == null) {
      return null;
    }
    return _ComposeDslRenderResult(
      tree: tree,
      state: _stringMap(value['state']),
      memo: _stringMap(value['memo']),
      // The outer envelope is already decoded; preserve the callback's type.
      actionResult: value['actionResult'],
    );
  }

  /// Resolves action result of for the Compose DSL renderer.
  static Object? actionResultOf(String? raw) {
    final value = _rootObject(raw);
    if (value == null) {
      return null;
    }
    final success = value['success'];
    if (success == false) {
      throw Exception((value['message'] ?? 'compose_dsl failed').toString());
    }
    // JSON-looking strings are callback data, not another transport envelope.
    return value['actionResult'];
  }

  static Map<Object?, Object?>? _rootObject(String? raw) {
    Object? value = raw;
    for (var i = 0; i < 3; i += 1) {
      if (value is String) {
        final trimmed = value.trim();
        if (trimmed.isEmpty) {
          break;
        }
        value = jsonDecode(trimmed);
      }
    }
    if (value is Map) {
      return Map<Object?, Object?>.from(value);
    }
    return null;
  }

  /// Resolves raw result summary for the Compose DSL renderer.
  static String _rawResultSummary(Object? raw) {
    final text = raw?.toString().trim();
    if (text == null || text.isEmpty) {
      return '<empty>';
    }
    const maxLength = 1200;
    if (text.length <= maxLength) {
      return text;
    }
    return '${text.substring(0, maxLength)}...';
  }
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

class _ComposeDslNode {
  /// Creates the compose dsl node instance.
  const _ComposeDslNode({
    required this.type,
    required this.props,
    required this.children,
    required this.slots,
  });

  final String type;
  final Map<String, Object?> props;
  final List<_ComposeDslNode> children;
  final Map<String, List<_ComposeDslNode>> slots;

  /// Parses one serialized DSL value.
  static _ComposeDslNode? parse(Object? raw) {
    if (raw is! Map) {
      return null;
    }
    final token = _normalizeToken((raw['type'] ?? '').toString());
    if (token.isEmpty) {
      return null;
    }
    final type = _composeNodeTypes[token];
    if (type == null) {
      throw FormatException('Unknown Compose node type: ${raw['type']}');
    }
    return _ComposeDslNode(
      type: type,
      props: _stringMap(raw['props']),
      children: _nodeList(raw['children']),
      slots: _slotMap(raw['slots']),
    );
  }
}

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
