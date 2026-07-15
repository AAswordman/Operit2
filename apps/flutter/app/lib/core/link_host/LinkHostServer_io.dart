// ignore_for_file: file_names, unused_element

import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import '../link/CoreLinkProtocol.dart';
import '../path/OperitClientPaths.dart';
import '../runtime/RuntimeDeviceInfoProvider.dart';
import 'LinkHostConfig.dart';

const String _webAccessAssetPrefix = 'assets/web_access/';
const String _webAccessVersionFile = 'web_access_version.json';
const String _webAccessVersionAsset =
    '$_webAccessAssetPrefix$_webAccessVersionFile';

class LinkHostServer extends ChangeNotifier {
  LinkHostServer._();

  static final LinkHostServer instance = LinkHostServer._();
  static const MethodChannel _runtimeChannel = MethodChannel('operit/runtime');

  bool _running = false;
  LinkHostConfig? _config;
  String? _shutdownToken;
  Timer? _pairingCodePoller;
  int _pairingCodeStartedAt = 0;
  PendingLinkPairingCodeRecord? _lastPairingCode;

  bool get isRunning => _running;
  LinkHostConfig? get currentConfig => _config;
  PendingLinkPairingCodeRecord? get lastPairingCode => _lastPairingCode;

  String? get baseUrl {
    final config = _config;
    if (config == null || !_running) {
      return null;
    }
    return _baseUrlForBindAddress(config.bindAddress);
  }

  Future<String> discoverDevices(int timeoutMs) async {
    final responseText = await _runtimeChannel.invokeMethod<String>(
      'discoverDevices',
      <String, Object?>{'timeoutMs': timeoutMs},
    );
    return responseText ?? '[]';
  }

  Future<List<String>> pairingBaseUrls(LinkHostConfig config) async {
    final endpoint = _parseBindAddress(config.bindAddress);
    if (_isWildcardHost(endpoint.host)) {
      final hosts = await _lanIpv4Hosts();
      return hosts
          .map((host) => 'http://$host:${endpoint.port}')
          .toList(growable: false);
    }
    if (_isLoopbackHost(endpoint.host)) {
      return <String>[];
    }
    return <String>['http://${endpoint.host}:${endpoint.port}'];
  }

  Future<void> initializeFromConfig() async {
    final config = await LinkHostConfigStore.read();
    if (config.webAccessEnabled || config.discoveryEnabled) {
      await start(config);
    }
  }

  Future<void> start(LinkHostConfig config) async {
    if (_running) {
      await stop(updateConfig: false);
    }
    final webRoot = await _materializeWebAccessBundle();
    final shutdownToken = LinkHostToken.generate();
    _shutdownToken = shutdownToken;
    _pairingCodeStartedAt = DateTime.now().millisecondsSinceEpoch;
    late final LinkHostConfig runningConfig;
    try {
      runningConfig = await _startNativeWebAccessServerWithPortMode(
        config,
        shutdownToken,
        webRoot,
      );
    } catch (_) {
      _config = null;
      _shutdownToken = null;
      rethrow;
    }
    _config = runningConfig;
    _running = true;
    _startPairingCodePolling();
    await _writeState(runningConfig);
  }

  Future<void> stop({bool updateConfig = true}) async {
    if (!_running) {
      return;
    }
    final shutdownToken = _shutdownToken;
    final baseUrl = this.baseUrl;
    if (shutdownToken != null && baseUrl != null) {
      await _requestNativeWebAccessClose(baseUrl, shutdownToken);
    }
    await _stopNativeWebAccessServer();
    _running = false;
    _pairingCodePoller?.cancel();
    _pairingCodePoller = null;
    _config = null;
    _shutdownToken = null;
    await _removeState();
    if (updateConfig) {
      final config = await LinkHostConfigStore.read();
      await LinkHostConfigStore.write(
        config.copyWith(
          webAccessEnabled: false,
          updatedAt: DateTime.now().millisecondsSinceEpoch,
        ),
      );
    }
  }

  void _startPairingCodePolling() {
    _pairingCodePoller?.cancel();
    _pairingCodePoller = Timer.periodic(const Duration(seconds: 1), (_) {
      unawaited(_pollPairingCode());
    });
    unawaited(_pollPairingCode());
  }

  Future<void> _pollPairingCode() async {
    final record = await PendingLinkPairingCodeStore.read();
    if (record == null || record.createdAt < _pairingCodeStartedAt) {
      return;
    }
    if (_lastPairingCode?.pairingId == record.pairingId) {
      return;
    }
    _lastPairingCode = record;
    notifyListeners();
  }

  Future<void> _writeState(LinkHostConfig config) async {
    final file = await OperitClientPaths.linkHostStateFile();
    await file.parent.create(recursive: true);
    final deviceId = await LinkHostDeviceIdStore.read();
    final content = const JsonEncoder.withIndent('  ').convert({
      'deviceId': deviceId,
      'bindAddress': config.bindAddress,
      'baseUrl': _baseUrlForBindAddress(config.bindAddress),
      'webAccessEnabled': config.webAccessEnabled,
      'discoveryEnabled': config.discoveryEnabled,
      'shutdownToken': _shutdownToken,
      'processId': pid,
      'startedAt': DateTime.now().millisecondsSinceEpoch,
    });
    await file.writeAsString(content);
  }

  Future<void> _removeState() async {
    final file = await OperitClientPaths.linkHostStateFile();
    if (await file.exists()) {
      await file.delete();
    }
  }

  Future<Directory> _materializeWebAccessBundle() async {
    final directory = await OperitClientPaths.linkHostWebAccessBundleDir();
    final bundledVersion = await _readBundledWebAccessVersion();
    if (await _isMaterializedWebAccessCurrent(directory, bundledVersion)) {
      return directory;
    }
    if (await directory.exists()) {
      await directory.delete(recursive: true);
    }
    await directory.create(recursive: true);
    final manifest = await AssetManifest.loadFromAssetBundle(rootBundle);
    final assetKeys =
        manifest
            .listAssets()
            .where((key) => key.startsWith(_webAccessAssetPrefix))
            .toList(growable: false)
          ..sort();
    if (!assetKeys.contains(_webAccessVersionAsset)) {
      throw StateError('Web Access version asset is not bundled');
    }
    for (final assetKey in assetKeys) {
      final relativePath = assetKey.substring(_webAccessAssetPrefix.length);
      final bytes = await rootBundle.load(assetKey);
      final file = File(
        _joinPath(<String>[directory.path, ...relativePath.split('/')]),
      );
      await file.parent.create(recursive: true);
      await file.writeAsBytes(
        bytes.buffer.asUint8List(bytes.offsetInBytes, bytes.lengthInBytes),
      );
    }
    return directory;
  }

  Future<void> _startNativeWebAccessServer(
    LinkHostConfig config,
    String shutdownToken,
    Directory webRoot,
  ) async {
    final acceptedSessions = await InboundLinkSessionStore.read();
    final acceptedSessionsFile =
        await OperitClientPaths.inboundLinkSessionsFile();
    final pairingCodeFile =
        await OperitClientPaths.pendingLinkPairingCodeFile();
    final deviceId = await LinkHostDeviceIdStore.read();
    final deviceInfo = await RuntimeDeviceInfoProvider.current();
    final responseText = await _runtimeChannel
        .invokeMethod<String>('startWebAccessServer', <String, Object?>{
          'bindAddress': config.bindAddress,
          'token': config.token,
          'shutdownToken': shutdownToken,
          'webRoot': webRoot.path,
          'deviceId': deviceId,
          'acceptedSessions': jsonEncode(
            acceptedSessions.map((key, value) => MapEntry(key, value.toJson())),
          ),
          'acceptedSessionStorePath': acceptedSessionsFile.path,
          'pairingCodePath': pairingCodeFile.path,
          'deviceInfo': jsonEncode(deviceInfo.toJson()),
          'enableWebAccess': config.webAccessEnabled.toString(),
          'enableDiscovery': config.discoveryEnabled.toString(),
        });
    _throwNativeWebAccessError(responseText);
  }

  Future<LinkHostConfig> _startNativeWebAccessServerWithPortMode(
    LinkHostConfig config,
    String shutdownToken,
    Directory webRoot,
  ) async {
    if (config.portMode == LinkHostPortMode.fixed) {
      await _startNativeWebAccessServer(config, shutdownToken, webRoot);
      return config;
    }
    final endpoint = _parseBindAddress(config.bindAddress);
    Object? lastError;
    StackTrace? lastStackTrace;
    for (final bindAddress in _automaticBindAddresses(endpoint.host)) {
      final candidate = config.copyWith(bindAddress: bindAddress);
      try {
        await _startNativeWebAccessServer(candidate, shutdownToken, webRoot);
        return candidate;
      } catch (error, stackTrace) {
        lastError = error;
        lastStackTrace = stackTrace;
      }
    }
    if (lastError != null && lastStackTrace != null) {
      Error.throwWithStackTrace(lastError, lastStackTrace);
    }
    throw StateError('no web access ports configured');
  }

  Future<void> _stopNativeWebAccessServer() async {
    final responseText = await _runtimeChannel.invokeMethod<String>(
      'stopWebAccessServer',
    );
    _throwNativeWebAccessError(responseText);
  }

  Future<void> _requestNativeWebAccessClose(
    String baseUrl,
    String shutdownToken,
  ) async {
    final client = HttpClient();
    try {
      final request = await client.postUrl(
        Uri.parse('$baseUrl/client/web-access/close'),
      );
      request.headers.set('x-operit-web-access-shutdown-token', shutdownToken);
      final response = await request.close();
      final body = await utf8.decoder.bind(response).join();
      if (response.statusCode < 200 || response.statusCode >= 300) {
        throw StateError('web access close failed: $body');
      }
    } finally {
      client.close(force: true);
    }
  }

  void _throwNativeWebAccessError(String? responseText) {
    if (responseText == null) {
      throw const CoreLinkError(
        code: 'EMPTY_RESPONSE',
        message: 'runtime bridge returned empty web access response',
      );
    }
    final response = jsonDecode(responseText) as Map<String, Object?>;
    if (response['ok'] == true) {
      return;
    }
    if (response.containsKey('code') && response.containsKey('message')) {
      throw CoreLinkError.fromJson(response);
    }
    throw CoreLinkError(
      code: 'INVALID_RESPONSE',
      message: 'runtime bridge web access response is invalid',
    );
  }
}

class _WebAccessBundleVersion {
  const _WebAccessBundleVersion({
    required this.version,
    required this.contentHash,
  });

  final int version;
  final String contentHash;

  /// Decodes one Web Access version manifest.
  factory _WebAccessBundleVersion.fromJsonString(
    String content,
    String source,
  ) {
    final decoded = jsonDecode(content);
    if (decoded is! Map) {
      throw FormatException(
        'Web Access version manifest must be an object',
        source,
      );
    }
    final manifest = decoded.cast<String, Object?>();
    final schemaVersion = manifest['schemaVersion'];
    final version = manifest['version'];
    final contentHash = manifest['contentHash'];
    if (schemaVersion != 1) {
      throw FormatException(
        'Unexpected Web Access manifest schema: $schemaVersion',
        source,
      );
    }
    if (version is! int || version < 1) {
      throw FormatException('Invalid Web Access version: $version', source);
    }
    if (contentHash is! String || contentHash.isEmpty) {
      throw FormatException('Invalid Web Access content hash', source);
    }
    return _WebAccessBundleVersion(version: version, contentHash: contentHash);
  }

  /// Returns whether two manifests describe the same generated bundle.
  bool matches(_WebAccessBundleVersion other) {
    return version == other.version && contentHash == other.contentHash;
  }
}

/// Reads the bundled Web Access version manifest from Flutter assets.
Future<_WebAccessBundleVersion> _readBundledWebAccessVersion() async {
  return _WebAccessBundleVersion.fromJsonString(
    await rootBundle.loadString(_webAccessVersionAsset),
    _webAccessVersionAsset,
  );
}

/// Reads the materialized Web Access version manifest from the client data dir.
Future<_WebAccessBundleVersion?> _readMaterializedWebAccessVersion(
  Directory directory,
) async {
  final file = File(_joinPath(<String>[directory.path, _webAccessVersionFile]));
  if (!await file.exists()) {
    return null;
  }
  return _WebAccessBundleVersion.fromJsonString(
    await file.readAsString(),
    file.path,
  );
}

/// Returns whether the client data Web Access bundle matches bundled assets.
Future<bool> _isMaterializedWebAccessCurrent(
  Directory directory,
  _WebAccessBundleVersion bundledVersion,
) async {
  final materializedVersion = await _readMaterializedWebAccessVersion(
    directory,
  );
  return materializedVersion?.matches(bundledVersion) ?? false;
}

class _BindEndpoint {
  const _BindEndpoint({required this.host, required this.port});

  final String host;
  final int port;
}

_BindEndpoint _parseBindAddress(String bindAddress) {
  final index = bindAddress.lastIndexOf(':');
  if (index <= 0 || index == bindAddress.length - 1) {
    throw FormatException('invalid bind address: $bindAddress');
  }
  return _BindEndpoint(
    host: bindAddress.substring(0, index),
    port: int.parse(bindAddress.substring(index + 1)),
  );
}

String _baseUrlForBindAddress(String bindAddress) {
  final endpoint = _parseBindAddress(bindAddress);
  final host = switch (endpoint.host) {
    '0.0.0.0' => '127.0.0.1',
    '::' => '127.0.0.1',
    _ => endpoint.host,
  };
  return 'http://$host:${endpoint.port}';
}

bool _isWildcardHost(String host) {
  return host == '0.0.0.0' || host == '::';
}

bool _isLoopbackHost(String host) {
  return host == '127.0.0.1' || host == 'localhost' || host == '::1';
}

Future<List<String>> _lanIpv4Hosts() async {
  final interfaces = await NetworkInterface.list(
    includeLoopback: false,
    type: InternetAddressType.IPv4,
  );
  final hosts = <String>{};
  for (final interface in interfaces) {
    for (final address in interface.addresses) {
      if (!address.isLoopback && !address.isLinkLocal) {
        hosts.add(address.address);
      }
    }
  }
  final sorted = hosts.toList(growable: false)..sort();
  return sorted;
}

List<String> _automaticBindAddresses(String host) {
  return LinkHostConfig.automaticPortSequence
      .map((port) => '$host:$port')
      .toList(growable: false);
}

String _joinPath(List<String> segments) {
  return segments.join(Platform.pathSeparator);
}
