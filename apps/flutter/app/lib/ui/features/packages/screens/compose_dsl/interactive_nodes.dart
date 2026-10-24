// ignore_for_file: file_names

part of '../ToolPkgUiLauncherScreen.dart';

/// Renders interactive nodes for Compose DSL nodes.
extension _ComposeInteractiveNodes on _ComposeProgramExecutorState {
  /// Builds pull to refresh box for the Compose DSL renderer.
  Widget _pullToRefreshBox(BuildContext context) {
    final refreshing = _bool(props['isRefreshing']);
    final content = RefreshIndicator(
      onRefresh: () async {
        final actionId = _actionId(props['onRefresh']);
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
      alignment: _alignment(props['contentAlignment']),
      children: <Widget>[
        content,
        Positioned(left: 0, top: 0, right: 0, child: _slotColumn('indicator')),
      ],
    );
  }

  /// Builds a dropdown menu using the shared application menu surface defaults.
  Widget _dropdownMenu(BuildContext context) {
    final items = _slotChildren('content', useChildren: true);
    final label = _plainSlotText('label') ?? _string(props['label']);
    final anchor = _dropdownMenuAnchor(label);
    final colorScheme = Theme.of(context).colorScheme;
    if (_bool(props['expanded'])) {
      final offset = _number(props['offset']) ?? 0;
      final selectActionId = _actionId(props['onClick']);
      return Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          anchor,
          SizedBox(height: offset),
          Material(
            color:
                _color(context, props['containerColor']) ??
                colorScheme.surfaceContainerHigh,
            elevation: _number(props['tonalElevation']) ?? 3,
            shape: RoundedRectangleBorder(
              borderRadius:
                  _borderRadius(props['shape']) ??
                  BorderRadius.circular(8),
            ),
            child: ConstrainedBox(
              constraints: BoxConstraints(
                minWidth: _number(props['menuMinWidth']) ?? 160,
                maxWidth: _number(props['menuMaxWidth']) ?? 360,
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
      onSelected: (index) => _invokeAction(props['onClick'], index),
      child: anchor,
    );
  }

  /// Builds dropdown menu anchor for the Compose DSL renderer.
  Widget _dropdownMenuAnchor(String label) {
    final width = _number(props['width']) ?? 200;
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
                _string(props['text'] ?? props['selectedText']),
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
      backgroundColor: _color(context, props['containerColor']),
      shape: _borderRadius(props['shape']) == null
          ? null
          : RoundedRectangleBorder(
              borderRadius: _borderRadius(props['shape'])!,
            ),
    );
    final dismissActionId = _actionId(props['onDismissRequest']);
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
        _color(context, props['color']) ??
        Theme.of(context).colorScheme.outlineVariant;
    return Center(
      child: Container(
        width: _number(props['width']) ?? 4,
        height: _number(props['height']) ?? 36,
        decoration: BoxDecoration(
          color: color,
          borderRadius: BorderRadius.circular(999),
        ),
      ),
    );
  }

  /// Builds switch for the Compose DSL renderer.
  Widget _switch(BuildContext context) {
    final actionId = _actionId(props['onCheckedChange']);
    final checked = _bool(props['checked']);
    final enabled = _enabled() && actionId != null;
    final checkedThumbColor = _color(context, props['checkedThumbColor']);
    final checkedTrackColor = _color(context, props['checkedTrackColor']);
    final uncheckedThumbColor = _color(
      context,
      props['uncheckedThumbColor'],
    );
    final uncheckedTrackColor = _color(
      context,
      props['uncheckedTrackColor'],
    );
    final fitToBounds =
        _number(props['width']) != null &&
        _number(props['height']) != null;
    final control = Switch(
      materialTapTargetSize: fitToBounds
          ? MaterialTapTargetSize.shrinkWrap
          : MaterialTapTargetSize.padded,
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
      return fitToBounds ? FittedBox(child: control) : control;
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
    final actionId = _actionId(props['onCheckedChange']);
    return Checkbox(
      value: _bool(props['checked']),
      onChanged: _enabled() && actionId != null
          ? (value) => _invokeAction(actionId, value)
          : null,
    );
  }

  /// Builds radio button for the Compose DSL renderer.
  Widget _radioButton() {
    final selected = _bool(props['selected']);
    return RadioGroup<bool>(
      groupValue: selected,
      onChanged: (_) => _invokeAction(props['onClick']),
      child: Radio<bool>(value: true, enabled: _enabled()),
    );
  }

  /// Presents a controlled modal node without occupying its parent's layout.
  Widget _dialog(BuildContext context, bool alert) {
    return _ComposeDialogHost(
      embedded: embedDialog,
      properties: _stringMap(props['properties']),
      onDismissRequest: () async {
        final action = _actionId(props['onDismissRequest']);
        if (action != null) await onAction(action);
      },
      closeOnDismissRequest: props['closeOnDismissRequest'] != false,
      dialogBuilder: (context, close) {
        if (!mounted || node.isDisposed) return const SizedBox.shrink();

        /// Dispatches an explicit dialog button action and applies its close policy.
        Future<void> activate(String actionName, String closeName) async {
          final action = _actionId(props[actionName]);
          if (action != null) await onAction(action);
          if (!mounted || node.isDisposed) return;
          if (props[closeName] != false) close();
        }

        final properties = _stringMap(props['properties']);
        final constraints = properties['usePlatformDefaultWidth'] == false
            ? const BoxConstraints()
            : null;
        final shape = _borderRadius(props['shape']);
        if (!alert) {
          final content = _withSlotColor(
            context,
            // Retain viewport bounds, including IME insets, in both modes.
            _ComposeBox(
              alignment: Alignment.center,
              nodes: _slotNodes('content', useChildren: true),
              children: _slotChildren(
                'content',
                useChildren: true,
                modifierScope: _ComposeDslModifierScope.box,
              ),
            ),
            _color(context, props['contentColor']),
          );
          if (properties['fullscreen'] == true) {
            return Dialog.fullscreen(
              backgroundColor: _color(context, props['containerColor']),
              child: content,
            );
          }
          return Dialog(
            constraints: constraints,
            backgroundColor: _color(context, props['containerColor']),
            elevation: _number(props['tonalElevation']),
            shape: shape == null
                ? null
                : RoundedRectangleBorder(borderRadius: shape),
            child: content,
          );
        }

        final plainText =
            props['text'] is String &&
            !_hasSlot('text') &&
            !_hasSlot('content') &&
            node.children.isEmpty &&
            props['markdown'] == null &&
            props['width'] == null &&
            props['height'] == null;
        return AlertDialog(
          constraints: constraints,
          backgroundColor: _color(context, props['containerColor']),
          elevation: _number(props['tonalElevation']),
          shape: shape == null
              ? null
              : RoundedRectangleBorder(borderRadius: shape),
          icon: _hasSlot('icon') ? _slotColumn('icon') : null,
          iconColor: _color(context, props['iconContentColor']),
          title: _hasSlot('title')
              ? _slotColumn('title')
              : props['title'] is String
              ? Text(props['title'] as String)
              : null,
          titleTextStyle: plainText
              ? null
              : Theme.of(context).textTheme.headlineSmall?.copyWith(
                  color: _color(context, props['titleContentColor']),
                ),
          contentTextStyle: plainText
              ? null
              : Theme.of(context).textTheme.bodyMedium?.copyWith(
                  color: _color(context, props['textContentColor']),
                ),
          content: plainText
              ? Text(props['text'] as String)
              : SizedBox(
                  width: _number(props['width']) ?? 560,
                  height: _number(props['height']) ?? 360,
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: <Widget>[
                      if (_hasSlot('text')) _slotColumn('text'),
                      if (props['text'] is String)
                        Text(props['text'] as String),
                      if (props['markdown'] is String)
                        _ComposeProgramExecutor(
                          node: _composeInlineProgram('Markdown', <String, Object?>{'text': props['markdown']}),
                          onAction: onAction,
                          onTextInput: onTextInput,
                          webViewHostContext: webViewHostContext,
                          splitMarkdownContent: splitMarkdownContent,
                          dispatchOnLoad: widget.dispatchOnLoad,
                        ),
                      ..._slotChildren('content', useChildren: true),
                    ],
                  ),
                ),
          actions: <Widget>[
            if (_hasSlot('dismissButton'))
              _slotInline('dismissButton')
            else if (props['dismissText'] != null ||
                _actionId(props['onDismiss']) != null)
              TextButton(
                onPressed: () => activate('onDismiss', 'closeOnDismiss'),
                child: Text(
                  props['dismissText'] as String? ??
                      MaterialLocalizations.of(context).cancelButtonLabel,
                ),
              ),
            if (_hasSlot('confirmButton'))
              _slotInline('confirmButton')
            else
              TextButton(
                onPressed: () => activate('onConfirm', 'closeOnConfirm'),
                child: Text(
                  props['confirmText'] as String? ??
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
        props['singleLine'] == true || props['isPassword'] == true;
    return _ComposeTextField(
      identity: _string(props['key']).trim().isEmpty
          ? nodePath
          : _string(props['key']).trim(),
      value: _string(props['value']),
      enabled: _enabled() && _actionId(props['onValueChange']) != null,
      readOnly: props['readOnly'] == true,
      obscureText: props['isPassword'] == true,
      singleLine: isSingleLine,
      minLines: isSingleLine ? null : (_int(props['minLines']) ?? 1),
      maxLines: isSingleLine ? 1 : _int(props['maxLines']),
      keyboardType: _textInputType(props['keyboardType']),
      textInputAction: _textInputAction(
        props['imeAction'] ?? props['keyboardAction'],
      ),
      isError: props['isError'] == true,
      textStyle: _textFieldStyle(context, props['style']),
      decoration: _textFieldDecoration(context, props['decoration']),
      labelText: labelSlot == null ? _plainSlotText('label') : null,
      label: labelSlot,
      hintText: placeholderSlot == null ? _plainSlotText('placeholder') : null,
      hint: placeholderSlot,
      prefixIcon: _hasSlot('leadingIcon')
          ? Center(
              widthFactor: 1,
              heightFactor: 1,
              child: _slotInline('leadingIcon'),
            )
          : null,
      suffixIconConstraints:
          props['suffixIconWidth'] != null ||
              props['suffixIconHeight'] != null
          ? BoxConstraints.tightFor(
              width: _number(props['suffixIconWidth']),
              height: _number(props['suffixIconHeight']),
            )
          : null,
      suffixIcon: _hasSlot('trailingIcon')
          ? Center(
              widthFactor: 1,
              heightFactor: 1,
              child: _slotInline('trailingIcon'),
            )
          : null,
      prefix: _hasSlot('prefix') ? _slotColumn('prefix') : null,
      suffix: _hasSlot('suffix') ? _slotColumn('suffix') : null,
      supportingText: supportingSlot,
      helperText: supportingSlot == null
          ? _plainSlotText('supportingText')
          : null,
      border: type == 'OutlinedTextField' ? const OutlineInputBorder() : null,
      onChanged: (value) => onTextInput(
        _requiredActionId(props['onValueChange'], 'onValueChange'),
        value,
      ),
    );
  }

  /// Builds canvas for the Compose DSL renderer.
  Widget _canvas(BuildContext context) {
    Widget canvas = CustomPaint(
      painter: _ComposeCanvasPainter(
        commands: _canvasCommands(props['commands']),
        colorScheme: Theme.of(context).colorScheme,
        textTheme: Theme.of(context).textTheme,
      ),
    );
    final transform = _stringMap(props['transform']);
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
    if (_actionId(props['onTransform']) != null) {
      canvas = _ComposeGestureRegion(
        kind: 'transformgestures',
        options: <String, Object?>{'onGesture': props['onTransform']},
        onAction: onAction,
        child: canvas,
      );
    }
    final actionId = _actionId(props['onSizeChanged']);
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
      props['onOpenChanged'],
      'onOpenChanged',
    );
    return AdaptiveSidePanel(
      open: props['open'] == true,
      onOpenChanged: (open) {
        unawaited(onAction(openChangedActionId, open));
      },
      breakpoint: _number(props['breakpoint']) ?? 600,
      defaultWidth: _number(props['defaultWidth']) ?? 360,
      minWidth: _number(props['minWidth']) ?? 280,
      minContentWidth: _number(props['minContentWidth']) ?? 320,
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
    return _ComposeProgramExecutor(
      node: nodes.single,
      onAction: onAction,
      onTextInput: onTextInput,
      webViewHostContext: webViewHostContext,
      splitMarkdownContent: splitMarkdownContent,
      dispatchOnLoad: widget.dispatchOnLoad,
      nodePath: '$nodePath:$name/0',
    );
  }

  /// Builds list items on demand while retaining their explicit plugin identities.
  Widget _lazyList(Axis axis) {
    final nodes = _slotNodes('content', useChildren: true);
    return _ComposeLazyList(
      axis: axis,
      reverse: props['reverseLayout'] == true,
      autoScrollToEnd: props['autoScrollToEnd'] == true,
      spacing: _number(props['spacing']) ?? 0,
      alignment: axis == Axis.vertical
          ? Alignment(switch (_normalizeToken(
              _string(props['horizontalAlignment']),
            )) {
              'center' || 'centerhorizontally' => 0,
              'end' || 'right' => 1,
              _ => -1,
            }, -1)
          : Alignment(-1, switch (_normalizeToken(
              _string(props['verticalAlignment']),
            )) {
              'center' || 'centervertically' => 0,
              'bottom' || 'end' => 1,
              _ => -1,
            }),
      nodes: nodes,
      window: props['viewport'] == null
          ? null
          : _ComposeLazyWindow.parse(props['viewport'], nodes.length),
      onAction: onAction,
      itemBuilder: (context, index) => _ComposeProgramExecutor(
        key: nodes[index].props['key'] == null
            ? null
            : ValueKey(nodes[index].props['key']),
        node: nodes[index],
        onAction: onAction,
        onTextInput: onTextInput,
        webViewHostContext: webViewHostContext,
        splitMarkdownContent: splitMarkdownContent,
        dispatchOnLoad: widget.dispatchOnLoad,
        nodePath: '$nodePath:content/$index',
      ),
    );
  }
}
