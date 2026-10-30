// Frozen pre-plugin UI oracle; never imported by production.
// ignore_for_file: file_names

/// One file changed during a completed assistant turn.
class WorkspaceDiffFile {
  const WorkspaceDiffFile({
    required this.path,
    required this.diffContent,
    required this.additions,
    required this.deletions,
  });

  final String path;
  final String diffContent;
  final int additions;
  final int deletions;

  String get fileName {
    final normalized = path.replaceAll('\\', '/');
    final segments = normalized.split('/');
    if (segments.isEmpty || segments.last.isEmpty) {
      return path;
    }
    return segments.last;
  }

  String get directoryPath {
    final normalized = path.replaceAll('\\', '/');
    final lastSlash = normalized.lastIndexOf('/');
    if (lastSlash < 0) {
      return '';
    }
    return normalized.substring(0, lastSlash + 1);
  }
}

/// The file diffs opened together in the workspace side panel.
class WorkspaceDiffReview {
  const WorkspaceDiffReview({required this.files});

  final List<WorkspaceDiffFile> files;

  int get additions => files.fold(0, (total, file) => total + file.additions);

  int get deletions => files.fold(0, (total, file) => total + file.deletions);
}
