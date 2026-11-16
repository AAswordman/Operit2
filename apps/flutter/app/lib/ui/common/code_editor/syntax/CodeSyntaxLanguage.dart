// ignore_for_file: file_names

import 'package:path/path.dart' as path;

import 'package:operit2/ui/common/code_editor/syntax/CodeSyntaxToken.dart';

/// Lists explicit file grammars instead of guessing languages from source text.
enum CodeSyntaxLanguage {
  plainText,
  dart,
  rust,
  javascript,
  typescript,
  python,
  java,
  kotlin,
  swift,
  c,
  cpp,
  csharp,
  go,
  php,
  ruby,
  lua,
  shell,
  powershell,
  sql,
  json,
  jsonc,
  yaml,
  toml,
  ini,
  html,
  xml,
  css,
  scss,
  less,
  markdown,
  dockerfile,
  makefile,
  cmake,
  diff;

  /// Resolves exact filenames and extensions using host-independent path syntax.
  static CodeSyntaxLanguage forPath(String filePath) {
    final name = path.posix
        .basename(filePath.replaceAll('\\', '/'))
        .toLowerCase();
    if (_filenames.containsKey(name)) return _filenames[name]!;
    if (RegExp(r'^dockerfile\.').hasMatch(name)) return dockerfile;
    if (RegExp(r'^\.env\.').hasMatch(name)) return ini;
    return switch (path.posix.extension(name)) {
      '.dart' => dart,
      '.rs' => rust,
      '.js' || '.mjs' || '.cjs' || '.jsx' => javascript,
      '.ts' || '.mts' || '.cts' || '.tsx' => typescript,
      '.py' || '.pyw' || '.pyi' => python,
      '.java' => java,
      '.kt' || '.kts' => kotlin,
      '.swift' => swift,
      '.c' || '.h' => c,
      '.cc' || '.cpp' || '.cxx' || '.hpp' || '.hh' || '.hxx' => cpp,
      '.cs' || '.csx' => csharp,
      '.go' => go,
      '.php' || '.phtml' => php,
      '.rb' || '.rake' || '.gemspec' => ruby,
      '.lua' => lua,
      '.sh' || '.bash' || '.zsh' || '.fish' => shell,
      '.ps1' || '.psm1' || '.psd1' => powershell,
      '.sql' => sql,
      '.json' => json,
      '.jsonc' || '.json5' => jsonc,
      '.yaml' || '.yml' => yaml,
      '.toml' => toml,
      '.ini' || '.cfg' || '.conf' || '.properties' || '.env' => ini,
      '.html' || '.htm' || '.vue' || '.svelte' => html,
      '.xml' || '.svg' || '.xhtml' || '.xsd' || '.plist' || '.xaml' => xml,
      '.css' => css,
      '.scss' || '.sass' => scss,
      '.less' => less,
      '.md' || '.markdown' || '.mdown' => markdown,
      '.mk' || '.mak' => makefile,
      '.cmake' => cmake,
      '.diff' || '.patch' => diff,
      _ => plainText,
    };
  }

  static const _filenames = <String, CodeSyntaxLanguage>{
    'dockerfile': dockerfile,
    'containerfile': dockerfile,
    'makefile': makefile,
    'gnumakefile': makefile,
    'cmakelists.txt': cmake,
    'gemfile': ruby,
    'rakefile': ruby,
    '.bashrc': shell,
    '.bash_profile': shell,
    '.profile': shell,
    '.zshrc': shell,
    '.zprofile': shell,
    '.env': ini,
  };
}

/// Selects a lexical family without introducing any operating-system branches.
enum CodeSyntaxFamily { code, data, markup, stylesheet, markdown, diff, plain }

/// Describes a cross-line lexical region and its exact delimiter grammar.
class CodeSyntaxDelimiter {
  /// Defines a comment, string, or raw region opener for a language.
  const CodeSyntaxDelimiter(
    this.pattern,
    this.end,
    this.kind, {
    this.multiline = true,
    this.escape = false,
    this.doubledEnd = false,
    this.nested = false,
    this.dynamicEnd = '',
    this.insideTagOnly = false,
    this.escapeCharacter = '\\',
  });

  final String pattern;
  final String end;
  final CodeSyntaxTokenKind kind;
  final bool multiline;
  final bool escape;
  final bool doubledEnd;
  final bool nested;
  final String dynamicEnd;
  final bool insideTagOnly;
  final String escapeCharacter;
}

/// Stores immutable grammar data shared by all documents of one language.
class CodeSyntaxDefinition {
  /// Precomputes keyword sets once rather than during scrolling or zooming.
  CodeSyntaxDefinition({
    this.family = CodeSyntaxFamily.code,
    String keywords = '',
    String types = '',
    String literals = 'true false null',
    this.lineComment = '',
    this.delimiters = const [],
    this.caseSensitive = true,
    this.extraPattern = '',
    this.variables = false,
    this.assignmentProperties = false,
  }) : keywords = _words(keywords, caseSensitive),
       types = _words(types, caseSensitive),
       literals = _words(literals, caseSensitive);

  final CodeSyntaxFamily family;
  final Set<String> keywords;
  final Set<String> types;
  final Set<String> literals;
  final String lineComment;
  final List<CodeSyntaxDelimiter> delimiters;
  final bool caseSensitive;
  final String extraPattern;
  final bool variables;
  final bool assignmentProperties;

  /// Builds exact lexical word tables with language-defined case sensitivity.
  static Set<String> _words(String words, bool caseSensitive) =>
      (caseSensitive ? words : words.toLowerCase())
          .split(' ')
          .where((s) => s.isNotEmpty)
          .toSet();
}

const _comment = CodeSyntaxTokenKind.comment;
const _string = CodeSyntaxTokenKind.string;
const _block = CodeSyntaxDelimiter(r'/\*', '*/', _comment);
const _nestedBlock = CodeSyntaxDelimiter(r'/\*', '*/', _comment, nested: true);
const _double = CodeSyntaxDelimiter(
  '"',
  '"',
  _string,
  multiline: false,
  escape: true,
);
const _single = CodeSyntaxDelimiter(
  "'",
  "'",
  _string,
  multiline: false,
  escape: true,
);
const _backtick = CodeSyntaxDelimiter('`', '`', _string, escape: true);
const _cTypes =
    'void bool char short int long float double signed unsigned size_t wchar_t';
const _cWords =
    'auto break case const continue default do else enum extern for goto if inline register restrict return sizeof static struct switch typedef union volatile while';

/// Returns the registered grammar; plain text is an explicit non-code format.
CodeSyntaxDefinition workspaceSyntaxDefinition(CodeSyntaxLanguage language) =>
    _definitions[language]!;

final _definitions = <CodeSyntaxLanguage, CodeSyntaxDefinition>{
  CodeSyntaxLanguage.plainText: CodeSyntaxDefinition(
    family: CodeSyntaxFamily.plain,
  ),
  CodeSyntaxLanguage.dart: CodeSyntaxDefinition(
    keywords:
        'abstract as assert async await base break case catch class const continue covariant default deferred do else enum export extends extension external factory final finally for get hide if implements import in interface is late library mixin new of on operator part required rethrow return sealed set show static super switch sync this throw try typedef var void when while with yield',
    types:
        'bool double int num String List Map Set Object Future Stream Duration DateTime Never dynamic',
    lineComment: r'//',
    delimiters: [
      _nestedBlock,
      CodeSyntaxDelimiter(
        r'(?:\br)?"""',
        '"""',
        _string,
        escape: true,
        dynamicEnd: 'dart',
      ),
      CodeSyntaxDelimiter(
        "(?:\\br)?'''",
        "'''",
        _string,
        escape: true,
        dynamicEnd: 'dart',
      ),
      CodeSyntaxDelimiter(
        r'(?:\br)?"',
        '"',
        _string,
        multiline: false,
        escape: true,
        dynamicEnd: 'dart',
      ),
      CodeSyntaxDelimiter(
        "(?:\\br)?'",
        "'",
        _string,
        multiline: false,
        escape: true,
        dynamicEnd: 'dart',
      ),
    ],
  ),
  CodeSyntaxLanguage.rust: CodeSyntaxDefinition(
    keywords:
        'as async await break const continue crate dyn else enum extern fn for if impl in let loop macro match mod move mut pub ref return self Self static struct super trait type unsafe use where while yield',
    types:
        'bool char str String i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 Vec Option Result Box Rc Arc Some None Ok Err',
    lineComment: r'//',
    delimiters: [
      _nestedBlock,
      CodeSyntaxDelimiter(r'\b(?:b|c)?r#*"', '', _string, dynamicEnd: 'rust'),
      _double,
    ],
    extraPattern:
        r"(?<character>'(?:\\(?:u\{[0-9a-fA-F]+\}|.)|[^'\\])')|(?<meta>'[A-Za-z_]\w*|#!?)",
  ),
  CodeSyntaxLanguage.javascript: CodeSyntaxDefinition(
    keywords:
        'as async await break case catch class const continue debugger default delete do else export extends finally for from function get if import in instanceof let new of return set static super switch this throw try typeof var void while with yield',
    types:
        'Array BigInt Boolean Date Error Function Map Number Object Promise RegExp Set String Symbol WeakMap WeakSet',
    literals: 'true false null undefined NaN Infinity',
    lineComment: r'//',
    delimiters: [_block, _backtick, _double, _single],
  ),
  CodeSyntaxLanguage.typescript: CodeSyntaxDefinition(
    keywords:
        'abstract any as asserts async await break case catch class const constructor continue debugger declare default delete do else enum export extends finally for from function get if implements import in infer instanceof interface is keyof let namespace never new of out override private protected public readonly require return satisfies set static super switch this throw try type typeof unique unknown var void while with yield',
    types:
        'boolean number string bigint symbol object Array Promise Record Partial Required Readonly Pick Omit Map Set',
    literals: 'true false null undefined NaN Infinity',
    lineComment: r'//',
    delimiters: [_block, _backtick, _double, _single],
    extraPattern: r'(?<meta>@[A-Za-z_]\w*)',
  ),
  CodeSyntaxLanguage.python: CodeSyntaxDefinition(
    keywords:
        'and as assert async await break case class continue def del elif else except finally for from global if import in is lambda match nonlocal not or pass raise return try while with yield',
    types:
        'int float complex bool str bytes list tuple set dict object type range',
    literals: 'True False None NotImplemented Ellipsis',
    lineComment: '#',
    delimiters: [
      CodeSyntaxDelimiter(
        r'(?:\b[rRuUbBfF]{1,2})?"""',
        '"""',
        _string,
        escape: true,
        dynamicEnd: 'python',
      ),
      CodeSyntaxDelimiter(
        "(?:\\b[rRuUbBfF]{1,2})?'''",
        "'''",
        _string,
        escape: true,
        dynamicEnd: 'python',
      ),
      CodeSyntaxDelimiter(
        r'(?:\b[rRuUbBfF]{1,2})?"',
        '"',
        _string,
        multiline: false,
        escape: true,
        dynamicEnd: 'python',
      ),
      CodeSyntaxDelimiter(
        "(?:\\b[rRuUbBfF]{1,2})?'",
        "'",
        _string,
        multiline: false,
        escape: true,
        dynamicEnd: 'python',
      ),
    ],
    extraPattern: r'(?<meta>@[\w.]+)',
  ),
  CodeSyntaxLanguage.java: CodeSyntaxDefinition(
    keywords:
        'abstract assert break case catch class const continue default do else enum exports extends final finally for if implements import instanceof interface module native new non-sealed open opens package permits private protected provides public record requires return sealed static strictfp super switch synchronized this throw throws to transient transitive try uses var volatile while with yield',
    types:
        'boolean byte char double float int long short void String Object Integer Boolean Long Double List Map Set',
    lineComment: r'//',
    delimiters: [
      CodeSyntaxDelimiter('"""', '"""', _string, escape: true),
      _block,
      _double,
      _single,
    ],
    extraPattern: r'(?<meta>@[\w.]+)',
  ),
  CodeSyntaxLanguage.kotlin: CodeSyntaxDefinition(
    keywords:
        'abstract actual annotation as break by catch class companion const constructor continue crossinline data delegate do dynamic else enum expect external final finally for fun get if import in infix init inline inner interface internal is lateinit noinline object open operator out override package private protected public reified return sealed set super suspend tailrec this throw try typealias val var vararg when where while',
    types:
        'Any Unit Nothing Boolean Byte Char Short Int Long Float Double String List Map Set Array',
    lineComment: r'//',
    delimiters: [
      _nestedBlock,
      CodeSyntaxDelimiter('"""', '"""', _string),
      _double,
      _single,
    ],
    extraPattern: r'(?<meta>@[\w.]+)',
  ),
  CodeSyntaxLanguage.swift: CodeSyntaxDefinition(
    keywords:
        'actor any as associatedtype async await borrow break case catch class consume continue convenience copy default defer deinit do else enum extension fallthrough fileprivate final for func get guard if import in indirect infix init inout internal is isolated lazy let macro mutating nonisolated nonmutating open operator optional override package postfix precedencegroup prefix private protocol public repeat required rethrows return self set some static struct subscript super switch throws throw try typealias unowned var weak where while',
    types:
        'Bool Int UInt Float Double String Character Array Dictionary Set Optional Void Any',
    literals: 'true false nil',
    lineComment: r'//',
    delimiters: [
      _nestedBlock,
      CodeSyntaxDelimiter('"""', '"""', _string, escape: true),
      _double,
    ],
    extraPattern: r'(?<meta>@[\w.]+|#[A-Za-z_]\w*)',
  ),
  CodeSyntaxLanguage.c: CodeSyntaxDefinition(
    keywords: _cWords,
    types: _cTypes,
    literals: 'true false NULL',
    lineComment: r'//',
    delimiters: [_block, _double, _single],
    extraPattern: r'(?<meta>^\s*#\s*\w+)',
  ),
  CodeSyntaxLanguage.cpp: CodeSyntaxDefinition(
    keywords:
        '$_cWords alignas alignof and and_eq asm bitand bitor catch class co_await co_return co_yield concept constexpr consteval constinit decltype delete explicit export friend mutable namespace new noexcept not not_eq nullptr operator or or_eq private protected public requires template this thread_local throw try typeid typename using virtual xor xor_eq',
    types: '$_cTypes string vector map set optional unique_ptr shared_ptr',
    literals: 'true false NULL nullptr',
    lineComment: r'//',
    delimiters: [
      _block,
      CodeSyntaxDelimiter(
        r'\b(?:u8|u|U|L)?R"[^\s()\\]{0,16}\(',
        '',
        _string,
        dynamicEnd: 'cpp',
      ),
      _double,
      _single,
    ],
    extraPattern: r'(?<meta>^\s*#\s*\w+)',
  ),
  CodeSyntaxLanguage.csharp: CodeSyntaxDefinition(
    keywords:
        'abstract as async await base break case catch checked class const continue default delegate do else enum event explicit extern finally fixed for foreach goto if implicit in interface internal is lock namespace new operator out override params partial private protected public readonly record ref required return sealed sizeof stackalloc static struct switch this throw try typeof unchecked unsafe using var virtual volatile when where while yield',
    types:
        'bool byte char decimal double dynamic float int long object sbyte short string uint ulong ushort void Task List Dictionary',
    lineComment: r'//',
    delimiters: [
      _block,
      CodeSyntaxDelimiter('@"', '"', _string, doubledEnd: true),
      CodeSyntaxDelimiter('"""', '"""', _string),
      _double,
      _single,
    ],
    extraPattern: r'(?<meta>^\s*#\s*\w+)',
  ),
  CodeSyntaxLanguage.go: CodeSyntaxDefinition(
    keywords:
        'break case chan const continue default defer else fallthrough for func go goto if import interface map package range return select struct switch type var',
    types:
        'bool byte rune string int int8 int16 int32 int64 uint uint8 uint16 uint32 uint64 uintptr float32 float64 complex64 complex128 error',
    literals: 'true false nil iota',
    lineComment: r'//',
    delimiters: [
      _block,
      CodeSyntaxDelimiter('`', '`', _string),
      _double,
      _single,
    ],
  ),
  CodeSyntaxLanguage.php: CodeSyntaxDefinition(
    keywords:
        'abstract and array as break callable case catch class clone const continue declare default die do echo else elseif empty enddeclare endfor endforeach endif endswitch endwhile enum eval exit extends final finally fn for foreach function global goto if implements include include_once instanceof interface isset list match namespace new or print private protected public readonly require require_once return static switch throw trait try unset use var while xor yield',
    types: 'bool float int string object mixed void never iterable',
    caseSensitive: false,
    lineComment: r'//|#',
    delimiters: [_block, _double, _single],
    variables: true,
    extraPattern: r'(?<meta><\?php|\?>)',
  ),
  CodeSyntaxLanguage.ruby: CodeSyntaxDefinition(
    keywords:
        'BEGIN END alias and begin break case class def defined do else elsif end ensure for if in module next not or redo rescue retry return self super then undef unless until when while yield',
    types: 'Array Hash String Symbol Integer Float Object Class Module',
    literals: 'true false nil',
    lineComment: '#',
    delimiters: [
      _double,
      CodeSyntaxDelimiter("'", "'", _string, escape: true),
      _backtick,
    ],
    variables: true,
    extraPattern: r'(?<specialVariable>@@?[A-Za-z_]\w*)',
  ),
  CodeSyntaxLanguage.lua: CodeSyntaxDefinition(
    keywords:
        'and break do else elseif end for function goto if in local not or repeat return then until while',
    literals: 'true false nil',
    lineComment: '--',
    delimiters: [
      CodeSyntaxDelimiter(r'--\[=*\[', '', _comment, dynamicEnd: 'lua'),
      CodeSyntaxDelimiter(r'\[=*\[', '', _string, dynamicEnd: 'lua'),
      _double,
      _single,
    ],
  ),
  CodeSyntaxLanguage.shell: CodeSyntaxDefinition(
    keywords:
        'if then else elif fi for while until do done case esac in function select time coproc export local readonly declare typeset unset source return break continue',
    literals: '',
    lineComment: '#',
    delimiters: [
      CodeSyntaxDelimiter('"', '"', _string, escape: true),
      CodeSyntaxDelimiter("'", "'", _string),
      _backtick,
    ],
    variables: true,
  ),
  CodeSyntaxLanguage.powershell: CodeSyntaxDefinition(
    keywords:
        'begin break catch class clean continue data do dynamicparam else elseif end enum exit filter finally for foreach from function if in param process return switch throw trap try until using var while workflow',
    literals: 'true false null',
    lineComment: '#',
    delimiters: [
      CodeSyntaxDelimiter('<#', '#>', _comment),
      CodeSyntaxDelimiter(
        '"',
        '"',
        _string,
        doubledEnd: true,
        escape: true,
        escapeCharacter: '`',
      ),
      CodeSyntaxDelimiter("'", "'", _string, doubledEnd: true),
    ],
    caseSensitive: false,
    variables: true,
    extraPattern: r'(?<meta>-[A-Za-z][\w-]*)',
  ),
  CodeSyntaxLanguage.sql: CodeSyntaxDefinition(
    keywords:
        'add all alter and any as asc begin between by call case check column commit constraint create cross database default delete desc distinct drop else end except exists explain fetch foreign from full function grant group having if in index inner insert intersect into is join key left like limit materialized not nulls offset on or order outer over partition primary procedure references returning right rollback row rows schema select sequence set table then transaction trigger truncate union unique update use using values view when where window with',
    types:
        'bigint binary bit blob boolean char date datetime decimal double float int integer interval json numeric real serial smallint text time timestamp uuid varchar',
    caseSensitive: false,
    lineComment: '--',
    delimiters: [
      _block,
      CodeSyntaxDelimiter("'", "'", _string, doubledEnd: true),
      CodeSyntaxDelimiter('"', '"', _string, doubledEnd: true),
      CodeSyntaxDelimiter(
        r'(?<![\w$])\$(?:[A-Za-z_]\w*)?\$',
        '',
        _string,
        dynamicEnd: 'sql',
      ),
    ],
  ),
  CodeSyntaxLanguage.json: CodeSyntaxDefinition(
    family: CodeSyntaxFamily.data,
    delimiters: [_double],
  ),
  CodeSyntaxLanguage.jsonc: CodeSyntaxDefinition(
    family: CodeSyntaxFamily.data,
    lineComment: r'//',
    delimiters: [_block, _double, _single],
  ),
  CodeSyntaxLanguage.yaml: CodeSyntaxDefinition(
    family: CodeSyntaxFamily.data,
    lineComment: '#',
    literals: 'true false null yes no on off True False Null TRUE FALSE NULL',
    delimiters: [
      CodeSyntaxDelimiter('"', '"', _string, escape: true),
      CodeSyntaxDelimiter("'", "'", _string, doubledEnd: true),
    ],
    extraPattern:
        r'(?<meta>^\s*(?:---|\.\.\.)\s*$|[&*!][\w.-]+)|(?<property>[A-Za-z_][\w .-]*(?=\s*:))|(?<yamlBlock>[|>](?:[+-]?[1-9]?|[1-9]?[+-]?)\s*(?:#.*)?$)',
  ),
  CodeSyntaxLanguage.toml: CodeSyntaxDefinition(
    family: CodeSyntaxFamily.data,
    lineComment: '#',
    delimiters: [
      CodeSyntaxDelimiter('"""', '"""', _string, escape: true),
      CodeSyntaxDelimiter("'''", "'''", _string),
      _double,
      CodeSyntaxDelimiter("'", "'", _string, multiline: false),
    ],
    assignmentProperties: true,
    extraPattern: r'(?<meta>^\s*\[.*\]\s*$)',
  ),
  CodeSyntaxLanguage.ini: CodeSyntaxDefinition(
    family: CodeSyntaxFamily.data,
    lineComment: r'#|;',
    delimiters: [_double, _single],
    assignmentProperties: true,
    extraPattern: r'(?<meta>^\s*\[.*\]\s*$)',
  ),
  CodeSyntaxLanguage.html: CodeSyntaxDefinition(
    family: CodeSyntaxFamily.markup,
    delimiters: [
      CodeSyntaxDelimiter('<!--', '-->', _comment),
      CodeSyntaxDelimiter(r'<!\[CDATA\[', ']]>', _string),
      CodeSyntaxDelimiter('"', '"', _string, insideTagOnly: true),
      CodeSyntaxDelimiter("'", "'", _string, insideTagOnly: true),
    ],
  ),
  CodeSyntaxLanguage.xml: CodeSyntaxDefinition(
    family: CodeSyntaxFamily.markup,
    delimiters: [
      CodeSyntaxDelimiter('<!--', '-->', _comment),
      CodeSyntaxDelimiter(r'<!\[CDATA\[', ']]>', _string),
      CodeSyntaxDelimiter('"', '"', _string, insideTagOnly: true),
      CodeSyntaxDelimiter("'", "'", _string, insideTagOnly: true),
    ],
  ),
  CodeSyntaxLanguage.css: CodeSyntaxDefinition(
    family: CodeSyntaxFamily.stylesheet,
    keywords: 'important inherit initial unset revert auto none',
    delimiters: [_block, _double, _single],
    extraPattern:
        r'(?<meta>@[\w-]+)|(?<specialNumber>#[0-9a-fA-F]{3,8}\b)|(?<property>[-\w]+(?=\s*:))|(?<tag>[.#][A-Za-z_][\w-]*)',
  ),
  CodeSyntaxLanguage.scss: CodeSyntaxDefinition(
    family: CodeSyntaxFamily.stylesheet,
    keywords:
        'important inherit initial unset revert auto none from through to in',
    lineComment: r'//',
    delimiters: [_block, _double, _single],
    variables: true,
    extraPattern:
        r'(?<meta>@[\w-]+)|(?<specialNumber>#[0-9a-fA-F]{3,8}\b)|(?<property>[-\w]+(?=\s*:))|(?<tag>[.#][A-Za-z_][\w-]*)',
  ),
  CodeSyntaxLanguage.less: CodeSyntaxDefinition(
    family: CodeSyntaxFamily.stylesheet,
    keywords: 'important inherit initial unset revert auto none when',
    lineComment: r'//',
    delimiters: [_block, _double, _single],
    extraPattern:
        r'(?<specialVariable>@[\w-]+)|(?<specialNumber>#[0-9a-fA-F]{3,8}\b)|(?<property>[-\w]+(?=\s*:))|(?<tag>[.#][A-Za-z_][\w-]*)',
  ),
  CodeSyntaxLanguage.markdown: CodeSyntaxDefinition(
    family: CodeSyntaxFamily.markdown,
    delimiters: [
      CodeSyntaxDelimiter(
        r'^ {0,3}(?:`{3,}|~{3,})[^\r\n]*',
        '',
        _string,
        dynamicEnd: 'markdown',
      ),
      CodeSyntaxDelimiter(
        r'`+',
        '',
        _string,
        multiline: false,
        dynamicEnd: 'inline',
      ),
    ],
    extraPattern:
        r'(?<heading>^\s{0,3}#{1,6}\s+.*|^\s*(?:=+|-+)\s*$)|(?<link>!?\[[^\]]*\]\([^)]*\))|(?<meta>^\s*(?:>|[-+*]\s|\d+\.\s)|\*\*[^*]+\*\*|__[^_]+__)',
  ),
  CodeSyntaxLanguage.dockerfile: CodeSyntaxDefinition(
    keywords:
        'add arg cmd copy entrypoint env expose from healthcheck label maintainer onbuild run shell stopsignal user volume workdir',
    caseSensitive: false,
    lineComment: '#',
    delimiters: [_double, _single],
    variables: true,
  ),
  CodeSyntaxLanguage.makefile: CodeSyntaxDefinition(
    keywords:
        'define endef ifdef ifndef ifeq ifneq else endif include override export unexport private vpath',
    lineComment: '#',
    delimiters: [_double, _single],
    variables: true,
    assignmentProperties: true,
    extraPattern: r'(?<meta>\.[A-Z_]+)|(?<property>[\w./%-]+(?=\s*:))',
  ),
  CodeSyntaxLanguage.cmake: CodeSyntaxDefinition(
    keywords:
        'if else elseif endif foreach endforeach while endwhile function endfunction macro endmacro return break continue include add_executable add_library target_link_libraries target_include_directories find_package project cmake_minimum_required set option',
    caseSensitive: false,
    literals: 'true false on off yes no',
    lineComment: '#',
    delimiters: [
      _double,
      CodeSyntaxDelimiter(r'\[=*\[', '', _string, dynamicEnd: 'lua'),
    ],
    variables: true,
  ),
  CodeSyntaxLanguage.diff: CodeSyntaxDefinition(family: CodeSyntaxFamily.diff),
};
