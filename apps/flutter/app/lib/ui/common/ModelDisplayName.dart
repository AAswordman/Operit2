// ignore_for_file: file_names

final _wordInitial = RegExp(r'(^|\s)(\S)');
final _numericSequence = RegExp(r'(?<![\w.])\d+(?:-\d+)+(?![\w.])');

/// Formats a model ID for UI labels without changing its stored identity.
String formatModelDisplayName(String modelId) {
  final name = modelId
      .substring(modelId.lastIndexOf('/') + 1)
      .replaceAllMapped(_numericSequence, (match) {
        final parts = match[0]!.split('-');
        return parts.every((part) => part.length <= 2)
            ? parts.join('.')
            : match[0]!;
      })
      .replaceAll('-', ' ');
  return name.replaceAllMapped(
    _wordInitial,
    (match) => '${match[1]}${match[2]!.toUpperCase()}',
  );
}
