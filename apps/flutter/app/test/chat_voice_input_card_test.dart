import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/voice_input/ChatVoiceInputCard.dart';
import 'package:operit2/ui/features/chat/voice_input/ChatVoiceInputSession.dart';
import 'package:operit2/ui/features/chat/voice_input/ChatVoiceWave.dart';
import 'package:record/record.dart';

import 'support/chat_voice_input_session.dart';
import 'support/record_platform_fake.dart';

/// Builds one switcher around [session] inside a phone-width composer.
Future<void> _pumpSwitcher(
  WidgetTester tester,
  ChatVoiceInputSession session, {
  Widget composer = const Text('composer'),
  Offset textOrigin = Offset.zero,
  double surfaceBottomInset = 0,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(
        body: Center(
          child: SizedBox(
            width: 390,
            child: ChatVoiceInputSwitcher(
              session: session,
              textStyle: const TextStyle(fontSize: 14, height: 20 / 14),
              textOrigin: textOrigin,
              surfaceBottomInset: surfaceBottomInset,
              composer: composer,
            ),
          ),
        ),
      ),
    ),
  );
}

/// Lets the cross-fade and the height animation of one swap finish.
Future<void> _settleSwap(WidgetTester tester) async {
  await tester.pump();
  await tester.pump(const Duration(milliseconds: 600));
}

/// Releases the card and its session so no ticker or timer outlives the test.
Future<void> _release(
  WidgetTester tester,
  ChatVoiceInputSession session,
) async {
  await tester.pumpWidget(const SizedBox());
  session.dispose();
  await tester.pump();
}

/// Creates a session that ends in the error card without touching a microphone.
ChatVoiceInputSession _unconfiguredSession() => ChatVoiceInputSession(
  core: FakeChatVoiceInputCore(),
  selectedConfigId: () async => null,
  commit: (_) => fail('nothing was spoken'),
);

/// Creates a session that listens through [core] without committing anything.
ChatVoiceInputSession _listeningSession(FakeChatVoiceInputCore core) =>
    ChatVoiceInputSession(
      core: core,
      selectedConfigId: () async => 'local-stt',
      commit: (_) => fail('nothing was spoken'),
    );

/// Finds the canvas the wave is painted in.
final Finder _waveCanvas = find.byWidgetPredicate(
  (widget) => widget is CustomPaint && widget.painter is ChatVoiceWavePainter,
);

void main() {
  late RecordPlatform originalPlatform;

  setUp(() {
    ClientLogger.initialize();
    originalPlatform = RecordPlatform.instance;
    RecordPlatform.instance = FakeRecordPlatform();
  });

  tearDown(() {
    RecordPlatform.instance = originalPlatform;
  });

  testWidgets('swaps the composer for the card while listening', (tester) async {
    final core = FakeChatVoiceInputCore();
    final session = _listeningSession(core);
    await _pumpSwitcher(tester, session);
    expect(find.text('composer'), findsOneWidget);

    await session.start();
    await _settleSwap(tester);
    expect(find.text('composer'), findsNothing);
    expect(find.text('Listening…'), findsOneWidget);
    expect(find.text('0:00'), findsOneWidget);
    expect(find.byTooltip('Stop listening'), findsOneWidget);

    core.partials.add('remind me');
    await tester.pump();
    expect(find.text('Listening…'), findsNothing);
    expect(find.text('remind'), findsOneWidget);
    expect(find.text(' me'), findsOneWidget);

    await _release(tester, session);
  });

  testWidgets('shows a missing configuration inside the card', (tester) async {
    final session = _unconfiguredSession();
    await _pumpSwitcher(tester, session);

    await session.start();
    await _settleSwap(tester);
    expect(find.text('composer'), findsNothing);
    expect(
      find.textContaining('Select a speech recognition configuration'),
      findsOneWidget,
    );
    expect(find.byTooltip('Stop listening'), findsNothing);

    session.advance();
    await _settleSwap(tester);
    expect(find.text('composer'), findsOneWidget);

    await _release(tester, session);
  });

  testWidgets('grows into the card and shrinks back smoothly', (tester) async {
    final session = _unconfiguredSession();
    await _pumpSwitcher(tester, session);
    final switcher = find.byType(ChatVoiceInputSwitcher);
    final composerHeight = tester.getSize(switcher).height;

    await session.start();
    await tester.pump();
    final opening = tester.getSize(switcher).height;
    await tester.pump(const Duration(milliseconds: 100));
    final growing = tester.getSize(switcher).height;
    await _settleSwap(tester);
    final cardHeight = tester.getSize(switcher).height;

    // The height starts from the composer and eases up to the compact card.
    expect(opening, composerHeight);
    expect(growing, greaterThan(composerHeight));
    expect(growing, lessThan(cardHeight));
    expect(cardHeight, lessThan(100));

    session.advance();
    await tester.pump();
    // The card fades out first, so the box holds its height until it is gone.
    expect(tester.getSize(switcher).height, cardHeight);
    await tester.pump(const Duration(milliseconds: 140));
    await tester.pump(const Duration(milliseconds: 120));
    final shrinking = tester.getSize(switcher).height;
    expect(shrinking, lessThan(cardHeight));
    expect(shrinking, greaterThan(composerHeight));

    await _settleSwap(tester);
    expect(tester.getSize(switcher).height, composerHeight);

    await _release(tester, session);
  });

  testWidgets('places words where the composer places its text', (tester) async {
    const origin = Offset(16, 10);
    final core = FakeChatVoiceInputCore();
    final session = _listeningSession(core);
    await _pumpSwitcher(tester, session, textOrigin: origin);
    final switcher = find.byType(ChatVoiceInputSwitcher);
    final originOnScreen = tester.getTopLeft(switcher) + origin;

    await session.start();
    await _settleSwap(tester);
    expect(tester.getTopLeft(find.text('Listening…')), originOnScreen);
    final placeholderHeight = tester.getSize(switcher).height;

    core.partials.add('remind me');
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 1500));
    // The first line of words lands top-left and leaves the height unchanged.
    expect(tester.getTopLeft(find.text('remind')), originOnScreen);
    expect(tester.getSize(switcher).height, placeholderHeight);

    await _release(tester, session);
  });

  testWidgets('runs the wave down to the edge of the composer surface', (
    tester,
  ) async {
    final session = _unconfiguredSession();
    await _pumpSwitcher(tester, session, surfaceBottomInset: 8);

    await session.start();
    await _settleSwap(tester);
    final card = tester.getRect(find.byType(ChatVoiceInputSwitcher));
    final wave = tester.getRect(_waveCanvas);
    expect(wave.top, card.top);
    expect(wave.bottom, card.bottom + 8);

    await _release(tester, session);
  });

  testWidgets('reports composer size changes at once while idle', (tester) async {
    final session = createIdleVoiceInputSession();
    final tall = ValueNotifier<bool>(false);
    addTearDown(tall.dispose);
    await _pumpSwitcher(
      tester,
      session,
      composer: ValueListenableBuilder<bool>(
        valueListenable: tall,
        builder: (context, value, child) => SizedBox(
          height: value ? 60 : 20,
          child: const Text('composer'),
        ),
      ),
    );
    final switcher = find.byType(ChatVoiceInputSwitcher);
    expect(tester.getSize(switcher).height, 20);

    // A size change outside a voice swap must not be held back or eased.
    tall.value = true;
    await tester.pump();
    expect(tester.getSize(switcher).height, 60);
    expect(tester.takeException(), isNull);

    await _release(tester, session);
    expect(tester.takeException(), isNull);
  });

  testWidgets('disposes cleanly without ever animating', (tester) async {
    final session = createIdleVoiceInputSession();
    await _pumpSwitcher(tester, session);
    expect(find.text('composer'), findsOneWidget);

    await _release(tester, session);
    expect(tester.takeException(), isNull);
  });
}
