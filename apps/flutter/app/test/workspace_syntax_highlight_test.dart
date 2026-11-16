import 'dart:math';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/code_editor/CodeTextLayout.dart';
import 'package:operit2/ui/common/code_editor/CodeEditorInput.dart';
import 'package:operit2/ui/common/code_editor/CodeEditorScrollController.dart';
import 'package:operit2/ui/common/code_editor/syntax/CodeSyntaxLanguage.dart';
import 'package:operit2/ui/common/code_editor/syntax/CodeSyntaxLexer.dart';
import 'package:operit2/ui/common/code_editor/syntax/CodeSyntaxPalette.dart';
import 'package:operit2/ui/common/code_editor/syntax/CodeSyntaxToken.dart';

const _samples = <CodeSyntaxLanguage, String>{
  CodeSyntaxLanguage.dart: 'final String text = "中文 👩‍💻"; // comment',
  CodeSyntaxLanguage.rust:
      'pub fn main() { let value: u32 = 42; } // comment',
  CodeSyntaxLanguage.javascript: 'const value = `hello`; // comment',
  CodeSyntaxLanguage.typescript:
      'interface Item { name: string; } // comment',
  CodeSyntaxLanguage.python: 'def greet(): return "hello" # comment',
  CodeSyntaxLanguage.java:
      'public class Main { int value = 42; } // comment',
  CodeSyntaxLanguage.kotlin:
      'fun main() { val name = "hello" } // comment',
  CodeSyntaxLanguage.swift:
      'func greet() { let value: Int = 42 } // comment',
  CodeSyntaxLanguage.c: 'const int value = 42; /* comment */',
  CodeSyntaxLanguage.cpp:
      'class Main { public: int value = 42; }; // comment',
  CodeSyntaxLanguage.csharp:
      'public class Main { string value = "hello"; }',
  CodeSyntaxLanguage.go: 'func main() { var value = 42 } // comment',
  CodeSyntaxLanguage.php:
      r'<?php function greet() { return $value; } // comment',
  CodeSyntaxLanguage.ruby: 'def greet; return "hello"; end # comment',
  CodeSyntaxLanguage.lua: 'local value = "hello" -- comment',
  CodeSyntaxLanguage.shell: r'if true; then echo "$HOME"; fi # comment',
  CodeSyntaxLanguage.powershell:
      r'function Get-Value { return $value } # comment',
  CodeSyntaxLanguage.sql:
      "SELECT id FROM users WHERE name = 'hello'; -- comment",
  CodeSyntaxLanguage.json:
      '{"name": "hello", "enabled": true, "count": 42}',
  CodeSyntaxLanguage.jsonc: '{"name": "hello", "count": 42} // comment',
  CodeSyntaxLanguage.yaml: 'name: "hello" # comment',
  CodeSyntaxLanguage.toml: 'name = "hello" # comment',
  CodeSyntaxLanguage.ini: 'name = "hello" ; comment',
  CodeSyntaxLanguage.html:
      '<div class="hello">world</div> <!-- comment -->',
  CodeSyntaxLanguage.xml: '<item name="hello" /> <!-- comment -->',
  CodeSyntaxLanguage.css:
      '.item { color: #ffffff; width: 12px; } /* comment */',
  CodeSyntaxLanguage.scss:
      r'$size: 12px; .item { width: $size; } // comment',
  CodeSyntaxLanguage.less:
      '@size: 12px; .item { width: @size; } // comment',
  CodeSyntaxLanguage.markdown:
      '# Heading with `code` and [link](https://example.test)',
  CodeSyntaxLanguage.dockerfile: 'FROM alpine:3.20 # comment',
  CodeSyntaxLanguage.makefile: 'include config.mk # comment',
  CodeSyntaxLanguage.cmake: 'project(example) # comment',
  CodeSyntaxLanguage.diff: '+ inserted line',
};

/// Verifies language grammars, incremental states, color-only layout, and input.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test(
    'filename and extension registry selects explicit host-neutral languages',
    () {
      const paths = <String, CodeSyntaxLanguage>{
        r'C:\src\main.DART': CodeSyntaxLanguage.dart,
        '/a.py.dir/main.rs': CodeSyntaxLanguage.rust,
        '/src/ui.tsx': CodeSyntaxLanguage.typescript,
        '/src/ui.jsx': CodeSyntaxLanguage.javascript,
        'Dockerfile': CodeSyntaxLanguage.dockerfile,
        'Dockerfile.release': CodeSyntaxLanguage.dockerfile,
        'Containerfile': CodeSyntaxLanguage.dockerfile,
        'Makefile': CodeSyntaxLanguage.makefile,
        'CMakeLists.txt': CodeSyntaxLanguage.cmake,
        '.bashrc': CodeSyntaxLanguage.shell,
        '.env.local': CodeSyntaxLanguage.ini,
        'Gemfile': CodeSyntaxLanguage.ruby,
        'app.yaml': CodeSyntaxLanguage.yaml,
        'app.jsonc': CodeSyntaxLanguage.jsonc,
        'image.svg': CodeSyntaxLanguage.xml,
        'component.vue': CodeSyntaxLanguage.html,
        'plain.txt': CodeSyntaxLanguage.plainText,
        'config.py/README': CodeSyntaxLanguage.plainText,
      };
      for (final entry in paths.entries) {
        expect(
          CodeSyntaxLanguage.forPath(entry.key),
          entry.value,
          reason: entry.key,
        );
      }
      expect(_samples.length, CodeSyntaxLanguage.values.length - 1);
    },
  );

  for (final entry in _samples.entries) {
    test(
      '${entry.key.name} produces ordered source-preserving lexical tokens',
      () {
        final lexer = CodeSyntaxLexer(entry.key);
        final result = lexer.scan(entry.value, CodeSyntaxState.empty);
        final indexed = lexer.scan(
          entry.value,
          CodeSyntaxState.empty,
          tokens: false,
        );
        expect(indexed.state, result.state);
        expect(indexed.tokens, isEmpty);
        expect(result.tokens, isNotEmpty);
        _verifyIntervals(entry.value, result.tokens);
        final layout = CodeTextLayout(entry.value, language: entry.key);
        addTearDown(layout.dispose);
        _configure(layout);
        expect(layout.paragraph(0).text!.toPlainText(), entry.value);
      },
    );
  }

  test('keywords and comments inside strings are never separately colored', () {
    final layout = CodeTextLayout(
      'const value = "return /* not a comment */"; // return 42',
      language: CodeSyntaxLanguage.javascript,
    );
    addTearDown(layout.dispose);
    expect(_kindAt(layout, 0, 'const'), CodeSyntaxTokenKind.keyword);
    expect(_kindAt(layout, 0, 'return'), CodeSyntaxTokenKind.string);
    expect(_kindAt(layout, 0, '/*'), CodeSyntaxTokenKind.string);
    expect(_kindAt(layout, 0, '//'), CodeSyntaxTokenKind.comment);
    expect(_kindAt(layout, 0, '42'), CodeSyntaxTokenKind.comment);
  });

  test(
    'raw and doubled quotes obey their language-specific closing grammar',
    () {
      final samples = <CodeSyntaxLanguage, String>{
        CodeSyntaxLanguage.python: r'value = r"\""; return 42',
        CodeSyntaxLanguage.dart: r'final value = r"\"; return 42;',
        CodeSyntaxLanguage.csharp:
            r'var value = @"first ""quoted"" last"; return 42;',
        CodeSyntaxLanguage.sql: "SELECT 'can''t'; SELECT 42;",
      };
      for (final entry in samples.entries) {
        final layout = CodeTextLayout(entry.value, language: entry.key);
        addTearDown(layout.dispose);
        expect(_kindAt(layout, 0, '42'), CodeSyntaxTokenKind.number);
        expect(layout.lines[0].syntaxEnd, CodeSyntaxState.empty);
      }
    },
  );

  test('operators adjacent to region openers cannot swallow lexer state', () {
    final code = CodeTextLayout(
      'let value = 1+/* first\nlast */2;',
      language: CodeSyntaxLanguage.javascript,
    );
    addTearDown(code.dispose);
    expect(_kindAt(code, 0, 'first'), CodeSyntaxTokenKind.comment);
    expect(_kindAt(code, 1, 'last'), CodeSyntaxTokenKind.comment);
    expect(_kindAt(code, 1, '2'), CodeSyntaxTokenKind.number);
    final yaml = CodeTextLayout(
      'key:|\n  literal\nnext: 42',
      language: CodeSyntaxLanguage.yaml,
    );
    addTearDown(yaml.dispose);
    expect(_kindAt(yaml, 1, 'literal'), CodeSyntaxTokenKind.string);
    expect(_kindAt(yaml, 2, '42'), CodeSyntaxTokenKind.number);
  });

  test('JSON keys, values, literals and Unicode source keep exact offsets', () {
    const text = '{"中文👩‍💻": "e\u0301", "flag": true, "count": 42}';
    final layout = CodeTextLayout(
      text,
      language: CodeSyntaxLanguage.json,
    );
    addTearDown(layout.dispose);
    expect(_kindAt(layout, 0, '中文'), CodeSyntaxTokenKind.property);
    expect(_kindAt(layout, 0, 'e\u0301'), CodeSyntaxTokenKind.string);
    expect(_kindAt(layout, 0, 'true'), CodeSyntaxTokenKind.literal);
    expect(_kindAt(layout, 0, '42'), CodeSyntaxTokenKind.number);
    _configure(layout);
    expect(layout.paragraph(0).text!.toPlainText(), text);
  });

  test(
    'Rust raw strings, lifetimes and nested comments retain multiline state',
    () {
      final layout = CodeTextLayout(
        '/* outer\n/* inner */ still outer\n*/ let text = r##"first\n"# is not the end\nlast"##;\nfn borrow<\'a>(value: &\'a str) {}',
        language: CodeSyntaxLanguage.rust,
      );
      addTearDown(layout.dispose);
      expect(
        _kindAt(layout, 1, 'still outer'),
        CodeSyntaxTokenKind.comment,
      );
      expect(_kindAt(layout, 2, 'let'), CodeSyntaxTokenKind.keyword);
      expect(_kindAt(layout, 3, 'not'), CodeSyntaxTokenKind.string);
      expect(_kindAt(layout, 4, 'last'), CodeSyntaxTokenKind.string);
      expect(_kindAt(layout, 5, 'fn'), CodeSyntaxTokenKind.keyword);
      expect(_kindAt(layout, 5, "'a"), CodeSyntaxTokenKind.meta);
      expect(layout.lines.last.syntaxEnd, CodeSyntaxState.empty);
    },
  );

  test(
    'Python and TOML triple quoted strings cross logical line boundaries',
    () {
      for (final language in [
        CodeSyntaxLanguage.python,
        CodeSyntaxLanguage.toml,
      ]) {
        final layout = CodeTextLayout(
          'name = """start\nreturn 42 # not a comment\nend"""\nvalue = 7',
          language: language,
        );
        addTearDown(layout.dispose);
        expect(_kindAt(layout, 1, 'return'), CodeSyntaxTokenKind.string);
        expect(_kindAt(layout, 1, '#'), CodeSyntaxTokenKind.string);
        expect(_kindAt(layout, 3, '7'), CodeSyntaxTokenKind.number);
      }
    },
  );

  test('Lua brackets, C++ raw delimiters and SQL dollar quotes close exactly', () {
    final samples = <CodeSyntaxLanguage, String>{
      CodeSyntaxLanguage.lua:
          'local text = [=[first\nreturn ]] still string\nend]=]\nreturn 42',
      CodeSyntaxLanguage.cpp:
          'auto text = R"tag(first\nreturn )" still string\nend)tag";\nreturn 42;',
      CodeSyntaxLanguage.sql:
          'SELECT \$body\$first\nSELECT \$other\$ still string\nend\$body\$;\nSELECT 42;',
    };
    for (final entry in samples.entries) {
      final layout = CodeTextLayout(entry.value, language: entry.key);
      addTearDown(layout.dispose);
      expect(_kindAt(layout, 1, 'still'), CodeSyntaxTokenKind.string);
      expect(_kindAt(layout, 3, '42'), CodeSyntaxTokenKind.number);
      expect(layout.lines.last.syntaxEnd, CodeSyntaxState.empty);
    }
  });

  test('markup attributes span lines without coloring quotes in body text', () {
    final layout = CodeTextLayout(
      '<node\n name="first\nlast">hello "world"</node>\n<!-- first\nlast -->',
      language: CodeSyntaxLanguage.xml,
    );
    addTearDown(layout.dispose);
    expect(_kindAt(layout, 0, '<node'), CodeSyntaxTokenKind.tag);
    expect(_kindAt(layout, 1, 'name'), CodeSyntaxTokenKind.property);
    expect(_kindAt(layout, 2, 'last'), CodeSyntaxTokenKind.string);
    expect(_kindAt(layout, 2, 'world'), isNull);
    expect(_kindAt(layout, 4, 'last'), CodeSyntaxTokenKind.comment);
    expect(layout.lines.last.syntaxEnd, CodeSyntaxState.empty);
  });

  test(
    'YAML block scalars stop at dedent and Markdown fences close by length',
    () {
      final yaml = CodeTextLayout(
        'message: |\n  return true # literal\n\nnext: 42',
        language: CodeSyntaxLanguage.yaml,
      );
      addTearDown(yaml.dispose);
      expect(_kindAt(yaml, 1, '#'), CodeSyntaxTokenKind.string);
      expect(_kindAt(yaml, 3, 'next'), CodeSyntaxTokenKind.property);
      expect(_kindAt(yaml, 3, '42'), CodeSyntaxTokenKind.number);
      final markdown = CodeTextLayout(
        '````dart\n# not a heading\n```\n````\n# Heading',
        language: CodeSyntaxLanguage.markdown,
      );
      addTearDown(markdown.dispose);
      expect(_kindAt(markdown, 1, '#'), CodeSyntaxTokenKind.string);
      expect(_kindAt(markdown, 2, '```'), CodeSyntaxTokenKind.string);
      expect(_kindAt(markdown, 4, '#'), CodeSyntaxTokenKind.heading);
      expect(markdown.lines.last.syntaxEnd, CodeSyntaxState.empty);
    },
  );

  test(
    'local edits retain tokens and paragraphs after lexical state converges',
    () {
      final source = List.generate(2000, (i) => 'let value$i = $i;').join('\n');
      final layout = CodeTextLayout(
        source,
        language: CodeSyntaxLanguage.rust,
      );
      addTearDown(layout.dispose);
      _configure(layout);
      final last = layout.paragraph(1999);
      final lastTokens = layout.syntax.tokens(layout.lines.last);
      final before = layout.syntax.debugStateScans;
      layout.updateText(source.replaceFirst('value2 = 2', 'value2 = 3'));
      expect(layout.syntax.debugStateScans - before, lessThanOrEqualTo(3));
      expect(identical(layout.paragraph(1999), last), isTrue);
      expect(
        identical(layout.syntax.tokens(layout.lines.last), lastTokens),
        isTrue,
      );
      final comments = CodeTextLayout(
        'let a = 1;\nlet b = 2;\n/* existing */\nlet c = 3;',
        language: CodeSyntaxLanguage.rust,
      );
      addTearDown(comments.dispose);
      _configure(comments);
      final middle = comments.paragraph(1);
      final tail = comments.paragraph(3);
      comments.updateText(
        '/* let a = 1;\nlet b = 2;\n/* existing */\n*/ let c = 3;',
      );
      expect(_kindAt(comments, 1, 'let'), CodeSyntaxTokenKind.comment);
      expect(identical(comments.paragraph(1), middle), isFalse);
      expect(_kindAt(comments, 3, 'let'), CodeSyntaxTokenKind.keyword);
      expect(identical(comments.paragraph(3), tail), isFalse);
    },
  );

  test('color-only highlighting preserves wrapping and caret geometry', () {
    const source =
        'final String 中文 = "e\u0301 👩‍💻 text"; // comment ${42}\n${42}';
    final plain = CodeTextLayout(source);
    final colored = CodeTextLayout(
      source,
      language: CodeSyntaxLanguage.dart,
    );
    addTearDown(plain.dispose);
    addTearDown(colored.dispose);
    for (final width in [80.0, 160.0, 320.0]) {
      _configure(plain, width: width);
      _configure(colored, width: width);
      for (var i = 0; i < plain.lines.length; i++) {
        final a = plain.paragraph(i);
        final b = colored.paragraph(i);
        expect(b.height, closeTo(a.height, 0.0001));
        for (
          var offset = 0;
          offset <= plain.lines[i].content.length;
          offset++
        ) {
          final pos = TextPosition(offset: offset);
          expect(
            b.getOffsetForCaret(pos, Rect.zero),
            a.getOffsetForCaret(pos, Rect.zero),
          );
        }
      }
    }
    final tokenScans = colored.syntax.debugTokenScans;
    colored.configureSyntax(
      CodeSyntaxLanguage.dart,
      const CodeSyntaxPalette(Brightness.dark),
    );
    expect(colored.paragraph(0).text!.toPlainText(), plain.lines[0].content);
    expect(colored.syntax.debugTokenScans, tokenScans);
    expect(colored.paragraph(0).height, plain.paragraph(0).height);
  });

  test('ten thousand source lines cache only requested colored paragraphs', () {
    final layout = CodeTextLayout(
      List.generate(
        10000,
        (i) => 'let value$i = "中文 👩‍💻"; // comment',
      ).join('\n'),
      language: CodeSyntaxLanguage.rust,
    );
    addTearDown(layout.dispose);
    _configure(layout);
    expect(layout.syntax.debugStateScans, 10000);
    expect(layout.syntax.debugTokenScans, 0);
    for (var page = 0; page < 150; page++) {
      final first = page * 60;
      final retained = <CodeTextLine>{};
      for (var i = first; i < first + 40; i++) {
        layout.paragraph(i);
        retained.add(layout.lines[i]);
      }
      layout.trimCache(retained);
      expect(layout.syntax.cachedLines, lessThanOrEqualTo(128));
      expect(layout.cachedParagraphs, lessThanOrEqualTo(128));
    }
    final scans = layout.syntax.debugTokenScans;
    for (final width in [100.0, 200.0, 300.0]) {
      _configure(layout, width: width);
      layout.paragraph(8970);
    }
    expect(layout.syntax.debugTokenScans, scans);
  });

  for (final language in [
    CodeSyntaxLanguage.rust,
    CodeSyntaxLanguage.python,
    CodeSyntaxLanguage.yaml,
    CodeSyntaxLanguage.xml,
    CodeSyntaxLanguage.javascript,
    CodeSyntaxLanguage.shell,
  ]) {
    test(
      '${language.name} incremental states match full indexing during unfinished edits',
      () {
        var text =
            '${_samples[language]}\n/* first\nlast */\n"""first\nlast"""\n# comment';
        final layout = CodeTextLayout(text, language: language);
        addTearDown(layout.dispose);
        final random = Random(194 + language.index);
        const insertions = [
          'a',
          '\n',
          '\r\n',
          '\r',
          '',
          '/*',
          '*/',
          '"',
          "'",
          'r#"',
          '\\',
          '#',
          '\${value}',
          '👩‍💻',
          '<node>',
          '```',
          '  key: |',
        ];
        for (var edit = 0; edit < 120; edit++) {
          final start = random.nextInt(text.length + 1);
          final end = start + random.nextInt(text.length - start + 1);
          text = text.replaceRange(
            start,
            end,
            insertions[random.nextInt(insertions.length)],
          );
          layout.updateText(text);
          final fresh = CodeTextLayout(text, language: language);
          for (var i = 0; i < layout.lines.length; i++) {
            expect(
              layout.lines[i].syntaxStart,
              fresh.lines[i].syntaxStart,
              reason: 'edit=$edit line=$i source=$text',
            );
            expect(
              layout.lines[i].syntaxEnd,
              fresh.lines[i].syntaxEnd,
              reason: 'edit=$edit line=$i source=$text',
            );
            final lexer = CodeSyntaxLexer(language);
            final indexed = lexer.scan(
              layout.lines[i].content,
              layout.lines[i].syntaxStart!,
              tokens: false,
            );
            final colored = lexer.scan(
              layout.lines[i].content,
              layout.lines[i].syntaxStart!,
            );
            expect(
              colored.state,
              indexed.state,
              reason: 'edit=$edit line=$i source=$text',
            );
            final tokens = layout.syntax.tokens(layout.lines[i]);
            _verifyIntervals(layout.lines[i].content, tokens);
            expect(
              tokens.map((t) => (t.start, t.end, t.kind)).toList(),
              fresh.syntax
                  .tokens(fresh.lines[i])
                  .map((t) => (t.start, t.end, t.kind))
                  .toList(),
            );
          }
          fresh.dispose();
        }
      },
    );
  }

  testWidgets(
    'colored ten-thousand-line editor keeps viewport-only work bounded',
    (tester) async {
      await tester.pumpWidget(
        _SyntaxEditor(
          text: List.generate(
            10000,
            (i) => 'let value$i = "中文 👩‍💻"; // comment',
          ).join('\n'),
          language: CodeSyntaxLanguage.rust,
        ),
      );
      await tester.pumpAndSettle();
      final state = tester.state<CodeEditorInputState>(
        find.byType(CodeEditorInput),
      );
      final render = state.renderEditable;
      final syntax = render.textLayout.syntax;
      expect(syntax.debugStateScans, 10000);
      expect(syntax.debugTokenScans, lessThan(100));
      expect(render.debugParagraphLayouts, lessThan(100));
      expect(render.debugPaintedParagraphs, lessThan(50));
      final scans = syntax.debugTokenScans;
      for (var frame = 0; frame < 20; frame++) {
        render.markNeedsPaint();
        await tester.pump();
      }
      expect(syntax.debugTokenScans, scans);
      state.widget.scrollController.jumpTo(50000);
      await tester.pump();
      expect(syntax.debugTokenScans - scans, lessThan(100));
      final beforeSelection = syntax.debugTokenScans;
      state.widget.controller.selection = TextSelection(
        baseOffset: 0,
        extentOffset: state.widget.controller.text.length,
      );
      await tester.pump();
      expect(syntax.debugTokenScans, beforeSelection);
      expect(syntax.cachedLines, lessThanOrEqualTo(128));
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'colored virtual editor retains native IME state and theme updates',
    (tester) async {
      const initial = 'final value = "中文";\n// comment';
      await tester.pumpWidget(const _SyntaxEditor(text: initial));
      await tester.pumpAndSettle();
      final state = tester.state<CodeEditorInputState>(
        find.byType(CodeEditorInput),
      );
      final layout = state.renderEditable.textLayout;
      expect(layout.syntax.language, CodeSyntaxLanguage.dart);
      final painter = layout.paragraph(0);
      state.requestKeyboard();
      await tester.pump();
      const edited = 'final value = "中文👩‍💻";\n// comment';
      const value = TextEditingValue(
        text: edited,
        selection: TextSelection.collapsed(offset: 18),
        composing: TextRange(start: 15, end: 18),
      );
      tester.testTextInput.updateEditingValue(value);
      await tester.pump();
      expect(state.textEditingValue, value);
      expect(layout.paragraph(0).text!.toPlainText(), edited.split('\n')[0]);
      await tester.pumpWidget(
        const _SyntaxEditor(text: initial, brightness: Brightness.dark),
      );
      await tester.pump();
      expect(state.textEditingValue, value);
      expect(identical(layout.paragraph(0), painter), isFalse);
      await tester.pumpWidget(
        const _SyntaxEditor(
          text: initial,
          language: CodeSyntaxLanguage.plainText,
          brightness: Brightness.dark,
        ),
      );
      await tester.pump();
      expect(layout.syntax.language, CodeSyntaxLanguage.plainText);
      expect(state.textEditingValue, value);
      expect(tester.takeException(), isNull);
    },
  );
}

/// Configures deterministic paragraph metrics independently from token colors.
void _configure(CodeTextLayout layout, {double width = 160}) =>
    layout.configure(
      width,
      const TextStyle(fontSize: 14, height: 1.45),
      TextScaler.noScaling,
      TextDirection.ltr,
    );

/// Ensures all lexical ranges remain ordered, nonempty UTF-16 source intervals.
void _verifyIntervals(String text, List<CodeSyntaxToken> tokens) {
  var end = 0;
  for (final token in tokens) {
    expect(token.start, greaterThanOrEqualTo(end));
    expect(token.end, greaterThan(token.start));
    expect(token.end, lessThanOrEqualTo(text.length));
    end = token.end;
  }
}

/// Reads the lexical category covering an exact source substring in a line.
CodeSyntaxTokenKind? _kindAt(
  CodeTextLayout layout,
  int line,
  String text,
) {
  final source = layout.lines[line];
  final offset = source.content.indexOf(text);
  expect(offset, greaterThanOrEqualTo(0));
  for (final token in layout.syntax.tokens(source)) {
    if (token.start <= offset && token.end > offset) return token.kind;
  }
  return null;
}

/// Keeps document and history ownership stable while grammar and theme change.
class _SyntaxEditor extends StatefulWidget {
  /// Creates a common-platform editor harness without host-specific branches.
  const _SyntaxEditor({
    required this.text,
    this.language = CodeSyntaxLanguage.dart,
    this.brightness = Brightness.light,
  });

  final String text;
  final CodeSyntaxLanguage language;
  final Brightness brightness;

  /// Creates persistent native editing resources for the syntax regression test.
  @override
  State<_SyntaxEditor> createState() => _SyntaxEditorState();
}

class _SyntaxEditorState extends State<_SyntaxEditor> {
  late final TextEditingController controller;
  final focus = FocusNode();
  final history = UndoHistoryController();
  final scroll = CodeEditorScrollController();

  /// Installs the full document once, independently from render configuration.
  @override
  void initState() {
    super.initState();
    controller = TextEditingController(text: widget.text);
  }

  /// Builds the same virtual editor used by workspace file tabs.
  @override
  Widget build(BuildContext context) => MaterialApp(
    theme: ThemeData(brightness: widget.brightness),
    home: Scaffold(
      body: CodeEditorInput(
        controller: controller,
        focusNode: focus,
        undoController: history,
        scrollController: scroll,
        groupId: this,
        style: const TextStyle(color: Colors.black, fontSize: 14, height: 1.45),
        language: widget.language,
        onChanged: _changed,
        scrollPhysics: null,
        active: true,
      ),
    ),
  );

  /// Accepts native edits without replacing the document controller.
  void _changed(String text) {}

  /// Releases all test resources after the native connection is detached.
  @override
  void dispose() {
    controller.dispose();
    focus.dispose();
    history.dispose();
    scroll.dispose();
    super.dispose();
  }
}
