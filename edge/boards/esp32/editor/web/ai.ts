import {applyOperations} from '../src/layout/layout-model.mjs';
import {findNode, nodePointer, pagesOf} from '../src/layout/project-model.mjs';
import {request} from './transport.js';
import {errorMessage, query} from './types.js';
import type {
  AiContext,
  AiProposal,
  ComponentSource,
  EditorSetupOptions,
} from './types.js';

interface ComponentAiEvent {
  requirement?: string;
}

/** Installs the AI task dialog and its reviewed layout/code proposal actions. */
export function setupAI({snapshot, commit, notify}: EditorSetupOptions): {updateContext: () => void} {
  const dialog = query<HTMLDialogElement>('#ai-dialog');
  const contextLabel = query<HTMLElement>('#ai-context-label');
  const providerStatus = query<HTMLElement>('#ai-provider-status');
  const taskInput = query<HTMLTextAreaElement>('#ai-task');
  const endpointInput = query<HTMLInputElement>('#ai-url');
  const modelInput = query<HTMLInputElement>('#ai-model');
  const keyInput = query<HTMLInputElement>('#ai-key');
  const answer = query<HTMLElement>('#ai-answer');
  const proposalOutput = query<HTMLElement>('#ai-proposal');
  const feedbackOutput = query<HTMLElement>('#ai-feedback');
  const proposalDetails = query<HTMLDetailsElement>('#ai-proposal-details');
  const sendButton = query<HTMLButtonElement>('#ai-send');
  const cancelButton = query<HTMLButtonElement>('#ai-cancel');
  const applyButton = query<HTMLButtonElement>('#ai-apply');
  const applyCodeButton = query<HTMLButtonElement>('#ai-apply-code');
  const copyButton = query<HTMLButtonElement>('#ai-copy');
  let proposal: AiProposal | null = null;
  let baseDocument = '';
  let controller: AbortController | null = null;
  let busy = false;

  /** Displays a message below the AI task controls. */
  function feedback(message: string): void {
    feedbackOutput.textContent = message;
  }

  /** Updates the task context label with the selected page and component. */
  function updateContext(): void {
    const state = snapshot();
    const page = pagesOf(state.document).find((item) => item.id === state.pageId);
    contextLabel.textContent =
      `${page?.name ?? '首页'} / ${state.componentId ?? '整页'}${state.dirty ? ' · 含未保存修改' : ''}`;
  }

  /** Describes the configured AI provider without persisting credentials. */
  function updateProviderStatus(): void {
    providerStatus.textContent = '任务发送到你填写的服务。支持读取项目源码并提出布局与代码修改。';
  }

  /** Opens the AI task dialog with the current editor context. */
  function openDialog(): void {
    updateContext();
    updateProviderStatus();
    if (!dialog.open) dialog.showModal();
  }

  /** Builds the source-aware context sent to the AI service. */
  async function buildContext(): Promise<AiContext> {
    const state = snapshot();
    const component = state.componentId ? findNode(state.document, state.componentId)?.node : undefined;
    const source = component
      ? await request<ComponentSource>(
          '/api/component-source?id=' +
            encodeURIComponent(component.id) +
            '&revision=' +
            encodeURIComponent(state.revision),
        )
      : null;
    const pointer = component
      ? nodePointer(state.document, component.id)
      : state.pageId === 'home'
        ? '/nodes'
        : `/pages/${state.document.pages?.findIndex((page) => page.id === state.pageId) ?? -1}`;
    return {
      ...state,
      source,
      component,
      pointer,
      sourceFile: 'edge/boards/esp32/ui/layout.json',
      instruction:
        '请阅读引用的项目源码，根据任务实现布局、交互和功能路由。layout.json 是界面源文件，保存布局无需编译，可打包部署到运行时。保留当前未保存草稿；新增底层能力才修改 C/Rust 并构建基础固件。',
    };
  }

  /** Checks that an AI response has the proposal shape used by the dialog. */
  function isProposal(value: unknown): value is AiProposal {
    if (typeof value !== 'object' || value === null || Array.isArray(value)) return false;
    if (!('summary' in value) || typeof value.summary !== 'string') return false;
    if (!('operations' in value) || !Array.isArray(value.operations)) return false;
    if (!('codeEdits' in value) || !Array.isArray(value.codeEdits)) return false;
    return true;
  }

  /** Renders a proposal and enables only the actions it contains. */
  function showProposal(result: AiProposal): void {
    proposal = result;
    answer.textContent = result.summary || '任务已发送到软件对话。';
    proposalOutput.textContent = JSON.stringify(
      {operations: result.operations, codeEdits: result.codeEdits},
      null,
      2,
    );
    applyButton.disabled = result.operations.length === 0;
    applyCodeButton.disabled = result.codeEdits.length === 0;
    proposalDetails.open = result.operations.length > 0 || result.codeEdits.length > 0;
  }

  /** Copies the current task and source-aware context to the clipboard. */
  async function copyContext(): Promise<void> {
    try {
      const context = await buildContext();
      await navigator.clipboard.writeText(JSON.stringify({task: taskInput.value, context}, null, 2));
      feedback('已复制任务和当前源码位置');
    } catch (error) {
      feedback(errorMessage(error));
    }
  }

  /** Sends the current task to the configured OpenAI-compatible endpoint. */
  async function sendTask(): Promise<void> {
    if (busy) return;
    const task = taskInput.value.trim();
    if (!task) {
      feedback('请填写开发任务');
      return;
    }
    proposal = null;
    applyButton.disabled = true;
    applyCodeButton.disabled = true;
    busy = true;
    sendButton.disabled = true;
    cancelButton.disabled = false;
    controller = new AbortController();
    const signal = controller.signal;
    try {
      const context = await buildContext();
      baseDocument = JSON.stringify(context.document);
      feedback('正在处理任务…');
      const result = await request<unknown>('/api/ai/develop', {
        method: 'POST',
        body: {
          endpoint: endpointInput.value.trim(),
          model: modelInput.value.trim(),
          key: keyInput.value,
          task,
          context,
        },
        signal,
      });
      if (!isProposal(result)) throw new Error('AI 返回的提案格式无效');
      showProposal(result);
      feedback('提案已生成，请查看修改后应用');
    } catch (error) {
      feedback(signal.aborted ? '已停止等待 AI 回复' : errorMessage(error));
    } finally {
      busy = false;
      sendButton.disabled = false;
      cancelButton.disabled = true;
      controller = null;
    }
  }

  /** Cancels the active AI request. */
  function cancelTask(): void {
    controller?.abort();
    feedback('正在停止…');
  }

  /** Applies the reviewed layout operations to the local editor draft. */
  function applyLayout(): void {
    try {
      if (!proposal) throw new Error('没有可应用的布局提案');
      const state = snapshot();
      if (JSON.stringify(state.document) !== baseDocument) {
        throw new Error('生成提案后布局已变化，请基于最新布局重新发送任务');
      }
      const next = applyOperations(state.document, proposal.operations);
      if (commit(next)) {
        baseDocument = JSON.stringify(next);
        applyButton.disabled = true;
        feedback('已应用到草稿，检查页面后保存项目并部署');
        notify('AI 已修改项目草稿');
      }
    } catch (error) {
      feedback(errorMessage(error));
    }
  }

  /** Applies the reviewed exact source edits through the local server. */
  async function applyCode(): Promise<void> {
    try {
      if (!proposal || proposal.codeEdits.length === 0) throw new Error('没有可写入的代码提案');
      await request('/api/ai/apply-code', {
        method: 'POST',
        body: {codeEdits: proposal.codeEdits},
      });
      applyCodeButton.disabled = true;
      feedback('代码修改已写入项目；由开发环境运行 npm run build:firmware 验证。布局提案仍需单独应用并保存。');
    } catch (error) {
      feedback(errorMessage(error));
    }
  }

  /** Accepts a component task event emitted by the component inspector. */
  function receiveComponentTask(event: Event): void {
    const detail = (event as CustomEvent<ComponentAiEvent>).detail;
    taskInput.value = detail?.requirement ?? '';
    openDialog();
  }

  query<HTMLButtonElement>('#open-ai').addEventListener('click', openDialog);
  query<HTMLButtonElement>('#ai-close').addEventListener('click', () => dialog.close());
  copyButton.addEventListener('click', () => void copyContext());
  sendButton.addEventListener('click', () => void sendTask());
  cancelButton.addEventListener('click', cancelTask);
  applyButton.addEventListener('click', applyLayout);
  applyCodeButton.addEventListener('click', () => void applyCode());
  window.addEventListener('operit-ai-component', receiveComponentTask);

  return {updateContext};
}
