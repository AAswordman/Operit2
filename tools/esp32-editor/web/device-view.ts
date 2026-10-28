import {draftPreview} from './chat-draft.ts';
import type {DeviceState} from './device-state.js';
import type {RuntimeModule} from './types.js';

interface SceneAdapter {
  update(runtime: RuntimeModule, summary?: DeviceState['scene']): void;
}

/** Apply the same bounded chat/plugin fields as firmware ui.rs. Session changes
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
    runtime.ccall('operit_ui_set_chat_history', null, ['number', 'number'], [chat.hasOlder ? 1 : 0, chat.hasNewer ? 1 : 0]);
    const character = chat.conversations?.find(item => item.id === chat.chatId)?.characterCardName || 'Operit';
    runtime.ccall('operit_ui_set_chat_identity', null, ['string', 'string'], [chat.chatId ?? '', character]);
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
    const conversations = (chat.conversations ?? []).slice(0, 24);
    conversations.forEach((item, index) => runtime.ccall('operit_ui_set_conversation', null,
      ['number', 'string', 'string', 'string', 'number'],
      [index, item.id, item.title, item.characterCardName ?? '', item.id === chat.chatId ? 1 : 0]));
    runtime.ccall('operit_ui_finish_conversations', null, ['number'], [conversations.length]);
    runtime.ccall('operit_ui_set_conversations_state', null, ['number', 'string'], [chat.conversationsLoading ? 1 : 0, chat.conversationsError ?? '']);
    const plugins = state.plugins;
    runtime.ccall('operit_ui_set_plugin_category', null, ['number'], [plugins?.category === 'exclusive' ? 1 : 0]);
    const items = (plugins?.items ?? []).slice(0, 6);
    items.forEach((item, index) => {
      const status = item.status === 'probing' ? 1 : item.status === 'success' ? 2 : item.status === 'failure' ? 3 : 0;
      const toolStatus = item.toolStatus === 'probing' ? 1 : item.toolStatus === 'success' ? 2 : item.toolStatus === 'failure' ? 3 : 0;
      runtime.ccall('operit_ui_set_plugin', null, ['number', 'string', 'string', 'number', 'number'],
        [index, item.id, item.name, status, item.latencyMs ?? 0]);
      runtime.ccall('operit_ui_set_plugin_test', null, ['number', 'number', 'number', 'string'],
        [index, toolStatus, item.toolLatencyMs ?? 0, item.testError ?? '']);
    });
    const details = plugins?.details;
    runtime.ccall('operit_ui_set_plugin_details', null, ['string', 'string', 'string', 'number', 'number', 'number', 'string'],
      [details?.id ?? '', details?.description ?? '', (details?.tools ?? []).slice(0, 3).join('\n'), details?.toolOffset ?? 0,
        details?.toolTotal ?? 0, details?.loading ? 1 : 0, details?.error ?? '']);
    runtime.ccall('operit_ui_set_plugin_testing', null, ['number'], [plugins?.testing ? 1 : 0]);
    runtime.ccall('operit_ui_finish_plugins', null, ['number', 'number', 'number', 'number', 'string'],
      [items.length, plugins?.offset ?? 0, plugins?.total ?? 0, plugins?.loading ? 1 : 0, plugins?.error ?? '']);
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
