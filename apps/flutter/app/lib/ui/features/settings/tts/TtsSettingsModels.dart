// ignore_for_file: file_names

part of 'TtsSettingsPanel.dart';

class _TtsSettingsData {
  const _TtsSettingsData({
    required this.configs,
    required this.currentConfigId,
    required this.providerCatalogEntries,
    required this.characterBoundConfigIds,
    required this.sttConfigs,
    required this.currentSttConfigId,
    required this.sttProviderCatalogEntries,
  });

  final List<core_proxy.TtsConfig> configs;
  final String currentConfigId;
  final List<core_proxy.TtsProviderCatalogEntry> providerCatalogEntries;
  final Set<String> characterBoundConfigIds;
  final List<core_proxy.SttConfig> sttConfigs;
  final String? currentSttConfigId;
  final List<core_proxy.SttProviderCatalogEntry> sttProviderCatalogEntries;
}

class _TtsProviderGroup {
  const _TtsProviderGroup({
    required this.key,
    required this.title,
    required this.providerType,
    required this.configs,
  });

  final String key;
  final String title;
  final String providerType;
  final List<core_proxy.TtsConfig> configs;
}

class _TtsProviderTypes {
  const _TtsProviderTypes._();

  static const String system = 'SYSTEM_TTS';
  static const String http = 'HTTP_TTS';
  static const String localModel = 'LOCAL_MODEL';
}

bool _isSystemProviderType(String providerType) {
  return providerType.trim().toUpperCase() == _TtsProviderTypes.system;
}

bool _isHttpProviderType(String providerType) {
  return providerType.trim().toUpperCase() == _TtsProviderTypes.http;
}

bool _isLocalModelProviderType(String providerType) {
  return providerType.trim().toUpperCase() == _TtsProviderTypes.localModel;
}

List<DropdownMenuItem<String>> _providerCatalogItems(
  List<core_proxy.TtsProviderCatalogEntry> entries,
) {
  return entries
      .map(
        (entry) => DropdownMenuItem<String>(
          value: entry.providerTypeId,
          child: Text(entry.displayName),
        ),
      )
      .toList(growable: false);
}

core_proxy.TtsProviderCatalogEntry _requiredTtsProviderCatalog(
  List<core_proxy.TtsProviderCatalogEntry> entries,
  String providerType,
) {
  final normalized = providerType.trim().toUpperCase();
  for (final entry in entries) {
    if (entry.providerTypeId.trim().toUpperCase() == normalized) {
      return entry;
    }
  }
  throw StateError('TTS provider catalog not found: $providerType');
}

bool _ttsProviderCatalogUsesPlaceholder(
  core_proxy.TtsProviderCatalogEntry catalog,
  String name,
) {
  return _templateHasPlaceholder(catalog.defaultEndpoint, name) ||
      _templateHasPlaceholder(catalog.defaultRequestBody, name) ||
      catalog.defaultHeaders.any(
        (header) => _templateHasPlaceholder(header.value, name),
      );
}

bool _ttsConfigUsesPlaceholder(core_proxy.TtsConfig config, String name) {
  return _templateHasPlaceholder(config.endpoint, name) ||
      _templateHasPlaceholder(config.requestBody, name) ||
      config.headers.any(
        (header) => _templateHasPlaceholder(header.value, name),
      );
}

bool _templateHasPlaceholder(String template, String name) {
  final needle = '{$name}';
  for (var index = 0; index <= template.length - needle.length; index++) {
    if (template.startsWith(needle, index)) {
      return true;
    }
  }
  return false;
}

List<_TtsProviderGroup> _ttsProviderGroups(List<core_proxy.TtsConfig> configs) {
  final byKey = <String, List<core_proxy.TtsConfig>>{};
  for (final config in configs) {
    final key = _ttsProviderGroupKey(config);
    byKey.putIfAbsent(key, () => <core_proxy.TtsConfig>[]).add(config);
  }
  final groups = <_TtsProviderGroup>[];
  for (final entry in byKey.entries) {
    final first = entry.value.first;
    groups.add(
      _TtsProviderGroup(
        key: entry.key,
        title: first.name,
        providerType: _ttsConfigProviderTypeText(first),
        configs: entry.value..sort(_compareTtsConfig),
      ),
    );
  }
  groups.sort(
    (left, right) =>
        left.title.toLowerCase().compareTo(right.title.toLowerCase()),
  );
  return groups;
}

String _ttsProviderGroupKey(core_proxy.TtsConfig config) {
  return '${config.providerType}\u001F${config.endpoint}\u001F${config.name}';
}

int _compareTtsConfig(core_proxy.TtsConfig left, core_proxy.TtsConfig right) {
  return _ttsConfigModelVoiceText(
    left,
  ).toLowerCase().compareTo(_ttsConfigModelVoiceText(right).toLowerCase());
}

String _ttsConfigProviderTypeText(core_proxy.TtsConfig config) {
  final endpoint = config.endpoint.trim();
  if (endpoint.isEmpty) {
    return config.providerType;
  }
  return '${config.providerType} · $endpoint';
}

String _ttsConfigModelVoiceText(core_proxy.TtsConfig config) {
  final model = config.model.trim();
  final voice = config.voice.trim();
  if (model.isEmpty && voice.isEmpty) {
    return '系统默认音色';
  }
  if (model.isEmpty) {
    return voice;
  }
  if (voice.isEmpty) {
    return model;
  }
  return '$model · $voice';
}

String? _ttsConfigDeleteBlockedReason(
  core_proxy.TtsConfig config,
  String currentConfigId,
  Set<String> characterBoundConfigIds,
  AppLocalizations l10n,
) {
  final id = config.id.trim();
  if (id == currentConfigId.trim()) {
    return l10n.settingsTtsCurrentConfigCannotDelete;
  }
  if (characterBoundConfigIds.contains(id)) {
    return l10n.settingsTtsConfigUsedByCharacter;
  }
  return null;
}

/// Returns why an entire TTS provider group cannot be deleted.
String? _ttsProviderGroupDeleteBlockedReason(
  _TtsProviderGroup group,
  String currentConfigId,
  Set<String> characterBoundConfigIds,
  AppLocalizations l10n,
) {
  for (final config in group.configs) {
    final reason = _ttsConfigDeleteBlockedReason(
      config,
      currentConfigId,
      characterBoundConfigIds,
      l10n,
    );
    if (reason != null) {
      return reason;
    }
  }
  return null;
}

String _availableTtsVoiceTitle(core_proxy.AvailableTtsVoice voice) {
  final displayName = voice.displayName.trim();
  if (displayName.isNotEmpty) {
    return displayName;
  }
  final model = voice.model.trim();
  final voiceName = voice.voice.trim();
  if (model.isEmpty && voiceName.isEmpty) {
    return '系统默认音色';
  }
  if (model.isEmpty) {
    return voiceName;
  }
  if (voiceName.isEmpty) {
    return model;
  }
  return '$model · $voiceName';
}

String _availableTtsVoiceSubtitle(core_proxy.AvailableTtsVoice voice) {
  final labels = <String>[];
  final model = voice.model.trim();
  final voiceName = voice.voice.trim();
  if (model.isNotEmpty) {
    labels.add(model);
  }
  if (voiceName.isNotEmpty) {
    labels.add(voiceName);
  }
  final responseFormat = voice.responseFormat.trim();
  if (responseFormat.isNotEmpty) {
    labels.add(responseFormat);
  }
  labels.add(voice.speed.toStringAsFixed(2));
  final description = voice.description.trim();
  if (description.isNotEmpty) {
    labels.add(description);
  }
  return labels.join(' · ');
}

List<core_proxy.TtsHttpHeader> _decodeHeaders(String raw, String label) {
  final decoded = _decodeJsonList(raw, label);
  return decoded
      .map((item) => core_proxy.TtsHttpHeader.fromJson(item))
      .toList(growable: false);
}

List<core_proxy.TtsHttpResponsePipelineStep> _decodePipeline(
  String raw,
  String label,
) {
  final decoded = _decodeJsonList(raw, label);
  return decoded
      .map((item) {
        final headersValue = item['headers'];
        final headers = headersValue == null
            ? const <core_proxy.TtsHttpHeader>[]
            : (headersValue as List<Object?>)
                  .map(
                    (header) => core_proxy.TtsHttpHeader.fromJson(
                      header as Map<String, Object?>,
                    ),
                  )
                  .toList(growable: false);
        final stepType =
            (item['stepType'] as String?) ?? (item['type'] as String);
        return core_proxy.TtsHttpResponsePipelineStep(
          stepType: stepType,
          path: item['path'] as String,
          headers: headers,
        );
      })
      .toList(growable: false);
}

List<Map<String, Object?>> _decodeJsonList(String raw, String label) {
  final text = raw.isEmpty ? '[]' : raw;
  final decoded = jsonDecode(text);
  if (decoded is! List<Object?>) {
    throw FormatException('$label 必须是数组');
  }
  return decoded
      .map((item) {
        if (item is! Map<String, Object?>) {
          throw FormatException('$label 每一项必须是对象');
        }
        return item;
      })
      .toList(growable: false);
}

String _encodeJsonList(List<Object> value) {
  return const JsonEncoder.withIndent(
    '  ',
  ).convert(value.map((item) => (item as dynamic).toJson()).toList());
}
