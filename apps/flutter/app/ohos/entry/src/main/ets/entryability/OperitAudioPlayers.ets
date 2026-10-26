import media from '@ohos.multimedia.media';
import audio from '@ohos.multimedia.audio';
import fs from '@ohos.file.fs';
import common from '@ohos.app.ability.common';
import { FlutterEngine } from '@ohos/flutter_ohos';
import MethodCall from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodCall';
import MethodChannel, { MethodResult } from '@ohos/flutter_ohos/src/main/ets/plugin/common/MethodChannel';
import EventChannel, { EventSink, StreamHandler } from '@ohos/flutter_ohos/src/main/ets/plugin/common/EventChannel';

/** Owns one audioplayers event subscription without sharing players between widgets. */
class AudioEvents implements StreamHandler {
  private sink: EventSink | null = null;
  /** Attaches Dart's existing broadcast stream. */
  onListen(_args: Object, events: EventSink): void { this.sink = events; }
  /** Detaches the listener while the player is retained. */
  onCancel(_args: Object): void { this.sink = null; }
  /** Emits the platform interface's documented event map. */
  emit(event: string, value: Object | null = null): void {
    const payload = new Map<string, Object | null>();
    payload.set('event', event);
    payload.set('value', value);
    this.sink?.success(payload);
  }
  /** Reports native playback failures to the player stream. */
  error(message: string): void { this.sink?.error('OHOS_AUDIO_ERROR', message, null); }
}

/** Adapts AVPlayer's asynchronous state machine to the existing audioplayers protocol. */
class AudioPlayer {
  readonly player: media.AVPlayer;
  readonly events: AudioEvents = new AudioEvents();
  readonly channel: EventChannel;
  private context: common.UIAbilityContext;
  private sourceFile: fs.File | null = null;
  private temporary: string | null = null;
  private prepared: (() => void) | null = null;
  private failed: ((error: Error) => void) | null = null;
  private volume: number = 1;
  private loop: boolean = false;

  /** Binds state and seek notifications before accepting a source. */
  constructor(engine: FlutterEngine, context: common.UIAbilityContext, id: string, player: media.AVPlayer) {
    this.context = context;
    this.player = player;
    this.channel = new EventChannel(engine.getDartExecutor(), `xyz.luan/audioplayers/events/${id}`);
    this.channel.setStreamHandler(this.events);
    player.on('stateChange', (state: media.AVPlayerState): void => {
      if (state === 'initialized') {
        player.audioRendererInfo = { usage: audio.StreamUsage.STREAM_USAGE_MUSIC, rendererFlags: 0 };
        player.prepare().catch((error: Error): void => this.rejectSource(error));
      } else if (state === 'prepared') {
        player.loop = this.loop;
        player.setVolume(this.volume);
        this.events.emit('audio.onDuration', player.duration);
        this.events.emit('audio.onPrepared', true);
        this.prepared?.();
      } else if (state === 'completed') {
        this.events.emit('audio.onComplete');
      }
    });
    player.on('seekDone', (_position: number): void => this.events.emit('audio.onSeekComplete'));
    player.on('error', (error: Error): void => {
      this.rejectSource(error);
      this.events.error(error.message);
    });
  }

  /** Rejects the source operation with the native error. */
  private rejectSource(error: Error): void { this.failed?.(error); }

  /** Opens URL, local-file, and byte sources with a retained file descriptor. */
  async setSource(call: MethodCall, id: string): Promise<void> {
    if (this.player.state !== 'idle') await this.player.reset();
    this.closeSource();
    let path: string | null = null;
    if (call.method === 'setSourceBytes') {
      const bytes = call.argument('bytes') as Uint8Array;
      path = `${this.context.tempDir}/operit-audio-${encodeURIComponent(id)}`;
      this.temporary = path;
      const file = fs.openSync(path, fs.OpenMode.CREATE | fs.OpenMode.TRUNC | fs.OpenMode.WRITE_ONLY);
      try {
        let offset = 0;
        while (offset < bytes.byteLength) {
          const count = fs.writeSync(file.fd, bytes.buffer.slice(bytes.byteOffset + offset, bytes.byteOffset + bytes.byteLength));
          if (count <= 0) throw new Error('Incomplete audio source write');
          offset += count;
        }
      } finally { fs.closeSync(file); }
    } else if (call.argument('isLocal') === true) {
      path = call.argument('url') as string;
    }
    await new Promise<void>((resolve, reject): void => {
      const timer = setTimeout((): void => this.rejectSource(new Error('Audio preparation timed out')), 30000);
      this.prepared = (): void => { clearTimeout(timer); this.prepared = null; this.failed = null; resolve(); };
      this.failed = (error: Error): void => { clearTimeout(timer); this.prepared = null; this.failed = null; reject(error); };
      try {
        if (path !== null) {
          this.sourceFile = fs.openSync(path, fs.OpenMode.READ_ONLY);
          this.player.fdSrc = { fd: this.sourceFile.fd, offset: 0, length: fs.statSync(this.sourceFile.fd).size };
        } else { this.player.url = call.argument('url') as string; }
      } catch (error) { this.rejectSource(new Error(`${error}`)); }
    });
  }

  /** Updates volume before or after source preparation. */
  setVolume(volume: number): void {
    this.volume = volume;
    if (this.player.state === 'prepared' || this.player.state === 'playing' || this.player.state === 'paused' || this.player.state === 'completed') {
      this.player.setVolume(volume);
    }
  }

  /** Retains the requested completion mode across sources. */
  setReleaseMode(mode: string): void {
    this.loop = mode === 'ReleaseMode.loop';
    if (this.player.state === 'prepared' || this.player.state === 'playing' || this.player.state === 'paused' || this.player.state === 'completed') {
      this.player.loop = this.loop;
    }
  }

  /** Releases only source resources after AVPlayer has reset or released them. */
  private closeSource(): void {
    if (this.sourceFile !== null) { fs.closeSync(this.sourceFile); this.sourceFile = null; }
    if (this.temporary !== null) { fs.unlinkSync(this.temporary); this.temporary = null; }
  }

  /** Resets media while retaining the player and event channel. */
  async releaseSource(): Promise<void> {
    if (this.player.state !== 'idle') await this.player.reset();
    this.closeSource();
  }

  /** Releases native playback, descriptors, temporary bytes, and event registration. */
  async dispose(): Promise<void> {
    this.rejectSource(new Error('Audio player disposed'));
    await this.player.release();
    this.closeSource();
    this.channel.setStreamHandler(null);
  }
}

/** Registers the standard audioplayers channels on the OHOS owner. */
export class OperitAudioPlayers {
  private engine: FlutterEngine;
  private context: common.UIAbilityContext;
  private players: Map<string, AudioPlayer> = new Map();
  private method: MethodChannel;
  private global: MethodChannel;
  private globalEvents: EventChannel;
  private events: AudioEvents = new AudioEvents();

  /** Installs protocol handlers before Flutter creates its first audio widget. */
  constructor(engine: FlutterEngine, context: common.UIAbilityContext) {
    this.engine = engine;
    this.context = context;
    this.method = new MethodChannel(engine.getDartExecutor(), 'xyz.luan/audioplayers');
    this.global = new MethodChannel(engine.getDartExecutor(), 'xyz.luan/audioplayers.global');
    this.globalEvents = new EventChannel(engine.getDartExecutor(), 'xyz.luan/audioplayers.global/events');
    this.globalEvents.setStreamHandler(this.events);
    this.method.setMethodCallHandler({ onMethodCall: (call, result): void => {
      this.handle(call).then((value: Object | null): void => result.success(value))
        .catch((error: Error): void => result.error('OHOS_AUDIO_ERROR', error.message, null));
    }});
    this.global.setMethodCallHandler({ onMethodCall: (call, result): void => this.handleGlobal(call, result) });
  }

  /** Implements global initialization, context, and diagnostics. */
  private handleGlobal(call: MethodCall, result: MethodResult): void {
    switch (call.method) {
      case 'init': case 'setAudioContext': result.success(null); return;
      case 'emitLog': this.events.emit('audio.onLog', call.argument('message') as string); result.success(null); return;
      case 'emitError': this.events.error(call.argument('message') as string); result.success(null); return;
      default: result.notImplemented();
    }
  }

  /** Executes controls against the exact player requested by Dart. */
  private async handle(call: MethodCall): Promise<Object | null> {
    const id = call.argument('playerId') as string;
    if (call.method === 'create') {
      if (this.players.has(id)) throw new Error(`Audio player already exists: ${id}`);
      this.players.set(id, new AudioPlayer(this.engine, this.context, id, await media.createAVPlayer()));
      return null;
    }
    const item = this.players.get(id);
    if (item === undefined) throw new Error(`Unknown audio player: ${id}`);
    const player = item.player;
    switch (call.method) {
      case 'setSourceBytes': case 'setSourceUrl': await item.setSource(call, id); break;
      case 'resume': await player.play(); break;
      case 'pause': await player.pause(); break;
      case 'stop':
        if (player.state === 'playing') await player.pause();
        if (player.state === 'prepared' || player.state === 'paused' || player.state === 'completed') player.seek(0);
        break;
      case 'seek': player.seek(call.argument('position') as number); break;
      case 'getCurrentPosition': return player.currentTime;
      case 'getDuration': return player.duration;
      case 'setVolume': item.setVolume(call.argument('volume') as number); break;
      case 'setReleaseMode': item.setReleaseMode(call.argument('releaseMode') as string); break;
      case 'setPlaybackRate': {
        const rate = call.argument('playbackRate') as number;
        const rates = new Map<number, media.PlaybackSpeed>([
          [0.5, media.PlaybackSpeed.SPEED_FORWARD_0_50_X], [0.75, media.PlaybackSpeed.SPEED_FORWARD_0_75_X],
          [1, media.PlaybackSpeed.SPEED_FORWARD_1_00_X], [1.25, media.PlaybackSpeed.SPEED_FORWARD_1_25_X],
          [1.5, media.PlaybackSpeed.SPEED_FORWARD_1_50_X], [1.75, media.PlaybackSpeed.SPEED_FORWARD_1_75_X],
          [2, media.PlaybackSpeed.SPEED_FORWARD_2_00_X]
        ]);
        const speed = rates.get(rate);
        if (speed === undefined) throw new Error(`Unsupported audio playback rate: ${rate}`);
        player.setSpeed(speed); break;
      }
      case 'setAudioContext': break;
      case 'setPlayerMode':
        if (call.argument('playerMode') !== 'PlayerMode.mediaPlayer') throw new Error('Low latency audio mode is unavailable');
        break;
      case 'setBalance':
        if (call.argument('balance') !== 0) throw new Error('AVPlayer does not expose stereo balance');
        break;
      case 'release': await item.releaseSource(); break;
      case 'dispose': await item.dispose(); this.players.delete(id); break;
      case 'emitLog': item.events.emit('audio.onLog', call.argument('message') as string); break;
      case 'emitError': item.events.error(call.argument('message') as string); break;
      default: throw new Error(`Unknown audio operation: ${call.method}`);
    }
    return null;
  }

  /** Detaches channels and releases players when the Flutter owner is destroyed. */
  async dispose(): Promise<void> {
    this.method.setMethodCallHandler(null);
    this.global.setMethodCallHandler(null);
    this.globalEvents.setStreamHandler(null);
    for (const item of this.players.values()) await item.dispose();
    this.players.clear();
  }
}
