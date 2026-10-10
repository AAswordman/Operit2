// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Exposes the structured action result contract for return-type regression tests.
@visibleForTesting
({Object? actionResult, Object? renderedActionResult, bool hasRenderResult})
parseComposeDslActionEventForTest(Map<String, Object?> event) {
  final phase = event['phase'];
  if (event['result'] is! Map) throw const FormatException('Compose event result must be structured');
  final response = _stringMap(event['result']);
  final result = _ComposeDslRenderResult.tryParse(response);
  return (
    actionResult: response['actionResult'],
    renderedActionResult: result?.actionResult,
    hasRenderResult: (phase == 'intermediate' || phase == 'final') && result != null,
  );
}

class _ComposeDslRenderResult {
  /// Stores the root handle and the latest structured runtime metadata.
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

  /// Reads an already structured snapshot supplied by CoreLink.
  static _ComposeDslRenderResult parse(Object? value) {
    final result = tryParse(value);
    if (result == null) throw const FormatException('compose_dsl snapshot requires a tree');
    return result;
  }

  /// Reads a headless snapshot without accepting or decoding JSON strings.
  static _ComposeDslRenderResult? tryParse(Object? value) {
    if (value is! Map) throw const FormatException('compose_dsl result must be structured');
    if (value['success'] == false) throw StateError(value['message'].toString());
    final tree = _ComposeDslNode.parse(value['tree']);
    if (tree == null) return null;
    return _ComposeDslRenderResult(
      tree: tree,
      state: _stringMap(value['state']),
      memo: _stringMap(value['memo']),
      actionResult: value['actionResult'],
    );
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

class _ComposeDslNode extends ChangeNotifier {
  /// Creates the compose dsl node instance.
  _ComposeDslNode({
    required String type,
    required Map<String, Object?> props,
    required List<_ComposeDslNode> children,
    required Map<String, List<_ComposeDslNode>> slots,
    this.id,
  }) : _type = type, _props = props, _children = children, _slots = slots;

  final String? id;
  String _type;
  Map<String, Object?> _props;
  List<_ComposeDslNode> _children;
  Map<String, List<_ComposeDslNode>> _slots;

  /// Records a renderer's dependency before returning this node's type.
  String get type { _ComposeDslReadScope.read(this); return _type; }

  /// Records a renderer's dependency before returning this node's properties.
  Map<String, Object?> get props { _ComposeDslReadScope.read(this); return _props; }

  /// Returns retained child handles without reconstructing a recursive tree.
  List<_ComposeDslNode> get children { _ComposeDslReadScope.read(this); return _children; }

  /// Returns retained slot handles without reconstructing their descendants.
  Map<String, List<_ComposeDslNode>> get slots { _ComposeDslReadScope.read(this); return _slots; }

  /// Atomically replaces one flat node record after all new handles have been allocated.
  void replace(core_proxy.ToolPkgComposeDslNodeRecord record, _ComposeDslNodeStore store) {
    final type = _composeNodeTypes[_normalizeToken(record.nodeType)];
    if (type == null) throw FormatException('Unknown Compose node type: ${record.nodeType}');
    _type = type;
    _props = record.props;
    _children = record.children.map(store.node).toList(growable: false);
    _slots = record.slots.map((name, ids) => MapEntry(name, ids.map(store.node).toList(growable: false)));
  }

  /// Invalidates only renderers that read this node during their latest build.
  void publish() { notifyListeners(); }

  /// Converts one typed headless snapshot without parsing a JSON UI tree.
  static _ComposeDslNode fromSnapshot(core_proxy.ToolPkgComposeDslNode node) {
    final type = _composeNodeTypes[_normalizeToken(node.type)];
    if (type == null) throw FormatException('Unknown Compose node type: ${node.type}');
    return _ComposeDslNode(
      type: type,
      props: node.props,
      children: node.children.map(fromSnapshot).toList(growable: false),
      slots: node.slots.map((name, children) => MapEntry(name, children.map(fromSnapshot).toList(growable: false))),
    );
  }

  /// Reads structured nodes supplied directly by isolated renderer tests.
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
