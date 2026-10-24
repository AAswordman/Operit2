// Copyright 2013 The Flutter Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

import 'dart:async';
import 'dart:js_interop';
import 'dart:js_interop_unsafe';

import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';
import 'package:flutter/foundation.dart' show visibleForTesting;
import 'package:flutter_web_plugins/flutter_web_plugins.dart';

import 'package:web/web.dart' as web;

import 'src/dom_helper.dart';
import 'src/utils.dart';

/// The web implementation of [FileSelectorPlatform].
///
/// This class implements the `package:file_selector` functionality for the web.
class FileSelectorWeb extends FileSelectorPlatform {
  /// Default constructor, initializes _domHelper that we can use
  /// to interact with the DOM.
  /// overrides parameter allows for testing to override functions
  FileSelectorWeb({@visibleForTesting DomHelper? domHelper})
    : _domHelper = domHelper ?? DomHelper();

  final DomHelper _domHelper;

  /// Registers this class as the default instance of [FileSelectorPlatform].
  static void registerWith(Registrar registrar) {
    FileSelectorPlatform.instance = FileSelectorWeb();
  }

  /// Counts streamed writes while preserving browser backpressure and user activation.
  @override
  Future<FileSaveLocation?> saveGeneratedFileWithProgress({
    required Future<XFile> Function() generate,
    required void Function() onChooseDestination,
    required Future<void> Function(XFile file, String destination) write,
    required void Function(int completed, int total) onProgress,
    List<XTypeGroup>? acceptedTypeGroups,
    SaveDialogOptions options = const SaveDialogOptions(),
  }) async {
    final web.FileSystemFileHandle handle;
    try {
      handle = await _SavePickerWindow(web.window)
          .showSaveFilePicker(
            <String, Object?>{'suggestedName': options.suggestedName}.jsify()!
                as JSObject,
          )
          .toDart;
    } catch (error) {
      final exception = error as JSObject;
      if (exception.getProperty<JSString>('name'.toJS).toDart == 'AbortError')
        return null;
      rethrow;
    }
    final output = await handle.createWritable().toDart;
    try {
      final file = await generate();
      final total = await file.length();
      var completed = 0;
      onProgress(completed, total);
      final response = await web.window.fetch(file.path.toJS).toDart;
      if (!response.ok || response.body == null) {
        throw StateError('Unable to read the generated backup');
      }
      final reader =
          response.body!.getReader() as web.ReadableStreamDefaultReader;
      try {
        while (true) {
          final chunk = await reader.read().toDart;
          if (chunk.done) break;
          final bytes = chunk.value! as JSUint8Array;
          await output.write(bytes).toDart;
          completed += bytes.toDart.length;
          onProgress(completed, total);
        }
      } finally {
        await reader.cancel().toDart;
        reader.releaseLock();
      }
      if (completed != total) throw StateError('Incomplete generated backup');
      await output.close().toDart;
      return FileSaveLocation(handle.name);
    } catch (error) {
      await output.abort().toDart;
      rethrow;
    }
  }

  /// Acquires the browser output under the user gesture and pipes the generated file with backpressure.
  @override
  Future<FileSaveLocation?> saveGeneratedFile({
    required Future<XFile> Function() generate,
    List<XTypeGroup>? acceptedTypeGroups,
    SaveDialogOptions options = const SaveDialogOptions(),
  }) async {
    final web.FileSystemFileHandle handle;
    try {
      handle = await _SavePickerWindow(web.window)
          .showSaveFilePicker(
            <String, Object?>{'suggestedName': options.suggestedName}.jsify()!
                as JSObject,
          )
          .toDart;
    } catch (error) {
      final exception = error as JSObject;
      if (exception.getProperty<JSString>('name'.toJS).toDart == 'AbortError') {
        return null;
      }
      rethrow;
    }
    final output = await handle.createWritable().toDart;
    try {
      final file = await generate();
      final response = await web.window.fetch(file.path.toJS).toDart;
      if (!response.ok || response.body == null) {
        throw StateError('Unable to read the generated snapshot file');
      }
      await response.body!.pipeTo(output).toDart;
      return FileSaveLocation(handle.name);
    } catch (error) {
      await output.abort().toDart;
      rethrow;
    }
  }

  @override
  Future<XFile?> openFile({
    List<XTypeGroup>? acceptedTypeGroups,
    String? initialDirectory,
    String? confirmButtonText,
  }) async {
    final List<XFile> files = await _openFiles(
      acceptedTypeGroups: acceptedTypeGroups,
    );
    return files.isNotEmpty ? files.first : null;
  }

  @override
  Future<List<XFile>> openFiles({
    List<XTypeGroup>? acceptedTypeGroups,
    String? initialDirectory,
    String? confirmButtonText,
  }) async {
    return _openFiles(acceptedTypeGroups: acceptedTypeGroups, multiple: true);
  }

  // This is intended to be passed to XFile, which ignores the path, but 'null'
  // indicates a canceled save on other platforms, so provide a non-null dummy
  // value.
  @override
  Future<String?> getSavePath({
    List<XTypeGroup>? acceptedTypeGroups,
    String? initialDirectory,
    String? suggestedName,
    String? confirmButtonText,
  }) async => '';

  @override
  Future<FileSaveLocation?> getSaveLocation({
    List<XTypeGroup>? acceptedTypeGroups,
    SaveDialogOptions options = const SaveDialogOptions(),
  }) async {
    // This is intended to be passed to XFile, which ignores the path, so
    // provide a non-null dummy value.
    return const FileSaveLocation('');
  }

  @override
  Future<String?> getDirectoryPath({
    String? initialDirectory,
    String? confirmButtonText,
  }) async => null;

  Future<List<XFile>> _openFiles({
    List<XTypeGroup>? acceptedTypeGroups,
    bool multiple = false,
  }) async {
    final String accept = acceptedTypesToString(acceptedTypeGroups);
    return _domHelper.getFiles(accept: accept, multiple: multiple);
  }
}

/// Declares the browser's system save picker at the registered Web plugin boundary.
extension type _SavePickerWindow(JSObject window) implements JSObject {
  /// Opens the browser save picker while the initiating user gesture is active.
  external JSPromise<web.FileSystemFileHandle> showSaveFilePicker(
    JSObject options,
  );
}
