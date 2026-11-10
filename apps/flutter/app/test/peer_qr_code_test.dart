import 'dart:async';
import 'dart:ui' as ui;
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:image/image.dart' as image;
import 'package:mobile_scanner/mobile_scanner.dart';
import 'package:qr_flutter/qr_flutter.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/runtime/PeerQrImage.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/PeerQrCodeDialog.dart';
import 'package:operit2/ui/common/DeviceSpaceDiscoveryPanel.dart';
import 'device_space_actions_test.dart' show DeviceBridge;

const sampleCode =
    'operit://connect/v2/eyJwIjp7InZlcnNpb24iOjIsIm5vZGVJZCI6ImNvcmUtdGFyZ2V0IiwiZGlzcGxheU5hbWUiOiJNYWMiLCJhZGRyZXNzZXMiOlsiaHR0cDovLzE5Mi4xNjguMS41OjM3MTk1Il19LCJpIjoiMDE5YjdiMTAtMDAwMC03MDAwLTgwMDAtMDAwMDAwMDAwMDAxIiwicyI6IkJ3Y0hCd2NIQndjSEJ3Y0hCd2NIQndjSEJ3Y0hCd2NIQndjSEJ3Y0hCd2MifQ';

class QrBridge extends DeviceBridge {
  bool failCode = false;
  Completer<Uint8List>? inspection;
  String? pairingFailure;
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    if (request.methodName == 'peerConnectionCode') {
      calls.add(request.methodName);
      if (failCode) throw StateError('Listener unavailable');
      return encodeCoreLink([0, sampleCode]);
    }
    if (request.methodName == 'inspectPeerConnectionCode') {
      calls.add(request.methodName);
      if (inspection != null) return inspection!.future;
      if ((request.args as Map)['code'] != sampleCode) {
        throw StateError('INVALID_CONNECTION_CODE');
      }
      return encodeCoreLink([
        0,
        {
          'version': 2,
          'nodeId': 'core-target',
          'displayName': 'Mac',
          'addresses': ['http://192.168.1.5:37195'],
        },
      ]);
    }
    if (request.methodName == 'startPairingFromCode') {
      calls.add(request.methodName);
      pairingArgs = Map<String, Object?>.from(request.args as Map);
      if (pairingFailure != null) throw StateError(pairingFailure!);
      return encodeCoreLink([
        0,
        {
          'nodeId': 'core-target',
          'displayName': 'Mac',
          'inbound': false,
          'outbound': true,
        },
      ]);
    }
    if (request.methodName == 'finishPairing') {
      calls.add(request.methodName);
      finishArgs = Map<String, Object?>.from(request.args as Map);
      return encodeCoreLink([
        0,
        {
          'nodeId': 'core-target',
          'displayName': 'Mac',
          'inbound': false,
          'outbound': true,
        },
      ]);
    }
    return super.callBytes(request);
  }
}

class QrCameraPlatform extends MobileScannerPlatform {
  final captures = StreamController<BarcodeCapture?>.broadcast();
  final starts = <StartOptions>[];
  bool permissionDenied = false;
  int stops = 0, disposals = 0;

  @override
  Stream<BarcodeCapture?> get barcodesStream => captures.stream;
  @override
  Stream<TorchState> get torchStateStream => const Stream.empty();
  @override
  Stream<double> get zoomScaleStateStream => const Stream.empty();
  @override
  Widget buildCameraView() => const SizedBox.expand();
  @override
  Future<void> updateScanWindow(Rect? window) async {}

  @override
  Future<MobileScannerViewAttributes> start(StartOptions options) async {
    starts.add(options);
    if (permissionDenied) {
      throw const MobileScannerException(
        errorCode: MobileScannerErrorCode.permissionDenied,
      );
    }
    return const MobileScannerViewAttributes(
      cameraDirection: CameraFacing.back,
      currentTorchMode: TorchState.unavailable,
      size: Size(640, 480),
      numberOfCameras: 1,
    );
  }

  @override
  Future<void> stop() async {
    stops++;
  }

  @override
  Future<void> dispose() async {
    disposals++;
  }
}

void main() {
  tearDown(() => debugDefaultTargetPlatformOverride = null);
  Future<void> mount(
    WidgetTester tester,
    QrBridge bridge, {
    PeerQrMode mode = PeerQrMode.scan,
    String? onlyNodeId,
    double scale = 1,
    ThemeMode theme = ThemeMode.light,
    void Function(String?)? onResult,
    TargetPlatform platform = TargetPlatform.linux,
  }) async {
    debugDefaultTargetPlatformOverride = platform;
    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('zh'),
        theme: ThemeData(colorSchemeSeed: Colors.indigo),
        darkTheme: ThemeData.dark(),
        themeMode: theme,
        supportedLocales: AppLocalizations.supportedLocales,
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        builder: (context, child) => MediaQuery(
          data: MediaQuery.of(
            context,
          ).copyWith(textScaler: TextScaler.linear(scale)),
          child: child!,
        ),
        home: Scaffold(
          body: Builder(
            builder: (context) => FilledButton(
              onPressed: () async {
                final code = await showPeerQrCodeDialog(
                  context,
                  clients: GeneratedCoreProxyClients(bridge),
                  mode: mode,
                  onlyNodeId: onlyNodeId,
                );
                onResult?.call(code);
              },
              child: const Text('open'),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();
  }

  Future<void> close(WidgetTester tester, QrBridge bridge) async {
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpAndSettle();
    await bridge.prompts.close();
    debugDefaultTargetPlatformOverride = null;
  }

  Future<void> inspect(WidgetTester tester, String text) async {
    await tester.enterText(find.byType(TextField), text);
    await tester.ensureVisible(find.byTooltip('识别连接信息'));
    await tester.tap(find.byTooltip('识别连接信息'));
    await tester.pumpAndSettle();
  }

  QrCameraPlatform installCamera() {
    final previous = MobileScannerPlatform.instance;
    final camera = QrCameraPlatform();
    MobileScannerPlatform.instance = camera;
    addTearDown(() async {
      MobileScannerPlatform.instance = previous;
      await camera.captures.close();
    });
    return camera;
  }

  testWidgets('iOS camera scans a QR code and requires confirmation', (
    tester,
  ) async {
    final camera = installCamera();
    final bridge = QrBridge();
    String? result;
    await mount(
      tester,
      bridge,
      platform: TargetPlatform.iOS,
      onResult: (code) => result = code,
    );
    expect(find.byType(MobileScanner), findsOneWidget);
    expect(camera.starts, hasLength(1));
    expect(camera.starts.single.formats, [BarcodeFormat.qrCode]);
    camera.captures.add(
      const BarcodeCapture(barcodes: [Barcode(rawValue: sampleCode)]),
    );
    await tester.pumpAndSettle();
    expect(bridge.calls, contains('inspectPeerConnectionCode'));
    expect(find.text('Mac'), findsOneWidget);
    expect(result, isNull);
    expect(bridge.calls, isNot(contains('startPairingFromCode')));
    expect(camera.disposals, 1);
    await tester.tap(find.text('确认加入'));
    await tester.pumpAndSettle();
    expect(result, sampleCode);
    await close(tester, bridge);
  });

  testWidgets('iOS camera stops in background and restarts on resume', (
    tester,
  ) async {
    final camera = installCamera();
    final bridge = QrBridge();
    await mount(tester, bridge, platform: TargetPlatform.iOS);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    await tester.pumpAndSettle();
    expect(camera.stops, 1);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pumpAndSettle();
    expect(camera.starts, hasLength(2));
    await close(tester, bridge);
    expect(camera.disposals, 1);
  });

  testWidgets('iOS camera permission denial keeps import and paste usable', (
    tester,
  ) async {
    final camera = installCamera()..permissionDenied = true;
    final bridge = QrBridge();
    await mount(tester, bridge, platform: TargetPlatform.iOS);
    expect(find.textContaining('需要相机权限才能扫码'), findsOneWidget);
    expect(
      tester
          .widget<OutlinedButton>(find.widgetWithText(OutlinedButton, '从图片识别'))
          .onPressed,
      isNotNull,
    );
    await inspect(tester, sampleCode);
    expect(find.text('Mac'), findsOneWidget);
    await close(tester, bridge);
    expect(camera.disposals, 1);
  });

  testWidgets(
    'show code uses live runtime coordinates and can retry a failed listener',
    (tester) async {
      final bridge = QrBridge()..failCode = true;
      await mount(tester, bridge, mode: PeerQrMode.show);
      expect(find.textContaining('Listener unavailable'), findsOneWidget);
      expect(find.byType(QrImageView), findsNothing);
      bridge.failCode = false;
      await tester.tap(find.text('刷新'));
      await tester.pumpAndSettle();
      expect(find.byType(QrImageView), findsOneWidget);
      expect(
        bridge.calls.where((c) => c == 'peerConnectionCode'),
        hasLength(2),
      );
      expect(bridge.calls, isNot(contains('discoverPeers')));
      expect(bridge.calls, isNot(contains('startPairing')));
      await close(tester, bridge);
    },
  );
  testWidgets(
    'invalid scan can recover and a valid code needs confirmation before pairing',
    (tester) async {
      final bridge = QrBridge();
      String? result;
      await mount(tester, bridge, onResult: (code) => result = code);
      await inspect(tester, 'https://unrelated.example');
      expect(find.textContaining('INVALID_CONNECTION_CODE'), findsOneWidget);
      await inspect(tester, sampleCode);
      expect(find.text('Mac'), findsOneWidget);
      expect(find.textContaining('INVALID_CONNECTION_CODE'), findsNothing);
      expect(result, isNull);
      expect(bridge.calls, isNot(contains('startPairingFromCode')));
      await tester.tap(find.text('确认加入'));
      await tester.pumpAndSettle();
      expect(result, sampleCode);
      await close(tester, bridge);
    },
  );
  testWidgets(
    'a focused device cannot be replaced by another scanned identity',
    (tester) async {
      final bridge = QrBridge();
      await mount(tester, bridge, onlyNodeId: 'selected-device');
      await inspect(tester, sampleCode);
      expect(find.textContaining('此连接码属于其他设备'), findsOneWidget);
      expect(find.text('确认加入'), findsNothing);
      await close(tester, bridge);
    },
  );
  testWidgets('switching modes discards a late failed inspection', (
    tester,
  ) async {
    final pending = Completer<Uint8List>();
    final bridge = QrBridge()..inspection = pending;
    await mount(tester, bridge);
    await tester.enterText(find.byType(TextField), sampleCode);
    await tester.tap(find.byTooltip('识别连接信息'));
    await tester.pump();
    await tester.ensureVisible(find.text('出示连接码'));
    await tester.pump();
    await tester.tap(find.text('出示连接码'));
    await tester.pumpAndSettle();
    pending.completeError(StateError('stale scan'));
    await tester.pumpAndSettle();
    expect(find.textContaining('stale scan'), findsNothing);
    expect(find.byType(QrImageView), findsOneWidget);
    await close(tester, bridge);
  });
  for (final mode in PeerQrMode.values) {
    for (final theme in [ThemeMode.light, ThemeMode.dark]) {
      testWidgets('$mode fits a narrow screen with large text in $theme', (
        tester,
      ) async {
        tester.view.physicalSize = const Size(360, 740);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final bridge = QrBridge();
        await mount(tester, bridge, mode: mode, scale: 1.8, theme: theme);
        expect(tester.takeException(), isNull);
        if (mode == PeerQrMode.show) {
          expect(find.byType(QrImageView), findsOneWidget);
        }
        await close(tester, bridge);
      });
    }
  }
  Future<void> mountDevicePicker(WidgetTester tester, QrBridge bridge) async {
    debugDefaultTargetPlatformOverride = TargetPlatform.linux;
    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('zh'),
        supportedLocales: AppLocalizations.supportedLocales,
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        home: Scaffold(
          body: DeviceSpaceDiscoveryPanel(
            clients: GeneratedCoreProxyClients(bridge),
            onJoined: (_) async {},
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('添加设备'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('扫码连接'));
    await tester.pumpAndSettle();
    await inspect(tester, sampleCode);
    await tester.tap(find.text('确认加入'));
    await tester.pumpAndSettle();
  }

  testWidgets(
    'one QR confirmation connects and submits joining without manual pairing',
    (tester) async {
      final discovery = Completer<Uint8List>();
      final bridge = QrBridge()..discovery = discovery;
      await mountDevicePicker(tester, bridge);
      expect(bridge.pairingArgs, {'code': sampleCode});
      expect(bridge.calls, isNot(contains('finishPairing')));
      expect(find.text('完成连接'), findsNothing);
      expect(find.byType(TextField), findsNothing);
      expect(bridge.calls, contains('requestDeviceSpaceJoin'));
      discovery.complete(encodeCoreLink([0, <Object?>[]]));
      await tester.pumpAndSettle();
      await close(tester, bridge);
    },
  );

  testWidgets(
    'cancelling scan confirmation creates no pairing or join request',
    (tester) async {
      final bridge = QrBridge();
      await mount(tester, bridge);
      await inspect(tester, sampleCode);
      await tester.tap(find.text('取消'));
      await tester.pumpAndSettle();
      expect(bridge.calls, isNot(contains('startPairingFromCode')));
      expect(bridge.calls, isNot(contains('finishPairing')));
      expect(bridge.calls, isNot(contains('requestDeviceSpaceJoin')));
      await close(tester, bridge);
    },
  );

  testWidgets(
    'a late discovery response cannot overwrite the QR pairing failure',
    (tester) async {
      final discovery = Completer<Uint8List>();
      final bridge = QrBridge()
        ..discovery = discovery
        ..pairingFailure = 'QR endpoint unavailable';
      await mountDevicePicker(tester, bridge);
      expect(find.textContaining('QR endpoint unavailable'), findsOneWidget);
      discovery.complete(encodeCoreLink([0, <Object?>[]]));
      await tester.pumpAndSettle();
      expect(find.textContaining('QR endpoint unavailable'), findsOneWidget);
      await close(tester, bridge);
    },
  );

  testWidgets('generated QR images decode after rotation or color inversion', (
    tester,
  ) async {
    await tester.runAsync(() async {
      final recorder = ui.PictureRecorder();
      final canvas = ui.Canvas(recorder)
        ..drawColor(Colors.white, ui.BlendMode.src);
      canvas.translate(32, 32);
      QrPainter(
        data: sampleCode,
        version: QrVersions.auto,
      ).paint(canvas, const Size(320, 320));
      final picture = recorder.endRecording();
      final raster = await picture.toImage(384, 384);
      final bytes = (await raster.toByteData(
        format: ui.ImageByteFormat.png,
      ))!.buffer.asUint8List();
      final frame = image.decodePng(bytes)!;
      expect(readPeerQrImage(bytes), sampleCode);
      expect(
        readPeerQrImage(image.encodePng(image.copyRotate(frame, angle: 90))),
        sampleCode,
      );
      expect(readPeerQrImage(image.encodePng(image.invert(frame))), sampleCode);
      expect(readPeerQrImage(Uint8List.fromList([1, 2, 3])), isNull);
      raster.dispose();
      picture.dispose();
    });
  });
}
