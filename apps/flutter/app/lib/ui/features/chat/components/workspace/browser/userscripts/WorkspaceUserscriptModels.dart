// ignore_for_file: file_names

enum WorkspaceUserscriptRunAt { documentStart, documentEnd, documentIdle }

class WorkspaceUserscriptMetadata {
  const WorkspaceUserscriptMetadata({
    required this.name,
    required this.namespace,
    required this.version,
    required this.description,
    required this.author,
    required this.homepage,
    required this.website,
    required this.supportUrl,
    required this.downloadUrl,
    required this.updateUrl,
    required this.matches,
    required this.includes,
    required this.excludes,
    required this.excludeMatches,
    required this.grants,
    required this.connects,
    required this.requires,
    required this.resources,
    required this.runAt,
    required this.noFrames,
  });

  final String name;
  final String namespace;
  final String version;
  final String description;
  final String author;
  final String homepage;
  final String website;
  final String supportUrl;
  final String downloadUrl;
  final String updateUrl;
  final List<String> matches;
  final List<String> includes;
  final List<String> excludes;
  final List<String> excludeMatches;
  final List<String> grants;
  final List<String> connects;
  final List<String> requires;
  final List<WorkspaceUserscriptResource> resources;
  final WorkspaceUserscriptRunAt runAt;
  final bool noFrames;

  factory WorkspaceUserscriptMetadata.fromJson(Map<String, Object?> json) {
    return WorkspaceUserscriptMetadata(
      name: json['name'] as String,
      namespace: json['namespace'] as String,
      version: json['version'] as String,
      description: json['description'] as String,
      author: json['author'] as String,
      homepage: json['homepage'] as String,
      website: json['website'] as String,
      supportUrl: json['supportUrl'] as String,
      downloadUrl: json['downloadUrl'] as String,
      updateUrl: json['updateUrl'] as String,
      matches: _stringList(json['matches']),
      includes: _stringList(json['includes']),
      excludes: _stringList(json['excludes']),
      excludeMatches: _stringList(json['excludeMatches']),
      grants: _stringList(json['grants']),
      connects: _stringList(json['connects']),
      requires: _stringList(json['requires']),
      resources: (json['resources'] as List<Object?>)
          .map(
            (item) => WorkspaceUserscriptResource.fromJson(
              item as Map<String, Object?>,
            ),
          )
          .toList(growable: false),
      runAt: WorkspaceUserscriptRunAt.values.byName(json['runAt'] as String),
      noFrames: json['noFrames'] as bool,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'name': name,
      'namespace': namespace,
      'version': version,
      'description': description,
      'author': author,
      'homepage': homepage,
      'website': website,
      'supportUrl': supportUrl,
      'downloadUrl': downloadUrl,
      'updateUrl': updateUrl,
      'matches': matches,
      'includes': includes,
      'excludes': excludes,
      'excludeMatches': excludeMatches,
      'grants': grants,
      'connects': connects,
      'requires': requires,
      'resources': resources
          .map((item) => item.toJson())
          .toList(growable: false),
      'runAt': runAt.name,
      'noFrames': noFrames,
    };
  }
}

class WorkspaceUserscriptResource {
  const WorkspaceUserscriptResource({required this.name, required this.url});

  final String name;
  final String url;

  factory WorkspaceUserscriptResource.fromJson(Map<String, Object?> json) {
    return WorkspaceUserscriptResource(
      name: json['name'] as String,
      url: json['url'] as String,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{'name': name, 'url': url};
  }
}

class WorkspaceUserscriptItem {
  const WorkspaceUserscriptItem({
    required this.id,
    required this.enabled,
    required this.source,
    required this.metadata,
    required this.knownGrants,
    required this.unknownGrants,
    required this.blockedReasons,
    this.values = const <String, Object?>{},
    this.requireSources = const <String, String>{},
    this.resourceTexts = const <String, String>{},
    this.resourceUrls = const <String, String>{},
    this.sourceUrl,
  });

  final String id;
  final bool enabled;
  final String source;
  final WorkspaceUserscriptMetadata metadata;
  final List<String> knownGrants;
  final List<String> unknownGrants;
  final List<String> blockedReasons;
  final Map<String, Object?> values;
  final Map<String, String> requireSources;
  final Map<String, String> resourceTexts;
  final Map<String, String> resourceUrls;
  final String? sourceUrl;

  factory WorkspaceUserscriptItem.fromJson(Map<String, Object?> json) {
    return WorkspaceUserscriptItem(
      id: json['id'] as String,
      enabled: json['enabled'] as bool,
      source: json['source'] as String,
      metadata: WorkspaceUserscriptMetadata.fromJson(
        json['metadata'] as Map<String, Object?>,
      ),
      knownGrants: _stringList(json['knownGrants']),
      unknownGrants: _stringList(json['unknownGrants']),
      blockedReasons: _stringList(json['blockedReasons']),
      values:
          (json['values'] as Map<Object?, Object?>?)?.map(
            (key, value) => MapEntry(key as String, value),
          ) ??
          const <String, Object?>{},
      requireSources: _stringMap(json['requireSources']),
      resourceTexts: _stringMap(json['resourceTexts']),
      resourceUrls: _stringMap(json['resourceUrls']),
      sourceUrl: json['sourceUrl'] as String?,
    );
  }

  Map<String, Object?> toJson() {
    return <String, Object?>{
      'id': id,
      'enabled': enabled,
      'source': source,
      'metadata': metadata.toJson(),
      'knownGrants': knownGrants,
      'unknownGrants': unknownGrants,
      'blockedReasons': blockedReasons,
      'values': values,
      'requireSources': requireSources,
      'resourceTexts': resourceTexts,
      'resourceUrls': resourceUrls,
      'sourceUrl': sourceUrl,
    };
  }

  WorkspaceUserscriptItem copyWith({
    bool? enabled,
    Map<String, Object?>? values,
    Map<String, String>? requireSources,
    Map<String, String>? resourceTexts,
    Map<String, String>? resourceUrls,
  }) {
    return WorkspaceUserscriptItem(
      id: id,
      enabled: enabled ?? this.enabled,
      source: source,
      metadata: metadata,
      knownGrants: knownGrants,
      unknownGrants: unknownGrants,
      blockedReasons: blockedReasons,
      values: values ?? this.values,
      requireSources: requireSources ?? this.requireSources,
      resourceTexts: resourceTexts ?? this.resourceTexts,
      resourceUrls: resourceUrls ?? this.resourceUrls,
      sourceUrl: sourceUrl,
    );
  }
}

List<String> _stringList(Object? value) {
  return (value as List<Object?>).cast<String>();
}

Map<String, String> _stringMap(Object? value) {
  final map = value as Map<Object?, Object?>?;
  if (map == null) {
    return const <String, String>{};
  }
  return map.map((key, value) => MapEntry(key as String, value as String));
}

class WorkspaceUserscriptInstallPreview {
  const WorkspaceUserscriptInstallPreview({
    required this.source,
    required this.metadata,
    required this.knownGrants,
    required this.unknownGrants,
    required this.blockedReasons,
    this.sourceUrl,
  });

  final String source;
  final WorkspaceUserscriptMetadata metadata;
  final List<String> knownGrants;
  final List<String> unknownGrants;
  final List<String> blockedReasons;
  final String? sourceUrl;
}

class WorkspaceUserscriptLogItem {
  const WorkspaceUserscriptLogItem({
    required this.scriptName,
    required this.message,
    required this.createdAt,
  });

  final String scriptName;
  final String message;
  final DateTime createdAt;
}

class WorkspaceUserscriptPageRun {
  const WorkspaceUserscriptPageRun({
    required this.scriptId,
    required this.scriptName,
    required this.url,
    required this.status,
    required this.message,
    required this.createdAt,
  });

  final String scriptId;
  final String scriptName;
  final String url;
  final String status;
  final String message;
  final DateTime createdAt;
}

class WorkspaceUserscriptMenuCommand {
  const WorkspaceUserscriptMenuCommand({
    required this.index,
    required this.scriptName,
    required this.caption,
  });

  final int index;
  final String scriptName;
  final String caption;

  factory WorkspaceUserscriptMenuCommand.fromJson(Map<String, Object?> json) {
    return WorkspaceUserscriptMenuCommand(
      index: json['index'] as int,
      scriptName: json['scriptName'] as String,
      caption: json['caption'] as String,
    );
  }
}
