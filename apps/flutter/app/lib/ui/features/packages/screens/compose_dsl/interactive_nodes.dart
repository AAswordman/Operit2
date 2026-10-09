// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Renders interactive nodes for Compose DSL nodes.
extension _ComposeInteractiveNodes on _ComposeDslRendererState {
  /// Builds pull to refresh box for the Compose DSL renderer.
  Widget _pullToRefreshBox(BuildContext context) {
    final refreshing = _bool(node.props['isRefreshing']);
    final content = RefreshIndicator(
      onRefresh: () async {
        final actionId = _actionId(node.props['onRefresh']);
        if (actionId != null) {
          await onAction(actionId);
        }
      },
      child: SingleChildScrollView(
        physics: const AlwaysScrollableScrollPhysics(),
        child: _slotOrChildren('content'),
      ),
    );
    if (!refreshing || !_hasSlot('indicator')) {
      return content;
    }
    return Stack(
      alignment: _alignment(node.props['contentAlignment']),
      children: <Widget>[
        content,
        Positioned(left: 0, top: 0, right: 0, child: _slotColumn('indicator')),
      ],
    );
  }

  /// Builds a dropdown menu using the shared application menu surface defaults.
  Widget _dropdownMenu(BuildContext context) {
    final items = _slotChildren('content', useChildren: true);
    final label = _plainSlotText('label') ?? _string(node.props['label']);
    final anchor = _dropdownMenuAnchor(label);
    final colorScheme = Theme.of(context).colorScheme;
    if (_bool(node.props['expanded'])) {
      final offset = _number(node.props['offset']) ?? 0;
      final selectActionId = _actionId(node.props['onClick']);
      return Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          anchor,
          SizedBox(height: offset),
          Material(
            color:
                _color(context, node.props['containerColor']) ??
                colorScheme.surfaceContainerHigh,
            elevation: _number(node.props['tonalElevation']) ?? 3,
            shape: RoundedRectangleBorder(
              borderRadius:
                  _borderRadius(node.props['shape']) ??
                  BorderRadius.circular(8),
            ),
            child: ConstrainedBox(
              constraints: BoxConstraints(
                minWidth: _number(node.props['menuMinWidth']) ?? 160,
                maxWidth: _number(node.props['menuMaxWidth']) ?? 360,
              ),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  for (var index = 0; index < items.length; index += 1)
                    selectActionId == null
                        ? items[index]
                        : InkWell(
                            onTap: () => onAction(selectActionId, index),
                            child: items[index],
                          ),
                ],
              ),
            ),
          ),
        ],
      );
    }
    return PopupMenuButton<int>(
      enabled: _enabled(),
      tooltip: label.isEmpty ? null : label,
      itemBuilder: (context) => <PopupMenuEntry<int>>[
        for (var index = 0; index < items.length; index += 1)
          PopupMenuItem<int>(value: index, child: items[index]),
      ],
      onSelected: (index) => _invokeAction(node.props['onClick'], index),
      child: anchor,
    );
  }

  /// Builds dropdown menu anchor for the Compose DSL renderer.
  Widget _dropdownMenuAnchor(String label) {
    final width = _number(node.props['width']) ?? 200;
    return SizedBox(
      width: width,
      child: InputDecorator(
        decoration: InputDecoration(
          labelText: label.isEmpty ? null : label,
          border: const OutlineInputBorder(),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Expanded(
              child: Text(
                _string(node.props['text'] ?? node.props['selectedText']),
                overflow: TextOverflow.ellipsis,
              ),
            ),
            const Icon(Icons.arrow_drop_down),
          ],
        ),
      ),
    );
  }

  /// Builds time picker dialog for the Compose DSL renderer.
  Widget _timePickerDialog(BuildContext context) {
    final dialog = AlertDialog(
      title: _hasSlot('title') ? _slotColumn('title') : null,
      content: _slotOrChildren('content'),
      actions: <Widget>[
        ..._slotChildren('modeToggleButton'),
        ..._slotChildren('dismissButton'),
        ..._slotChildren('confirmButton'),
      ],
      backgroundColor: _color(context, node.props['containerColor']),
      shape: _borderRadius(node.props['shape']) == null
          ? null
          : RoundedRectangleBorder(
              borderRadius: _borderRadius(node.props['shape'])!,
            ),
    );
    final dismissActionId = _actionId(node.props['onDismissRequest']);
    if (dismissActionId == null) {
      return dialog;
    }
    return Focus(
      autofocus: true,
      onKeyEvent: (node, event) {
        if (event is KeyDownEvent &&
            event.logicalKey == LogicalKeyboardKey.escape) {
          onAction(dismissActionId);
          return KeyEventResult.handled;
        }
        return KeyEventResult.ignored;
      },
      child: dialog,
    );
  }

  /// Builds vertical drag handle for the Compose DSL renderer.
  Widget _verticalDragHandle(BuildContext context) {
    final color =
        _color(context, node.props['color']) ??
        Theme.of(context).colorScheme.outlineVariant;
    return Center(
      child: Container(
        width: _number(node.props['width']) ?? 4,
        height: _number(node.props['height']) ?? 36,
        decoration: BoxDecoration(
          color: color,
          borderRadius: BorderRadius.circular(999),
        ),
      ),
    );
  }

  /// Builds switch for the Compose DSL renderer.
  Widget _switch(BuildContext context) {
    final actionId = _actionId(node.props['onCheckedChange']);
    final checked = _bool(node.props['checked']);
    final enabled = _enabled() && actionId != null;
    final checkedThumbColor = _color(context, node.props['checkedThumbColor']);
    final checkedTrackColor = _color(context, node.props['checkedTrackColor']);
    final uncheckedThumbColor = _color(
      context,
      node.props['uncheckedThumbColor'],
    );
    final uncheckedTrackColor = _color(
      context,
      node.props['uncheckedTrackColor'],
    );
    final control = Switch(
      value: checked,
      onChanged: enabled ? (value) => _invokeAction(actionId, value) : null,
      thumbColor: _stateColor(
        checked: checkedThumbColor,
        unchecked: uncheckedThumbColor,
      ),
      trackColor: _stateColor(
        checked: checkedTrackColor,
        unchecked: uncheckedTrackColor,
      ),
    );
    if (!_hasSlot('thumbContent')) {
      return control;
    }
    return SizedBox(
      width: 58,
      height: 36,
      child: Stack(
        alignment: Alignment.center,
        children: <Widget>[
          control,
          AnimatedAlign(
            duration: const Duration(milliseconds: 120),
            curve: Curves.easeOut,
            alignment: checked ? Alignment.centerRight : Alignment.centerLeft,
            child: IgnorePointer(
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 8),
                child: SizedBox(
                  width: 18,
                  height: 18,
                  child: FittedBox(child: _slotInline('thumbContent')),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }

  /// Builds checkbox for the Compose DSL renderer.
  Widget _checkbox() {
    final actionId = _actionId(node.props['onCheckedChange']);
    return Checkbox(
      value: _bool(node.props['checked']),
      onChanged: _enabled() && actionId != null
          ? (value) => _invokeAction(actionId, value)
          : null,
    );
  }

  /// Builds radio button for the Compose DSL renderer.
  Widget _radioButton() {
    final selected = _bool(node.props['selected']);
    return RadioGroup<bool>(
      groupValue: selected,
      onChanged: (_) => _invokeAction(node.props['onClick']),
      child: Radio<bool>(value: true, enabled: _enabled()),
    );
  }

  /// Presents a controlled modal node without occupying its parent's layout.
  Widget _dialog(BuildContext context, bool alert) {
    return _ComposeDialogHost(
      embedded: embedDialog,
      properties: _stringMap(node.props['properties']),
      onDismissRequest: () async {
        final action = _actionId(node.props['onDismissRequest']);
        if (action != null) await onAction(action);
      },
      closeOnDismissRequest: node.props['closeOnDismissRequest'] != false,
      dialogBuilder: (context, close) {
        /// Dispatches an explicit dialog button action and applies its close policy.
        Future<void> activate(String actionName, String closeName) async {
          final action = _actionId(node.props[actionName]);
          if (action != null) await onAction(action);
          if (node.props[closeName] != false) close();
        }

        final properties = _stringMap(node.props['properties']);
        final constraints = properties['usePlatformDefaultWidth'] == false
            ? const BoxConstraints()
            : null;
        final shape = _borderRadius(node.props['shape']);
        if (!alert) {
          return Dialog(
            constraints: constraints,
            backgroundColor: _color(context, node.props['containerColor']),
            elevation: _number(node.props['tonalElevation']),
            shape: shape == null
                ? null
                : RoundedRectangleBorder(borderRadius: shape),
            child: _withSlotColor(
              context,
              // Dialog content must retain the viewport bounds, including IME insets.
              // An implicit Column gives its children infinite height and forces width.
              _ComposeBox(
                alignment: Alignment.center,
                nodes: _slotNodes('content', useChildren: true),
                children: _slotChildren(
                  'content',
                  useChildren: true,
                  modifierScope: _ComposeDslModifierScope.box,
                ),
              ),
              _color(context, node.props['contentColor']),
            ),
          );
        }
        return AlertDialog(
          constraints: constraints,
          backgroundColor: _color(context, node.props['containerColor']),
          elevation: _number(node.props['tonalElevation']),
          shape: shape == null
              ? null
              : RoundedRectangleBorder(borderRadius: shape),
          icon: _hasSlot('icon') ? _slotColumn('icon') : null,
          iconColor: _color(context, node.props['iconContentColor']),
          title: _hasSlot('title')
              ? _slotColumn('title')
              : node.props['title'] is String
              ? Text(node.props['title'] as String)
              : null,
          titleTextStyle: Theme.of(context).textTheme.headlineSmall?.copyWith(
            color: _color(context, node.props['titleContentColor']),
          ),
          contentTextStyle: Theme.of(context).textTheme.bodyMedium?.copyWith(
            color: _color(context, node.props['textContentColor']),
          ),
          content: SizedBox(
            width: _number(node.props['width']) ?? 560,
            height: _number(node.props['height']) ?? 360,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                if (_hasSlot('text')) _slotColumn('text'),
                if (node.props['text'] is String)
                  Text(node.props['text'] as String),
                if (node.props['markdown'] is String)
                  _ComposeDslRenderer(
                    node: _ComposeDslNode(
                      type: 'Markdown',
                      props: <String, Object?>{'text': node.props['markdown']},
                      children: const [],
                      slots: const {},
                    ),
                    onAction: onAction,
                    onTextInput: onTextInput,
                    webViewHostContext: webViewHostContext,
                    splitMarkdownContent: splitMarkdownContent,
                  ),
                ..._slotChildren('content', useChildren: true),
              ],
            ),
          ),
          actions: <Widget>[
            if (_hasSlot('dismissButton'))
              _slotInline('dismissButton')
            else if (node.props['dismissText'] != null ||
                _actionId(node.props['onDismiss']) != null)
              TextButton(
                onPressed: () => activate('onDismiss', 'closeOnDismiss'),
                child: Text(
                  node.props['dismissText'] as String? ??
                      MaterialLocalizations.of(context).cancelButtonLabel,
                ),
              ),
            if (_hasSlot('confirmButton'))
              _slotInline('confirmButton')
            else
              TextButton(
                onPressed: () => activate('onConfirm', 'closeOnConfirm'),
                child: Text(
                  node.props['confirmText'] as String? ??
                      MaterialLocalizations.of(context).okButtonLabel,
                ),
              ),
          ],
        );
      },
    );
  }

  /// Builds text field for the Compose DSL renderer.
  Widget _textField(BuildContext context, String type) {
    final labelSlot = _hasSlot('label') ? _slotColumn('label') : null;
    final placeholderSlot = _hasSlot('placeholder')
        ? _slotColumn('placeholder')
        : null;
    final supportingSlot = _hasSlot('supportingText')
        ? _slotColumn('supportingText')
        : null;
    final isSingleLine =
        node.props['singleLine'] == true || node.props['isPassword'] == true;
    return _ComposeTextField(
      identity: _string(node.props['key']).trim().isEmpty
          ? nodePath
          : _string(node.props['key']).trim(),
      value: _string(node.props['value']),
      enabled: _enabled() && _actionId(node.props['onValueChange']) != null,
      readOnly: node.props['readOnly'] == true,
      obscureText: node.props['isPassword'] == true,
      singleLine: isSingleLine,
      minLines: isSingleLine ? null : (_int(node.props['minLines']) ?? 1),
      maxLines: isSingleLine ? 1 : _int(node.props['maxLines']),
      keyboardType: _textInputType(node.props['keyboardType']),
      textInputAction: _textInputAction(
        node.props['imeAction'] ?? node.props['keyboardAction'],
      ),
      isError: node.props['isError'] == true,
      textStyle: _textFieldStyle(context, node.props['style']),
      labelText: labelSlot == null ? _plainSlotText('label') : null,
      label: labelSlot,
      hintText: placeholderSlot == null ? _plainSlotText('placeholder') : null,
      hint: placeholderSlot,
      prefixIcon: _hasSlot('leadingIcon') ? _slotColumn('leadingIcon') : null,
      suffixIcon: _hasSlot('trailingIcon') ? _slotColumn('trailingIcon') : null,
      prefix: _hasSlot('prefix') ? _slotColumn('prefix') : null,
      suffix: _hasSlot('suffix') ? _slotColumn('suffix') : null,
      supportingText: supportingSlot,
      helperText: supportingSlot == null
          ? _plainSlotText('supportingText')
          : null,
      border: type == 'TextField'
          ? const OutlineInputBorder()
          : const OutlineInputBorder(),
      onChanged: (value) => onTextInput(
        _requiredActionId(node.props['onValueChange'], 'onValueChange'),
        value,
      ),
    );
  }

  /// Builds canvas for the Compose DSL renderer.
  Widget _canvas(BuildContext context) {
    Widget canvas = CustomPaint(
      painter: _ComposeCanvasPainter(
        commands: _canvasCommands(node.props['commands']),
        colorScheme: Theme.of(context).colorScheme,
        textTheme: Theme.of(context).textTheme,
      ),
    );
    final transform = _stringMap(node.props['transform']);
    if (transform.isNotEmpty) {
      final pivot = Offset(
        _number(transform['pivotX']) ?? 0,
        _number(transform['pivotY']) ?? 0,
      );
      canvas = Transform.translate(
        offset: Offset(
          _number(transform['offsetX']) ?? 0,
          _number(transform['offsetY']) ?? 0,
        ),
        child: Transform.scale(
          scale: _number(transform['scale']) ?? 1,
          origin: pivot,
          alignment: Alignment.topLeft,
          child: canvas,
        ),
      );
    }
    if (_actionId(node.props['onTransform']) != null) {
      canvas = _ComposeGestureRegion(
        kind: 'transformgestures',
        options: <String, Object?>{'onGesture': node.props['onTransform']},
        onAction: onAction,
        child: canvas,
      );
    }
    final actionId = _actionId(node.props['onSizeChanged']);
    if (actionId == null) {
      return canvas;
    }
    return _SizeReportingBox(
      onSizeChanged: (size) {
        onAction(actionId, <String, Object?>{
          'width': size.width,
          'height': size.height,
        });
      },
      child: canvas,
    );
  }

  /// Builds a plugin-controlled responsive trailing panel around the primary slot.
  Widget _adaptiveSidePanel() {
    final openChangedActionId = _requiredActionId(
      node.props['onOpenChanged'],
      'onOpenChanged',
    );
    return AdaptiveSidePanel(
      open: node.props['open'] == true,
      onOpenChanged: (open) {
        unawaited(onAction(openChangedActionId, open));
      },
      breakpoint: _number(node.props['breakpoint']) ?? 600,
      defaultWidth: _number(node.props['defaultWidth']) ?? 360,
      minWidth: _number(node.props['minWidth']) ?? 280,
      minContentWidth: _number(node.props['minContentWidth']) ?? 320,
      panel: _adaptiveSidePanelSlot('side'),
      child: _adaptiveSidePanelSlot('content', useChildren: true),
    );
  }

  /// Renders one adaptive side-panel slot while preserving its available bounds.
  Widget _adaptiveSidePanelSlot(String name, {bool useChildren = false}) {
    final nodes = _slotNodes(name, useChildren: useChildren);
    if (nodes.length != 1) {
      throw StateError('AdaptiveSidePanel.$name requires exactly one child');
    }
    return _ComposeDslRenderer(
      node: nodes.single,
      onAction: onAction,
      onTextInput: onTextInput,
      webViewHostContext: webViewHostContext,
      splitMarkdownContent: splitMarkdownContent,
      nodePath: '$nodePath:$name/0',
    );
  }

  /// Builds list items on demand while retaining their explicit plugin identities.
  Widget _lazyList(Axis axis) {
    final nodes = _slotNodes('content', useChildren: true);
    return _ComposeLazyList(
      axis: axis,
      reverse: node.props['reverseLayout'] == true,
      autoScrollToEnd: node.props['autoScrollToEnd'] == true,
      spacing: _number(node.props['spacing']) ?? 0,
      alignment: axis == Axis.vertical
          ? Alignment(switch (_normalizeToken(
              _string(node.props['horizontalAlignment']),
            )) {
              'center' || 'centerhorizontally' => 0,
              'end' || 'right' => 1,
              _ => -1,
            }, -1)
          : Alignment(-1, switch (_normalizeToken(
              _string(node.props['verticalAlignment']),
            )) {
              'center' || 'centervertically' => 0,
              'bottom' || 'end' => 1,
              _ => -1,
            }),
      nodes: nodes,
      itemBuilder: (context, index) => _ComposeDslRenderer(
        key: nodes[index].props['key'] == null
            ? null
            : ValueKey(nodes[index].props['key']),
        node: nodes[index],
        onAction: onAction,
        onTextInput: onTextInput,
        webViewHostContext: webViewHostContext,
        splitMarkdownContent: splitMarkdownContent,
        nodePath: '$nodePath:content/$index',
      ),
    );
  }
}
