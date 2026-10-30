// Frozen pre-plugin UI oracle; never imported by production.
// ignore_for_file: file_names

import 'dart:convert';
import 'dart:io';

import 'package:path/path.dart' as p;

enum WorkspaceOpenerType {
  explorer,
  vsCode,
  terminal,
  copyPath,
}

class GitStatusEntry {
  const GitStatusEntry({required this.status, required this.path});

  final String status;
  final String path;
}

class WorkspaceGitChange {
  WorkspaceGitChange({
    required this.root,
    required this.status,
    required this.relativePath,
    required this.absolutePath,
    this.diff = '',
    this.additions = 0,
    this.deletions = 0,
    this.isLoaded = false,
  });

  final String root;
  final String status;
  final String relativePath;
  final String absolutePath;
  String diff;
  int additions;
  int deletions;
  bool isLoaded;

  String get directory {
    final separator = absolutePath.contains('\\') ? '\\' : '/';
    final index = absolutePath.lastIndexOf(separator);
    if (index <= 0) {
      return absolutePath;
    }
    return absolutePath.substring(0, index);
  }
}

/// Stores the last execution error from Git subprocess for user-visible diagnostics.
String? lastGitExecutionError;

String _gitExecutable() {
  if (Platform.isWindows) {
    try {
      final test = Process.runSync(
        'git',
        const <String>['--version'],
        runInShell: true,
      );
      if (test.exitCode == 0) {
        return 'git';
      }
    } catch (_) {}

    final localAppData = Platform.environment['LOCALAPPDATA'];
    final programFiles = Platform.environment['ProgramFiles'];
    final programFilesX86 = Platform.environment['ProgramFiles(x86)'];
    final candidates = <String>[
      if (localAppData != null) ...<String>[
        '$localAppData\\Programs\\Git\\cmd\\git.exe',
        '$localAppData\\Programs\\Git\\bin\\git.exe',
      ],
      if (programFiles != null) ...<String>[
        '$programFiles\\Git\\cmd\\git.exe',
        '$programFiles\\Git\\bin\\git.exe',
      ],
      if (programFilesX86 != null) ...<String>[
        '$programFilesX86\\Git\\cmd\\git.exe',
        '$programFilesX86\\Git\\bin\\git.exe',
      ],
      r'C:\Program Files\Git\cmd\git.exe',
      r'C:\Program Files\Git\bin\git.exe',
      r'C:\Users\Administrator\AppData\Local\Programs\Git\cmd\git.exe',
    ];
    for (final candidate in candidates) {
      if (File(candidate).existsSync()) {
        return candidate;
      }
    }
  }
  return 'git';
}

/// Parses `git diff --name-status -z` output.
List<GitStatusEntry> parseGitDiffNameStatus(String text) {
  final parts = text.split('\u0000').where((part) => part.isNotEmpty).toList();
  final entries = <GitStatusEntry>[];
  for (var index = 0; index < parts.length; index += 1) {
    final statusPart = parts[index];
    if (statusPart.isEmpty) {
      continue;
    }
    final letter = _statusLetter(statusPart);
    if (statusPart.startsWith('R') || statusPart.startsWith('C')) {
      index += 2;
      if (index < parts.length) {
        entries.add(GitStatusEntry(status: letter, path: parts[index]));
      }
    } else {
      index += 1;
      if (index < parts.length) {
        entries.add(GitStatusEntry(status: letter, path: parts[index]));
      }
    }
  }
  return entries;
}

/// Reads `git status -z` records.
List<GitStatusEntry> parseGitPorcelain(String text) {
  final parts = text.split('\u0000').where((part) => part.isNotEmpty).toList();
  final entries = <GitStatusEntry>[];
  for (var index = 0; index < parts.length; index += 1) {
    final record = parts[index];
    if (record.length < 3) {
      continue;
    }
    final status = record.substring(0, 2).trim();
    var path = record.length > 3 ? record.substring(3) : '';
    if (status.startsWith('R') || status.startsWith('C')) {
      index += 1;
      if (index < parts.length) {
        path = parts[index];
      }
    }
    if (path.isNotEmpty) {
      entries.add(GitStatusEntry(status: _statusLetter(status), path: path));
    }
  }
  return entries;
}

Map<String, ({int add, int del})> parseGitNumstat(String? text) {
  final stats = <String, ({int add, int del})>{};
  if (text == null || text.trim().isEmpty) {
    return stats;
  }
  for (final line in text.split('\n')) {
    final parts = line.split('\t');
    if (parts.length >= 3) {
      final add = int.tryParse(parts[0]) ?? 0;
      final del = int.tryParse(parts[1]) ?? 0;
      final path = parts.sublist(2).join('\t').trim();
      stats[path] = (add: add, del: del);
    }
  }
  return stats;
}

String _statusLetter(String status) {
  if (status.contains('?')) {
    return 'U';
  }
  if (status.contains('U')) {
    return 'U';
  }
  if (status.contains('A')) {
    return 'A';
  }
  if (status.contains('D')) {
    return 'D';
  }
  if (status.contains('R') || status.contains('C')) {
    return 'R';
  }
  return 'M';
}

/// Loads changed files from mounted local physical directories.
Future<List<WorkspaceGitChange>> loadGitChanges(
  Iterable<({String name, String path})> mounts,
) async {
  lastGitExecutionError = null;
  final seenRoots = <String>{};
  final changes = <WorkspaceGitChange>[];

  for (final mount in mounts) {
    final path = resolveVfsToNativePath(mount.path.trim());
    if (path.isEmpty || !Directory(path).existsSync()) {
      continue;
    }
    final root = await _gitRoot(path);
    if (root == null || !seenRoots.add(root)) {
      continue;
    }
    final status = await _git(
      root,
      <String>['status', '--porcelain=v1', '-uall', '-z'],
    );
    if (status == null || status.trim().isEmpty) {
      continue;
    }

    final numstat = await _git(
      root,
      <String>['diff', '--numstat', 'HEAD'],
    );
    final statsMap = parseGitNumstat(numstat);

    final entries = parseGitPorcelain(status);
    for (final entry in entries) {
      final absolutePath = _join(root, entry.path);
      final stat = statsMap[entry.path] ?? (add: 0, del: 0);
      changes.add(
        WorkspaceGitChange(
          root: root,
          status: entry.status,
          relativePath: entry.path,
          absolutePath: absolutePath,
          diff: '',
          additions: stat.add,
          deletions: stat.del,
          isLoaded: false,
        ),
      );
    }
  }
  if (changes.isNotEmpty) {
    await loadDiffForChange(changes.first);
  }
  return changes;
}

/// Detects the target base reference to diff against (e.g. upstream, main, or master).
Future<String?> detectBranchBaseRef(String root) async {
  final candidates = <String>[
    '@{u}',
    'origin/main',
    'main',
    'origin/master',
    'master',
    'HEAD~1',
  ];
  for (final ref in candidates) {
    final verified = await _git(
      root,
      <String>['rev-parse', '--verify', ref],
      silent: true,
    );
    if (verified != null && verified.trim().isNotEmpty) {
      return ref;
    }
  }
  return null;
}

/// Loads changed files between current branch and upstream/base branch for mounted directories.
Future<List<WorkspaceGitChange>> loadBranchChanges(
  Iterable<({String name, String path})> mounts,
) async {
  lastGitExecutionError = null;
  final seenRoots = <String>{};
  final changes = <WorkspaceGitChange>[];

  for (final mount in mounts) {
    final path = resolveVfsToNativePath(mount.path.trim());
    if (path.isEmpty || !Directory(path).existsSync()) {
      continue;
    }
    final root = await _gitRoot(path);
    if (root == null || !seenRoots.add(root)) {
      continue;
    }
    final baseRef = await detectBranchBaseRef(root);
    if (baseRef == null) {
      continue;
    }
    var diffStatus = await _git(
      root,
      <String>['diff', '--name-status', '-z', '$baseRef...HEAD'],
    );
    if (diffStatus == null || diffStatus.trim().isEmpty) {
      diffStatus = await _git(
        root,
        <String>['diff', '--name-status', '-z', baseRef, 'HEAD'],
      );
    }
    if (diffStatus == null || diffStatus.trim().isEmpty) {
      continue;
    }

    var numstat = await _git(
      root,
      <String>['diff', '--numstat', '$baseRef...HEAD'],
    );
    if (numstat == null || numstat.trim().isEmpty) {
      numstat = await _git(
        root,
        <String>['diff', '--numstat', baseRef, 'HEAD'],
      );
    }
    final statsMap = parseGitNumstat(numstat);

    final entries = parseGitDiffNameStatus(diffStatus);
    for (final entry in entries) {
      final absolutePath = _join(root, entry.path);
      final stat = statsMap[entry.path] ?? (add: 0, del: 0);
      changes.add(
        WorkspaceGitChange(
          root: root,
          status: entry.status,
          relativePath: entry.path,
          absolutePath: absolutePath,
          diff: '',
          additions: stat.add,
          deletions: stat.del,
          isLoaded: false,
        ),
      );
    }
  }
  if (changes.isNotEmpty) {
    final baseRef = changes.isNotEmpty
        ? await detectBranchBaseRef(changes.first.root)
        : null;
    await loadDiffForChange(changes.first, baseRef: baseRef);
  }
  return changes;
}

/// Loads unified diff for one specific file on-demand.
Future<String> loadDiffForChange(
  WorkspaceGitChange change, {
  String? baseRef,
}) async {
  if (change.isLoaded && change.diff.isNotEmpty) {
    return change.diff;
  }
  final root = change.root;
  String? diff;
  if (change.status == 'U') {
    final nullDevice = Platform.isWindows ? 'NUL' : '/dev/null';
    diff = await _git(
      root,
      <String>[
        'diff',
        '--no-index',
        '--unified=3',
        '--',
        nullDevice,
        change.absolutePath,
      ],
      acceptDifference: true,
    );
    if (diff == null && File(change.absolutePath).existsSync()) {
      try {
        final content = await File(change.absolutePath).readAsString();
        final lines = content.split('\n');
        diff =
            '--- /dev/null\n+++ ${change.relativePath}\n@@ -0,0 +1,${lines.length} @@\n' +
            lines.map((line) => '+$line').join('\n');
      } catch (_) {}
    }
  } else if (baseRef != null && baseRef.isNotEmpty) {
    diff = await _git(
      root,
      <String>[
        'diff',
        '--unified=3',
        '--no-ext-diff',
        '$baseRef...HEAD',
        '--',
        change.relativePath,
      ],
    );
    if (diff == null || diff.trim().isEmpty) {
      diff = await _git(
        root,
        <String>[
          'diff',
          '--unified=3',
          '--no-ext-diff',
          baseRef,
          'HEAD',
          '--',
          change.relativePath,
        ],
      );
    }
  } else {
    diff = await _git(
      root,
      <String>[
        'diff',
        '--unified=3',
        '--no-ext-diff',
        'HEAD',
        '--',
        change.relativePath,
      ],
    );
  }
  change.diff = diff ?? '';
  change.isLoaded = true;
  return change.diff;
}

bool _hasGitMetadata(String path) {
  try {
    var current = Directory(path);
    while (true) {
      final gitDir = Directory('${current.path}${Platform.pathSeparator}.git');
      final gitFile = File('${current.path}${Platform.pathSeparator}.git');
      if (gitDir.existsSync() || gitFile.existsSync()) {
        return true;
      }
      final parent = current.parent;
      if (parent.path == current.path) {
        break;
      }
      current = parent;
    }
  } catch (_) {}
  return false;
}

Future<String?> _gitRoot(String rawPath) async {
  final path = rawPath.trim();
  if (path.isEmpty || !Directory(path).existsSync()) {
    return null;
  }
  if (!_hasGitMetadata(path)) {
    return null;
  }

  final inside = await _git(
    path,
    const <String>['rev-parse', '--is-inside-work-tree'],
    silent: true,
  );
  if (inside?.trim() != 'true') {
    return null;
  }
  final root = await _git(
    path,
    const <String>['rev-parse', '--show-toplevel'],
    silent: true,
  );
  final trimmed = root?.trim();
  if (trimmed == null || trimmed.isEmpty) {
    return null;
  }
  return trimmed;
}

Future<String?> _git(
  String root,
  List<String> args, {
  bool acceptDifference = false,
  bool silent = false,
}) async {
  if (root.isEmpty || !Directory(root).existsSync()) {
    return null;
  }
  final exe = _gitExecutable();
  try {
    final result = await Process.run(
      exe,
      <String>['-C', root, ...args],
      runInShell: Platform.isWindows,
      stdoutEncoding: const Utf8Codec(allowMalformed: true),
      stderrEncoding: const Utf8Codec(allowMalformed: true),
    );
    if (result.exitCode == 0 || (acceptDifference && result.exitCode == 1)) {
      return result.stdout as String;
    }
    if (!silent) {
      lastGitExecutionError =
          'Git 命令退出异常 (code ${result.exitCode}): ${result.stderr.toString().trim()}';
      stderr.writeln('[_git error] $lastGitExecutionError');
    }
    return null;
  } on ProcessException catch (error) {
    if (!silent) {
      lastGitExecutionError = '无法启动 Git 进程 ($exe): $error';
      stderr.writeln('[_git ProcessException] $lastGitExecutionError');
    }
    return null;
  }
}

String _join(String root, String relativePath) {
  final separator = root.contains('\\') ? '\\' : '/';
  final normalized = relativePath.replaceAll('/', separator);
  if (root.endsWith(separator)) {
    return '$root$normalized';
  }
  return '$root$separator$normalized';
}

final Map<String, String> _vfsMountCache = <String, String>{};

/// Resolves a VFS path (e.g. /mnt/local/folders/<id>) or encoded mount to a native host path.
String resolveVfsToNativePath(String rawPath) {
  final trimmed = rawPath.trim();
  if (trimmed.isEmpty) {
    return trimmed;
  }

  final cached = _vfsMountCache[trimmed];
  if (cached != null) {
    return cached;
  }

  // 1. Direct physical path check
  try {
    if (Directory(trimmed).existsSync() || File(trimmed).existsSync()) {
      _vfsMountCache[trimmed] = trimmed;
      return trimmed;
    }
  } catch (_) {}

  // 2. operit-mount: JSON format
  if (trimmed.startsWith('operit-mount:')) {
    try {
      final jsonStr = trimmed.substring('operit-mount:'.length);
      final decoded = jsonDecode(jsonStr);
      if (decoded is Map<String, dynamic>) {
        final root = decoded['root']?.toString() ?? '';
        if (root.isNotEmpty && Directory(root).existsSync()) {
          _vfsMountCache[trimmed] = root;
          return root;
        }
      }
    } catch (_) {}
  }

  // 3. /mnt/ VFS namespace mapping via vfs_mounts.json catalog
  if (trimmed.startsWith('/mnt/') || trimmed.startsWith('mnt/')) {
    final nativeMapped = _resolveFromVfsCatalogs(trimmed);
    if (nativeMapped != null && nativeMapped.isNotEmpty) {
      _vfsMountCache[trimmed] = nativeMapped;
      return nativeMapped;
    }
  }

  return trimmed;
}

String? _resolveFromVfsCatalogs(String vfsPath) {
  final normalizedVfs = vfsPath.startsWith('/') ? vfsPath : '/$vfsPath';
  final catalogDirs = <String>[];

  if (Platform.isWindows) {
    final appData = Platform.environment['APPDATA'];
    if (appData != null && appData.isNotEmpty) {
      catalogDirs.add(
        p.join(appData, 'Operit2', 'runtime', 'identities'),
      );
    }
    final localAppData = Platform.environment['LOCALAPPDATA'];
    if (localAppData != null && localAppData.isNotEmpty) {
      catalogDirs.add(
        p.join(localAppData, 'Operit2', 'runtime', 'identities'),
      );
    }
  } else {
    final home = Platform.environment['HOME'];
    if (home != null && home.isNotEmpty) {
      if (Platform.isMacOS) {
        catalogDirs.add(
          p.join(home, 'Library', 'Application Support', 'Operit2', 'runtime', 'identities'),
        );
      } else {
        catalogDirs.add(
          p.join(home, '.config', 'Operit2', 'runtime', 'identities'),
        );
        catalogDirs.add(
          p.join(home, '.local', 'share', 'Operit2', 'runtime', 'identities'),
        );
      }
    }
  }

  for (final identitiesDir in catalogDirs) {
    try {
      final dir = Directory(identitiesDir);
      if (!dir.existsSync()) {
        continue;
      }
      for (final identityEntity in dir.listSync()) {
        if (identityEntity is! Directory) {
          continue;
        }
        final catalogFile = File(
          p.join(identityEntity.path, 'config', 'vfs_mounts.json'),
        );
        if (!catalogFile.existsSync()) {
          continue;
        }
        try {
          final content = catalogFile.readAsStringSync();
          final parsed = jsonDecode(content);
          if (parsed is Map<String, dynamic> && parsed['mounts'] is List) {
            final mounts = parsed['mounts'] as List;
            for (final item in mounts) {
              if (item is Map) {
                final id = item['id']?.toString() ?? '';
                final namespace = item['namespace']?.toString() ?? '';
                final root = item['root']?.toString() ?? '';
                if (id.isEmpty || root.isEmpty) {
                  continue;
                }
                final mountVfs = '$namespace/$id';
                if (normalizedVfs == mountVfs) {
                  if (Directory(root).existsSync()) {
                    return root;
                  }
                } else if (normalizedVfs.startsWith('$mountVfs/')) {
                  final relative = normalizedVfs.substring(mountVfs.length + 1);
                  final candidate = p.join(
                    root,
                    relative.replaceAll('/', Platform.pathSeparator),
                  );
                  if (Directory(candidate).existsSync() || File(candidate).existsSync()) {
                    return candidate;
                  }
                }
              }
            }
          }
        } catch (_) {}
      }
    } catch (_) {}
  }

  return null;
}

/// Opens the target file or directory using the specified tool.
Future<void> openWorkspaceTarget(
  String rawTargetPath, {
  required WorkspaceOpenerType opener,
}) async {
  final targetPath = resolveVfsToNativePath(rawTargetPath);
  final isDirectory = Directory(targetPath).existsSync();
  final directory = isDirectory ? targetPath : File(targetPath).parent.path;
  switch (opener) {
    case WorkspaceOpenerType.explorer:
      final executable = Platform.isWindows
          ? 'explorer'
          : Platform.isMacOS
              ? 'open'
              : 'xdg-open';
      final args = Platform.isWindows && !isDirectory
          ? <String>['/select,', targetPath]
          : <String>[directory];
      await Process.start(
        executable,
        args,
        mode: ProcessStartMode.detached,
        runInShell: Platform.isWindows,
      );
    case WorkspaceOpenerType.vsCode:
      await Process.start(
        'code',
        <String>['-g', targetPath],
        mode: ProcessStartMode.detached,
        runInShell: Platform.isWindows,
      );
    case WorkspaceOpenerType.terminal:
      if (Platform.isWindows) {
        await Process.start(
          'cmd',
          <String>['/c', 'start', 'cmd', '/k', 'cd /d "$directory"'],
          mode: ProcessStartMode.detached,
        );
      } else if (Platform.isMacOS) {
        await Process.start(
          'open',
          <String>['-a', 'Terminal', directory],
          mode: ProcessStartMode.detached,
        );
      } else {
        await Process.start(
          'x-terminal-emulator',
          <String>['--working-directory=$directory'],
          mode: ProcessStartMode.detached,
        );
      }
    case WorkspaceOpenerType.copyPath:
      break;
  }
}

/// Opens the previewed file's directory in VS Code or the system file manager.
Future<void> openPreviewDirectory(
  String directory, {
  required bool vsCode,
}) async {
  await openWorkspaceTarget(
    directory,
    opener: vsCode ? WorkspaceOpenerType.vsCode : WorkspaceOpenerType.explorer,
  );
}
