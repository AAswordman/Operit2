import {draftPreview} from './chat-draft.ts';
import type {DeviceState} from './device-state.js';
import type {RuntimeModule} from './types.js';

interface SceneAdapter {
  update(runtime: RuntimeModule, summary?: DeviceState['scene']): void;
}

/** Apply the same bounded chat fields as firmware ui.rs. Session changes
 * invalidate scene revisions and send receipts even if the stopped poll was missed. */
export class DeviceView {
  private sessionId?: string;
  private lastSendResult?: number;
  private lastSpeechResult?: number;
  private runtime: RuntimeModule;
  private scene: SceneAdapter;
  private onSpeechText?: (text: string) => void;
  private onSendResult: (ok: boolean, error?: string, sendId?: string | null) => void;
  constructor(runtime: RuntimeModule, scene: SceneAdapter,
    onSendResult: (ok: boolean, error?: string, sendId?: string | null) => void,
    onSpeechText?: (text: string) => void) {
    this.runtime = runtime;
    this.scene = scene;
    this.onSendResult = onSendResult;
    this.onSpeechText = onSpeechText;
  }

  update(state: DeviceState): void {
    const runtime = this.runtime;
    if (state.sessionId !== this.sessionId) {
      this.scene.update(runtime);
      this.sessionId = state.sessionId;
      this.lastSendResult = undefined;
      this.lastSpeechResult = undefined;
    }
    this.scene.update(runtime, state.scene);
    if (state.paired !== undefined) runtime._operit_ui_set_paired(state.paired);
    runtime.ccall('operit_ui_set_pairing_code', null, ['string'], [state.pairingCode ?? '']);
    runtime.ccall('operit_ui_set_space_state', null, ['string'], [state.spaceState ?? '等待连接 Operit']);
    runtime.ccall('operit_ui_set_space_join_prompt', null, ['string', 'number'],
      [state.spaceJoinPrompt ?? '', state.spaceJoinBusy ? 1 : 0]);
    const deviceSpace = state.deviceSpace ?? {};
    runtime.ccall('operit_ui_set_device_space_state', null,
      ['number', 'string', 'string', 'string', 'string', 'number'],
      [deviceSpace.phase ?? 0, deviceSpace.status ?? '', deviceSpace.candidates?.[0] ?? '',
        deviceSpace.candidates?.[1] ?? '', deviceSpace.candidates?.[2] ?? '', deviceSpace.pendingJoin ? 1 : 0]);
    const speech = state.speech;
    runtime.ccall('operit_ui_set_speech_state', null, ['number'], [speech?.phase ?? 0]);
    const chat = state.chat ?? {};
    runtime.ccall('operit_ui_set_chat_identity', null, ['string', 'string'], [chat.chatId ?? '', 'Operit']);
    if (speech?.resultId !== undefined && speech.resultId !== this.lastSpeechResult) {
      this.lastSpeechResult = speech.resultId;
      if (speech.text) {
        this.onSpeechText?.(speech.text);
        runtime.ccall('operit_ui_set_chat_draft', null, ['string'],
          [this.onSpeechText ? draftPreview(speech.text) : speech.text]);
      }
      if (speech.error) runtime.ccall('operit_ui_action_error', null, ['string'], [speech.error]);
    }
    const messages = (chat.messages ?? []).filter(message => message.text?.trim()).slice(-12);
    messages.forEach((message, index) => {
      runtime.ccall('operit_ui_set_message', null,
        ['number', 'number', 'string'], [index, message.sender === 'user' ? 1 : 0, message.text]);
    });
    runtime.ccall('operit_ui_finish_messages', null, ['number'], [messages.length]);
    const chatScreen = state.chatScreen ?? (chat.messages?.length ? '' : state.running ? '连接 Operit 后开始聊天' : '模拟设备已停止');
    runtime.ccall('operit_ui_set_chat_screen', null, ['string'], [chatScreen]);
    runtime.ccall('operit_ui_set_chat_task', null, ['string'], [state.chatTask ?? '离线']);
    const result = state.chatSendResult;
    if (result && result.id !== this.lastSendResult) {
      this.lastSendResult = result.id;
      this.onSendResult(result.ok, result.error, result.sendId);
    }
  }
}
