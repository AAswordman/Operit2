// ignore_for_file: file_names

/// Identifies lexical categories without changing text metrics or source offsets.
enum CodeSyntaxTokenKind {
  keyword,
  type,
  literal,
  number,
  string,
  comment,
  function,
  property,
  variable,
  operator,
  punctuation,
  tag,
  meta,
  heading,
  link,
  inserted,
  deleted,
}

/// Stores one colored UTF-16 interval within an unchanged logical source line.
class CodeSyntaxToken {
  /// Records an exclusive-end interval returned by the language lexer.
  const CodeSyntaxToken(this.start, this.end, this.kind);

  final int start;
  final int end;
  final CodeSyntaxTokenKind kind;
}
