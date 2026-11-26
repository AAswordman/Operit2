// ignore_for_file: file_names

import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/widgets.dart';

import '../../../../core/host/LocalSpeechRecorder.dart';
import '../../../../core/host/RuntimeWindowRecording.dart';
import '../../../../core/logging/ClientLogger.dart';
import 'ChatVoiceWave.dart';

/// Lifecycle of one voice input session in the chat composer.
enum ChatVoiceInputPhase { idle, listening, thinking, typing, empty, error }

/// One unit of a transcript that flies into the voice card on its own.
class ChatVoiceToken {
  const ChatVoiceToken(this.text, {required this.spaced});

  /// Token text, without the word gap that may precede it.
  final String text;

  /// Whether a word gap precedes this unit in the transcript.
  final bool spaced;
}

/// Error shown by the voice card instead of returning to the composer.
class ChatVoiceInputProblem {
  const ChatVoiceInputProblem.missingConfiguration() : error = null;

  const ChatVoiceInputProblem.failed(this.error);

  /// The exception that ended the session, if any.
  final Object? error;

  /// Whether no speech recognition configuration is selected.
  bool get isMissingConfiguration => error == null;
}

/// Core streaming speech recognition driven by one voice session.
abstract class ChatVoiceInputCore {
  /// Opens a streaming session bound to [configId] and returns its id.
  Future<String> startStreamSession({required String configId});

  /// Streams the transcript recognized so far in [sessionId].
  Stream<String> streamPartialText({required String sessionId});

  /// Appends 16 kHz mono PCM16 samples to [sessionId].
  Future<void> pushStreamAudio({
    required String sessionId,
    required Uint8List audio,
  });

  /// Returns the complete transcript of [sessionId] and releases it.
  Future<String> finishStreamSession({required String sessionId});

  /// Releases [sessionId] without recognizing its audio.
  Future<void> cancelStreamSession({required String sessionId});
}

final RegExp _cjkRun = RegExp(
  r'[\u3040-\u30ff\u3400-\u4dbf\u4e00-\u9fff\uf900-\ufaff]+',
);
final RegExp _whitespaceRun = RegExp(r'\s+');
final RegExp _letterOrDigit = RegExp(r'\p{L}|\p{N}', unicode: true);

/// Splits a transcript into reveal units: whitespace-delimited words, with
/// unspaced CJK runs broken into pairs so Chinese still lands word by word.
List<ChatVoiceToken> chatVoiceTokensOf(String transcript) {
  final tokens = <ChatVoiceToken>[];
  for (final word in transcript.trim().split(_whitespaceRun)) {
    if (word.isEmpty) continue;
    final start = tokens.length;
    var cursor = 0;
    for (final run in _cjkRun.allMatches(word)) {
      _appendVerbatim(tokens, word.substring(cursor, run.start));
      _appendCjkRun(tokens, run.group(0)!);
      cursor = run.end;
    }
    _appendVerbatim(tokens, word.substring(cursor));
    if (start > 0 && tokens.length > start) {
      tokens[start] = ChatVoiceToken(tokens[start].text, spaced: true);
    }
  }
  return List<ChatVoiceToken>.unmodifiable(tokens);
}

/// Appends a non-CJK run, folding bare punctuation into the unit before it.
void _appendVerbatim(List<ChatVoiceToken> tokens, String text) {
  if (text.isEmpty) return;
  if (tokens.isNotEmpty && !_letterOrDigit.hasMatch(text)) {
    final previous = tokens.last;
    tokens[tokens.length - 1] = ChatVoiceToken(
      '${previous.text}$text',
      spaced: previous.spaced,
    );
    return;
  }
  tokens.add(ChatVoiceToken(text, spaced: false));
}

  /// Appends one CJK run two characters at a time.
void _appendCjkRun(List<ChatVoiceToken> tokens, String run) {
  final characters = run.characters;
  for (var index = 0; index < characters.length; index += 2) {
    tokens.add(
      ChatVoiceToken(characters.skip(index).take(2).join(), spaced: false),
    );
  }
}

/// Splits [text] into reveal units while keeping [tokens] of [known] unchanged
/// when [text] only extends it. Core appends recognized phrases, so this stops
/// a short Chinese phrase from re-pairing the words that follow it.
List<ChatVoiceToken> chatVoiceTokensAfter(
  String text, {
  required String known,
  required List<ChatVoiceToken> tokens,
}) {
  if (text.length <= known.length || !text.startsWith(known)) {
    return chatVoiceTokensOf(text);
  }
  final suffix = chatVoiceTokensOf(text.substring(known.length));
  final result = List<ChatVoiceToken>.of(tokens);
  final gap = _whitespaceRun.hasMatch(
    text.substring(known.length, known.length + 1),
  );
  for (var index = 0; index < suffix.length; index += 1) {
    final token = suffix[index];
    if (index == 0 && result.isNotEmpty && !_letterOrDigit.hasMatch(token.text)) {
      result[result.length - 1] = ChatVoiceToken(
        '${result.last.text}${token.text}',
        spaced: result.last.spaced,
      );
      continue;
    }
    result.add(
      index == 0 && gap ? ChatVoiceToken(token.text, spaced: true) : token,
    );
  }
  return List<ChatVoiceToken>.unmodifiable(result);
}

/// Runs one voice input session: it streams microphone audio to Core, shows
/// recognized words as they arrive and hands the final transcript to [commit].
class ChatVoiceInputSession extends ChangeNotifier {
  /// Creates a session bound to Core through [core].
  ChatVoiceInputSession({
    required ChatVoiceInputCore core,
    required Future<String?> Function() selectedConfigId,
    required void Function(String text) commit,
  }) : _core = core,
       _selectedConfigId = selectedConfigId,
       _commit = commit;

  static const String _logTag = 'LocalSTT';

  /// Audio delivered to Core per push, 160 ms of 16 kHz mono PCM16.
  static const int _pushBytes = LocalSpeechRecorder.sampleRate * 320 ~/ 1000;

  /// Trailing silence that ends the capture after speech, 1.2 s.
  static const int _trailingSilenceMs = 1200;

  /// Longest capture accepted from one voice session, 120 s.
  static const int _maxCaptureMs = 120000;

  /// Time a transcript takes to land word by word when nothing streamed live.
  static const Duration _typedDuration = Duration(milliseconds: 2600);

  /// Pause between words of a transcript with at most [_typedWordLimit] words.
  static const Duration _typedWordInterval = Duration(milliseconds: 260);

  /// Words that are still revealed at the full [_typedWordInterval] pause.
  static const int _typedWordLimit = 10;

  /// How long the "nothing heard" notice stays before the composer returns.
  static const Duration _emptyNoticeDuration = Duration(milliseconds: 1600);

  /// Time one token takes to fly in and sharpen after it is shown.
  static Duration get _settleDuration =>
      ChatVoiceWave.tokenRevealDelay + ChatVoiceWave.tokenRevealDuration;

  final ChatVoiceInputCore _core;
  final Future<String?> Function() _selectedConfigId;
  final void Function(String text) _commit;
  final Stopwatch _elapsed = Stopwatch();
  final BytesBuilder _pendingPcm = BytesBuilder();

  ChatVoiceInputPhase _phase = ChatVoiceInputPhase.idle;
  List<ChatVoiceToken> _tokens = const <ChatVoiceToken>[];
  ChatVoiceInputProblem? _problem;
  Timer? _clock;
  Timer? _emptyTimer;
  RuntimeWindowRecording? _capture;
  StreamSubscription<String>? _partialTexts;
  String? _sessionId;
  String? _transcript;
  String? _recognizedText;
  Future<void> _pushChain = Future<void>.value();
  Object? _pushError;
  DateTime? _tokensChangedAt;
  int _generation = 0;
  bool _finishing = false;

  /// Current phase of the session.
  ChatVoiceInputPhase get phase => _phase;

  /// Transcript units shown so far, in reveal order.
  List<ChatVoiceToken> get tokens => _tokens;

  /// Error shown during [ChatVoiceInputPhase.error].
  ChatVoiceInputProblem? get problem => _problem;

  /// Time spent listening, frozen once the capture ends.
  Duration get elapsed => _elapsed.elapsed;

  /// Smoothed microphone level from zero to one during the capture.
  double get level => _capture?.level ?? 0.0;

  /// Starts listening. A notice or error from an earlier session is cleared.
  Future<void> start() async {
    if (_phase != ChatVoiceInputPhase.idle &&
        _phase != ChatVoiceInputPhase.empty &&
        _phase != ChatVoiceInputPhase.error) {
      return;
    }
    final generation = ++_generation;
    _emptyTimer?.cancel();
    _tokens = const <ChatVoiceToken>[];
    _tokensChangedAt = null;
    _problem = null;
    _transcript = null;
    _recognizedText = null;
    _pushError = null;
    _pendingPcm.clear();
    _finishing = false;
    _setPhase(ChatVoiceInputPhase.listening);
    final configId = await _selectedConfigId();
    if (generation != _generation) return;
    if (configId == null) {
      _fail(const ChatVoiceInputProblem.missingConfiguration());
      return;
    }
    try {
      final sessionId = await _core.startStreamSession(configId: configId);
      if (generation != _generation) {
        unawaited(_core.cancelStreamSession(sessionId: sessionId).catchError((_) {}));
        return;
      }
      _sessionId = sessionId;
      _partialTexts = _core
          .streamPartialText(sessionId: sessionId)
          .listen(
            _handlePartialText,
            onError: (Object error, StackTrace stackTrace) => ClientLogger.e(
              'speech partial text failed',
              tag: _logTag,
              error: error,
              stackTrace: stackTrace,
            ),
          );
      final capture = RuntimeWindowRecording(
        _maxCaptureMs,
        _trailingSilenceMs,
        onChunk: _queuePcm,
      );
      _capture = capture;
      _elapsed
        ..reset()
        ..start();
      _clock = Timer.periodic(
        const Duration(seconds: 1),
        (_) => notifyListeners(),
      );
      unawaited(
        capture.result.then<void>(
          (_) => _finishCapture(generation),
          onError: (Object error, StackTrace stackTrace) {
            if (generation != _generation) return;
            _fail(ChatVoiceInputProblem.failed(error), stackTrace);
          },
        ),
      );
    } catch (error, stackTrace) {
      if (generation != _generation) return;
      _fail(ChatVoiceInputProblem.failed(error), stackTrace);
    }
  }

  /// Stops listening and finalizes the transcript.
  Future<void> stop() async {
    if (_phase != ChatVoiceInputPhase.listening) return;
    final capture = _capture;
    if (capture == null) {
      // The session is still opening; start() abandons it on its next check.
      _generation += 1;
      await _releaseCore();
      _setPhase(ChatVoiceInputPhase.idle);
      return;
    }
    await capture.finish(discard: true);
  }

  /// Skips the word-by-word reveal, or returns a notice or error to the composer.
  void advance() {
    switch (_phase) {
      case ChatVoiceInputPhase.typing:
        final transcript = _transcript;
        if (transcript == null) return;
        _setPhase(ChatVoiceInputPhase.idle);
        _commit(transcript);
      case ChatVoiceInputPhase.empty:
        _emptyTimer?.cancel();
        _setPhase(ChatVoiceInputPhase.idle);
      case ChatVoiceInputPhase.error:
        _setPhase(ChatVoiceInputPhase.idle);
      case ChatVoiceInputPhase.idle:
      case ChatVoiceInputPhase.listening:
      case ChatVoiceInputPhase.thinking:
        break;
    }
  }

  @override
  void dispose() {
    _generation += 1;
    _clock?.cancel();
    _emptyTimer?.cancel();
    unawaited(_capture?.finish(discard: true));
    unawaited(_releaseCore());
    super.dispose();
  }

  void _setPhase(ChatVoiceInputPhase phase) {
    if (_phase == phase) return;
    _phase = phase;
    notifyListeners();
  }

  /// Shows words Core recognized while the user is still speaking.
  void _handlePartialText(String text) {
    if (_phase != ChatVoiceInputPhase.listening) return;
    _showTokens(_tokensFor(text));
  }

  /// Splits [text], leaving the units of the transcript shown so far intact.
  List<ChatVoiceToken> _tokensFor(String text) {
    final tokens = chatVoiceTokensAfter(
      text,
      known: _recognizedText ?? '',
      tokens: _tokens,
    );
    _recognizedText = text;
    return tokens;
  }

  /// Shows [tokens], ignoring an update that repeats what is already shown so
  /// an unchanged final transcript does not wait for a second flight.
  void _showTokens(List<ChatVoiceToken> tokens) {
    final previous = _tokens;
    var changed = previous.length != tokens.length;
    for (var index = 0; !changed && index < tokens.length; index += 1) {
      changed = previous[index].text != tokens[index].text;
    }
    if (!changed) return;
    _tokens = tokens;
    _tokensChangedAt = DateTime.now();
    notifyListeners();
  }

  /// Batches microphone chunks so each Core push carries a useful amount.
  void _queuePcm(Uint8List bytes) {
    if (_sessionId == null || _finishing) return;
    _pendingPcm.add(bytes);
    if (_pendingPcm.length < _pushBytes) return;
    _flushPcm();
  }

  /// Pushes buffered audio to Core in order, one push at a time.
  void _flushPcm() {
    final sessionId = _sessionId;
    if (sessionId == null || _pendingPcm.isEmpty) return;
    final chunk = Uint8List.fromList(_pendingPcm.takeBytes());
    _pushChain = _pushChain.then((_) async {
      try {
        await _core.pushStreamAudio(sessionId: sessionId, audio: chunk);
      } catch (error, stackTrace) {
        ClientLogger.e(
          'speech audio push failed',
          tag: _logTag,
          error: error,
          stackTrace: stackTrace,
        );
        _pushError ??= error;
      }
    });
  }

  /// Delivers the remaining audio, waits for the transcript and hands it over.
  Future<void> _finishCapture(int generation) async {
    if (generation != _generation || _finishing) return;
    _finishing = true;
    _elapsed.stop();
    _clock?.cancel();
    _capture = null;
    _setPhase(ChatVoiceInputPhase.thinking);
    _flushPcm();
    await _pushChain;
    if (generation != _generation) return;
    final sessionId = _sessionId;
    final pushError = _pushError;
    if (sessionId == null || pushError != null) {
      await _releaseCore();
      if (pushError != null) {
        _fail(ChatVoiceInputProblem.failed(pushError));
      } else {
        _setPhase(ChatVoiceInputPhase.idle);
      }
      return;
    }
    final String transcript;
    try {
      transcript = await _core.finishStreamSession(sessionId: sessionId);
    } catch (error, stackTrace) {
      if (generation == _generation) {
        // Core releases the session before reporting the failure.
        _sessionId = null;
        await _partialTexts?.cancel();
        _partialTexts = null;
        _fail(ChatVoiceInputProblem.failed(error), stackTrace);
      }
      return;
    } finally {
      _finishing = false;
    }
    if (generation != _generation) return;
    _sessionId = null;
    await _partialTexts?.cancel();
    _partialTexts = null;
    await _deliver(transcript, generation);
  }

  /// Reveals the final transcript and commits it once its last word settled.
  Future<void> _deliver(String transcript, int generation) async {
    final tokens = _tokensFor(transcript);
    if (tokens.isEmpty) {
      _setPhase(ChatVoiceInputPhase.empty);
      _emptyTimer = Timer(_emptyNoticeDuration, () {
        if (_phase == ChatVoiceInputPhase.empty) {
          _setPhase(ChatVoiceInputPhase.idle);
        }
      });
      return;
    }
    _transcript = transcript;
    if (_tokens.isEmpty) {
      // Nothing streamed live, so the transcript lands word by word now.
      _setPhase(ChatVoiceInputPhase.typing);
      final interval = tokens.length > _typedWordLimit
          ? Duration(
              microseconds: _typedDuration.inMicroseconds ~/ tokens.length,
            )
          : _typedWordInterval;
      for (var count = 1; count <= tokens.length; count += 1) {
        if (_phase != ChatVoiceInputPhase.typing || generation != _generation) {
          return;
        }
        _showTokens(List<ChatVoiceToken>.unmodifiable(tokens.take(count)));
        await Future<void>.delayed(interval);
      }
    } else {
      _showTokens(tokens);
    }
    final changedAt = _tokensChangedAt;
    final remaining = changedAt == null
        ? Duration.zero
        : changedAt.add(_settleDuration).difference(DateTime.now());
    if (remaining > Duration.zero) {
      await Future<void>.delayed(remaining);
    }
    if (generation != _generation) return;
    _setPhase(ChatVoiceInputPhase.idle);
    _commit(transcript);
  }

  /// Ends the session with [problem], leaving the transcript uncommitted.
  void _fail(ChatVoiceInputProblem problem, [StackTrace? stackTrace]) {
    final error = problem.error;
    if (error != null) {
      ClientLogger.e(
        'speech input failed',
        tag: _logTag,
        error: error,
        stackTrace: stackTrace,
      );
    }
    _elapsed.stop();
    _clock?.cancel();
    unawaited(_releaseCore());
    _problem = problem;
    _setPhase(ChatVoiceInputPhase.error);
  }

  /// Cancels the Core session of the current capture, if one was opened.
  Future<void> _releaseCore() async {
    final sessionId = _sessionId;
    _sessionId = null;
    await _partialTexts?.cancel();
    _partialTexts = null;
    if (sessionId == null) return;
    try {
      await _core.cancelStreamSession(sessionId: sessionId);
    } catch (error, stackTrace) {
      ClientLogger.e(
        'speech stream cancel failed',
        tag: _logTag,
        error: error,
        stackTrace: stackTrace,
      );
    }
  }
}
