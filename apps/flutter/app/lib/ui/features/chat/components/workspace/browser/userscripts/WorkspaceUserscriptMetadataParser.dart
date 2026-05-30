// ignore_for_file: file_names

import 'WorkspaceUserscriptModels.dart';

WorkspaceUserscriptMetadata parseWorkspaceUserscriptMetadata(String source) {
  final values = <String, List<String>>{};
  final lines = source.split(RegExp(r'\r?\n'));
  var inBlock = false;
  for (final line in lines) {
    final trimmed = line.trim();
    if (trimmed == '// ==UserScript==') {
      inBlock = true;
      continue;
    }
    if (trimmed == '// ==/UserScript==') {
      break;
    }
    if (!inBlock || !trimmed.startsWith('// @')) {
      continue;
    }
    final body = trimmed.substring(4).trim();
    final splitIndex = body.indexOf(RegExp(r'\s'));
    final key = splitIndex < 0 ? body : body.substring(0, splitIndex);
    final value = splitIndex < 0 ? '' : body.substring(splitIndex).trim();
    values.putIfAbsent(key, () => <String>[]).add(value);
  }
  return WorkspaceUserscriptMetadata(
    name: _first(values, 'name') ?? '未命名脚本',
    namespace: _first(values, 'namespace') ?? '',
    version: _first(values, 'version') ?? '',
    description: _first(values, 'description') ?? '',
    author: _first(values, 'author') ?? '',
    homepage: _first(values, 'homepage') ?? '',
    website: _first(values, 'website') ?? '',
    supportUrl: _first(values, 'supportURL') ?? '',
    downloadUrl: _first(values, 'downloadURL') ?? '',
    updateUrl: _first(values, 'updateURL') ?? '',
    matches: values['match'] ?? const <String>[],
    includes: values['include'] ?? const <String>[],
    excludes: values['exclude'] ?? const <String>[],
    excludeMatches: values['exclude-match'] ?? const <String>[],
    grants: values['grant'] ?? const <String>[],
    connects: values['connect'] ?? const <String>[],
    requires: values['require'] ?? const <String>[],
    resources: _resources(values['resource'] ?? const <String>[]),
    runAt: _runAt(_first(values, 'run-at')),
    noFrames: values.containsKey('noframes'),
  );
}

String? _first(Map<String, List<String>> values, String key) {
  final list = values[key];
  if (list == null || list.isEmpty) {
    return null;
  }
  return list.first;
}

WorkspaceUserscriptRunAt _runAt(String? value) {
  switch (value) {
    case 'document-start':
      return WorkspaceUserscriptRunAt.documentStart;
    case 'document-idle':
      return WorkspaceUserscriptRunAt.documentIdle;
    case 'document-end':
    default:
      return WorkspaceUserscriptRunAt.documentEnd;
  }
}

List<WorkspaceUserscriptResource> _resources(List<String> values) {
  return values
      .map((value) {
        final parts = value.split(RegExp(r'\s+'));
        if (parts.length < 2) {
          return null;
        }
        return WorkspaceUserscriptResource(
          name: parts.first,
          url: parts.sublist(1).join(' '),
        );
      })
      .whereType<WorkspaceUserscriptResource>()
      .toList(growable: false);
}
