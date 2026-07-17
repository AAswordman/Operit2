// ignore_for_file: file_names, unused_element

import 'dart:convert';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import '../link/CoreLinkProtocol.dart';
import '../path/OperitClientPaths.dart';
import '../runtime/RuntimeDeviceInfoProvider.dart';
import 'LinkAccessHostConfig.dart';

const String _webAccessAssetPrefix = 'assets/web_access/';
const String _webAccessVersionFile = 'web_access_version.json';
const String _webAccessVersionAsset =
    '$_webAccessAssetPrefix$_webAccessVersionFile';

class LinkAccessHost extends ChangeNotifier {
  LinkAccessHost._();

  static final LinkAccessHost instance = LinkAccessHost._();
  static const MethodChannel _runtimeChannel = MethodChannel('operit/runtime');

  bool _running = false;
  LinkAccessHostConfig? _config;
  String? _shutdownToken;
  String? _deviceId;

  bool get isRunning => _running;
  LinkAccessHostConfig? get currentConfig => _config;
  String? get deviceId => _deviceId;
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

  Future<List<String>> pairingBaseUrls(LinkAccessHostConfig config) async {
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
    final config = await LinkAccessHostConfigStore.read();
    if (config.webAccessEnabled || config.discoveryEnabled) {
      await start(config);
    }
  }

  Future<void> start(LinkAccessHostConfig config) async {
    if (_running) {
      await stop(updateConfig: false);
    }
    final webRoot = await _materializeWebAccessBundle();
    final shutdownToken = LinkAccessHostToken.generate();
    _shutdownToken = shutdownToken;
    late final ({LinkAccessHostConfig config, String deviceId}) started;
    try {
      started = await _startNativeWebAccessServerWithPortMode(
        config,
        shutdownToken,
        webRoot,
      );
    } catch (_) {
      _config = null;
      _shutdownToken = null;
      rethrow;
    }
    _config = started.config;
    _deviceId = started.deviceId;
    _running = true;
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
    _config = null;
    _shutdownToken = null;
    _deviceId = null;
    if (updateConfig) {
      final config = await LinkAccessHostConfigStore.read();
      await LinkAccessHostConfigStore.write(
        config.copyWith(
          webAccessEnabled: false,
          updatedAt: DateTime.now().millisecondsSinceEpoch,
        ),
      );
    }
  }

  Future<Directory> _materializeWebAccessBundle() async {
    final directory = await OperitClientPaths.linkAccessWebAccessBundleDir();
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

  /// Starts the native Access Host and returns its Core-owned identity.
  Future<String> _startNativeWebAccessServer(
    LinkAccessHostConfig config,
    String shutdownToken,
    Directory webRoot,
  ) async {
    final deviceInfo = await RuntimeDeviceInfoProvider.current();
    final responseText = await _runtimeChannel
        .invokeMethod<String>('startWebAccessServer', <String, Object?>{
          'bindAddress': config.bindAddress,
          'token': config.token,
          'shutdownToken': shutdownToken,
          'webRoot': webRoot.path,
          'deviceInfo': jsonEncode(deviceInfo.toJson()),
          'enableWebAccess': config.webAccessEnabled.toString(),
          'enableDiscovery': config.discoveryEnabled.toString(),
        });
    final response = _throwNativeWebAccessError(responseText);
    final deviceId = response['deviceId'];
    if (deviceId is! String || deviceId.isEmpty) {
      throw const CoreLinkError(
        code: 'INVALID_RESPONSE',
        message: 'runtime bridge did not return the Link Access identity',
      );
    }
    return deviceId;
  }

  /// Starts the native Access Host on the configured port selection.
  Future<({LinkAccessHostConfig config, String deviceId})>
  _startNativeWebAccessServerWithPortMode(
    LinkAccessHostConfig config,
    String shutdownToken,
    Directory webRoot,
  ) async {
    if (config.portMode == LinkAccessHostPortMode.fixed) {
      final deviceId = await _startNativeWebAccessServer(
        config,
        shutdownToken,
        webRoot,
      );
      return (config: config, deviceId: deviceId);
    }
    final endpoint = _parseBindAddress(config.bindAddress);
    Object? lastError;
    StackTrace? lastStackTrace;
    for (final bindAddress in _automaticBindAddresses(endpoint.host)) {
      final candidate = config.copyWith(bindAddress: bindAddress);
      try {
        final deviceId = await _startNativeWebAccessServer(
          candidate,
          shutdownToken,
          webRoot,
        );
        return (config: candidate, deviceId: deviceId);
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

  /// Validates a native Access Host response and returns its JSON payload.
  Map<String, Object?> _throwNativeWebAccessError(String? responseText) {
    if (responseText == null) {
      throw const CoreLinkError(
        code: 'EMPTY_RESPONSE',
        message: 'runtime bridge returned empty web access response',
      );
    }
    final response = jsonDecode(responseText) as Map<String, Object?>;
    if (response['ok'] == true) {
      return response;
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
  return LinkAccessHostConfig.automaticPortSequence
      .map((port) => '$host:$port')
      .toList(growable: false);
}

String _joinPath(List<String> segments) {
  return segments.join(Platform.pathSeparator);
}
