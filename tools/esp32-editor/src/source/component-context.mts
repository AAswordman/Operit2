import {findNode, nodePointer, projectRoutes} from '../layout/project-model.mts';
import type {LayoutDocument, LayoutNode} from '../layout/project-model.mts';
import {validate, applyOperations} from '../layout/layout-model.mts';
import {routes, eventBindings} from '../layout/routes.mts';
import type {Route, EventBinding} from '../layout/routes.mts';
import type {ComponentSource} from './component-source.mts';

export interface ComponentContext {
  kind: 'operit.hardware.component';
  version: 2;
  source: string;
  revision: string;
  dirty: boolean;
  componentId: string;
  component: LayoutNode;
  document: LayoutDocument;
  routes: Route[];
  eventBindings: EventBinding[];
  requirement: string;
  codeReference: {
    status: 'unsaved-component' | 'saved-component' | 'modified-component' | 'unresolved';
    location: ComponentSource['location'];
    draftPointer?: string | null;
    revision?: string;
    implementation?: ComponentSource['implementation'];
  };
  instructions: string;
}

export interface RouteReply {
  componentId: string;
  operations: unknown[];
}

/** Builds the structured chat payload for the currently selected hardware component. */
export function componentContext(
  document: LayoutDocument,
  componentId: string,
  revision: string,
  dirty: boolean,
  requirement = '',
  sourceReference: ComponentSource | null = null,
): ComponentContext {
  const component = findNode(document, componentId)?.node;
  if (!component) throw new Error('组件已删除，请重新选择');
  const codeReference = sourceReference
    ? {
        ...sourceReference,
        status: (!sourceReference.location
          ? 'unsaved-component'
          : JSON.stringify(sourceReference.location.component) === JSON.stringify(component)
            ? 'saved-component'
            : 'modified-component') as ComponentContext['codeReference']['status'],
        draftPointer: nodePointer(document, componentId),
      }
    : {status: 'unresolved' as const, location: null};
  return structuredClone({
    kind: 'operit.hardware.component',
    version: 2,
    source: 'apps/esp32/ui/layout.json',
    revision,
    dirty,
    componentId,
    component,
    document,
    routes: [...projectRoutes(document), ...routes],
    eventBindings,
    requirement,
    codeReference,
    instructions:
      '这是用户选中的硬件组件代码引用。根据需求直接定位并修改布局、事件绑定和功能路由实现。用 componentId 核对组件；文件行号仅对应给定 revision，修改前重新读取源码。既有路由可修改 action / longAction；新增跳转页面或设备功能须同时实现共用 C/Rust 代码、路由目录与必要校验，并验证预览和固件构建。无需仅返回固定格式 JSON。dirty=true 时保留草稿，先协调草稿再改写布局；新组件未保存时没有磁盘行号。不要编辑 generated/ 构建产物。',
  });
}

/** Chat replies configure only this component's bindings, never execute code. */
export function applyRouteReply(document: LayoutDocument, componentId: string, reply: RouteReply): LayoutDocument {
  if (!reply || reply.componentId !== componentId) throw new Error('回复组件与当前组件不一致');
  if (!Array.isArray(reply.operations) || !reply.operations.length || reply.operations.length > 2) {
    throw new Error('请提供 1–2 项路由 update 操作');
  }
  for (const op of reply.operations) {
    if (
      !op || typeof op !== 'object' || Array.isArray(op) ||
      (op as {op?: unknown}).op !== 'update' ||
      (op as {id?: unknown}).id !== componentId ||
      !(op as {changes?: unknown}).changes ||
      typeof (op as {changes?: unknown}).changes !== 'object' ||
      !Object.keys((op as {changes: object}).changes).length ||
      Object.keys((op as {changes: object}).changes).some((key) => !['action', 'longAction'].includes(key))
    ) {
      throw new Error('这里只接受当前组件的 action / longAction 修改');
    }
  }
  const next = applyOperations(document, reply.operations);
  const errors = validate(next);
  if (errors.length) throw new Error(errors.join('; '));
  return next;
}
