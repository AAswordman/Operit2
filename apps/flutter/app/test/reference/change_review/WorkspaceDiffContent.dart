// Frozen pre-plugin UI oracle; never imported by production.
// ignore_for_file: file_names

import 'dart:async';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';

import 'package:operit2/ui/features/chat/viewmodel/WorkspaceFileModels.dart';
import 'WorkspaceDiffReview.dart';
import 'WorkspaceGitChanges.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceInlineDiff.dart';

enum _ChangeSource {
  git,
  branch,
  lastTurn,
}

enum _LeftPaneMode {
  changes,
  workspaceTree,
  closed,
}

enum _DiffLayout {
  unified,
  split,
}

const String _kSvgVsCode = '''
<svg role="img" viewBox="0 0 24 24" fill="#007ACC" xmlns="http://www.w3.org/2000/svg">
<path d="M23.15 2.587L18.21.21a1.494 1.494 0 0 0-1.705.29l-9.46 8.63-4.12-3.128a.999.999 0 0 0-1.276.057L.327 7.261A1 1 0 0 0 .326 8.74L3.899 12 .326 15.26a1 1 0 0 0 .001 1.479L1.65 17.94a.999.999 0 0 0 1.276.057l4.12-3.128 9.46 8.63a1.492 1.492 0 0 0 1.704.29l4.942-2.377A1.5 1.5 0 0 0 24 20.06V3.939a1.5 1.5 0 0 0-.85-1.352zm-5.146 14.861L10.826 12l7.178-5.448v10.896z"/>
</svg>
''';

const String _kSvgTerminal = '''
<svg role="img" viewBox="0 0 24 24" fill="#5391FE" xmlns="http://www.w3.org/2000/svg">
<path d="M24 18.064c0 1.34-1.077 2.42-2.408 2.42H2.408C1.077 20.484 0 19.404 0 18.064V5.936C0 4.596 1.077 3.516 2.408 3.516h19.184C22.923 3.516 24 4.596 24 5.936v12.128zm-11.455-6.07l-6.86-5.474a.81.81 0 0 0-1.12.115.815.815 0 0 0 .114 1.127l5.966 4.76-5.966 4.761a.813.813 0 0 0-.114 1.127.81.81 0 0 0 1.12.115l6.86-5.474a.816.816 0 0 0 0-1.057zm7.42 4.417h-6.286a.81.81 0 1 0 0 1.621h6.286a.81.81 0 1 0 0-1.621z"/>
</svg>
''';

const String _kSvgTreeHierarchy = '''
<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">
<rect x="9" y="3" width="6" height="5" rx="1"/>
<rect x="3" y="15" width="6" height="5" rx="1"/>
<rect x="15" y="15" width="6" height="5" rx="1"/>
<path d="M12 8v4M6 15v-3h12v3"/>
</svg>
''';

const String _kSvgLangPython = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M11.91 2c-5.06 0-4.73 2.19-4.73 2.19l.01 2.27h4.81v.68H5.21S2 6.77 2 11.88s2.8 5 2.8 5h1.67v-2.33s-.09-2.8 2.76-2.8h4.74s2.66.04 2.66-2.6V4.6s.4-2.6-4.72-2.6zm-2.58 1.48a.86.86 0 1 1 0 1.72.86.86 0 0 1 0-1.72z" fill="#3776AB"/>
<path d="M12.09 22c5.06 0 4.73-2.19 4.73-2.19l-.01-2.27h-4.81v-.68h6.78s3.21.37 3.21-4.74-2.8-5-2.8-5h-1.67v2.33s.09 2.8-2.76 2.8H10.1s-2.66-.04-2.66 2.6V19.4s-.4 2.6 4.72 2.6zm2.58-1.48a.86.86 0 1 1 0-1.72.86.86 0 0 1 0 1.72z" fill="#FFD43B"/>
</svg>
''';

const String _kSvgLangRust = '''
<svg viewBox="0 0 24 24" fill="#DEA584" xmlns="http://www.w3.org/2000/svg">
<path d="M23.83 12c0-.29-.02-.57-.05-.84l-1.92-.32a9.98 9.98 0 0 0-.58-1.41l1.33-1.44a11.96 11.96 0 0 0-1.23-1.9l-1.84.66a9.97 9.97 0 0 0-1.08-1.08l.66-1.84a11.97 11.97 0 0 0-1.9-1.23l-1.44 1.33a10.03 10.03 0 0 0-1.41-.59l-.32-1.92A11.75 11.75 0 0 0 12 .17c-.29 0-.57.02-.84.05l-.32 1.92a9.98 9.98 0 0 0-1.41.58l-1.44-1.33a11.96 11.96 0 0 0-1.9 1.23l.66 1.84a9.98 9.98 0 0 0-1.08 1.08l-1.84-.66a11.97 11.97 0 0 0-1.23 1.9l1.33 1.44c-.23.45-.43.93-.59 1.41l-1.92.32a11.77 11.77 0 0 0-.05.85c0 .28.02.56.05.84l1.92.32c.16.49.36.96.59 1.41l-1.33 1.44a11.96 11.96 0 0 0 1.23 1.9l1.84-.66c.34.4.7.76 1.08 1.08l-.66 1.84a11.97 11.97 0 0 0 1.9 1.23l1.44-1.33c.45.23.93.43 1.41.59l.32 1.92c.28.03.56.05.84.05.29 0 .57-.02.84-.05l.32-1.92a10.02 10.02 0 0 0 1.41-.59l1.44 1.33a11.96 11.96 0 0 0 1.9-1.23l-.66-1.84a9.98 9.98 0 0 0 1.08-1.08l1.84.66a11.97 11.97 0 0 0 1.23-1.9l-1.33-1.44c.23-.45.43-.93.59-1.41l1.92-.32c.03-.28.05-.56.05-.84zm-11.83 6.64a6.64 6.64 0 1 1 0-13.28 6.64 6.64 0 0 1 0 13.28zm-2.02-9.61h2.24c1.17 0 1.91.56 1.91 1.48 0 .76-.48 1.28-1.24 1.43l1.43 2.7h-1.28l-1.28-2.48h-.6v2.48H9.98V9.03zm1.18 2.11h.96c.55 0 .9-.25.9-.66 0-.42-.35-.66-.9-.66h-.96v1.32z"/>
</svg>
''';

const String _kSvgLangC = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M12 2.5l8.23 4.75v9.5L12 21.5l-8.23-4.75v-9.5L12 2.5z" fill="#00599C"/>
<path d="M14.8 8.8a4.2 4.2 0 0 0-5.6 0 4 4 0 0 0 0 6.4 4.2 4.2 0 0 0 5.6 0" stroke="#FFFFFF" stroke-width="2.4" stroke-linecap="round"/>
</svg>
''';

const String _kSvgLangCpp = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M12 2.5l8.23 4.75v9.5L12 21.5l-8.23-4.75v-9.5L12 2.5z" fill="#004482"/>
<path d="M12 9.2a3.5 3.5 0 0 0-4.8 0 3.4 3.4 0 0 0 0 5.6 3.5 3.5 0 0 0 4.8 0" stroke="#FFFFFF" stroke-width="2.2" stroke-linecap="round"/>
<path d="M14 12h2.5M15.25 10.75v2.5M17.5 12h2.5M18.75 10.75v2.5" stroke="#659AD2" stroke-width="1.5" stroke-linecap="round"/>
</svg>
''';

const String _kSvgLangDart = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M4.1 3.5L3 13.2l5.7 5.7 8.2-8.2L4.1 3.5z" fill="#0175C2"/>
<path d="M16.9 10.7l-8.2 8.2 2.1 2.1c1.2 1.2 3.1 1.2 4.3 0l5.8-5.8-4-4.5z" fill="#02569B"/>
<path d="M12.9 2.5c-1.2-1.2-3.1-1.2-4.3 0L3 8.1l1.1 5.1 12.8-2.5-4-8.2z" fill="#00B4AB"/>
<path d="M16.9 10.7l4 4.5 1.1-5.1-5.1-7.6-4 8.2 4 0z" fill="#29B6F6"/>
</svg>
''';

const String _kSvgLangTs = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<rect width="22" height="22" x="1" y="1" rx="4" fill="#3178C6"/>
<path d="M11 9H5v2.2h2V18h2.3v-6.8H11V9zm3.8 4.7c.8.6 1.7.9 2.7.9 1 0 1.6-.4 1.6-1 0-.6-.5-.9-1.8-1.4-1.7-.6-2.8-1.5-2.8-3 0-1.8 1.4-3.2 3.6-3.2 1.3 0 2.3.4 3.1 1l-1 1.8c-.7-.5-1.4-.7-2.1-.7-.8 0-1.3.4-1.3.9 0 .6.5.8 1.7 1.3 1.9.7 2.9 1.6 2.9 3.2 0 1.9-1.4 3.3-3.8 3.3-1.6 0-2.8-.5-3.8-1.2l1-1.9z" fill="#FFFFFF"/>
</svg>
''';

const String _kSvgLangJs = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<rect width="22" height="22" x="1" y="1" rx="4" fill="#F7DF1E"/>
<path d="M7.5 17.5c1 .6 2.1.9 3.2.9 1.8 0 2.8-.9 2.8-2.5V9H11v6.9c0 .6-.3.9-.9.9-.4 0-.8-.1-1.2-.3l-1.4 1zM15 17.2c1 .6 2.1 1 3.4 1 2.2 0 3.6-1.1 3.6-2.8 0-1.6-1-2.4-2.6-3-1.1-.5-1.6-.8-1.6-1.3 0-.5.4-.8 1.2-.8.8 0 1.5.3 2.1.7l1.1-1.8c-.9-.6-2-1-3.2-1-2.1 0-3.5 1.2-3.5 2.8 0 1.5 1 2.4 2.5 3 1.1.5 1.7.8 1.7 1.4 0 .6-.5.9-1.4.9-.9 0-1.8-.3-2.5-.8l-1.3 1.9z" fill="#000000"/>
</svg>
''';

const String _kSvgLangGo = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<rect width="22" height="22" x="1" y="1" rx="5" fill="#00ADD8"/>
<path d="M10.8 12.3H8.3v1.8h4.5c-.4 2.3-2 3.6-4.5 3.6-2.9 0-5.1-2.1-5.1-5.4s2.2-5.4 5.1-5.4c1.8 0 3.1.8 3.9 1.8l-1.6 1.4c-.6-.7-1.3-1.2-2.3-1.2-1.7 0-3 1.4-3 3.4s1.3 3.4 3 3.4c1.2 0 2-.6 2.4-1.5H10.8v-1.9zm8.4-.3c0 3.3-2.3 5.7-5.5 5.7s-5.5-2.4-5.5-5.7 2.3-5.7 5.5-5.7 5.5 2.4 5.5 5.7zm-2.4 0c0-2-1.3-3.6-3.1-3.6s-3.1 1.6-3.1 3.6 1.3 3.6 3.1 3.6 3.1-1.6 3.1-3.6z" fill="#FFFFFF"/>
</svg>
''';

const String _kSvgLangJava = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M4 10h12v6a4 4 0 0 1-4 4H8a4 4 0 0 1-4-4v-6zm12 2h2a2 2 0 0 1 2 2v0a2 2 0 0 1-2 2h-2v-4z" stroke="#EA2D2E" stroke-width="2" stroke-linecap="round"/>
<path d="M7 6c0-1.5 1-2.5 1-4M10 6c0-1.5 1-2.5 1-4M13 6c0-1.5 1-2.5 1-4" stroke="#5382A1" stroke-width="1.8" stroke-linecap="round"/>
<path d="M2 21h18" stroke="#EA2D2E" stroke-width="2" stroke-linecap="round"/>
</svg>
''';

const String _kSvgLangKotlin = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<rect width="22" height="22" x="1" y="1" rx="4" fill="#7F52FF"/>
<path d="M4 20L12 12 4 4v16zM12 12l8 8H12v-8zM12 12l8-8H12v8z" fill="#FFFFFF" fill-opacity="0.85"/>
</svg>
''';

const String _kSvgLangSwift = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<rect width="22" height="22" x="1" y="1" rx="5" fill="#F05138"/>
<path d="M19.2 17.5c-3.1 2.2-7.3 1.8-10.4-.9 4.2 1.4 8.7-.3 10.4-.9-2.3-1.6-4.5-4-5.2-6.8 1.8 1.4 4.1 2 6.4 1.4-1.9-2.6-4.9-5-8.5-5.8 4.2.9 7.8 3.5 9.7 7.2-.6-1.5-1.5-2.8-2.6-3.9 3.2 2.7 4.1 7.1.2 9.7z" fill="#FFFFFF"/>
</svg>
''';

const String _kSvgLangHtml = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M4 3l1.5 16.5L12 21.5l6.5-2L20 3H4z" fill="#E34F26"/>
<path d="M12 4.5v15.2l5.1-1.5 1.2-13.7H12z" fill="#EF652A"/>
<path d="M12 7.8H8.2l.2 2.5h3.6V7.8zm0 5h-2.1l-.1-1.3H8.3l.3 3.6h3.4v-2.3zm0-5v2.5h3.4l-.3 3.5-3.1.8v2.4l5.3-1.5.6-7.7H12z" fill="#FFFFFF"/>
</svg>
''';

const String _kSvgLangCss = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M4 3l1.5 16.5L12 21.5l6.5-2L20 3H4z" fill="#1572B6"/>
<path d="M12 4.5v15.2l5.1-1.5 1.2-13.7H12z" fill="#33A9DC"/>
<path d="M12 7.8H8.2l.2 2.5h3.6V7.8zm0 5h-3.5l.3 3.6 3.2.9v-2.4l-1.4-.4-.1-1.7H12v-0zm0-5v2.5h3.4l-.3 3.5-3.1.8v2.4l5.3-1.5.6-7.7H12z" fill="#FFFFFF"/>
</svg>
''';

const String _kSvgLangJson = '''
<svg viewBox="0 0 24 24" fill="none" stroke="#CB85E8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">
<path d="M9 5c-1.5 0-2 1-2 2v3c0 1-1 2-2 2 1 0 2 1 2 2v3c0 1 .5 2 2 2M15 5c1.5 0 2 1 2 2v3c0 1 1 2 2 2-1 0-2 1-2 2v3c0 1-.5 2-2 2"/>
</svg>
''';

const String _kSvgLangYaml = '''
<svg viewBox="0 0 24 24" fill="none" stroke="#E5C07B" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">
<path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6"/>
</svg>
''';

const String _kSvgLangMarkdown = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<rect width="22" height="16" x="1" y="4" rx="3" stroke="#79B8FF" stroke-width="2"/>
<path d="M4 15V9l2.5 3L9 9v6M13 12h3M14.5 9.5v5.5m0 0l-2-2m2 2l2-2" stroke="#79B8FF" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>
</svg>
''';

const String _kSvgLangShell = '''
<svg viewBox="0 0 24 24" fill="none" stroke="#5391FE" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">
<polyline points="4 17 10 11 4 5"/>
<line x1="12" y1="19" x2="20" y2="19"/>
</svg>
''';

const String _kSvgLangSql = '''
<svg viewBox="0 0 24 24" fill="none" stroke="#E5C07B" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">
<ellipse cx="12" cy="5" rx="9" ry="3"/>
<path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/>
</svg>
''';

const String _kSvgLangDocker = '''
<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
<path d="M2.5 12.5h19c0 4.5-3.5 8-8 8-4 0-7.2-2.8-7.9-6.5l-3.1-1.5z" fill="#0db7ed"/>
<rect x="5" y="9" width="2" height="2" fill="#0db7ed"/>
<rect x="8" y="9" width="2" height="2" fill="#0db7ed"/>
<rect x="11" y="9" width="2" height="2" fill="#0db7ed"/>
<rect x="8" y="6" width="2" height="2" fill="#0db7ed"/>
<rect x="11" y="6" width="2" height="2" fill="#0db7ed"/>
<rect x="14" y="9" width="2" height="2" fill="#0db7ed"/>
</svg>
''';

const String _kSvgLangGit = '''
<svg viewBox="0 0 24 24" fill="none" stroke="#F05032" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">
<circle cx="6" cy="6" r="3"/>
<circle cx="6" cy="18" r="3"/>
<circle cx="18" cy="9" r="3"/>
<line x1="6" y1="9" x2="6" y2="15"/>
<path d="M6 12a9 9 0 0 0 9-3"/>
</svg>
''';

const String _kSvgLangLock = '''
<svg viewBox="0 0 24 24" fill="none" stroke="#8B949E" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">
<rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>
<path d="M7 11V7a5 5 0 0 1 10 0v4"/>
</svg>
''';

const String _kSvgLangDefault = '''
<svg viewBox="0 0 24 24" fill="none" stroke="#8B949E" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">
<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
<polyline points="14 2 14 8 20 8"/>
</svg>
''';

class _ChangeRow {
  const _ChangeRow({
    required this.name,
    required this.location,
    required this.fullPath,
    required this.status,
    required this.diff,
    required this.directory,
    required this.additions,
    required this.deletions,
    this.gitChange,
  });

  final String name;
  final String location;
  final String fullPath;
  final String status;
  final String diff;
  final String? directory;
  final int additions;
  final int deletions;
  final WorkspaceGitChange? gitChange;
}

class _TreeNode {
  const _TreeNode({required this.entry, required this.depth});

  final WorkspaceFileEntry entry;
  final int depth;
}

/// Workspace change browser: Git uncommitted, branch comparison, or previous turn,
/// plus the mounted file tree and external editors.
class WorkspaceDiffContent extends StatefulWidget {
  const WorkspaceDiffContent({
    super.key,
    required this.review,
    this.workspacePath,
    this.onListWorkspaceFiles,
    this.onReadWorkspaceTextFile,
    this.loadGitChanges,
    this.loadBranchChanges,
  });

  final WorkspaceDiffReview review;
  final String? workspacePath;
  final Future<List<WorkspaceFileEntry>> Function(String path)?
  onListWorkspaceFiles;
  final Future<String> Function(String path)? onReadWorkspaceTextFile;
  final Future<List<WorkspaceGitChange>> Function(
    Iterable<({String name, String path})> mounts,
  )?
  loadGitChanges;
  final Future<List<WorkspaceGitChange>> Function(
    Iterable<({String name, String path})> mounts,
  )?
  loadBranchChanges;

  @override
  State<WorkspaceDiffContent> createState() => _WorkspaceDiffContentState();
}

class _WorkspaceDiffContentState extends State<WorkspaceDiffContent> {
  _ChangeSource _source = _ChangeSource.lastTurn;
  _LeftPaneMode _leftPaneMode = _LeftPaneMode.changes;
  _LeftPaneMode _previousLeftPaneMode = _LeftPaneMode.changes;
  _DiffLayout _diffLayout = _DiffLayout.unified;
  bool _wrapLines = false;

  /// When true the diff view fills the unchanged regions with real file lines
  /// instead of collapsing them behind a bar.
  bool _showFullFile = false;

  /// Current text of the selected file, loaded lazily so the compact diff stays
  /// instant. Null means "not loaded yet" or "unavailable".
  List<String>? _selectedFileLines;
  String? _selectedFileLinesKey;
  var _listWidth = 220.0;
  String? _selectedLocation;
  var _loading = false;
  var _loadingDiff = false;
  String? _gitError;
  String? _branchBaseRef;
  WorkspaceOpenerType _opener = WorkspaceOpenerType.vsCode;
  final TextEditingController _searchController = TextEditingController();

  List<WorkspaceGitChange> _gitChanges = const <WorkspaceGitChange>[];
  List<WorkspaceGitChange> _branchChanges = const <WorkspaceGitChange>[];
  List<WorkspaceFileEntry> _roots = const <WorkspaceFileEntry>[];
  final Map<String, List<WorkspaceFileEntry>> _children =
      <String, List<WorkspaceFileEntry>>{};
  final Set<String> _expanded = <String>{};
  String? _treeError;
  WorkspaceFileEntry? _previewFile;
  String? _previewText;
  Object? _previewError;
  var _previewLoading = false;

  @override
  void dispose() {
    _searchController.removeListener(_onSearchChanged);
    _searchController.dispose();
    super.dispose();
  }

  @override
  void initState() {
    super.initState();
    _searchController.addListener(_onSearchChanged);
    if (widget.review.files.isEmpty) {
      _source = _ChangeSource.git;
    }
    unawaited(_loadWorkspace());
  }

  @override
  void didUpdateWidget(covariant WorkspaceDiffContent oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.review != widget.review) {
      if (widget.review.files.isNotEmpty) {
        _source = _ChangeSource.lastTurn;
      } else {
        _source = _ChangeSource.git;
      }
    }
    if (oldWidget.workspacePath != widget.workspacePath ||
        oldWidget.onListWorkspaceFiles != widget.onListWorkspaceFiles) {
      unawaited(_loadWorkspace());
    }
  }

  void _onSearchChanged() => setState(() {});

  Future<void> _loadWorkspace() async {
    final list = widget.onListWorkspaceFiles;
    final wp = widget.workspacePath?.trim();
    final nativeWp = (wp != null && wp.isNotEmpty)
        ? resolveVfsToNativePath(wp)
        : '';
    final isLocalPhysicalDir =
        nativeWp.isNotEmpty && Directory(nativeWp).existsSync();

    setState(() {
      _loading = true;
      _gitError = null;
    });

    var roots = <WorkspaceFileEntry>[];
    String? listError;
    if (list != null) {
      try {
        roots = await list('');
      } catch (error) {
        listError = '$error';
      }
    }

    if (roots.isEmpty && isLocalPhysicalDir) {
      try {
        final dir = Directory(nativeWp);
        final entities = dir
            .listSync(followLinks: false)
            .where(
              (f) =>
                  !f.path.split(Platform.pathSeparator).last.startsWith('.'),
            )
            .toList();
        roots = entities.map((entity) {
          final name = entity.path.split(Platform.pathSeparator).last;
          final isDir = entity is Directory;
          return WorkspaceFileEntry(
            name: name,
            path: entity.path,
            relativePath: name,
            isDirectory: isDir,
            size: isDir ? 0 : (entity is File ? entity.lengthSync() : 0),
            lastModified: '',
          );
        }).toList();
        roots.sort((a, b) {
          if (a.isDirectory != b.isDirectory) {
            return a.isDirectory ? -1 : 1;
          }
          return a.name.toLowerCase().compareTo(b.name.toLowerCase());
        });
      } catch (_) {}
    }

    if (!mounted) {
      return;
    }

    setState(() {
      _roots = roots;
      _treeError = roots.isEmpty ? listError : null;
    });

    final mounts = _collectMounts(roots, wp);
    if (mounts.isEmpty) {
      if (!mounted) {
        return;
      }
      setState(() {
        _loading = false;
        _gitChanges = const <WorkspaceGitChange>[];
        _branchChanges = const <WorkspaceGitChange>[];
        _gitError = null;
      });
      return;
    }

    await _fetchActiveChanges(mounts);
  }

  List<({String name, String path})> _collectMounts(
    List<WorkspaceFileEntry> entries,
    String? wp,
  ) {
    final mounts = <({String name, String path})>[];
    if (wp != null && wp.isNotEmpty) {
      final nativePath = resolveVfsToNativePath(wp);
      if (nativePath.isNotEmpty && Directory(nativePath).existsSync()) {
        mounts.add((name: 'workspace', path: nativePath));
      }
    }
    for (final entry in entries) {
      if (entry.isDirectory && entry.path.isNotEmpty) {
        final nativePath = resolveVfsToNativePath(entry.path);
        if (nativePath.isNotEmpty && Directory(nativePath).existsSync()) {
          if (!mounts.any((m) => m.path == nativePath)) {
            mounts.add((name: entry.name, path: nativePath));
          }
        }
      }
    }
    return mounts;
  }

  Future<void> _fetchActiveChanges(
    List<({String name, String path})> mounts,
  ) async {
    try {
      if (_source == _ChangeSource.git) {
        final changes = await (widget.loadGitChanges ?? loadGitChanges)(mounts);
        if (!mounted) {
          return;
        }
        setState(() {
          _gitChanges = changes;
          _loading = false;
          _gitError = changes.isEmpty
              ? (lastGitExecutionError ?? '当前分支工作区无未提交的 Git 改动')
              : null;
        });
      } else if (_source == _ChangeSource.branch) {
        final changes =
            await (widget.loadBranchChanges ?? loadBranchChanges)(mounts);
        if (!mounted) {
          return;
        }
        setState(() {
          _branchChanges = changes;
          _loading = false;
          _gitError = changes.isEmpty
              ? (lastGitExecutionError ?? '当前分支与基准分支无差异或未检测到上游基准')
              : null;
        });
      } else {
        if (!mounted) {
          return;
        }
        setState(() => _loading = false);
      }
    } catch (error) {
      if (!mounted) {
        return;
      }
      setState(() {
        _loading = false;
        _gitError = 'Git 读取失败: $error';
      });
    }
  }

  void _switchSource(_ChangeSource newSource) {
    if (_source == newSource) {
      return;
    }
    setState(() {
      _source = newSource;
      _selectedLocation = null;
      _previewFile = null;
      _gitError = null;
    });

    final wp = widget.workspacePath?.trim();
    final mounts = _collectMounts(_roots, wp);
    if (newSource == _ChangeSource.git && _gitChanges.isEmpty) {
      setState(() => _loading = true);
      unawaited(_fetchActiveChanges(mounts));
    } else if (newSource == _ChangeSource.branch && _branchChanges.isEmpty) {
      setState(() => _loading = true);
      unawaited(_fetchActiveChanges(mounts));
    }
  }

  List<_ChangeRow> get _allRows {
    final wp = widget.workspacePath?.trim() ?? '';
    if (_source == _ChangeSource.git) {
      return <_ChangeRow>[
        for (final change in _gitChanges)
          _ChangeRow(
            name: _fileName(change.relativePath),
            location: change.relativePath,
            fullPath: change.absolutePath,
            status: change.status,
            diff: change.diff,
            directory: _parentDirectory(change.relativePath),
            additions: change.additions,
            deletions: change.deletions,
            gitChange: change,
          ),
      ];
    }
    if (_source == _ChangeSource.branch) {
      return <_ChangeRow>[
        for (final change in _branchChanges)
          _ChangeRow(
            name: _fileName(change.relativePath),
            location: change.relativePath,
            fullPath: change.absolutePath,
            status: change.status,
            diff: change.diff,
            directory: _parentDirectory(change.relativePath),
            additions: change.additions,
            deletions: change.deletions,
            gitChange: change,
          ),
      ];
    }
    return <_ChangeRow>[
      for (final file in widget.review.files)
        _ChangeRow(
          name: file.fileName,
          location: file.path,
          fullPath: wp.isNotEmpty
              ? _joinPath(resolveVfsToNativePath(wp), file.path)
              : file.path,
          status: 'M',
          diff: file.diffContent,
          directory: _parentDirectory(file.path),
          additions: file.additions,
          deletions: file.deletions,
        ),
    ];
  }

  List<_ChangeRow> get _rows {
    final query = _searchController.text.trim().toLowerCase();
    return _allRows.where((row) {
      return query.isEmpty ||
          row.name.toLowerCase().contains(query) ||
          row.location.toLowerCase().contains(query);
    }).toList(growable: false);
  }

  Future<void> _selectChangeRow(_ChangeRow row) async {
    setState(() {
      _selectedLocation = row.location;
      _previewFile = null;
    });
    if (_showFullFile) {
      unawaited(_ensureSelectedFileLines(row));
    }
    final change = row.gitChange;
    if (change != null && !change.isLoaded) {
      setState(() => _loadingDiff = true);
      final baseRef = _source == _ChangeSource.branch ? _branchBaseRef : null;
      await loadDiffForChange(change, baseRef: baseRef);
      if (!mounted) {
        return;
      }
      setState(() => _loadingDiff = false);
    }
  }

  void _navigateChange(int delta, List<_ChangeRow> rows) {
    if (rows.isEmpty) {
      return;
    }
    var currentIndex = rows.indexWhere(
      (row) => row.location == _selectedLocation,
    );
    if (currentIndex < 0) {
      currentIndex = 0;
    }
    final nextIndex = (currentIndex + delta).clamp(0, rows.length - 1);
    final row = rows[nextIndex];
    unawaited(_selectChangeRow(row));
  }

  Future<void> _handleOpenerAction(
    WorkspaceOpenerType chosenOpener,
    _ChangeRow? selected,
  ) async {
    final targetPath =
        _previewFile?.path ?? selected?.fullPath ?? widget.workspacePath;
    if (targetPath == null || targetPath.isEmpty) {
      return;
    }

    if (chosenOpener == WorkspaceOpenerType.copyPath) {
      await Clipboard.setData(ClipboardData(text: targetPath));
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(
            content: Text('已复制路径: $targetPath'),
            duration: const Duration(seconds: 2),
          ),
        );
      }
      return;
    }

    setState(() => _opener = chosenOpener);
    await openWorkspaceTarget(targetPath, opener: chosenOpener);
  }

  @override
  Widget build(BuildContext context) {
    final rows = _rows;
    var selectedIndex = rows.indexWhere(
      (row) => row.location == _selectedLocation,
    );
    if (selectedIndex < 0) {
      selectedIndex = 0;
    }
    final selected = rows.isEmpty ? null : rows[selectedIndex];

    // Auto-load selected diff if not yet loaded
    if (selected != null &&
        selected.gitChange != null &&
        !selected.gitChange!.isLoaded &&
        !_loadingDiff) {
      unawaited(_selectChangeRow(selected));
    }

    final allRows = _allRows;
    final totalAdditions = allRows.fold<int>(
      0,
      (total, row) => total + row.additions,
    );
    final totalDeletions = allRows.fold<int>(
      0,
      (total, row) => total + row.deletions,
    );

    final theme = Theme.of(context);
    final isDark = theme.brightness == Brightness.dark;
    final dividerColor = isDark
        ? const Color(0xFF262626)
        : theme.colorScheme.outlineVariant.withValues(alpha: 0.35);

    return LayoutBuilder(
      builder: (context, constraints) {
        final totalWidth = constraints.maxWidth;
        // Ensure sidebar never eats all width, leaving at least 150px for the diff main pane
        final effectiveListWidth = totalWidth.isFinite && totalWidth > 330
            ? _listWidth.clamp(180.0, (totalWidth - 150.0).clamp(180.0, 500.0))
            : _listWidth.clamp(180.0, 500.0);

        return Row(
          children: <Widget>[
            // Collapsible Left Sidebar (Matching Image 1)
            if (_leftPaneMode != _LeftPaneMode.closed) ...<Widget>[
              SizedBox(
                width: effectiveListWidth,
                child: _buildSidebar(
                  rows: rows,
                  selectedIndex: selectedIndex,
                  totalAdditions: totalAdditions,
                  totalDeletions: totalDeletions,
                ),
              ),
              MouseRegion(
                cursor: SystemMouseCursors.resizeColumn,
                child: GestureDetector(
                  behavior: HitTestBehavior.opaque,
                  onHorizontalDragUpdate: (details) {
                    setState(() {
                      final maxAllowed = totalWidth.isFinite && totalWidth > 330
                          ? (totalWidth - 150.0).clamp(180.0, 500.0)
                          : 500.0;
                      _listWidth = (_listWidth + details.delta.dx).clamp(
                        180.0,
                        maxAllowed,
                      );
                    });
                  },
                  child: Container(
                    width: 1,
                    color: dividerColor,
                  ),
                ),
              ),
            ],
            // Right Diff / Preview Inspector (Matching Image 2)
            Expanded(
              child: _buildMainPane(
                selected: selected,
                rows: rows,
              ),
            ),
          ],
        );
      },
    );
  }

  Widget _buildSidebar({
    required List<_ChangeRow> rows,
    required int selectedIndex,
    required int totalAdditions,
    required int totalDeletions,
  }) {
    final theme = Theme.of(context);
    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest.withValues(
          alpha: theme.brightness == Brightness.dark ? 0.12 : 0.04,
        ),
      ),
      child: Column(
        children: <Widget>[
          // Action Bar matching Image 1: [Tree Button] [Source Dropdown] [+Add -Del]
          _SidebarTopActionBar(
            source: _source,
            mode: _leftPaneMode,
            additions: totalAdditions,
            deletions: totalDeletions,
            onToggleTreeMode: () {
              setState(() {
                if (_leftPaneMode == _LeftPaneMode.workspaceTree) {
                  _leftPaneMode = _LeftPaneMode.changes;
                } else {
                  _leftPaneMode = _LeftPaneMode.workspaceTree;
                }
              });
            },
            onCloseSidebar: () {
              setState(() {
                _previousLeftPaneMode = _leftPaneMode;
                _leftPaneMode = _LeftPaneMode.closed;
              });
            },
            onSelectSource: _switchSource,
          ),
          const Divider(height: 1),

          // Search Filter Box
          Padding(
            padding: const EdgeInsets.fromLTRB(8, 6, 8, 6),
            child: Container(
              height: 27,
              decoration: BoxDecoration(
                color: theme.colorScheme.surfaceContainerHighest.withValues(
                  alpha: 0.35,
                ),
                borderRadius: BorderRadius.circular(4),
                border: Border.all(
                  color: theme.colorScheme.outlineVariant.withValues(
                    alpha: 0.28,
                  ),
                  width: 1,
                ),
              ),
              child: Row(
                children: <Widget>[
                  Padding(
                    padding: const EdgeInsets.only(left: 7, right: 5),
                    child: Icon(
                      Icons.search,
                      size: 13,
                      color: theme.colorScheme.outline.withValues(alpha: 0.7),
                    ),
                  ),
                  Expanded(
                    child: TextField(
                      controller: _searchController,
                      cursorHeight: 13,
                      cursorWidth: 1.2,
                      style: TextStyle(
                        fontSize: 11.5,
                        color: theme.colorScheme.onSurface,
                      ),
                      decoration: InputDecoration(
                        hintText: '筛选文件...',
                        hintStyle: TextStyle(
                          fontSize: 11.5,
                          color: theme.colorScheme.outline.withValues(
                            alpha: 0.5,
                          ),
                        ),
                        isDense: true,
                        border: InputBorder.none,
                        focusedBorder: InputBorder.none,
                        enabledBorder: InputBorder.none,
                        contentPadding: const EdgeInsets.symmetric(
                          vertical: 5,
                        ),
                      ),
                    ),
                  ),
                  if (_searchController.text.isNotEmpty)
                    InkWell(
                      borderRadius: BorderRadius.circular(3),
                      onTap: _searchController.clear,
                      child: Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 6,
                          vertical: 4,
                        ),
                        child: Icon(
                          Icons.close,
                          size: 12,
                          color: theme.colorScheme.outline.withValues(
                            alpha: 0.7,
                          ),
                        ),
                      ),
                    ),
                ],
              ),
            ),
          ),
          const Divider(height: 1),

          if (_leftPaneMode == _LeftPaneMode.changes) ...<Widget>[
            Expanded(
              child: rows.isEmpty
                  ? WorkspaceDiffEmptyState(
                      message: _gitError ?? '没有匹配的改动文件',
                    )
                  : _ChangeList(
                      rows: rows,
                      selected: selectedIndex,
                      onSelected: (index) =>
                          unawaited(_selectChangeRow(rows[index])),
                    ),
            ),
          ] else ...<Widget>[
            Expanded(child: _buildTree()),
          ],
        ],
      ),
    );
  }

  Widget _buildMainPane({
    required _ChangeRow? selected,
    required List<_ChangeRow> rows,
  }) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        // Top Toolbar matching Image 2: [Nav ← →] [Opener Menu with Simple-Icons] [Layout Unified/Split]
        _DiffToolbar(
          opener: _opener,
          layout: _diffLayout,
          hasPrev: rows.isNotEmpty,
          hasNext: rows.isNotEmpty,
          sidebarOpen: _leftPaneMode != _LeftPaneMode.closed,
          wrapLines: _wrapLines,
          onPrev: () => _navigateChange(-1, rows),
          onNext: () => _navigateChange(1, rows),
          onOpenerSelected: (chosen) => _handleOpenerAction(chosen, selected),
          onLayoutChanged: (layout) => setState(() => _diffLayout = layout),
          showFullFile: _showFullFile,
          onToggleFullFile: selected == null
              ? null
              : () => _toggleShowFullFile(selected),
          onToggleSidebar: () {
            setState(() {
              if (_leftPaneMode == _LeftPaneMode.closed) {
                _leftPaneMode = _previousLeftPaneMode == _LeftPaneMode.closed
                    ? _LeftPaneMode.changes
                    : _previousLeftPaneMode;
              } else {
                _previousLeftPaneMode = _leftPaneMode;
                _leftPaneMode = _LeftPaneMode.closed;
              }
            });
          },
          onToggleWrap: () => setState(() => _wrapLines = !_wrapLines),
        ),
        const Divider(height: 1),

        // Active File Ribbon
        if (selected != null && _previewFile == null)
          _SelectedFileRibbon(row: selected),
        if (selected != null && _previewFile == null)
          Divider(
            height: 1,
            color: theme.colorScheme.outlineVariant.withValues(alpha: 0.25),
          ),

        // Body
        Expanded(
          child: _previewFile != null
              ? _FilePreview(
                  name: _previewFile!.name,
                  path: _previewFile!.relativePath,
                  loading: _previewLoading,
                  error: _previewError,
                  text: _previewText,
                )
              : selected == null
              ? WorkspaceDiffEmptyState(
                  title: '尚无文件变更',
                  subtitle: _gitError ?? '项目变更将显示在此处',
                  message: _gitError ?? '项目变更将显示在此处',
                )
              : _loadingDiff
              ? const Center(
                  child: SizedBox(
                    width: 22,
                    height: 22,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  ),
                )
              : _diffLayout == _DiffLayout.unified
              ? WorkspaceInlineDiff(
                  key: ValueKey('unified_${selected.location}'),
                  diff: selected.diff,
                  filePath: selected.location,
                  wrapLines: _wrapLines,
                  fileLines: _showFullFile ? _selectedFileLines : null,
                  showFullFile: _showFullFile,
                )
              : WorkspaceSplitDiff(
                  key: ValueKey('split_${selected.location}'),
                  diff: selected.diff,
                  filePath: selected.location,
                  wrapLines: _wrapLines,
                  fileLines: _showFullFile ? _selectedFileLines : null,
                  showFullFile: _showFullFile,
                ),
        ),
      ],
    );
  }

  Widget _buildTree() {
    if (_treeError != null) {
      return Padding(
        padding: const EdgeInsets.all(12),
        child: Text(_treeError!),
      );
    }
    if (_loading && _roots.isEmpty) {
      return const Center(
        child: SizedBox(
          width: 18,
          height: 18,
          child: CircularProgressIndicator(strokeWidth: 2),
        ),
      );
    }
    final nodes = <_TreeNode>[];
    for (final root in _roots) {
      _collectTree(nodes, root, 0);
    }
    if (nodes.isEmpty) {
      return const WorkspaceDiffEmptyState(message: '工作区暂无目录或文件');
    }
    final theme = Theme.of(context);
    return ListView.builder(
      padding: const EdgeInsets.symmetric(vertical: 4),
      itemCount: nodes.length,
      itemBuilder: (context, index) {
        final node = nodes[index];
        final expanded = _expanded.contains(node.entry.relativePath);
        final selectedFile =
            _previewFile?.relativePath == node.entry.relativePath;

        return InkWell(
          onTap: () => unawaited(_toggleTree(node.entry)),
          child: Container(
            height: 24,
            padding: EdgeInsets.only(left: 6.0 + node.depth * 14.0, right: 6),
            color: selectedFile
                ? theme.colorScheme.primary.withValues(alpha: 0.12)
                : null,
            child: Row(
              children: <Widget>[
                if (node.entry.isDirectory)
                  Icon(
                    expanded
                        ? Icons.keyboard_arrow_down
                        : Icons.keyboard_arrow_right,
                    size: 13,
                    color: theme.colorScheme.outline,
                  )
                else
                  const SizedBox(width: 13),
                const SizedBox(width: 3),
                if (node.entry.isDirectory)
                  Icon(
                    expanded ? Icons.folder_open : Icons.folder,
                    size: 14,
                    color: selectedFile
                        ? theme.colorScheme.primary
                        : const Color(0xFFEBCB8B),
                  )
                else
                  _buildFileIcon(node.entry.name, size: 14),
                const SizedBox(width: 6),
                Expanded(
                  child: Text(
                    node.entry.name,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(
                      color: selectedFile ? theme.colorScheme.primary : null,
                      fontWeight: selectedFile ? FontWeight.bold : null,
                      fontSize: 11.5,
                    ),
                  ),
                ),
              ],
            ),
          ),
        );
      },
    );
  }

  void _collectTree(
    List<_TreeNode> nodes,
    WorkspaceFileEntry entry,
    int depth,
  ) {
    nodes.add(_TreeNode(entry: entry, depth: depth));
    if (!_expanded.contains(entry.relativePath)) {
      return;
    }
    final children = _children[entry.relativePath];
    if (children == null) {
      return;
    }
    for (final child in children) {
      _collectTree(nodes, child, depth + 1);
    }
  }

  Future<void> _toggleTree(WorkspaceFileEntry entry) async {
    if (!entry.isDirectory) {
      _selectTreeFile(entry);
      return;
    }
    final path = entry.relativePath;
    if (_expanded.contains(path)) {
      setState(() => _expanded.remove(path));
      return;
    }
    final list = widget.onListWorkspaceFiles;
    if (!_children.containsKey(path)) {
      List<WorkspaceFileEntry>? children;
      if (list != null) {
        try {
          children = await list(path);
        } catch (_) {}
      }
      if (children == null || children.isEmpty) {
        try {
          final dir = Directory(entry.path);
          if (dir.existsSync()) {
            final entities = dir
                .listSync(followLinks: false)
                .where(
                  (f) =>
                      !f.path
                          .split(Platform.pathSeparator)
                          .last
                          .startsWith('.'),
                )
                .toList();
            children = entities.map((entity) {
              final name = entity.path.split(Platform.pathSeparator).last;
              final isDir = entity is Directory;
              final rel = '${entry.relativePath}/$name';
              return WorkspaceFileEntry(
                name: name,
                path: entity.path,
                relativePath: rel,
                isDirectory: isDir,
                size: isDir ? 0 : (entity is File ? entity.lengthSync() : 0),
                lastModified: '',
              );
            }).toList();
            children.sort((a, b) {
              if (a.isDirectory != b.isDirectory) {
                return a.isDirectory ? -1 : 1;
              }
              return a.name.toLowerCase().compareTo(b.name.toLowerCase());
            });
          }
        } catch (_) {}
      }
      if (!mounted) {
        return;
      }
      _children[path] = children ?? const <WorkspaceFileEntry>[];
    }
    if (!mounted) {
      return;
    }
    setState(() => _expanded.add(path));
  }

  void _selectTreeFile(WorkspaceFileEntry entry) {
    setState(() {
      _previewFile = entry;
      _previewText = null;
      _previewError = null;
      _previewLoading = true;
    });
    final read = widget.onReadWorkspaceTextFile;
    unawaited(_readTreeText(read, entry));
  }

  /// Loads the selected file's current text so the diff can show unchanged
  /// regions in full. Cached per location so switching the toggle is instant.
  Future<void> _ensureSelectedFileLines(_ChangeRow selected) async {
    final read = widget.onReadWorkspaceTextFile;
    final key = selected.location;
    if (read == null) {
      if (mounted && _selectedFileLinesKey != key) {
        setState(() {
          _selectedFileLinesKey = key;
          _selectedFileLines = null;
        });
      }
      return;
    }
    if (_selectedFileLinesKey == key) return;
    setState(() {
      _selectedFileLinesKey = key;
      _selectedFileLines = null;
    });
    try {
      final text = await read(key);
      if (!mounted || _selectedFileLinesKey != key) return;
      setState(() {
        _selectedFileLines = text.split('\n');
      });
    } catch (_) {
      if (!mounted || _selectedFileLinesKey != key) return;
      setState(() {
        _selectedFileLines = null;
      });
    }
  }

  void _toggleShowFullFile(_ChangeRow selected) {
    setState(() => _showFullFile = !_showFullFile);
    if (_showFullFile) {
      unawaited(_ensureSelectedFileLines(selected));
    }
  }

  Future<void> _readTreeText(
    Future<String> Function(String path)? read,
    WorkspaceFileEntry entry,
  ) async {
    try {
      String? text;
      if (read != null) {
        try {
          text = await read(entry.relativePath);
        } catch (_) {}
      }
      if (text == null && File(entry.path).existsSync()) {
        text = await File(entry.path).readAsString();
      }
      if (!mounted || _previewFile?.relativePath != entry.relativePath) {
        return;
      }
      setState(() {
        _previewLoading = false;
        _previewText = text ?? '';
      });
    } catch (error) {
      if (!mounted || _previewFile?.relativePath != entry.relativePath) {
        return;
      }
      setState(() {
        _previewLoading = false;
        _previewError = error;
      });
    }
  }
}

// ---------------------------------------------------------------------------
// Sidebar Action Bar (Matching Image 1: DAG Button + Source Dropdown + Counts)
// ---------------------------------------------------------------------------
class _SidebarTopActionBar extends StatelessWidget {
  const _SidebarTopActionBar({
    required this.source,
    required this.mode,
    required this.additions,
    required this.deletions,
    required this.onToggleTreeMode,
    required this.onCloseSidebar,
    required this.onSelectSource,
  });

  final _ChangeSource source;
  final _LeftPaneMode mode;
  final int additions;
  final int deletions;
  final VoidCallback onToggleTreeMode;
  final VoidCallback onCloseSidebar;
  final ValueChanged<_ChangeSource> onSelectSource;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isTree = mode == _LeftPaneMode.workspaceTree;
    final sourceLabel = switch (source) {
      _ChangeSource.git => 'Git 变更',
      _ChangeSource.branch => '分支变更',
      _ChangeSource.lastTurn => '上一轮变更',
    };

    return Container(
      height: 32,
      padding: const EdgeInsets.symmetric(horizontal: 8),
      child: LayoutBuilder(
        builder: (context, constraints) {
          return SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: ConstrainedBox(
              constraints: BoxConstraints(minWidth: constraints.maxWidth),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: <Widget>[
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      // Tree Hierarchy DAG Button: Directly toggles between Workspace Tree & Changed Files List
                      Tooltip(
                        message: isTree ? '切换至变更文件列表' : '切换至项目工作树',
                        child: InkWell(
                          borderRadius: BorderRadius.circular(5),
                          onTap: onToggleTreeMode,
                          child: Container(
                            width: 25,
                            height: 25,
                            padding: const EdgeInsets.all(4.5),
                            decoration: BoxDecoration(
                              color: isTree
                                  ? theme.colorScheme.primary.withValues(alpha: 0.18)
                                  : theme.colorScheme.surface.withValues(alpha: 0.8),
                              borderRadius: BorderRadius.circular(5),
                              border: Border.all(
                                color: isTree
                                    ? theme.colorScheme.primary.withValues(alpha: 0.5)
                                    : theme.colorScheme.outlineVariant.withValues(
                                        alpha: 0.3,
                                      ),
                              ),
                            ),
                            child: SvgPicture.string(
                              _kSvgTreeHierarchy,
                              colorFilter: ColorFilter.mode(
                                isTree
                                    ? theme.colorScheme.primary
                                    : theme.colorScheme.onSurfaceVariant,
                                BlendMode.srcIn,
                              ),
                            ),
                          ),
                        ),
                      ),
                      const SizedBox(width: 6),

                      // Source Dropdown matching Image 1: [Git 变更 / 分支变更 / 上一轮变更]
                      PopupMenuButton<_ChangeSource>(
                        tooltip: '切换变更对比源',
                        onSelected: onSelectSource,
                        padding: EdgeInsets.zero,
                        itemBuilder: (context) => <PopupMenuEntry<_ChangeSource>>[
                          PopupMenuItem<_ChangeSource>(
                            height: 32,
                            value: _ChangeSource.git,
                            child: Row(
                              children: <Widget>[
                                const Text('Git 变更', style: TextStyle(fontSize: 12)),
                                const Spacer(),
                                if (source == _ChangeSource.git)
                                  const Icon(Icons.check, size: 14),
                              ],
                            ),
                          ),
                          PopupMenuItem<_ChangeSource>(
                            height: 32,
                            value: _ChangeSource.branch,
                            child: Row(
                              children: <Widget>[
                                const Text('分支变更', style: TextStyle(fontSize: 12)),
                                const Spacer(),
                                if (source == _ChangeSource.branch)
                                  const Icon(Icons.check, size: 14),
                              ],
                            ),
                          ),
                          PopupMenuItem<_ChangeSource>(
                            height: 32,
                            value: _ChangeSource.lastTurn,
                            child: Row(
                              children: <Widget>[
                                const Text('上一轮变更', style: TextStyle(fontSize: 12)),
                                const Spacer(),
                                if (source == _ChangeSource.lastTurn)
                                  const Icon(Icons.check, size: 14),
                              ],
                            ),
                          ),
                        ],
                        child: Container(
                          height: 24,
                          padding: const EdgeInsets.symmetric(horizontal: 7),
                          decoration: BoxDecoration(
                            color: theme.colorScheme.surface.withValues(alpha: 0.8),
                            borderRadius: BorderRadius.circular(5),
                            border: Border.all(
                              color: theme.colorScheme.outlineVariant.withValues(
                                alpha: 0.3,
                              ),
                            ),
                          ),
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: <Widget>[
                              Text(
                                sourceLabel,
                                style: theme.textTheme.labelSmall?.copyWith(
                                  fontWeight: FontWeight.w600,
                                  fontSize: 11.5,
                                ),
                              ),
                              const SizedBox(width: 2),
                              Icon(
                                Icons.arrow_drop_down,
                                size: 15,
                                color: theme.colorScheme.onSurfaceVariant,
                              ),
                            ],
                          ),
                        ),
                      ),
                    ],
                  ),
                  const SizedBox(width: 6),

                  // Colored Totals: +27738 -20065
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      _StatText(
                        value: '+$additions',
                        color: const Color(0xFF3FB950),
                      ),
                      const SizedBox(width: 5),
                      _StatText(
                        value: '-$deletions',
                        color: const Color(0xFFF85149),
                      ),
                      const SizedBox(width: 6),
                      Tooltip(
                        message: '关闭文件区',
                        child: InkWell(
                          borderRadius: BorderRadius.circular(4),
                          onTap: onCloseSidebar,
                          child: Container(
                            width: 20,
                            height: 20,
                            alignment: Alignment.center,
                            decoration: BoxDecoration(
                              color: theme.colorScheme.surface.withValues(alpha: 0.8),
                              borderRadius: BorderRadius.circular(4),
                              border: Border.all(
                                color: theme.colorScheme.outlineVariant.withValues(
                                  alpha: 0.3,
                                ),
                              ),
                            ),
                            child: Icon(
                              Icons.view_sidebar_outlined,
                              size: 13,
                              color: theme.colorScheme.onSurfaceVariant,
                            ),
                          ),
                        ),
                      ),
                    ],
                  ),
                ],
              ),
            ),
          );
        },
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Right Diff Toolbar (Matching Image 2: Simple-Icons Opener Menu)
// ---------------------------------------------------------------------------
class _DiffToolbar extends StatelessWidget {
  const _DiffToolbar({
    required this.opener,
    required this.layout,
    required this.hasPrev,
    required this.hasNext,
    required this.sidebarOpen,
    required this.wrapLines,
    required this.onPrev,
    required this.onNext,
    required this.onOpenerSelected,
    required this.onLayoutChanged,
    required this.showFullFile,
    required this.onToggleFullFile,
    required this.onToggleSidebar,
    required this.onToggleWrap,
  });

  final WorkspaceOpenerType opener;
  final _DiffLayout layout;
  final bool hasPrev;
  final bool hasNext;
  final bool sidebarOpen;
  final bool wrapLines;
  final bool showFullFile;
  final VoidCallback onPrev;
  final VoidCallback onNext;
  final ValueChanged<WorkspaceOpenerType> onOpenerSelected;
  final ValueChanged<_DiffLayout> onLayoutChanged;
  final VoidCallback? onToggleFullFile;
  final VoidCallback onToggleSidebar;
  final VoidCallback onToggleWrap;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Container(
      height: 30,
      padding: const EdgeInsets.symmetric(horizontal: 6),
      color: theme.colorScheme.surfaceContainerHighest.withValues(alpha: 0.25),
      child: LayoutBuilder(
        builder: (context, constraints) {
          return SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            child: ConstrainedBox(
              constraints: BoxConstraints(minWidth: constraints.maxWidth),
              child: Row(
                mainAxisAlignment: MainAxisAlignment.spaceBetween,
                children: <Widget>[
                  Row(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      // Toggle Sidebar Button (open/close left pane)
                      IconButton(
                        tooltip: sidebarOpen ? '收起文件区' : '展开文件区',
                        onPressed: onToggleSidebar,
                        icon: Icon(
                          Icons.view_sidebar_outlined,
                          size: 14,
                          color: sidebarOpen
                              ? theme.colorScheme.onSurfaceVariant
                              : theme.colorScheme.primary,
                        ),
                        visualDensity: VisualDensity.compact,
                        padding: EdgeInsets.zero,
                        constraints: const BoxConstraints(
                          minWidth: 22,
                          minHeight: 22,
                        ),
                      ),
                      const SizedBox(
                        height: 14,
                        child: VerticalDivider(width: 8),
                      ),
                      // Jump Previous
                      IconButton(
                        tooltip: '上一个改动 (P)',
                        onPressed: hasPrev ? onPrev : null,
                        icon: const Icon(Icons.arrow_back, size: 14),
                        visualDensity: VisualDensity.compact,
                        padding: EdgeInsets.zero,
                        constraints: const BoxConstraints(
                          minWidth: 22,
                          minHeight: 22,
                        ),
                      ),
                      // Jump Next
                      IconButton(
                        tooltip: '下一个改动 (N)',
                        onPressed: hasNext ? onNext : null,
                        icon: const Icon(Icons.arrow_forward, size: 14),
                        visualDensity: VisualDensity.compact,
                        padding: EdgeInsets.zero,
                        constraints: const BoxConstraints(
                          minWidth: 22,
                          minHeight: 22,
                        ),
                      ),
                      const SizedBox(
                        height: 14,
                        child: VerticalDivider(width: 8),
                      ),

                      // External Opener Dropdown with Simple-Icons (Matching Image 2)
                      PopupMenuButton<WorkspaceOpenerType>(
                        tooltip: '选择打开方式',
                        onSelected: onOpenerSelected,
                        padding: EdgeInsets.zero,
                        itemBuilder: (context) => <PopupMenuEntry<WorkspaceOpenerType>>[
                          const PopupMenuItem<WorkspaceOpenerType>(
                            enabled: false,
                            height: 26,
                            child: Text(
                              '打开方式',
                              style: TextStyle(
                                fontSize: 10.5,
                                fontWeight: FontWeight.bold,
                                color: Colors.grey,
                              ),
                            ),
                          ),
                          PopupMenuItem<WorkspaceOpenerType>(
                            height: 32,
                            value: WorkspaceOpenerType.explorer,
                            child: Row(
                              children: <Widget>[
                                const Icon(
                                  Icons.folder,
                                  size: 15,
                                  color: Color(0xFFFFA000),
                                ),
                                const SizedBox(width: 8),
                                const Text('文件资源管理器', style: TextStyle(fontSize: 12)),
                                if (opener == WorkspaceOpenerType.explorer) ...<Widget>[
                                  const Spacer(),
                                  const Icon(Icons.check, size: 14),
                                ],
                              ],
                            ),
                          ),
                          PopupMenuItem<WorkspaceOpenerType>(
                            height: 32,
                            value: WorkspaceOpenerType.vsCode,
                            child: Row(
                              children: <Widget>[
                                SvgPicture.string(
                                  _kSvgVsCode,
                                  width: 15,
                                  height: 15,
                                ),
                                const SizedBox(width: 8),
                                const Text('Visual Studio Code',
                                    style: TextStyle(fontSize: 12)),
                                if (opener == WorkspaceOpenerType.vsCode) ...<Widget>[
                                  const Spacer(),
                                  const Icon(Icons.check, size: 14),
                                ],
                              ],
                            ),
                          ),
                          PopupMenuItem<WorkspaceOpenerType>(
                            height: 32,
                            value: WorkspaceOpenerType.terminal,
                            child: Row(
                              children: <Widget>[
                                SvgPicture.string(
                                  _kSvgTerminal,
                                  width: 15,
                                  height: 15,
                                ),
                                const SizedBox(width: 8),
                                const Text('终端 (Terminal)',
                                    style: TextStyle(fontSize: 12)),
                                if (opener == WorkspaceOpenerType.terminal) ...<Widget>[
                                  const Spacer(),
                                  const Icon(Icons.check, size: 14),
                                ],
                              ],
                            ),
                          ),
                          const PopupMenuDivider(height: 6),
                          const PopupMenuItem<WorkspaceOpenerType>(
                            height: 30,
                            value: WorkspaceOpenerType.copyPath,
                            child: Row(
                              children: <Widget>[
                                Icon(Icons.copy, size: 14),
                                SizedBox(width: 8),
                                Text('复制路径', style: TextStyle(fontSize: 12)),
                              ],
                            ),
                          ),
                        ],
                        child: Container(
                          height: 22,
                          padding: const EdgeInsets.symmetric(horizontal: 6),
                          decoration: BoxDecoration(
                            color: theme.colorScheme.surface.withValues(alpha: 0.6),
                            borderRadius: BorderRadius.circular(4),
                            border: Border.all(
                              color: theme.colorScheme.outlineVariant.withValues(
                                alpha: 0.3,
                              ),
                            ),
                          ),
                          child: Row(
                            mainAxisSize: MainAxisSize.min,
                            children: <Widget>[
                              _buildOpenerIcon(opener),
                              const SizedBox(width: 4),
                              Icon(
                                Icons.arrow_drop_down,
                                size: 14,
                                color: theme.colorScheme.onSurfaceVariant,
                              ),
                            ],
                          ),
                        ),
                      ),
                    ],
                  ),

                  const SizedBox(width: 8),

                  // Layout Switcher: Unified vs Split
                  Container(
                    height: 22,
                    padding: const EdgeInsets.all(1.5),
                    decoration: BoxDecoration(
                      color: theme.colorScheme.surface.withValues(alpha: 0.6),
                      borderRadius: BorderRadius.circular(4),
                      border: Border.all(
                        color: theme.colorScheme.outlineVariant.withValues(alpha: 0.25),
                      ),
                    ),
                    child: Row(
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        Tooltip(
                          message: '统一视图 (Unified)',
                          child: InkWell(
                            onTap: () => onLayoutChanged(_DiffLayout.unified),
                            child: Container(
                              padding: const EdgeInsets.symmetric(horizontal: 5),
                              decoration: BoxDecoration(
                                color: layout == _DiffLayout.unified
                                    ? theme.colorScheme.primary.withValues(alpha: 0.15)
                                    : null,
                                borderRadius: BorderRadius.circular(3),
                              ),
                              child: Icon(
                                Icons.table_rows_outlined,
                                size: 14,
                                color: layout == _DiffLayout.unified
                                    ? theme.colorScheme.primary
                                    : theme.colorScheme.outline,
                              ),
                            ),
                          ),
                        ),
                        Tooltip(
                          message: '分栏视图 (Split)',
                          child: InkWell(
                            onTap: () => onLayoutChanged(_DiffLayout.split),
                            child: Container(
                              padding: const EdgeInsets.symmetric(horizontal: 5),
                              decoration: BoxDecoration(
                                color: layout == _DiffLayout.split
                                    ? theme.colorScheme.primary.withValues(alpha: 0.15)
                                    : null,
                                borderRadius: BorderRadius.circular(3),
                              ),
                              child: Icon(
                                Icons.vertical_split_outlined,
                                size: 14,
                                color: layout == _DiffLayout.split
                                    ? theme.colorScheme.primary
                                    : theme.colorScheme.outline,
                              ),
                            ),
                          ),
                        ),
                      ],
                    ),
                  ),

                  const SizedBox(width: 6),

                  // Full File Switcher: fills unchanged gaps with real lines.
                  Tooltip(
                    message: onToggleFullFile == null
                        ? '请选择文件后再切换完整视图'
                        : showFullFile
                        ? '仅显示改动附近的行 (收起未更改区域)'
                        : '完整展示整个文件 (展开未更改区域)',
                    child: InkWell(
                      onTap: onToggleFullFile,
                      child: Container(
                        height: 22,
                        padding: const EdgeInsets.symmetric(horizontal: 5),
                        decoration: BoxDecoration(
                          color: showFullFile
                              ? theme.colorScheme.primary.withValues(alpha: 0.15)
                              : theme.colorScheme.surface.withValues(alpha: 0.6),
                          borderRadius: BorderRadius.circular(4),
                          border: Border.all(
                            color: showFullFile
                                ? theme.colorScheme.primary.withValues(alpha: 0.5)
                                : theme.colorScheme.outlineVariant.withValues(alpha: 0.25),
                          ),
                        ),
                        child: Icon(
                          showFullFile
                              ? Icons.unfold_more_outlined
                              : Icons.compress_outlined,
                          size: 14,
                          color: onToggleFullFile == null
                              ? theme.colorScheme.outlineVariant
                              : showFullFile
                              ? theme.colorScheme.primary
                              : theme.colorScheme.outline,
                        ),
                      ),
                    ),
                  ),

                  const SizedBox(width: 6),

                  // Word Wrap Switcher
                  Tooltip(
                    message: wrapLines ? '关闭换行 (不换行)' : '开启换行 (自动换行)',
                    child: InkWell(
                      onTap: onToggleWrap,
                      child: Container(
                        height: 22,
                        padding: const EdgeInsets.symmetric(horizontal: 5),
                        decoration: BoxDecoration(
                          color: wrapLines
                              ? theme.colorScheme.primary.withValues(alpha: 0.15)
                              : theme.colorScheme.surface.withValues(alpha: 0.6),
                          borderRadius: BorderRadius.circular(4),
                          border: Border.all(
                            color: wrapLines
                                ? theme.colorScheme.primary.withValues(alpha: 0.5)
                                : theme.colorScheme.outlineVariant.withValues(alpha: 0.25),
                          ),
                        ),
                        child: Icon(
                          Icons.wrap_text_outlined,
                          size: 14,
                          color: wrapLines
                              ? theme.colorScheme.primary
                              : theme.colorScheme.outline,
                        ),
                      ),
                    ),
                  ),
                ],
              ),
            ),
          );
        },
      ),
    );
  }

  Widget _buildOpenerIcon(WorkspaceOpenerType type) {
    switch (type) {
      case WorkspaceOpenerType.explorer:
        return const Icon(
          Icons.folder,
          size: 14,
          color: Color(0xFFFFA000),
        );
      case WorkspaceOpenerType.vsCode:
        return SvgPicture.string(_kSvgVsCode, width: 14, height: 14);
      case WorkspaceOpenerType.terminal:
        return SvgPicture.string(_kSvgTerminal, width: 14, height: 14);
      case WorkspaceOpenerType.copyPath:
        return const Icon(Icons.copy, size: 14);
    }
  }
}

// ---------------------------------------------------------------------------
// Selected File Ribbon (Matching OpenCode Image 2: [Status] [Icon] Name path/)
// ---------------------------------------------------------------------------
class _SelectedFileRibbon extends StatelessWidget {
  const _SelectedFileRibbon({required this.row});

  final _ChangeRow row;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final statusColor = _statusColor(row.status);

    return Container(
      height: 32,
      padding: const EdgeInsets.symmetric(horizontal: 10),
      color: theme.colorScheme.surface.withValues(alpha: 0.35),
      child: Row(
        children: <Widget>[
          // Status badge matching OpenCode Image 2: [D] / [M] / [A]
          Container(
            padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 1),
            decoration: BoxDecoration(
              color: statusColor.withValues(alpha: 0.15),
              borderRadius: BorderRadius.circular(3),
            ),
            child: Text(
              row.status,
              style: TextStyle(
                color: statusColor,
                fontWeight: FontWeight.bold,
                fontSize: 10.5,
              ),
            ),
          ),
          const SizedBox(width: 7),
          _buildFileIcon(row.name, size: 14),
          const SizedBox(width: 7),
          Expanded(
            child: Text.rich(
              TextSpan(
                children: <TextSpan>[
                  // File name in bold prominence
                  TextSpan(
                    text: row.name,
                    style: TextStyle(
                      color: theme.colorScheme.onSurface,
                      fontWeight: FontWeight.w700,
                      fontSize: 12,
                    ),
                  ),
                  // Parent directory in subtle muted gray
                  if (row.directory != null && row.directory!.isNotEmpty) ...<TextSpan>[
                    const TextSpan(text: '  '),
                    TextSpan(
                      text: '${row.directory}/',
                      style: TextStyle(
                        color: theme.colorScheme.outline.withValues(alpha: 0.70),
                        fontSize: 11.5,
                      ),
                    ),
                  ],
                ],
              ),
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ),
          const SizedBox(width: 8),
          _StatText(
            value: '+${row.additions}',
            color: const Color(0xFF3FB950),
          ),
          const SizedBox(width: 4),
          _StatText(
            value: '-${row.deletions}',
            color: const Color(0xFFF85149),
          ),
        ],
      ),
    );
  }
}

// ---------------------------------------------------------------------------
// Compact Change List
// ---------------------------------------------------------------------------
class _ChangeList extends StatelessWidget {
  const _ChangeList({
    required this.rows,
    required this.selected,
    required this.onSelected,
  });

  final List<_ChangeRow> rows;
  final int selected;
  final ValueChanged<int> onSelected;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return ListView.builder(
      padding: const EdgeInsets.symmetric(vertical: 2),
      itemCount: rows.length,
      itemBuilder: (context, index) {
        final row = rows[index];
        final isSelected = index == selected;

        return InkWell(
          onTap: () => onSelected(index),
          child: Container(
            height: 26,
            padding: const EdgeInsets.symmetric(horizontal: 8),
            decoration: BoxDecoration(
              color: isSelected
                  ? theme.colorScheme.primary.withValues(alpha: 0.15)
                  : null,
              border: isSelected
                  ? Border(
                      left: BorderSide(
                        color: theme.colorScheme.primary,
                        width: 2.5,
                      ),
                    )
                  : null,
            ),
            child: Row(
              children: <Widget>[
                // File icon
                _buildFileIcon(row.name, size: 14),
                const SizedBox(width: 6),

                // File name & directory
                Expanded(
                  child: Text.rich(
                    TextSpan(
                      children: <TextSpan>[
                        TextSpan(
                          text: '${row.name} ',
                          style: TextStyle(
                            color: isSelected
                                ? theme.colorScheme.primary
                                : theme.colorScheme.onSurface,
                            fontWeight: isSelected
                                ? FontWeight.bold
                                : FontWeight.w500,
                            fontSize: 11.5,
                          ),
                        ),
                        if (row.directory != null && row.directory!.isNotEmpty)
                          TextSpan(
                            text: row.directory,
                            style: TextStyle(
                              color: theme.colorScheme.outline,
                              fontSize: 10,
                            ),
                          ),
                      ],
                    ),
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                ),

                // Additions / Deletions
                if (row.additions > 0) ...<Widget>[
                  Text(
                    '+${row.additions}',
                    style: const TextStyle(
                      color: Color(0xFF3FB950),
                      fontSize: 10.5,
                      fontFamily: 'monospace',
                    ),
                  ),
                  const SizedBox(width: 3),
                ],
                if (row.deletions > 0)
                  Text(
                    '-${row.deletions}',
                    style: const TextStyle(
                      color: Color(0xFFF85149),
                      fontSize: 10.5,
                      fontFamily: 'monospace',
                    ),
                  ),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _StatText extends StatelessWidget {
  const _StatText({required this.value, required this.color});

  final String value;
  final Color color;

  @override
  Widget build(BuildContext context) {
    return Text(
      value,
      maxLines: 1,
      style: TextStyle(
        color: color,
        fontWeight: FontWeight.bold,
        fontFamily: 'monospace',
        fontSize: 11,
      ),
    );
  }
}

class _FilePreview extends StatelessWidget {
  const _FilePreview({
    required this.name,
    required this.path,
    required this.loading,
    required this.error,
    required this.text,
  });

  final String name;
  final String path;
  final bool loading;
  final Object? error;
  final String? text;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        Padding(
          padding: const EdgeInsets.fromLTRB(12, 8, 12, 8),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Row(
                children: <Widget>[
                  Icon(
                    Icons.description_outlined,
                    size: 16,
                    color: theme.colorScheme.primary,
                  ),
                  const SizedBox(width: 7),
                  Expanded(
                    child: Text(
                      name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.titleSmall,
                    ),
                  ),
                ],
              ),
              const SizedBox(height: 2),
              Text(
                path,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: theme.colorScheme.outline,
                ),
              ),
            ],
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: loading
              ? const Center(
                  child: SizedBox(
                    width: 18,
                    height: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  ),
                )
              : error != null
              ? Padding(
                  padding: const EdgeInsets.all(12),
                  child: Text('$error'),
                )
              : SingleChildScrollView(
                  padding: const EdgeInsets.all(12),
                  child: SelectableText(
                    text ?? '',
                    style: theme.textTheme.bodySmall?.copyWith(
                      fontFamily: 'monospace',
                    ),
                  ),
                ),
        ),
      ],
    );
  }
}

Color _statusColor(String status) {
  return switch (status) {
    'U' || 'A' => const Color(0xFF3FB950),
    'D' => const Color(0xFFF85149),
    'R' => const Color(0xFFD29922),
    _ => const Color(0xFF58A6FF),
  };
}

Widget buildWorkspaceFileIcon(String name, {double size = 14}) {
  final ext = name.contains('.') ? name.split('.').last.toLowerCase() : '';
  final lowerName = name.toLowerCase();

  if (lowerName == 'dockerfile' || lowerName.startsWith('dockerfile.')) {
    return SvgPicture.string(_kSvgLangDocker, width: size, height: size);
  }
  if (lowerName.startsWith('.git') ||
      lowerName == '.gitignore' ||
      lowerName == '.gitmodules') {
    return SvgPicture.string(_kSvgLangGit, width: size, height: size);
  }

  final svgString = switch (ext) {
    'c' || 'h' => _kSvgLangC,
    'cpp' || 'cc' || 'cxx' || 'hpp' || 'hxx' => _kSvgLangCpp,
    'py' || 'pyw' || 'ipynb' => _kSvgLangPython,
    'rs' => _kSvgLangRust,
    'dart' => _kSvgLangDart,
    'ts' || 'tsx' || 'mts' || 'cts' => _kSvgLangTs,
    'js' || 'jsx' || 'mjs' || 'cjs' => _kSvgLangJs,
    'go' => _kSvgLangGo,
    'java' || 'class' || 'jar' => _kSvgLangJava,
    'kt' || 'kts' => _kSvgLangKotlin,
    'swift' => _kSvgLangSwift,
    'html' || 'htm' => _kSvgLangHtml,
    'css' || 'scss' || 'less' => _kSvgLangCss,
    'json' || 'jsonc' => _kSvgLangJson,
    'yaml' || 'yml' || 'toml' => _kSvgLangYaml,
    'md' || 'markdown' => _kSvgLangMarkdown,
    'sh' || 'bash' || 'zsh' || 'ps1' || 'bat' || 'cmd' => _kSvgLangShell,
    'sql' => _kSvgLangSql,
    'lock' => _kSvgLangLock,
    _ => _kSvgLangDefault,
  };

  return SvgPicture.string(svgString, width: size, height: size);
}

Widget _buildFileIcon(String name, {double size = 14}) =>
    buildWorkspaceFileIcon(name, size: size);

String _fileName(String path) {
  final normalized = path.replaceAll('\\', '/');
  final segments = normalized.split('/');
  if (segments.isEmpty || segments.last.isEmpty) {
    return path;
  }
  return segments.last;
}

String? _parentDirectory(String path) {
  final separator = path.contains('\\') ? '\\' : '/';
  final index = path.lastIndexOf(separator);
  if (index <= 0) {
    return null;
  }
  return path.substring(0, index);
}

String _joinPath(String root, String relativePath) {
  final separator = root.contains('\\') ? '\\' : '/';
  final normalized = relativePath.replaceAll('/', separator);
  if (root.endsWith(separator)) {
    return '$root$normalized';
  }
  return '$root$separator$normalized';
}