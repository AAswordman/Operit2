// ignore_for_file: file_names

import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';

class CharacterAvatarStore {
  static const String runtimeStoragePrefix =
      'runtime/data/user_assets/character_avatars/';

  /// Creates a character avatar store backed by runtime storage.
  CharacterAvatarStore({
    GeneratedRepositoryRuntimeStorageRepositoryCoreProxy? runtimeStorage,
  }) : _runtimeStorage =
           runtimeStorage ??
           const GeneratedCoreProxyClients(
             ProxyCoreRuntimeBridge(),
           ).repositoryRuntimeStorageRepository;

  final GeneratedRepositoryRuntimeStorageRepositoryCoreProxy _runtimeStorage;

  /// Imports a selected avatar file into runtime storage.
  Future<String> importFile(XFile file) async {
    return importBytes(bytes: await file.readAsBytes(), fileName: file.name);
  }

  /// Imports avatar bytes into runtime storage and returns their stable path.
  Future<String> importBytes({
    required Uint8List bytes,
    required String fileName,
  }) async {
    final extension = _characterAvatarFileExtension(fileName);
    final directory = await _runtimeStorage.characterAvatarsDirPath();
    final digest = sha256.convert(bytes).toString().substring(0, 16);
    final storagePath = '$directory/$digest.$extension';
    await _runtimeStorage.writeBase64(
      path: storagePath,
      base64Content: base64Encode(bytes),
    );
    return storagePath;
  }

  /// Reads an avatar asset from runtime storage.
  Future<Uint8List> readBytes(String storagePath) async {
    final base64Content = await _runtimeStorage.readBase64(path: storagePath);
    if (base64Content == null) {
      throw StateError('character avatar is missing: $storagePath');
    }
    return base64Decode(base64Content);
  }
}

class CharacterAvatarImage extends StatefulWidget {
  /// Creates an image widget backed by a character avatar runtime asset.
  const CharacterAvatarImage({
    super.key,
    required this.avatarUri,
    required this.fit,
  });

  static const String defaultAvatarAsset = 'assets/images/operit_avatar.png';

  final String? avatarUri;
  final BoxFit fit;

  /// Creates the state object that loads avatar bytes.
  @override
  State<CharacterAvatarImage> createState() => _CharacterAvatarImageState();
}

class _CharacterAvatarImageState extends State<CharacterAvatarImage> {
  Future<Uint8List>? _bytesFuture;

  /// Starts loading avatar bytes for the initial image frame.
  @override
  void initState() {
    super.initState();
    _refreshBytesFuture();
  }

  /// Reloads avatar bytes when the referenced runtime asset changes.
  @override
  void didUpdateWidget(covariant CharacterAvatarImage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.avatarUri != widget.avatarUri) {
      _refreshBytesFuture();
    }
  }

  /// Builds the decoded avatar image once its runtime bytes are available.
  @override
  Widget build(BuildContext context) {
    final bytesFuture = _bytesFuture;
    if (bytesFuture == null) {
      return SizedBox.expand(
        child: Image.asset(CharacterAvatarImage.defaultAvatarAsset, fit: widget.fit),
      );
    }
    return FutureBuilder<Uint8List>(
      future: bytesFuture,
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          Error.throwWithStackTrace(
            snapshot.error!,
            snapshot.stackTrace ?? StackTrace.current,
          );
        }
        final bytes = snapshot.data;
        if (bytes == null) {
          return const SizedBox.expand();
        }
        return SizedBox.expand(
          child: Image.memory(
            bytes,
            fit: widget.fit,
            width: double.infinity,
            height: double.infinity,
          ),
        );
      },
    );
  }

  /// Refreshes the runtime image request for the current avatar URI.
  void _refreshBytesFuture() {
    final avatarUri = widget.avatarUri?.trim();
    _bytesFuture = avatarUri == null || avatarUri.isEmpty
        ? null
        : CharacterAvatarStore().readBytes(avatarUri);
  }
}

/// Validates and returns the supported image extension for one avatar file.
String _characterAvatarFileExtension(String fileName) {
  final dotIndex = fileName.lastIndexOf('.');
  if (dotIndex <= 0 || dotIndex == fileName.length - 1) {
    throw ArgumentError.value(
      fileName,
      'fileName',
      'avatar file must have an extension',
    );
  }
  final extension = fileName.substring(dotIndex + 1).toLowerCase();
  const allowedExtensions = <String>{
    'jpg',
    'jpeg',
    'png',
    'webp',
    'bmp',
    'gif',
  };
  if (!allowedExtensions.contains(extension)) {
    throw ArgumentError.value(
      fileName,
      'fileName',
      'avatar file type is not supported',
    );
  }
  return extension;
}
