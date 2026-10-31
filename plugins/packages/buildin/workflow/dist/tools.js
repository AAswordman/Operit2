"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.list = list;
exports.import_workflow = import_workflow;
exports.run = run;
exports.save = save;
exports.cancel = cancel;
const validation_1 = require("./validation");
/* METADATA
{
  "name": "workflow",
  "display_name": {"zh":"工作流","en":"Workflow"},
  "description": {"zh":"管理和执行可视化工作流。工作流保存在当前插件中。","en":"Manage and execute plugin-owned visual workflows."},
  "tools": [
    {"name":"list","display":{"label": {"zh": "查看工作流", "en": "List workflows"}, "icon": "package"},"description":"列出工作流和最近执行记录","parameters":[]},
    {"name":"import_workflow","display":{"label": {"zh": "导入工作流", "en": "Import workflow"}, "icon": "package"},"description":"导入工作流 JSON；生成新 ID，初始停用，返回导入结果。","parameters":[{"name":"json","type":"string","required":true,"description":"Workflow JSON"}]},
    {"name":"run","display":{"label": {"zh": "运行工作流", "en": "Run workflow"}, "icon": "package", "targetParameter": "id"},"description":"运行已启用的工作流；工具调用使用宿主权限。","parameters":[{"name":"id","type":"string","required":true,"description":"工作流 ID"},{"name":"triggerId","type":"string","required":false,"description":"指定触发节点 ID"}]},
    {"name":"save","display":{"label": {"zh": "保存工作流", "en": "Save workflow"}, "icon": "edit"},"description":"更新工作流；必须保留 list 返回的 revision，用于防止覆盖其他编辑。","parameters":[{"name":"json","type":"string","required":true,"description":"包含 revision 的完整 Workflow JSON"}]},
    {"name":"cancel","display":{"label": {"zh": "取消工作流执行", "en": "Cancel workflow"}, "icon": "package", "targetParameter": "id"},"description":"请求在当前节点结束后取消后续执行","parameters":[{"name":"id","type":"string","required":true,"description":"工作流 ID"}]}
  ]
}
*/
const workflowServiceName = "workflow.service";
/** Calls the one main-runtime owner shared by UI, tools and schedules. */
function call(request) { return ToolPkg.ipc.call(workflowServiceName, request, { targetRuntime: "main" }); }
/** Lists workflows together with retained execution records. */
async function list() { return call({ action: "list" }); }
/** Imports a new disabled workflow after structural validation. */
async function import_workflow(params) { return call({ action: "import", json: params.json }); }
/** Executes all entry nodes or one explicitly selected trigger. */
async function run(params) {
    return call({ action: "run", id: params.id, triggerId: params.triggerId === undefined ? null : params.triggerId, extras: {} });
}
/** Updates an existing graph using the caller's exact saved revision. */
async function save(params) {
    const raw = (0, validation_1.object)(JSON.parse(params.json), "工作流");
    const workflow = (0, validation_1.parseWorkflow)(raw);
    if (typeof raw.revision !== "number" || !Number.isSafeInteger(raw.revision))
        throw new Error("revision 必须是整数");
    workflow.revision = raw.revision;
    return call({ action: "save", workflow });
}
/** Requests cooperative cancellation without claiming to interrupt host tools. */
async function cancel(params) { return call({ action: "cancel", id: params.id }); }
