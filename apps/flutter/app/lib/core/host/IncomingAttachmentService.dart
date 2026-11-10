// ignore_for_file: file_names

import 'dart:async';

import 'package:operit_folder_access/operit_folder_access.dart';

import '../logging/ClientLogger.dart';
import 'AppToastController.dart';

/// Holds external file deliveries until the main conversation composer is ready.
class IncomingAttachmentService {
  /// Creates an independently owned delivery queue.
  IncomingAttachmentService();

  static final instance = IncomingAttachmentService();
  final _pending = <List<SelectedFileInput>>[];
  void Function()? _openConversation;
  Object? _composerOwner;
  Future<void> Function(List<SelectedFileInput>)? _composer;
  bool _delivering = false;
  bool _initialized = false;

  /// Starts the platform adapter registered by Flutter, without platform tests.
  void initialize() {
    if (_initialized) return;
    _initialized = true;
    unawaited(
      OperitFolderAccessPlatform.instance
          .receiveIncomingFiles(onFiles: receive, onError: _reportError)
          .catchError(_reportError),
    );
  }

  /// Queues one share/open operation and reveals its destination conversation.
  void receive(List<SelectedFileInput> files) {
    if (files.isEmpty) return;
    _pending.add(List.unmodifiable(files));
    _openConversation?.call();
    _scheduleDelivery();
  }

  /// Connects navigation after the main screen has initialized its route catalog.
  void installNavigation(void Function() openConversation) {
    _openConversation = openConversation;
    if (_pending.isNotEmpty) openConversation();
    _scheduleDelivery();
  }

  /// Disconnects navigation when the main application screen is disposed.
  void clearNavigation() {
    _openConversation = null;
  }

  /// Binds delivery to a ready main composer and its selected conversation.
  void installComposer(
    Object owner,
    Future<void> Function(List<SelectedFileInput>) handler,
  ) {
    _composerOwner = owner;
    _composer = handler;
    _scheduleDelivery();
  }

  /// Removes only the composer owned by the disposing or switching surface.
  void clearComposer(Object owner) {
    if (!identical(_composerOwner, owner)) return;
    _composerOwner = null;
    _composer = null;
  }

  /// Serializes shares and releases all native inputs after each delivery attempt.
  void _scheduleDelivery() {
    if (_delivering) return;
    _delivering = true;
    unawaited(
      Future<void>(() async {
        try {
          while (_pending.isNotEmpty &&
              _openConversation != null &&
              _composer != null) {
            final handler = _composer!;
            final files = _pending.removeAt(0);
            try {
              await handler(files);
            } catch (error, stack) {
              _reportError(error, stack);
            } finally {
              for (final file in files) {
                try {
                  await file.close();
                } catch (error, stack) {
                  _reportError(error, stack);
                }
              }
            }
          }
        } finally {
          _delivering = false;
        }
      }),
    );
  }

  /// Records native reception, upload, and resource-release failures.
  void _reportError(Object error, StackTrace stack) {
    ClientLogger.e(
      'Incoming attachment failed',
      tag: 'IncomingAttachment',
      error: error,
      stackTrace: stack,
    );
    AppToastController.instance.show('无法接收附件: $error');
  }
}
