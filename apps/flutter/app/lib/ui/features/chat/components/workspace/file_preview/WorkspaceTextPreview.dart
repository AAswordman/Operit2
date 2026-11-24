// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../../../l10n/generated/app_localizations.dart';
import '../../../../../common/components/RetainedPage.dart';
import '../../../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../../../../common/markdown/MarkdownLink.dart';
import '../../../../../common/markdown/MarkdownCodeTypeface.dart';
import '../../../../../theme/OperitGlassSurface.dart';
import '../WorkspaceTabModels.dart';
import 'WorkspaceTextDocument.dart';
import 'package:operit2/ui/common/code_editor/CodeEditorInput.dart';
import 'package:operit2/ui/common/code_editor/CodeEditor.dart';

/// Edits workspace text through the shared file API without browser chrome.
class WorkspaceTextPreview extends StatefulWidget {
  /// Creates an editor for a text or Markdown workspace tab.
  const WorkspaceTextPreview({
    super.key,
    required this.tab,
    required this.onWriteWorkspaceFileBytes,
    required this.onOpenBrowser,
    required this.splitMarkdownContent,
  });

  final WorkspaceTab tab;
  final Future<void> Function(String path, Uint8List bytes)
  onWriteWorkspaceFileBytes;
  final void Function({
    String? url,
    String? localFilePath,
    String? workspaceHtmlPath,
  })
  onOpenBrowser;
  final MarkdownContentSplitter splitMarkdownContent;

  /// Creates the editor's input and gesture state.
  @override
  State<WorkspaceTextPreview> createState() => _WorkspaceTextPreviewState();
}

class _WorkspaceTextPreviewState extends State<WorkspaceTextPreview> {
  late TextEditingController _controller;
  late CodeSyntaxLanguage _syntaxLanguage;
  final ValueNotifier<int> _viewportChanges = ValueNotifier<int>(0);
  final ValueNotifier<int> _chromeChanges = ValueNotifier<int>(0);
  late ({bool dirty, bool saving, Object? error}) _chromeState;
  final FocusNode _editorFocus = FocusNode();
  late UndoHistoryController _undoController;
  final Object _editorTapGroup = Object();
  final CodeEditorScrollController _editorScroll = CodeEditorScrollController();
  final CodeEditorScrollController _previewScroll =
      CodeEditorScrollController();
  GlobalKey<CodeEditorState> _editorKey = GlobalKey<CodeEditorState>();
  final _viewportKey = GlobalKey<CodeEditorState>();
  bool _showMarkdown = false;

  /// Returns the draft owned by this workspace tab.
  WorkspaceTextDocument get _document => widget.tab.textDocument!;

  /// Subscribes to the persistent draft and creates the editable input.
  @override
  void initState() {
    super.initState();
    _syntaxLanguage = CodeSyntaxLanguage.forPath(widget.tab.filePath!);
    _controller = TextEditingController.fromValue(
      TextEditingValue(
        text: _document.text,
        selection: const TextSelection.collapsed(offset: 0),
      ),
    );
    _undoController = UndoHistoryController();
    _chromeState = _readChromeState();
    _document.addListener(_documentChanged);
  }

  /// Rebinds the input when the surrounding widget changes its document.
  @override
  void didUpdateWidget(covariant WorkspaceTextPreview oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.tab.filePath != widget.tab.filePath) {
      _syntaxLanguage = CodeSyntaxLanguage.forPath(widget.tab.filePath!);
    }
    if (oldWidget.tab.textDocument == _document) return;
    oldWidget.tab.textDocument!.removeListener(_documentChanged);
    _controller.dispose();
    _undoController.dispose();
    _editorKey = GlobalKey<CodeEditorState>();
    _controller = TextEditingController.fromValue(
      TextEditingValue(
        text: _document.text,
        selection: const TextSelection.collapsed(offset: 0),
      ),
    );
    _undoController = UndoHistoryController();
    _chromeState = _readChromeState();
    _document.addListener(_documentChanged);
    _showMarkdown = false;
  }

  /// Releases input resources without discarding the tab's draft.
  @override
  void dispose() {
    _document.removeListener(_documentChanged);
    _controller.dispose();
    _undoController.dispose();
    _editorFocus.dispose();
    _editorScroll.dispose();
    _previewScroll.dispose();
    _viewportChanges.dispose();
    _chromeChanges.dispose();
    super.dispose();
  }

  /// Captures only the state that can change toolbar and error presentation.
  ({bool dirty, bool saving, Object? error}) _readChromeState() => (
    dirty: _document.isDirty,
    saving: _document.isSaving,
    error: _document.saveError,
  );

  /// Updates text and zoom independently from the static controls and symbols.
  void _documentChanged() {
    if (_controller.text != _document.text) {
      _controller.value = TextEditingValue(
        text: _document.text,
        selection: TextSelection.collapsed(offset: _document.text.length),
      );
    }
    final chrome = _readChromeState();
    if (_chromeState != chrome) {
      _chromeState = chrome;
      _chromeChanges.value++;
    }
    _viewportChanges.value++;
  }

  /// Saves UTF-8 text using the existing workspace host capability.
  Future<void> _save() => _document.save(
    (text) => widget.onWriteWorkspaceFileBytes(
      widget.tab.filePath!,
      Uint8List.fromList(utf8.encode(text)),
    ),
  );

  /// Uses the same viewport zoom action as read-only code review.
  void _zoomTo(double scale) =>
      (_showMarkdown ? _viewportKey : _editorKey).currentState!.zoomTo(scale);

  /// Switches Markdown presentation without carrying an old zoom anchor.
  void _toggleMarkdown() {
    setState(() {
      _showMarkdown = !_showMarkdown;
    });
  }

  /// Keeps the preview's link handler stable while its viewport zooms.
  void _openMarkdownLink(String url) {
    final path = markdownFileLinkPath(url);
    if (path == null) {
      widget.onOpenBrowser(url: url);
      return;
    }
    final file = widget.tab.absolutePath ?? widget.tab.filePath!;
    final separator = file.lastIndexOf('/');
    final base = separator < 0 ? null : file.substring(0, separator + 1);
    openMarkdownFileLink(
      url,
      null,
      basePath: base,
      onOpenFile: MarkdownLinkScope.maybeOf(context)?.onOpenFile,
    );
  }

  /// Finds the full-document native input client retained by this editor.
  CodeEditorInputState get _editableState => _editorKey.currentState!.input;

  /// Inserts exactly one shortcut token and lets Flutter update IME and history.
  void _insertSymbol(String symbol) {
    final value = _controller.value;
    final selection = value.selection;
    assert(selection.isValid);
    final next = TextEditingValue(
      text: value.text.replaceRange(selection.start, selection.end, symbol),
      selection: TextSelection.collapsed(
        offset: selection.start + symbol.length,
      ),
    );
    final editable = _editableState;
    editable.userUpdateTextEditingValue(next, SelectionChangedCause.keyboard);
    editable.requestKeyboard();
  }

  /// Keeps editor focus while invoking its native undo history.
  void _undo() {
    _undoController.undo();
    _editableState.requestKeyboard();
  }

  /// Keeps editor focus while replaying its native redo history.
  void _redo() {
    _undoController.redo();
    _editableState.requestKeyboard();
  }

  /// Fixes toolbar geometry without padded tap targets or inherited density.
  Widget _buildToolbarButton({
    required String tooltip,
    required VoidCallback? onPressed,
    required IconData icon,
  }) => IconButton(
    tooltip: tooltip,
    onPressed: onPressed,
    icon: Icon(icon),
    iconSize: 24,
    padding: EdgeInsets.zero,
    constraints: const BoxConstraints.tightFor(width: 40, height: 40),
    visualDensity: VisualDensity.standard,
    style: IconButton.styleFrom(
      fixedSize: const Size.square(40),
      padding: EdgeInsets.zero,
      tapTargetSize: MaterialTapTargetSize.shrinkWrap,
      visualDensity: VisualDensity.standard,
    ),
  );

  /// Uses the top row for editing actions instead of gesture zoom controls.
  Widget _buildControls(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final isMarkdown =
        widget.tab.previewKind == WorkspaceFilePreviewKind.markdown;
    return TextFieldTapRegion(
      groupId: _editorTapGroup,
      child: ValueListenableBuilder<UndoHistoryValue>(
        valueListenable: _undoController,
        builder: (context, history, child) => Row(
          children: <Widget>[
            _buildToolbarButton(
              tooltip: l10n.workspaceUndo,
              onPressed: !_showMarkdown && history.canUndo ? _undo : null,
              icon: Icons.undo,
            ),
            _buildToolbarButton(
              tooltip: l10n.workspaceRedo,
              onPressed: !_showMarkdown && history.canRedo ? _redo : null,
              icon: Icons.redo,
            ),
            const Spacer(),
            if (isMarkdown)
              _buildToolbarButton(
                tooltip: _showMarkdown ? l10n.edit : l10n.markdownPreview,
                onPressed: _toggleMarkdown,
                icon: _showMarkdown
                    ? Icons.edit_outlined
                    : Icons.visibility_outlined,
              ),
            if (_document.isSaving)
              const SizedBox.square(
                dimension: 40,
                child: Center(
                  child: SizedBox.square(
                    dimension: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  ),
                ),
              )
            else
              _buildToolbarButton(
                tooltip: l10n.save,
                onPressed: _document.isDirty ? _save : null,
                icon: _document.isDirty ? Icons.save : Icons.save_outlined,
              ),
          ],
        ),
      ),
    );
  }

  /// Provides a compact scrollable symbol row without stealing input focus.
  Widget _buildQuickInput(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    const symbols = <String>[
      '{',
      '}',
      '[',
      ']',
      '(',
      ')',
      '<',
      '>',
      '"',
      "'",
      '`',
      ';',
      ':',
      ',',
      '.',
      '=',
      '/',
      r'\',
      '_',
      '-',
      '+',
      '*',
      '&',
      '|',
      '!',
      '?',
      '#',
      r'$',
    ];
    return TextFieldTapRegion(
      groupId: _editorTapGroup,
      child: DecoratedBox(
        decoration: BoxDecoration(
          border: Border(
            top: BorderSide(
              color: Theme.of(context).colorScheme.outlineVariant,
            ),
          ),
        ),
        child: SingleChildScrollView(
          key: const ValueKey('workspace-quick-input'),
          scrollDirection: Axis.horizontal,
          child: Row(
            children: <Widget>[
              for (final symbol in symbols)
                Tooltip(
                  message: l10n.workspaceInsertSymbol(symbol),
                  child: TextButton(
                    key: ValueKey('workspace-insert-$symbol'),
                    style: TextButton.styleFrom(
                      minimumSize: Size.zero,
                      fixedSize: const Size(36, 32),
                      padding: EdgeInsets.zero,
                      tapTargetSize: MaterialTapTargetSize.shrinkWrap,
                      visualDensity: VisualDensity.standard,
                      textStyle: TextStyle(
                        fontFamily: operitCodeFontFamilyOf(context),
                        fontFamilyFallback:
                            operitCodeFontFamilyFallbackOf(context),
                        fontSize: 18,
                        height: 1,
                      ),
                    ),
                    onPressed: () => _insertSymbol(symbol),
                    child: Text(symbol),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }

  /// Builds an editable, scrollable document with touch and keyboard zoom.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final style = theme.textTheme.bodyMedium!.copyWith(
      color: theme.colorScheme.onSurface,
      fontFamily: operitCodeFontFamilyOf(context),
      fontFamilyFallback: operitCodeFontFamilyFallbackOf(context),
      fontSize: 14,
      height: 1.45,
    );

    return CallbackShortcuts(
      bindings: <ShortcutActivator, VoidCallback>{
        const SingleActivator(LogicalKeyboardKey.keyS, control: true): _save,
        const SingleActivator(LogicalKeyboardKey.keyS, meta: true): _save,
        const SingleActivator(LogicalKeyboardKey.equal, control: true): () =>
            _zoomTo(_document.scale + 0.1),
        const SingleActivator(LogicalKeyboardKey.minus, control: true): () =>
            _zoomTo(_document.scale - 0.1),
        const SingleActivator(LogicalKeyboardKey.digit0, control: true): () =>
            _zoomTo(1),
      },
      child: OperitGlassSurface(
        color: theme.colorScheme.surface,
        layer: OperitGlassSurfaceLayer.panel,
        transparentAlpha: 0.025,
        child: Column(
          children: <Widget>[
            RepaintBoundary(
              child: ListenableBuilder(
                listenable: _chromeChanges,
                builder: (context, child) => _buildControls(context),
              ),
            ),
            ListenableBuilder(
              listenable: _chromeChanges,
              builder: (context, child) {
                final error = _document.saveError;
                return error == null
                    ? const SizedBox.shrink()
                    : Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 12,
                          vertical: 4,
                        ),
                        child: Text(
                          error.toString(),
                          style: TextStyle(color: theme.colorScheme.error),
                        ),
                      );
              },
            ),
            Expanded(
              child: RepaintBoundary(
                child: ListenableBuilder(
                  listenable: _viewportChanges,
                  builder: (context, child) => IndexedStack(
                    index: _showMarkdown ? 1 : 0,
                    children: [
                      RetainedPage(
                        active: !_showMarkdown,
                        child: CodeEditor(
                          key: _editorKey,
                          identity: _document,
                          groupId: _editorTapGroup,
                          controller: _controller,
                          undoController: _undoController,
                          focusNode: _editorFocus,
                          scrollController: _editorScroll,
                          onChanged: _document.updateText,
                          style: style,
                          language: _syntaxLanguage,
                          active: !_showMarkdown,
                          scale: _document.scale,
                          onScaleChanged: _document.updateScale,
                        ),
                      ),
                      if (_showMarkdown)
                        // Let viewport drags win before mouse text selection.
                        SelectionArea(
                          child: CodeEditor(
                            key: _viewportKey,
                            identity: _document,
                            scrollController: _previewScroll,
                            scale: _document.scale,
                            onScaleChanged: _document.updateScale,
                            readOnly: true,
                            wrapToViewport: false,
                            contentBuilder: (context) => StreamMarkdownRenderer(
                              selectionRoot: false,
                              scrollController: _previewScroll,
                              scrollPhysics: CodeEditorViewportScope.maybeOf(
                                context,
                              )?.physics,
                              scrollPadding: const EdgeInsets.all(12),
                              content: _document.text,
                              isStreaming: false,
                              textColor: theme.colorScheme.onSurface,
                              backgroundColor: Colors.transparent,
                              onLinkClick: _openMarkdownLink,
                              splitMarkdownContent: widget.splitMarkdownContent,
                            ),
                          ),
                        )
                      else
                        const SizedBox.shrink(),
                    ],
                  ),
                ),
              ),
            ),
            if (!_showMarkdown)
              RepaintBoundary(child: _buildQuickInput(context)),
          ],
        ),
      ),
    );
  }
}

/// Displays extracted text from read-only document formats.
class WorkspaceTextBody extends StatelessWidget {
  /// Creates a selectable document text view.
  const WorkspaceTextBody({
    super.key,
    required this.text,
    required this.monospace,
  });

  final String text;
  final bool monospace;

  /// Builds the read-only text surface for extracted document contents.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return OperitGlassSurface(
      color: theme.colorScheme.surface.withValues(alpha: 0.42),
      layer: OperitGlassSurfaceLayer.panel,
      transparentAlpha: 0.025,
      child: SingleChildScrollView(
        padding: const EdgeInsets.all(12),
        child: SelectableText(
          text,
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurface,
            fontFamily: monospace ? operitCodeFontFamilyOf(context) : null,
            fontFamilyFallback: monospace
                ? operitCodeFontFamilyFallbackOf(context)
                : null,
            height: 1.45,
          ),
        ),
      ),
    );
  }
}
