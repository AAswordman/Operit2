import 'dart:async';
import 'dart:convert';

import 'package:crypto/crypto.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  Future<Map<String, dynamic>> fixture() async =>
      jsonDecode(
            await rootBundle.loadString(
              'packages/operit_ui/assets/fixtures/theme-v1.json',
            ),
          )
          as Map<String, dynamic>;

  test('internal snapshot transport is unchanged and complete', () {
    const snapshot = ThemePreferenceDefaults.snapshot;
    expect(snapshot.toJson(), hasLength(103));
    expect(
      ThemePreferenceSnapshot.fromJson(snapshot.toJson()).toJson(),
      snapshot.toJson(),
    );
  });

  test(
    'public rgba is converted to argb without deriving a seed scheme',
    () async {
      final json = await fixture();
      (json['palette']['light'] as Map<String, dynamic>)['primary'] =
          '#12345678';
      final spec = ThemeVisualSpecV1.fromCheckedContract(json);
      expect(spec.colorScheme(Brightness.light).primary.toARGB32(), 0x78123456);
      expect(spec.colorScheme(Brightness.dark).primary.toARGB32(), 0xFFA0CAFD);
    },
  );

  test(
    'projection keeps user mode, scale, avatar content and visibility',
    () async {
      final settings = ThemePreferenceSnapshot.fromJson(<String, Object?>{
        ...ThemePreferenceDefaults.snapshot.toJson(),
        'themeMode': 'dark',
        'fontScale': 1.75,
        'showThinkingProcess': false,
        'showMessageTimestamp': true,
        'customUserAvatarUri': 'runtime/user-avatar.png',
      });
      final spec = ThemeVisualSpecV1.fromCheckedContract(await fixture());
      final effective = spec.toSnapshot(settings);
      expect(effective.themeMode, ThemeMode.dark);
      expect(effective.fontScale, 1.75);
      expect(effective.showThinkingProcess, isFalse);
      expect(effective.showMessageTimestamp, isTrue);
      expect(effective.customUserAvatarUri, 'runtime/user-avatar.png');
      expect(effective.backgroundMediaType, 'image');
      expect(spec.fontReferences['global'], 'noto-sans-sc');
    },
  );

  test('asset resolver is immutable and missing assets are errors', () async {
    final bytes = Uint8List.fromList(<int>[1, 2, 3]);
    final resolver = MemoryThemeAssetResolver(<String, Uint8List>{
      'image': bytes,
    });
    bytes[0] = 9;
    final first = await resolver.readBytes('image');
    expect(first, <int>[1, 2, 3]);
    first[0] = 8;
    expect(await resolver.readBytes('image'), <int>[1, 2, 3]);
    await expectLater(resolver.readBytes('missing'), throwsStateError);
  });

  test('static theme SVG uses the shared Flutter image decoder', () async {
    final svg = Uint8List.fromList(
      utf8.encode(
        '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="12" viewBox="0 0 16 12">'
        '<path d="M1 1 L15 11" fill="#334455"/></svg>',
      ),
    );
    final image = await decodeThemeImageBytes('assets/images/mark.svg', svg);
    expect(image.width, 16);
    expect(image.height, 12);
    image.dispose();
  });

  test(
    'font family is derived from complete bytes and preparation is strict',
    () async {
      final bytes = Uint8List.fromList(<int>[1, 2, 3, 4]);
      final digest = sha256.convert(bytes).toString();
      var registrations = 0;
      final registry = ThemeFontRegistry(
        registration: (family, registeredBytes) async {
          registrations++;
          expect(family, 'OperitThemeFont_$digest');
          expect(registeredBytes, bytes);
        },
      );
      final resolver = MemoryThemeAssetResolver(<String, Uint8List>{
        'assets/fonts/example.ttf': bytes,
      });
      final prepared = await registry.prepare(
        'assets/fonts/example.ttf',
        resolver: resolver,
        expectedSha256: digest,
      );
      expect(prepared.sha256, digest);
      expect(prepared.family, 'OperitThemeFont_$digest');
      expect(registrations, 1);
      await expectLater(
        registry.prepare(
          'assets/fonts/example.ttf',
          resolver: resolver,
          expectedSha256: '0' * 64,
        ),
        throwsStateError,
      );
      expect(
        () => validateThemeFontReference('https://example.test/font.ttf'),
        throwsArgumentError,
      );
    },
  );

  test(
    'pinned builtin fonts are static, licensed, and cover the probe corpus',
    () async {
      final fonts = await BuiltinThemeFonts.load(bundle: rootBundle);
      final sans = await fonts.readBytes(kNotoSansScFontId);
      ThemeFontGlyphs(sans).requireText(
        ThemeSurfaceProbe.glyphCorpus,
        reference: kNotoSansScFontId,
      );
      final mono = await fonts.readBytes(kCascadiaMonoFontId);
      ThemeFontGlyphs(mono).requireText(
        'Operit2 Theme Preview 0123456789 {}[]()',
        reference: kCascadiaMonoFontId,
      );
    },
  );

  test(
    'only the latest prepared revision can obtain a frame receipt',
    () async {
      final controller = ThemeProbePreparationController();
      addTearDown(controller.dispose);
      expect(
        () => controller.markRendered(
          revision: 0,
          fixtureVersion: 'surfaces-v1',
          fixtureSourceSha256: 'a' * 64,
        ),
        throwsStateError,
      );
      final registry = ThemeFontRegistry(registration: (_, _) async {});
      final font = await registry.registerBytes(
        reference: 'test-font',
        bytes: Uint8List.fromList([1]),
      );
      Future<ThemeProbeResources> resources() => ThemeProbeResources.prepare(
        sans: Future.value(font),
        mono: Future.value(font),
        user: Future.value(font),
        ai: Future.value(font),
        assets: const {},
        imageReferences: const {},
      );
      await controller.prepare(revision: 1, load: resources);
      expect(controller.receipt, isNull);
      controller.markRendered(
        revision: 1,
        fixtureVersion: 'surfaces-v1',
        fixtureSourceSha256: 'a' * 64,
        themeSha256: 'b' * 64,
      );
      expect(controller.receipt!.toJson()['themeSha256'], 'b' * 64);
      final delayed = Completer<ThemeProbeResources>();
      final oldLoad = controller.prepare(
        revision: 2,
        load: () => delayed.future,
      );
      expect(controller.receipt, isNull);
      expect(controller.phase, ThemeProbePhase.preparing);
      await controller.prepare(revision: 3, load: resources);
      final superseded = await resources();
      delayed.complete(superseded);
      await oldLoad;
      expect(controller.revision, 3);
      await expectLater(superseded.readBytes('anything'), throwsStateError);
      controller.markRendered(
        revision: 2,
        fixtureVersion: 'surfaces-v1',
        fixtureSourceSha256: 'a' * 64,
      );
      expect(controller.receipt, isNull);
      controller.markRendered(
        revision: 3,
        fixtureVersion: 'surfaces-v1',
        fixtureSourceSha256: 'a' * 64,
      );
      expect(controller.receipt!.revision, 3);
      controller.failFrame(revision: 2, error: StateError('old frame'));
      expect(controller.receipt!.revision, 3);
      await controller.prepare(
        revision: 4,
        load: () => Future.error(StateError('invalid draft')),
      );
      expect(controller.phase, ThemeProbePhase.failed);
      expect(controller.receipt, isNull);
    },
  );

  for (final inputStyle in <String>['agent', 'classic']) {
    testWidgets(
      'shared surfaces render without application runtime: $inputStyle',
      (tester) async {
        final json = await tester.runAsync(fixture);
        final spec = ThemeVisualSpecV1.fromCheckedContract(json!);
        final snapshot = ThemePreferenceSnapshot.fromJson(<String, Object?>{
          ...ThemePreferenceDefaults.snapshot.toJson(),
          'inputStyle': inputStyle,
        });
        await tester.pumpWidget(
          ThemeAssetScope(
            resolver: MemoryThemeAssetResolver(const <String, Uint8List>{}),
            child: ThemeRenderRoot(
              themePreferenceSnapshot: snapshot,
              lightColorScheme: spec.colorScheme(Brightness.light),
              darkColorScheme: spec.colorScheme(Brightness.dark),
              fontFamily: 'Ahem',
              fontFamilyFallback: const <String>[],
              themeMode: ThemeMode.light,
              themeAnimationDuration: Duration.zero,
              builder: (context, child) => ThemeBackground(
                themePreferenceSnapshot: snapshot,
                media: null,
                child: child!,
              ),
              home: Scaffold(
                body: SingleChildScrollView(
                  child: ThemeSurfaceProbe(snapshot: snapshot),
                ),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        expect(find.byType(BubbleSurface), findsNWidgets(2));
        expect(find.byType(ThemeGlassSurface), findsOneWidget);
        expect(
          find.byType(AgentInputSurface),
          inputStyle == 'agent' ? findsOneWidget : findsNothing,
        );
        expect(
          find.byType(ClassicInputSurface),
          inputStyle == 'classic' ? findsOneWidget : findsNothing,
        );
        expect(tester.takeException(), isNull);
      },
    );
  }

  test('buildOperitThemeData supports non-unit fontScale in light and dark modes', () async {
    final spec = ThemeVisualSpecV1.fromCheckedContract(await fixture());
    for (final scale in <double>[0.85, 1.0, 1.25]) {
      final snapshot = ThemePreferenceSnapshot.fromJson(<String, Object?>{
        ...ThemePreferenceDefaults.snapshot.toJson(),
        'fontScale': scale,
      });
      final lightTheme = buildOperitThemeData(
        spec.colorScheme(Brightness.light),
        snapshot,
        fontFamily: 'Ahem',
        fontFamilyFallback: const <String>[],
      );
      final darkTheme = buildOperitThemeData(
        spec.colorScheme(Brightness.dark),
        snapshot,
        fontFamily: 'Ahem',
        fontFamilyFallback: const <String>[],
      );
      expect(lightTheme.textTheme.bodyMedium?.fontSize, closeTo(14.0 * scale, 0.001));
      expect(darkTheme.textTheme.bodyMedium?.fontSize, closeTo(14.0 * scale, 0.001));
    }
  });
}
