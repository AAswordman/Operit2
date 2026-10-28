import {componentContext, applyRouteReply} from '../src/source/component-context.mjs';
import {findNode, projectRoutes} from '../src/layout/project-model.mjs';
import {routes} from '../src/layout/routes.mjs';
import {errorMessage, query} from './types.js';
import type {ComponentSource, ComponentSetupOptions, LayoutDocument, LayoutNode} from './types.js';
import type {RouteReply} from '../src/source/component-context.mts';

/** Installs component source inspection, route editing and AI handoff actions. */
export function setupInteractions({snapshot, commit, notify}: ComponentSetupOptions): {open: (id: string) => void} {
  const dialog = query<HTMLDialogElement>('#component-dialog');
  const title = query<HTMLElement>('#component-title');
  const description = query<HTMLElement>('#component-description');
  const sourceOutput = query<HTMLElement>('#component-source-ref');
  const requestInput = query<HTMLTextAreaElement>('#component-request');
  const replyInput = query<HTMLTextAreaElement>('#component-reply');
  const contextOutput = query<HTMLTextAreaElement>('#component-context');
  const feedbackOutput = query<HTMLElement>('#component-feedback');
  const clickRouteSelect = query<HTMLSelectElement>('#component-click-route');
  const holdRouteSelect = query<HTMLSelectElement>('#component-hold-route');
  const sendButton = query<HTMLButtonElement>('#component-send');
  const contextDetails = query<HTMLDetailsElement>('#component-context-details');
  let componentId: string | null = null;
  let baseDocument: LayoutDocument | null = null;
  let replyBase: string | null = null;
  let sourceReference: ComponentSource | null = null;
  let session = 0;

  /** Displays a component inspector status message. */
  function feedback(message: string): void {
    feedbackOutput.textContent = message;
  }

  /** Returns the selected component from the inspector state. */
  function selectedComponent(document: LayoutDocument, id: string): LayoutNode {
    const node = findNode(document, id)?.node;
    if (!node) throw new Error('组件已删除，请重新选择');
    return node;
  }

  /** Builds the source-aware component context shown to the user and AI. */
  function buildContext(): ReturnType<typeof componentContext> {
    if (!componentId || !baseDocument) throw new Error('尚未选择组件');
    const state = snapshot();
    return componentContext(
      baseDocument,
      componentId,
      state.revision,
      state.dirty,
      requestInput.value.trim(),
      sourceReference,
    );
  }

  /** Refreshes the read-only JSON context after inspector input changes. */
  function refreshContext(): void {
    contextOutput.value = JSON.stringify(buildContext(), null, 2);
  }

  /** Resolves source line references for the selected component revision. */
  async function resolveSource(expectedSession = session): Promise<boolean> {
    if (!componentId) throw new Error('尚未选择组件');
    const state = snapshot();
    const result = await fetchComponentSource(componentId, state.revision);
    if (expectedSession !== session || !dialog.open) return false;
    sourceReference = result;
    refreshContext();
    const context = buildContext();
    const location = context.codeReference.location;
    sourceOutput.textContent = location
      ? `${location.path}:${location.startLine}–${location.endLine} · ${location.jsonPointer}` +
        (context.codeReference.status === 'modified-component'
          ? ' · 当前组件有未保存修改，行号指向磁盘版本'
          : '')
      : `新组件尚未保存 · ${context.codeReference.draftPointer ?? ''} · 暂无磁盘行号`;
    return true;
  }

  /** Fetches the component source reference from the editor server. */
  async function fetchComponentSource(id: string, revision: string): Promise<ComponentSource> {
    const result = await import('./transport.js').then(({request}) =>
      request<ComponentSource>(
        '/api/component-source?id=' + encodeURIComponent(id) + '&revision=' + encodeURIComponent(revision),
      ),
    );
    return result;
  }

  /** Ensures the inspector still describes the same document revision. */
  function ensureUnchanged(): void {
    if (!baseDocument) throw new Error('尚未选择组件');
    const current = snapshot();
    if (JSON.stringify(current.document) !== JSON.stringify(baseDocument)) {
      throw new Error('布局在对话期间已变化，请关闭后重新选择组件');
    }
  }

  /** Opens the inspector for a component in the current editor page. */
  function open(id: string): void {
    const state = snapshot();
    const node = findNode(state.document, id)?.node;
    if (!node) return;
    session += 1;
    componentId = id;
    baseDocument = state.document;
    sourceReference = null;
    replyBase = null;
    const routeOptions = [...projectRoutes(state.document), ...routes];
    clickRouteSelect.replaceChildren(...routeOptions.map((route) => new Option(route.label, route.id)));
    holdRouteSelect.replaceChildren(...routeOptions.map((route) => new Option(route.label, route.id)));
    title.textContent = '引用组件 · ' + id;
    description.textContent = `${node.type} · ${node.w} × ${node.h} · ${node.text || '无文本'}`;
    clickRouteSelect.value = node.action;
    holdRouteSelect.value = node.longAction ?? '';
    requestInput.value = '';
    replyInput.value = '';
    sendButton.disabled = false;
    feedback('填写需求，连同源码位置一起发送到独立 AI 代理，让它定位并修改功能实现。');
    sourceOutput.textContent = '正在定位布局源码…';
    refreshContext();
    if (!dialog.open) dialog.showModal();
    const openingSession = session;
    void resolveSource(openingSession).catch((error: unknown) => {
      if (openingSession !== session) return;
      sourceOutput.textContent = '源码定位失败';
      feedback(errorMessage(error));
    });
  }

  /** Sends the selected component task to the main AI development dialog. */
  function sendComponentTask(): void {
    try {
      ensureUnchanged();
      dialog.close();
      window.dispatchEvent(
        new CustomEvent<{requirement: string}>('operit-ai-component', {
          detail: {requirement: requestInput.value},
        }),
      );
    } catch (error) {
      feedback(errorMessage(error));
    }
  }

  /** Copies the current source-aware component context to the clipboard. */
  async function copyContext(): Promise<void> {
    try {
      ensureUnchanged();
      if (!(await resolveSource())) return;
      ensureUnchanged();
      await navigator.clipboard.writeText(JSON.stringify(buildContext(), null, 2));
      replyBase = JSON.stringify(baseDocument);
      feedback('已复制组件、源码行号和需求，粘贴到 AI 对话后可直接定位实现。');
    } catch (error) {
      contextDetails.open = true;
      contextOutput.select();
      feedback('无法自动复制，请从展开的上下文中手动复制。' + errorMessage(error));
    }
  }

  /** Applies an accepted route change to the editor draft. */
  function apply(next: LayoutDocument): void {
    if (!componentId) throw new Error('尚未选择组件');
    if (commit(next, componentId)) {
      baseDocument = structuredClone(next);
      replyInput.value = '';
      refreshContext();
      feedback('路由已应用到草稿。关闭后切换运行模式，触摸组件验证；保存后同步固件。');
      notify('组件路由已更新 · 切换运行模式测试，保存后同步固件');
      return;
    }
    feedback('路由未变化');
  }

  /** Applies the manually selected click and long-press routes. */
  function applySelectedRoutes(): void {
    try {
      ensureUnchanged();
      if (!componentId || !baseDocument) throw new Error('尚未选择组件');
      apply({
        ...applyRouteReply(baseDocument, componentId, {
          componentId,
          operations: [
            {
              op: 'update',
              id: componentId,
              changes: {action: clickRouteSelect.value, longAction: holdRouteSelect.value},
            },
          ],
        }),
      });
    } catch (error) {
      feedback(errorMessage(error));
    }
  }

  /** Applies a pasted structured route proposal for the selected component. */
  function applyReply(): void {
    try {
      ensureUnchanged();
      if (!componentId || !baseDocument) throw new Error('尚未选择组件');
      if (replyBase && replyBase !== JSON.stringify(baseDocument)) {
        throw new Error('回复基于旧草稿，请重新发送上下文');
      }
      if (replyInput.value.length > 65536) throw new Error('回复超过 64 KiB');
      const reply = JSON.parse(replyInput.value) as unknown as RouteReply;
      const next = applyRouteReply(baseDocument, componentId, reply);
      apply(next);
      const node = selectedComponent(next, componentId);
      clickRouteSelect.value = node.action;
      holdRouteSelect.value = node.longAction ?? '';
    } catch (error) {
      feedback('未应用：' + errorMessage(error));
    }
  }

  requestInput.addEventListener('input', refreshContext);
  query<HTMLButtonElement>('#component-close').addEventListener('click', () => dialog.close());
  sendButton.addEventListener('click', sendComponentTask);
  query<HTMLButtonElement>('#component-copy').addEventListener('click', () => void copyContext());
  query<HTMLButtonElement>('#component-apply-routes').addEventListener('click', applySelectedRoutes);
  query<HTMLButtonElement>('#component-apply-reply').addEventListener('click', applyReply);

  return {open};
}
