import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/update/AppUpdateModels.dart';
import 'package:operit2/core/update/CoreAppUpdateBackend.dart';

import '../support/native_sidebar_bridge.dart';

/// Exercises the generated MessagePack API used by the client. Only the core
/// fixture knows a native package exists; no package path crosses this boundary.
class UpdateBridge extends NativeSidebarBridge {
  final calls = <CoreCallRequest>[];
  final progress = StreamController<CoreEvent>();
  final download = Completer<String>();
  Object? baseError;
  Object? downloadError;
  CoreWatchRequest? watch;

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request);
    final Object? value;
    switch (request.methodName) {
      case 'checkApplicationUpdate':
        value = {
          'currentVersion': '1.0.0',
          'version': '1.1.0',
          'releasePageUrl':
              'https://github.com/AAswordman/Operit2/releases/tag/v1.1.0',
          'releaseNotes': 'Fixture release',
        };
      case 'downloadApplicationUpdate':
        expect(request.args, {'version': '1.1.0'});
        if (downloadError != null) throw downloadError!;
        value = await download.future;
      case 'cancelApplicationUpdateDownload':
        download.completeError(StateError('UPDATE_CANCELLED'));
        value = null;
      case 'installApplicationUpdate':
        expect(request.args, {'receipt': 'opaque-receipt'});
        value = {
          'installState': 'installer_opened',
          'details': 'Complete confirmation in the system installer',
        };
      case 'planApplicationUpdate':
      case 'selectApplicationUpdateFull':
        expect(request.args, {'version': '1.1.0'});
        value = {
          'mode': 'full',
          'fullAvailable': true,
          'incrementalAvailable': false,
          'baseVersion': null,
          'fullBytes': 120,
          'incrementalBytes': null,
        };
      case 'supplyApplicationUpdateBase':
        if (baseError != null) throw baseError!;
        expect(request.args, {
          'version': '1.1.0',
          'sourcePath': '/operitapp/data/temp/selection/original.zip',
        });
        value = {
          'mode': 'incremental',
          'fullAvailable': false,
          'incrementalAvailable': true,
          'baseVersion': '1.0.0',
          'fullBytes': 120,
          'incrementalBytes': 48,
        };
      default:
        throw StateError('Unexpected update API: ${request.methodName}');
    }
    return encodeCoreLink([0, value]);
  }

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) {
    expect(request.propertyName, 'applicationUpdateProgressFlow');
    watch = request;
    return progress.stream;
  }

  void reportProgress() {
    final request = watch!;
    progress.add(
      CoreEvent.raw(
        requestId: request.requestId,
        target: request.target,
        propertyName: request.propertyName,
        kind: 'Snapshot',
        valueBytes: encodeCoreLink({
          'stage': 'downloading',
          'receivedBytes': 48,
          'totalBytes': 120,
        }),
        decodeValue: (bytes) => decodeCoreLink<Object?>(bytes),
      ),
    );
  }
}

void main() {
  setUpAll(ClientLogger.initialize);
  test(
    'maps explicit base rejection and vanished patch routes to presentation outcomes',
    () async {
      final bridge = UpdateBridge()
        ..baseError = const CoreLinkError(
          code: 'INTERNAL',
          message: 'UPDATE_BASE_MISMATCH: wrong bytes',
        )
        ..downloadError = const CoreLinkError(
          code: 'INTERNAL',
          message: 'UPDATE_UNAVAILABLE: patch removed',
        );
      final backend = CoreAppUpdateBackend(
        clients: GeneratedCoreProxyClients(bridge),
      );
      final release = (await backend.check())!;
      await expectLater(
        backend.supplyBase(release, '/wrong'),
        throwsA(isA<AppUpdateBaseMismatch>()),
      );
      await expectLater(
        backend.download(release, (_, _) {}),
        throwsA(isA<AppUpdateUnavailable>()),
      );
      expect(bridge.progress.hasListener, isFalse);
      await bridge.progress.close();
    },
  );
  test(
    'generated plan APIs preserve availability and keep package paths inside Core',
    () async {
      final bridge = UpdateBridge();
      final backend = CoreAppUpdateBackend(
        clients: GeneratedCoreProxyClients(bridge),
      );
      final release = (await backend.check())!;
      final plan = await backend.plan(release);
      expect(plan.mode, AppUpdateMode.full);
      expect(plan.fullAvailable, isTrue);
      expect((await backend.chooseFull(release)).mode, AppUpdateMode.full);
      final supplied = await backend.supplyBase(
        release,
        '/operitapp/data/temp/selection/original.zip',
      );
      expect(supplied.mode, AppUpdateMode.incremental);
      expect(supplied.fullAvailable, isFalse);
      expect(supplied.baseVersion, '1.0.0');
      expect(supplied.incrementalBytes, 48);
      await bridge.progress.stream.listen((_) {}).cancel();
      await bridge.progress.close();
    },
  );
  test(
    'generated update API transports metadata, progress and opaque receipt',
    () async {
      final bridge = UpdateBridge();
      final backend = CoreAppUpdateBackend(
        clients: GeneratedCoreProxyClients(bridge),
      );
      final release = (await backend.check())!;
      final received = <(int, int?)>[];
      final pending = backend.download(
        release,
        (bytes, total) => received.add((bytes, total)),
      );
      bridge.reportProgress();
      await Future<void>.delayed(Duration.zero);
      expect(received, [(48, 120)]);
      // A progress transport failure degrades to indeterminate progress while the
      // authoritative download call can still finish normally.
      bridge.progress.addError(StateError('progress disconnected'));
      await Future<void>.delayed(Duration.zero);
      expect(received.last, (0, null));
      bridge.download.complete('opaque-receipt');
      final receipt = await pending;
      final result = await backend.install(receipt);
      expect(result.state, AppUpdateInstallState.installerOpened);
      expect(bridge.calls.map((call) => call.methodName), [
        'checkApplicationUpdate',
        'downloadApplicationUpdate',
        'installApplicationUpdate',
      ]);
      expect(bridge.progress.hasListener, isFalse);
      await bridge.progress.close();
    },
  );

  test(
    'cancels through the generated core API and maps the native cancellation',
    () async {
      final bridge = UpdateBridge();
      final backend = CoreAppUpdateBackend(
        clients: GeneratedCoreProxyClients(bridge),
      );
      final release = (await backend.check())!;
      final pending = backend.download(release, (_, _) {});
      final cancelled = expectLater(
        pending,
        throwsA(isA<AppUpdateCancelled>()),
      );
      await backend.cancelDownload();
      await cancelled;
      expect(bridge.progress.hasListener, isFalse);
      expect(bridge.calls.last.methodName, 'cancelApplicationUpdateDownload');
      await bridge.progress.close();
    },
  );
}
