import 'package:flutter/foundation.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/ui/features/chat/voice_input/ChatVoiceInputSession.dart';
import 'package:record/record.dart';

import 'support/chat_voice_input_session.dart';
import 'support/record_platform_fake.dart';

/// Polls until [condition] holds, so timing never decides whether a test passes.
Future<void> _waitFor(bool Function() condition) async {
  final deadline = DateTime.now().add(const Duration(seconds: 10));
  while (!condition()) {
    if (DateTime.now().isAfter(deadline)) {
      fail('the voice session never reached the expected state');
    }
    await Future<void>.delayed(const Duration(milliseconds: 20));
  }
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  setUp(ClientLogger.initialize);

  group('chatVoiceTokensOf', () {
    test('keeps whitespace-delimited words apart', () {
      final tokens = chatVoiceTokensOf('  remind me to water the ferns ');
      expect(tokens.map((token) => token.text), <String>[
        'remind',
        'me',
        'to',
        'water',
        'the',
        'ferns',
      ]);
      expect(tokens.map((token) => token.spaced), <bool>[
        false,
        true,
        true,
        true,
        true,
        true,
      ]);
    });

    test('breaks unspaced Chinese into pairs without gaps', () {
      final tokens = chatVoiceTokensOf('帮我打开浏览器');
      expect(tokens.map((token) => token.text), <String>[
        '帮我',
        '打开',
        '浏览',
        '器',
      ]);
      expect(tokens.every((token) => !token.spaced), isTrue);
    });

    test('folds punctuation into the unit before it', () {
      final mixed = chatVoiceTokensOf('好的，打开GitHub页面。');
      expect(mixed.map((token) => token.text), <String>[
        '好的，',
        '打开',
        'GitHub',
        '页面。',
      ]);
      final latin = chatVoiceTokensOf('well, done');
      expect(latin.map((token) => token.text), <String>['well,', 'done']);
      expect(latin[1].spaced, isTrue);
    });

    test('keeps earlier units when a transcript only grows', () {
      final short = chatVoiceTokensOf('嗯');
      final grown = chatVoiceTokensAfter(
        '嗯好的谢谢',
        known: '嗯',
        tokens: short,
      );
      expect(grown.map((token) => token.text), <String>['嗯', '好的', '谢谢']);

      final latin = chatVoiceTokensOf('remind me');
      final extended = chatVoiceTokensAfter(
        'remind me to water',
        known: 'remind me',
        tokens: latin,
      );
      expect(extended.map((token) => token.text), <String>[
        'remind',
        'me',
        'to',
        'water',
      ]);
      expect(extended.map((token) => token.spaced), <bool>[
        false,
        true,
        true,
        true,
      ]);

      final punctuated = chatVoiceTokensAfter(
        'hello, world',
        known: 'hello',
        tokens: chatVoiceTokensOf('hello'),
      );
      expect(punctuated.map((token) => token.text), <String>['hello,', 'world']);
    });

    test('splits a revised transcript again as a whole', () {
      final revised = chatVoiceTokensAfter(
        'two waters',
        known: 'to water',
        tokens: chatVoiceTokensOf('to water'),
      );
      expect(revised.map((token) => token.text), <String>['two', 'waters']);
    });
  });

  group('ChatVoiceInputSession', () {
    late RecordPlatform originalPlatform;
    late FakeRecordPlatform microphone;

    setUp(() {
      originalPlatform = RecordPlatform.instance;
      microphone = FakeRecordPlatform();
      RecordPlatform.instance = microphone;
      debugDefaultTargetPlatformOverride = TargetPlatform.android;
    });

    tearDown(() {
      RecordPlatform.instance = originalPlatform;
      debugDefaultTargetPlatformOverride = null;
    });

    test('shows words streamed while listening and commits the transcript', () async {
      final core = FakeChatVoiceInputCore()
        ..finalTranscript = 'remind me to water the ferns';
      final committed = <String>[];
      final session = ChatVoiceInputSession(
        core: core,
        selectedConfigId: () async => 'local-stt',
        commit: committed.add,
      );
      addTearDown(session.dispose);

      await session.start();
      await microphone.started.future;
      await Future<void>.delayed(Duration.zero);
      expect(session.phase, ChatVoiceInputPhase.listening);

      microphone.emit(10, 1200);
      core.partials.add('remind me to');
      await _waitFor(() => session.tokens.length == 3);
      expect(session.tokens.map((token) => token.text), <String>[
        'remind',
        'me',
        'to',
      ]);
      expect(session.phase, ChatVoiceInputPhase.listening);
      await _waitFor(() => core.pushedBytes.isNotEmpty);
      final pushed = core.pushedBytes.reduce((sum, bytes) => sum + bytes);
      expect(pushed, 10 * 20 * 2 * 16);

      await session.stop();
      await _waitFor(() => session.phase == ChatVoiceInputPhase.idle);
      expect(core.finished, <String>['session-local-stt']);
      expect(committed, <String>['remind me to water the ferns']);
    });

    test('ends with an error when no speech configuration is selected', () async {
      final core = FakeChatVoiceInputCore();
      final session = ChatVoiceInputSession(
        core: core,
        selectedConfigId: () async => null,
        commit: (_) => fail('nothing was spoken'),
      );
      addTearDown(session.dispose);
      final phases = <ChatVoiceInputPhase>[];
      session.addListener(() => phases.add(session.phase));

      await session.start();
      expect(session.phase, ChatVoiceInputPhase.error);
      expect(session.problem!.isMissingConfiguration, isTrue);
      expect(core.started, isEmpty);

      session.advance();
      expect(session.phase, ChatVoiceInputPhase.idle);
      expect(phases, <ChatVoiceInputPhase>[
        ChatVoiceInputPhase.listening,
        ChatVoiceInputPhase.error,
        ChatVoiceInputPhase.idle,
      ]);
    });

    test('shows a notice and returns when nothing was heard', () async {
      final core = FakeChatVoiceInputCore();
      final session = ChatVoiceInputSession(
        core: core,
        selectedConfigId: () async => 'local-stt',
        commit: (_) => fail('nothing was spoken'),
      );
      addTearDown(session.dispose);

      await session.start();
      await microphone.started.future;
      await Future<void>.delayed(Duration.zero);
      await session.stop();
      await _waitFor(() => session.phase == ChatVoiceInputPhase.empty);
      await _waitFor(() => session.phase == ChatVoiceInputPhase.idle);
      expect(core.finished, <String>['session-local-stt']);
    });
  });
}
