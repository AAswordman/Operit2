import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/ui/features/chat/tts/TtsPlaybackController.dart';

/// Verifies persisted message voice lookup without any current-binding or name resolution.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final controller = TtsPlaybackController.instance;

  setUp(() async {
    await controller.stop();
    controller.clearStopped();
    await _flushEvents();
  });
  tearDown(() async {
    await controller.stop();
    controller.clearStopped();
    await _flushEvents();
  });

  test(
    'uses the displayed chat, timestamp, and exact selected historical variant',
    () async {
      final bridge = _bridge(providerType: 'SYSTEM_TTS');
      final snapshots = <TtsPlaybackState>[];

      /// Records the authoritative voice title published during playback preparation.
      void collect() => snapshots.add(controller.state);
      controller.addListener(collect);
      addTearDown(() => controller.removeListener(collect));
      await controller.speakForMessage(
        bridge: bridge,
        chatId: 'displayed-chat',
        messageTimestamp: 1728000000000,
        variantIndex: 2,
        text: 'Selected variant speech',
      );
      await _flushEvents();
      expect(_call(bridge, 'chatConfigurationForMessage').args, {
        'chatId': 'displayed-chat',
        'messageTimestamp': 1728000000000,
        'variantIndex': 2,
      });
      expect(_call(bridge, 'speakWithConfig').args, {
        'ttsConfigId': 'saved-voice-v2',
        'text': 'Selected variant speech',
        'interrupt': true,
      });
      expect(_call(bridge, 'getTtsConfig').args, {'id': 'saved-voice-v2'});
      expect(
        snapshots.any((state) => state.title == 'Persisted variant speaker'),
        isTrue,
      );
      _expectNoIdentityGuessing(bridge);
    },
  );

  test(
    'retains the original locator while a displayed chat or variant changes',
    () async {
      final bridge = _bridge(providerType: 'SYSTEM_TTS');
      final release = Completer<core.ChatParticipantProfile>();
      bridge.nextProfile = release.future;
      var displayedChatId = 'displayed-chat';
      var selectedVariant = 2;
      final playback = controller.speakForMessage(
        bridge: bridge,
        chatId: displayedChatId,
        messageTimestamp: 1728000000000,
        variantIndex: selectedVariant,
        text: 'Original transcript snapshot',
      );
      await _flushEvents();
      displayedChatId = 'another-chat';
      selectedVariant = 0;
      release.complete(_profile('saved-voice-v2'));
      await playback;
      await _flushEvents();
      expect(displayedChatId, 'another-chat');
      expect(selectedVariant, 0);
      expect(_call(bridge, 'chatConfigurationForMessage').args, {
        'chatId': 'displayed-chat',
        'messageTimestamp': 1728000000000,
        'variantIndex': 2,
      });
      expect(
        _call(bridge, 'speakWithConfig').args,
        containsPair('ttsConfigId', 'saved-voice-v2'),
      );
      _expectNoIdentityGuessing(bridge);
    },
  );

  test(
    'remote synthesis uses the same resolved saved voice and existing audio host',
    () async {
      final bridge = _bridge(providerType: 'REMOTE');
      await controller.speakForMessage(
        bridge: bridge,
        chatId: 'displayed-chat',
        messageTimestamp: 1728000000000,
        variantIndex: 2,
        text: 'Remote voice speech',
      );
      await _flushEvents();
      expect(_call(bridge, 'synthesizeWithConfig').args, {
        'ttsConfigId': 'saved-voice-v2',
        'text': 'Remote voice speech',
      });
      expect(_call(bridge, 'playAudio').args, {
        'path': 'runtime:audio/voice.wav',
      });
      expect(
        bridge.calls.where((call) => call.methodName == 'speakWithConfig'),
        isEmpty,
      );
      expect(
        bridge.calls.where(
          (call) => call.methodName == 'chatConfigurationForMessage',
        ),
        hasLength(1),
      );
      _expectNoIdentityGuessing(bridge);
    },
  );

  test(
    'missing historical message profile is an explicit terminal voice error',
    () async {
      final bridge = _bridge(providerType: 'SYSTEM_TTS');
      bridge.profiles.clear();
      await expectLater(
        controller.speakForMessage(
          bridge: bridge,
          chatId: 'displayed-chat',
          messageTimestamp: 1728000000000,
          variantIndex: 2,
          text: 'No historical identity',
        ),
        throwsStateError,
      );
      expect(controller.state.phase, TtsPlaybackPhase.error);
      expect(
        controller.state.error,
        contains('No persisted execution profile'),
      );
      _expectNoPlayback(bridge);
      _expectNoIdentityGuessing(bridge);
    },
  );

  test(
    'blank saved voice IDs never select a default or current TTS configuration',
    () async {
      final bridge = _bridge(providerType: 'SYSTEM_TTS', voiceId: ' ');
      await expectLater(
        controller.speakForMessage(
          bridge: bridge,
          chatId: 'displayed-chat',
          messageTimestamp: 1728000000000,
          variantIndex: 2,
          text: 'No saved voice',
        ),
        throwsStateError,
      );
      expect(controller.state.phase, TtsPlaybackPhase.error);
      expect(
        controller.state.error,
        contains('saved execution profile has no TTS voice'),
      );
      expect(
        bridge.calls.where((call) => call.methodName == 'getTtsConfig'),
        isEmpty,
      );
      _expectNoPlayback(bridge);
      _expectNoIdentityGuessing(bridge);
    },
  );

  test(
    'stopping during historical lookup cancels before any voice backend runs',
    () async {
      final bridge = _bridge(providerType: 'SYSTEM_TTS');
      final release = Completer<core.ChatParticipantProfile>();
      bridge.nextProfile = release.future;
      final playback = controller.speakForMessage(
        bridge: bridge,
        chatId: 'displayed-chat',
        messageTimestamp: 1728000000000,
        variantIndex: 2,
        text: 'Cancelled pending lookup',
      );
      final cancelled = expectLater(
        playback,
        throwsA(isA<TtsPlaybackCancelledException>()),
      );
      await _flushEvents();
      expect(
        bridge.calls.where(
          (call) => call.methodName == 'chatConfigurationForMessage',
        ),
        hasLength(1),
      );
      await controller.stop();
      await cancelled;
      release.complete(_profile('saved-voice-v2'));
      await _flushEvents();
      expect(controller.state.phase, TtsPlaybackPhase.stopped);
      expect(
        bridge.calls.where((call) => call.methodName == 'getTtsConfig'),
        isEmpty,
      );
      _expectNoPlayback(bridge);
    },
  );

  test(
    'stopping pending synthesis rejects its late audio without starting playback',
    () async {
      final bridge = _bridge(providerType: 'REMOTE');
      final release = Completer<void>();
      bridge.synthesisGate = release.future;
      final playback = controller.speakForMessage(
        bridge: bridge,
        chatId: 'displayed-chat',
        messageTimestamp: 1728000000000,
        variantIndex: 2,
        text: 'Cancelled pending synthesis',
      );
      final cancelled = expectLater(
        playback,
        throwsA(isA<TtsPlaybackCancelledException>()),
      );
      await _flushEvents();
      expect(
        bridge.calls.where((call) => call.methodName == 'synthesizeWithConfig'),
        hasLength(1),
      );
      await controller.stop();
      await cancelled;
      release.complete();
      await _flushEvents();
      expect(controller.state.phase, TtsPlaybackPhase.stopped);
      expect(
        bridge.calls.where((call) => call.methodName == 'playAudio'),
        isEmpty,
      );
    },
  );

  test(
    'late system speech startup is stopped within the existing generation guard',
    () async {
      final bridge = _bridge(providerType: 'SYSTEM_TTS');
      final release = Completer<void>();
      bridge.speechGate = release.future;
      final playback = controller.speakForMessage(
        bridge: bridge,
        chatId: 'displayed-chat',
        messageTimestamp: 1728000000000,
        variantIndex: 2,
        text: 'Cancelled pending host startup',
      );
      final cancelled = expectLater(
        playback,
        throwsA(isA<TtsPlaybackCancelledException>()),
      );
      await _flushEvents();
      expect(
        bridge.calls.where((call) => call.methodName == 'speakWithConfig'),
        hasLength(1),
      );
      await controller.stop();
      await cancelled;
      release.complete();
      await _flushEvents();
      expect(
        bridge.calls.where((call) => call.methodName == 'stopSpeech'),
        hasLength(2),
      );
      expect(controller.state.phase, TtsPlaybackPhase.stopped);
    },
  );

  test(
    'synthesis startup errors are surfaced without attempting another voice source',
    () async {
      final bridge = _bridge(providerType: 'REMOTE');
      bridge.synthesisError = StateError('Authoritative synthesis failure');
      await expectLater(
        controller.speakForMessage(
          bridge: bridge,
          chatId: 'displayed-chat',
          messageTimestamp: 1728000000000,
          variantIndex: 2,
          text: 'Backend failure',
        ),
        throwsStateError,
      );
      expect(controller.state.phase, TtsPlaybackPhase.error);
      expect(
        controller.state.error,
        contains('Authoritative synthesis failure'),
      );
      expect(
        bridge.calls.where((call) => call.methodName == 'playAudio'),
        isEmpty,
      );
      expect(
        bridge.calls.where((call) => call.methodName == 'speakWithConfig'),
        isEmpty,
      );
      _expectNoIdentityGuessing(bridge);
    },
  );

  test(
    'settings voice preview remains explicit and never queries historical identity',
    () async {
      final bridge = _bridge(providerType: 'SYSTEM_TTS');
      await controller.speakWithConfig(
        bridge: bridge,
        ttsConfigId: 'explicit-preview-voice',
        text: 'Preview speech',
        title: 'Explicit preview',
      );
      await _flushEvents();
      expect(_call(bridge, 'speakWithConfig').args, {
        'ttsConfigId': 'explicit-preview-voice',
        'text': 'Preview speech',
        'interrupt': true,
      });
      expect(
        bridge.calls.where(
          (call) => call.methodName == 'chatConfigurationForMessage',
        ),
        isEmpty,
      );
      _expectNoIdentityGuessing(bridge);
    },
  );

  test(
    'invalid transcript or selected variant is rejected before any Core lookup',
    () async {
      final bridge = _bridge(providerType: 'SYSTEM_TTS');
      await expectLater(
        controller.speakForMessage(
          bridge: bridge,
          chatId: '',
          messageTimestamp: 1728000000000,
          variantIndex: 2,
          text: 'Invalid transcript',
        ),
        throwsArgumentError,
      );
      await expectLater(
        controller.speakForMessage(
          bridge: bridge,
          chatId: 'displayed-chat',
          messageTimestamp: 1728000000000,
          variantIndex: -1,
          text: 'Invalid variant',
        ),
        throwsArgumentError,
      );
      expect(bridge.calls, isEmpty);
    },
  );
}

/// Drains asynchronous lookup and queue transitions without native hooks or polling delays.
Future<void> _flushEvents() async {
  for (var index = 0; index < 12; index += 1) {
    await Future<void>.delayed(Duration.zero);
  }
}

/// Reads a single actual generated-proxy call rather than inspecting mock identity logic.
CoreCallRequest _call(_MessageVoiceBridge bridge, String method) =>
    bridge.calls.singleWhere((call) => call.methodName == method);

/// Ensures no obsolete manager or current-binding API participates in voice resolution.
void _expectNoIdentityGuessing(_MessageVoiceBridge bridge) {
  const forbidden = {
    'getAllCharacterCards',
    'getCharacterCard',
    'getCurrentTtsConfig',
    'chatConfiguration',
    'readChatConfigurationBinding',
    'speakForCharacter',
    'synthesizeForCharacter',
  };
  expect(
    bridge.calls.where((call) => forbidden.contains(call.methodName)),
    isEmpty,
  );
}

/// Ensures failed historical identity lookup never enters a playback or synthesis backend.
void _expectNoPlayback(_MessageVoiceBridge bridge) {
  const methods = {'speakWithConfig', 'synthesizeWithConfig', 'playAudio'};
  expect(
    bridge.calls.where((call) => methods.contains(call.methodName)),
    isEmpty,
  );
}

/// Defines the persisted profile independently of display names and live chat bindings.
core.ChatParticipantProfile _profile(String voiceId) =>
    core.ChatParticipantProfile(
      id: 'saved-execution-participant',
      name: 'Persisted variant speaker',
      avatarUri: null,
      introPrompt: '',
      userPreferencesText: '',
      openingStatement: '',
      modelBinding: const core.ProviderFunctionModelBinding(
        providerId: 'provider',
        modelId: 'model',
      ),
      ttsConfigId: voiceId,
      toolAccess: const core.ChatToolAccess(
        enabled: false,
        allowedBuiltinTools: [],
        allowedPackages: [],
        allowedSkills: [],
        allowedMcpServers: [],
      ),
      resources: const [],
    );

/// Creates explicit saved-message data with no production default provider fixture.
_MessageVoiceBridge _bridge({
  required String providerType,
  String voiceId = 'saved-voice-v2',
}) => _MessageVoiceBridge(
  providerType: providerType,
  profiles: {('displayed-chat', 1728000000000, 2): _profile(voiceId)},
);

/// Encodes authoritative historical profile and TTS responses through real proxy codecs.
class _MessageVoiceBridge extends OperitRuntimeBridge {
  /// Requires a known playback backend and persisted message/variant map.
  _MessageVoiceBridge({required this.providerType, required this.profiles});

  final String providerType;
  final Map<(String, int, int), core.ChatParticipantProfile> profiles;
  final List<CoreCallRequest> calls = [];
  Future<core.ChatParticipantProfile>? nextProfile;
  Future<void>? synthesisGate;
  Future<void>? speechGate;
  Object? synthesisError;

  /// Implements exact contract calls and fails unknown identity or alternate-source requests.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request);
    final args = request.args as Map<String, Object?>;
    Object? value;
    switch (request.methodName) {
      case 'chatConfigurationForMessage':
        final locator = (
          args['chatId'] as String,
          args['messageTimestamp'] as int,
          args['variantIndex'] as int,
        );
        final pending = nextProfile;
        nextProfile = null;
        final core.ChatParticipantProfile profile;
        if (pending != null) {
          profile = await pending;
        } else {
          final stored = profiles[locator];
          if (stored == null) {
            throw StateError('No persisted execution profile for $locator');
          }
          profile = stored;
        }
        value = {
          'contextKey': 'opaque-unused-by-speech',
          'messageExtension': null,
          'profile': profile.toJson(),
          'participants': [profile.toJson()],
        };
      case 'getTtsConfig':
        value = _config(args['id'] as String, providerType).toJson();
      case 'runtimeHostDescriptor':
        value = _hostDescriptor().toJson();
      case 'speakWithConfig':
        await speechGate;
        value = _speechStatus().toJson();
      case 'synthesizeWithConfig':
        await synthesisGate;
        if (synthesisError case final Object error) {
          throw error;
        }
        value = const core.TtsSynthesisResult(
          audioPaths: ['runtime:audio/voice.wav'],
          audioStoragePaths: ['audio/voice.wav'],
        ).toJson();
      case 'playAudio':
        value = core.TtsPlaybackResult(
          path: args['path'] as String,
          started: true,
          details: 'Started',
        ).toJson();
      case 'speechState':
      case 'stopSpeech':
        value = _speechStatus().toJson();
      default:
        throw StateError(
          'Unexpected message voice call: ${request.methodName}',
        );
    }
    return encodeCoreLink([0, value]);
  }

  /// Rejects uploads outside the historical speech contract.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw StateError('Unexpected TTS push');

  /// Rejects watch snapshots because the voice lifecycle uses authoritative service calls.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw StateError('Unexpected TTS snapshot');

  /// Rejects alternate watch-based identity sources explicitly.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw StateError('Unexpected TTS watch');
}

/// Provides a complete actual voice configuration for the exact saved or preview ID.
core.TtsConfig _config(String id, String providerType) => core.TtsConfig(
  id: id,
  name: 'Voice $id',
  providerType: providerType,
  endpoint: '',
  apiKey: '',
  model: '',
  voice: 'fixture-voice',
  responseFormat: 'wav',
  speed: 1,
  httpMethod: 'POST',
  requestBody: '',
  contentType: 'application/json',
  headers: const [],
  responsePipeline: const [],
  createdAt: 0,
  updatedAt: 0,
);

/// Reports immediate completed host speech so tests need no artificial polling backend.
core.TtsHostPlaybackResult _speechStatus() => const core.TtsHostPlaybackResult(
  path: 'runtime:audio/voice.wav',
  active: false,
  paused: false,
  details: 'Completed',
);

/// Supplies the real capability model without introducing a platform-specific production branch.
core.RuntimeHostDescriptor _hostDescriptor() =>
    const core.RuntimeHostDescriptor(
      id: 'test-host',
      displayName: 'Test host',
      platform: core.HostPlatform.other,
      privilege: core.HostPrivilege.normal,
      isolation: core.HostIsolation.none,
      pathStyleDescriptionEn: '',
      pathStyleDescriptionCn: '',
      examplePaths: [],
      usesEnvironmentParameter: false,
      environmentParameterDescriptionEn: '',
      environmentParameterDescriptionCn: '',
      capabilities: [],
      structuredCapabilities: [],
      onboardingRequirements: [],
      workspaceRoots: [],
      fileSystemHost: true,
      webVisitHost: false,
      systemOperationHost: false,
      audioPlaybackHost: true,
      ttsSynthesisHost: true,
      ttsPlaybackHost: true,
      systemTtsPlaybackHost: true,
      managedRuntimeHost: false,
      runtimeStorageHost: true,
      runtimeSqliteHost: false,
      browserAutomationHost: false,
      composeDslWebViewHost: false,
      terminalHost: false,
      hostRuntimeEventHost: false,
    );
