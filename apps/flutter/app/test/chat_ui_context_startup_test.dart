import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/ui/common/contributions/ToolPkgChatUiCatalog.dart';

/// Covers the classification of package-registry startup-race errors on the
/// chat UI context chain: the two transient Core texts degrade silently while
/// genuine load failures stay fatal.
void main() {
  const loadNotCompleted = CoreLinkError(
    code: 'COMMAND_ERROR',
    message: 'Package registry load has not completed',
  );
  const startupNotScheduled = CoreLinkError(
    code: 'COMMAND_ERROR',
    message: 'Package registry startup has not been scheduled',
  );
  const realStartupFailure = CoreLinkError(
    code: 'COMMAND_ERROR',
    message: 'original registry startup failure',
  );

  test('both transient readiness texts classify as a startup race', () {
    expect(isTransientPackageRegistryReadinessError(loadNotCompleted), isTrue);
    expect(
      isTransientPackageRegistryReadinessError(startupNotScheduled),
      isTrue,
    );
    expect(
      chatUiContextFailurePolicy(loadNotCompleted),
      ChatUiContextFailurePolicy.silentRetry,
    );
    expect(
      chatUiContextFailurePolicy(startupNotScheduled),
      ChatUiContextFailurePolicy.silentRetry,
    );
  });

  test('genuine failures never classify as a startup race', () {
    expect(
      isTransientPackageRegistryReadinessError(realStartupFailure),
      isFalse,
    );
    expect(
      chatUiContextFailurePolicy(realStartupFailure),
      ChatUiContextFailurePolicy.fatal,
    );
    const wrongCode = CoreLinkError(
      code: 'INVALID_RESPONSE',
      message: 'Package registry load has not completed',
    );
    expect(isTransientPackageRegistryReadinessError(wrongCode), isFalse);
    expect(
      chatUiContextFailurePolicy(wrongCode),
      ChatUiContextFailurePolicy.fatal,
    );
    final stateError = StateError('Package registry load has not completed');
    expect(isTransientPackageRegistryReadinessError(stateError), isFalse);
    expect(
      chatUiContextFailurePolicy(stateError),
      ChatUiContextFailurePolicy.fatal,
    );
  });

  test(
    'the catalog propagates the discovery error for policy handling',
    () async {
      final transient = await _loadContextFailure(loadNotCompleted);
      expect(transient, same(loadNotCompleted));
      expect(
        chatUiContextFailurePolicy(transient),
        ChatUiContextFailurePolicy.silentRetry,
      );

      final real = await _loadContextFailure(realStartupFailure);
      expect(real, same(realStartupFailure));
      expect(
        chatUiContextFailurePolicy(real),
        ChatUiContextFailurePolicy.fatal,
      );
    },
  );
}

/// Loads the chat UI context against a discovery call failing with [failure].
Future<Object> _loadContextFailure(Object failure) async {
  final catalog = ToolPkgChatUiCatalog(
    clients: GeneratedCoreProxyClients(_DiscoveryFailureBridge(failure)),
  );
  try {
    await catalog.loadContext(chatId: null);
  } catch (error) {
    return error;
  }
  throw StateError('The failing discovery call must reject the catalog load.');
}

/// Fails the chat context discovery request with the configured error.
class _DiscoveryFailureBridge extends OperitRuntimeBridge {
  /// Prepares the exact failure raised for discovery.
  _DiscoveryFailureBridge(this.failure);

  final Object failure;

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    if (request.methodName == 'getToolPkgPublicApiOwners') {
      final args = request.args as Map<String, Object?>;
      if (args['apiName'] != 'chat.context.actions') {
        throw StateError('Unexpected public API discovery.');
      }
      throw failure;
    }
    throw StateError('Unexpected catalog method: ${request.methodName}');
  }

  /// Rejects push operations because discovery performs no uploads.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw StateError('Unexpected startup-race push.');

  /// Rejects snapshot operations because this fixture covers one call only.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw StateError('Unexpected startup-race snapshot.');

  /// Rejects unrelated stream operations rather than fabricating state.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw StateError('Unexpected startup-race stream.');
}
