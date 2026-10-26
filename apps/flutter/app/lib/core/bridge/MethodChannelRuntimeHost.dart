// ignore_for_file: file_names

import 'dart:convert';

import 'package:flutter/services.dart';

import 'CoreProxy.dart';

class MethodChannelRuntimeHost implements PlatformRuntimeHost {
  /// Creates the platform lifecycle and storage adapter.
  const MethodChannelRuntimeHost({
    MethodChannel channel = const MethodChannel('operit/runtime'),
  }) : _channel = channel;

  final MethodChannel _channel;

  /// Validates the offline Host response before exposing maintenance results.
  @override
  Future<Object?> dataRescue(Map<String, Object?> request) async {
    final encoded = await _channel.invokeMethod<String>(
      'dataRescue',
      jsonEncode(request),
    );
    if (encoded == null) {
      throw StateError('Data rescue returned an empty response');
    }
    final response = jsonDecode(encoded) as Map<String, Object?>;
    if (response['ok'] != true) throw StateError(response['error'] as String);
    return response['value'];
  }

  /// Reads the native platform's default Runtime storage roots.
  @override
  Future<Map<Object?, Object?>> runtimeStorageDefaults() async {
    final result = await _channel.invokeMapMethod<Object?, Object?>(
      'localRuntimeStorageDefaults',
    );
    if (result == null) {
      throw StateError('local runtime storage defaults response is empty');
    }
    return result;
  }

  /// Normalizes explicit Runtime storage roots through the native Host.
  @override
  Future<Map<Object?, Object?>> runtimeStoragePaths(
    String runtimeRoot,
    String workspaceRoot,
  ) async {
    final result = await _channel.invokeMapMethod<Object?, Object?>(
      'localRuntimeStoragePaths',
      <String, Object?>{
        'runtimeRoot': runtimeRoot,
        'workspaceRoot': workspaceRoot,
      },
    );
    if (result == null) {
      throw StateError('local runtime storage paths response is empty');
    }
    return result;
  }

  /// Installs identity-isolated Runtime storage roots on the native Host.
  @override
  Future<void> setRuntimeStorageRoots(
    String runtimeRoot,
    String workspaceRoot,
  ) {
    return _channel.invokeMethod<void>(
      'setLocalRuntimeStorage',
      <String, Object?>{
        'runtimeRoot': runtimeRoot,
        'workspaceRoot': workspaceRoot,
      },
    );
  }

  /// Reads and validates the Rust bootstrap storage response.
  @override
  Future<String?> runtimeBootstrapRead() async {
    final encoded = await _channel.invokeMethod<String>('runtimeBootstrapRead');
    if (encoded == null) {
      throw StateError('runtime bootstrap read response is empty');
    }
    final response = jsonDecode(encoded) as Map<String, Object?>;
    if (response['ok'] != true) {
      throw StateError(response['error'] as String);
    }
    return response['value'] as String?;
  }

  /// Persists one bootstrap record through the Rust storage Host.
  @override
  Future<void> runtimeBootstrapWrite(String content) async {
    final encoded = await _channel.invokeMethod<String>(
      'runtimeBootstrapWrite',
      content,
    );
    if (encoded == null) {
      throw StateError('runtime bootstrap write response is empty');
    }
    final response = jsonDecode(encoded) as Map<String, Object?>;
    if (response['ok'] != true) {
      throw StateError(response['error'] as String);
    }
  }

  /// Requests the required native application exit after an identity change.
  @override
  Future<void> restartApplication() {
    return _channel.invokeMethod<void>('restartApplication');
  }
}
