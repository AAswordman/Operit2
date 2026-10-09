// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Renders navigation nodes for Compose DSL nodes.
extension _ComposeNavigationNodes on _ComposeDslRendererState {
  /// Builds navigation bar for the Compose DSL renderer.
  Widget _navigationBar(BuildContext context) {
    final items = _navigationContentNodes()
        .where(
          (child) =>
              child.type == 'NavigationBarItem' ||
              child.type == 'ShortNavigationBarItem',
        )
        .toList(growable: false);
    final destinations = items
        .map(_navigationDestination)
        .toList(growable: false);
    final selectedIndex = _selectedIndex(items, node.props['selectedIndex']);
    return NavigationBar(
      selectedIndex: selectedIndex,
      backgroundColor: _color(context, node.props['containerColor']),
      elevation: _number(node.props['tonalElevation']),
      destinations: destinations.isEmpty
          ? const <Widget>[
              NavigationDestination(icon: Icon(Icons.widgets), label: ''),
            ]
          : destinations,
      onDestinationSelected: (index) {
        if (index >= 0 && index < items.length) {
          final item = items[index];
          if (item.props['enabled'] != false) {
            _invokeAction(item.props['onClick']);
          }
        }
      },
    );
  }

  NavigationDestination _navigationDestination(_ComposeDslNode item) {
    return NavigationDestination(
      icon: _navigationItemIcon(item, selected: false),
      selectedIcon: _hasSlotFrom(item, 'selectedIcon')
          ? _navigationItemIcon(item, selected: true)
          : null,
      label: _plainTextFrom(item, 'label') ?? _string(item.props['label']),
    );
  }

  /// Builds navigation rail for the Compose DSL renderer.
  Widget _navigationRail(BuildContext context) {
    final items = _navigationContentNodes()
        .where(
          (child) =>
              child.type == 'NavigationRailItem' ||
              child.type == 'WideNavigationRailItem',
        )
        .toList(growable: false);
    final header = _slotChildren('header');
    return SizedBox(
      height: _navigationRailHeight(items, header),
      child: NavigationRail(
        selectedIndex: _selectedIndex(items, node.props['selectedIndex']),
        backgroundColor: _color(context, node.props['containerColor']),
        leading: header.isEmpty
            ? null
            : Column(mainAxisSize: MainAxisSize.min, children: header),
        labelType: items.any((item) => _bool(item.props['alwaysShowLabel']))
            ? NavigationRailLabelType.all
            : NavigationRailLabelType.selected,
        destinations: items.isEmpty
            ? const <NavigationRailDestination>[
                NavigationRailDestination(
                  icon: Icon(Icons.widgets),
                  label: Text(''),
                ),
              ]
            : items
                  .map(
                    (item) => NavigationRailDestination(
                      icon: _navigationItemIcon(item, selected: false),
                      selectedIcon: _hasSlotFrom(item, 'selectedIcon')
                          ? _navigationItemIcon(item, selected: true)
                          : null,
                      label: Text(
                        _plainTextFrom(item, 'label') ??
                            _string(item.props['label']),
                      ),
                    ),
                  )
                  .toList(growable: false),
        onDestinationSelected: (index) {
          if (index >= 0 && index < items.length) {
            final item = items[index];
            if (item.props['enabled'] != false) {
              _invokeAction(item.props['onClick']);
            }
          }
        },
      ),
    );
  }

  /// Builds navigation item tile for the Compose DSL renderer.
  Widget _navigationItemTile() {
    final selected = _bool(node.props['selected']);
    final leading = _navigationItemIcon(
      node,
      selected: selected,
      includeBadge: false,
    );
    return ListTile(
      selected: selected,
      leading: leading,
      title: _slotOrText('label'),
      trailing: _hasSlot('badge') ? _slotInline('badge') : null,
      enabled: _enabled(),
      shape: RoundedRectangleBorder(
        borderRadius: _borderRadius(node.props['shape']) ?? BorderRadius.zero,
      ),
      onTap: _enabled() ? () => _invokeAction(node.props['onClick']) : null,
    );
  }

  /// Builds navigation item icon for the Compose DSL renderer.
  Widget _navigationItemIcon(
    _ComposeDslNode item, {
    required bool selected,
    bool includeBadge = true,
  }) {
    final icon =
        (selected ? _slotFrom(item, 'selectedIcon') : null) ??
        _slotFrom(item, 'icon') ??
        const Icon(Icons.circle_outlined);
    if (!includeBadge || !_hasSlotFrom(item, 'badge')) {
      return icon;
    }
    return Badge(label: _slotFrom(item, 'badge'), child: icon);
  }

  /// Resolves navigation content nodes for the Compose DSL renderer.
  List<_ComposeDslNode> _navigationContentNodes() {
    final content = node.slots['content'];
    return content != null && content.isNotEmpty ? content : node.children;
  }

  /// Resolves navigation rail height for the Compose DSL renderer.
  double _navigationRailHeight(
    List<_ComposeDslNode> items,
    List<Widget> header,
  ) {
    final explicit = _number(node.props['height']);
    if (explicit != null && explicit > 0) {
      return explicit;
    }
    return math.max(
      120,
      72.0 * math.max(1, items.length) + 48.0 * header.length,
    );
  }

  /// Builds scaffold for the Compose DSL renderer.
  Widget _scaffold(BuildContext context) {
    final topBar = _slotChildren('topBar');
    final bottomBar = _slotChildren('bottomBar');
    final snackbarHost = _slotChildren('snackbarHost');
    final contentColor = _color(context, node.props['contentColor']);
    final content = contentColor == null
        ? _slotOrChildren('content')
        : IconTheme.merge(
            data: IconThemeData(color: contentColor),
            child: DefaultTextStyle.merge(
              style: TextStyle(color: contentColor),
              child: _slotOrChildren('content'),
            ),
          );
    return Material(
      color:
          _color(context, node.props['containerColor']) ??
          Theme.of(context).colorScheme.surface,
      child: Stack(
        children: <Widget>[
          Column(
            children: <Widget>[
              ...topBar,
              Expanded(child: content),
              ...bottomBar,
            ],
          ),
          if (_hasSlot('floatingActionButton'))
            Positioned(
              right: 16,
              bottom: bottomBar.isEmpty ? 16 : 88,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.end,
                children: _slotChildren('floatingActionButton'),
              ),
            ),
          if (snackbarHost.isNotEmpty)
            Positioned(
              left: 16,
              right: 16,
              bottom: 16,
              child: _slotColumn('snackbarHost'),
            ),
        ],
      ),
    );
  }

  /// Builds drawer sheet for the Compose DSL renderer.
  Widget _drawerSheet(BuildContext context) {
    return Material(
      color:
          _color(context, node.props['drawerContainerColor']) ??
          Theme.of(context).colorScheme.surface,
      elevation: _number(node.props['drawerTonalElevation']) ?? 0,
      child: SafeArea(child: _slotOrChildren('content')),
    );
  }

  /// Builds navigation drawer for the Compose DSL renderer.
  Widget _navigationDrawer(BuildContext context) {
    final drawer = _slotChildren('drawerContent');
    final content = _slotOrChildren('content');
    if (drawer.isEmpty) {
      return content;
    }
    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        SizedBox(
          width: _number(node.props['drawerWidth']) ?? 304,
          child: drawer.first,
        ),
        Expanded(child: content),
      ],
    );
  }

  /// Builds tab item for the Compose DSL renderer.
  Widget _tabItem(BuildContext context, {required bool leadingIcon}) {
    final selected = _bool(node.props['selected']);
    final selectedContentColor = _color(
      context,
      node.props['selectedContentColor'],
    );
    final unselectedContentColor = _color(
      context,
      node.props['unselectedContentColor'],
    );
    final contentColor = selected
        ? selectedContentColor
        : unselectedContentColor;
    final effectiveColor =
        contentColor ??
        (selected
            ? Theme.of(context).colorScheme.primary
            : Theme.of(context).colorScheme.onSurfaceVariant);
    return InkWell(
      onTap: _enabled() ? () => _invokeAction(node.props['onClick']) : null,
      child: IconTheme.merge(
        data: IconThemeData(color: effectiveColor),
        child: DefaultTextStyle.merge(
          style: TextStyle(color: effectiveColor),
          child: Padding(
            padding: _edgeInsetsFromValue(
              node.props['contentPadding'] ??
                  (leadingIcon
                      ? const <Object?>[16, 12]
                      : const <Object?>[18, 14]),
            ),
            child: leadingIcon ? _leadingIconTabContent() : _tabContent(),
          ),
        ),
      ),
    );
  }

  /// Builds tab content for the Compose DSL renderer.
  Widget _tabContent() {
    final content = _slotChildren('content', useChildren: true);
    if (content.isNotEmpty) {
      return Column(mainAxisSize: MainAxisSize.min, children: content);
    }
    final text = _plainSlotText('text') ?? _string(node.props['text']);
    return Text(text);
  }

  /// Builds leading icon tab content for the Compose DSL renderer.
  Widget _leadingIconTabContent() {
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        if (_hasSlot('icon')) ...<Widget>[
          _slotInline('icon'),
          const SizedBox(width: 8),
        ],
        if (_hasSlot('text'))
          _slotInline('text')
        else
          Text(_string(node.props['text'])),
      ],
    );
  }

  /// Builds tab row for the Compose DSL renderer.
  Widget _tabRow(BuildContext context, String type) {
    final contentColor = _color(context, node.props['contentColor']);
    final tabWidgets = _slotChildren('tabs');
    Widget tabs = Row(mainAxisSize: MainAxisSize.min, children: tabWidgets);
    if (type.contains('Scrollable')) {
      final edgePadding = _number(node.props['edgePadding']) ?? 0;
      tabs = SingleChildScrollView(
        scrollDirection: Axis.horizontal,
        child: Padding(
          padding: EdgeInsets.symmetric(horizontal: edgePadding),
          child: tabs,
        ),
      );
    }
    final tabBand = Stack(
      alignment: Alignment.bottomCenter,
      children: <Widget>[
        tabs,
        if (_hasSlot('indicator')) _slotInline('indicator'),
      ],
    );
    return Material(
      color:
          _color(context, node.props['containerColor']) ??
          Theme.of(context).colorScheme.surface,
      child: DefaultTextStyle.merge(
        style: TextStyle(color: contentColor),
        child: IconTheme.merge(
          data: IconThemeData(color: contentColor),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              tabBand,
              if (_hasSlot('divider'))
                _slotColumn('divider')
              else
                Divider(
                  height: 1,
                  thickness: 1,
                  color: Theme.of(context).colorScheme.outlineVariant,
                ),
            ],
          ),
        ),
      ),
    );
  }
}
