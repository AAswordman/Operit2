# Shared code editor syntax highlighting

This module is shared by the native code editor and plugin source rows. It does not use a hidden full-document text field, source truncation, platform-specific renderers, or automatic content-based language guessing.

## Files

- `CodeSyntaxLanguage.dart`: explicit filename/extension registry and immutable lexical grammar definitions.
- `CodeSyntaxLexer.dart`: shared line lexer for state-only indexing and visible token generation, including multiline region context.
- `CodeSyntaxHighlighter.dart`: incremental context propagation, bounded token cache, and exact-source span construction.
- `CodeSyntaxPalette.dart`: light/dark color-only styles. Font size, weight, family, strut, and letter spacing always come from the editor.
- `CodeSyntaxToken.dart`: UTF-16 token intervals and lexical categories.

## Registered languages and formats

Dart, Rust, JavaScript, TypeScript, Python, Java, Kotlin, Swift, C, C++, C#, Go, PHP, Ruby, Lua, Shell, PowerShell, SQL, JSON, JSONC, YAML, TOML, INI, HTML, XML, CSS, SCSS, Less, Markdown, Dockerfile, Makefile, CMake, and Diff.

The registry includes extension aliases such as JSX/TSX, Vue/Svelte, SVG/XAML, and JSON5, plus exact names such as `.bashrc`, `.env`, `Dockerfile`, `Makefile`, and `CMakeLists.txt`. Aliases use the corresponding registered lexical grammar. Unregistered files are explicitly plain text.

## Incremental behavior

The first language configuration scans source lines for lightweight incoming/outgoing lexical states; it does not shape text or retain a whole-document token tree. Local edits reuse unchanged source-line identities and propagate lexical changes only until the unchanged suffix has the same incoming state. A changed multiline comment or string can necessarily propagate farther than the edited lines.

Colored intervals are produced only for requested paragraphs and retained in a bounded cache alongside the virtual paragraph working set. Zoom and wrapping do not invalidate lexical intervals. Theme changes replace paragraph colors while retaining tokens. Source characters and CRLF/CR/LF separators are never rewritten.

## Scope

This is lexical highlighting, not compiler or language-server semantic analysis. It colors declared keywords, built-in types, literals, numbers, strings, comments, callable names, keys, tags, attributes, directives, and format-specific markers. Multiline support includes nested block comments in their registered languages, triple-quoted strings, Rust/C++ raw delimiters, Lua brackets, SQL dollar quotes, markup attributes/comments, YAML block scalars, and Markdown fences.

Embedded JavaScript/CSS inside HTML/Vue/Svelte, language-specific code inside Markdown fences, template interpolations, shell heredocs, and complete JSX/TSX parsing are not separate nested grammars. Syntax coloring must not be treated as source validation.

## Extending the registry

1. Add a `CodeSyntaxLanguage` entry and exact filename/extension mappings.
2. Register a `CodeSyntaxDefinition` with keyword/type/literal sets, comment/string delimiters, and optional named lexical token rules.
3. Introduce new state only for syntax that genuinely crosses logical lines. State-only and token-producing scans must end with identical context, including unfinished drafts.
4. Add language samples and multiline/edit cases to `test/workspace_syntax_highlight_test.dart`. Run the existing editor zoom, wrapping, input, and history tests as well.
