"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.test = test;
exports.start_pet = start_pet;
exports.renew_pet = renew_pet;
exports.pet_status = pet_status;
exports.close_pet = close_pet;
/* METADATA
{
  "name": "edge_pixel_pet",
  "display_name": "像素桌宠 · Edge 场景示例",
  "description": "养成数据由 Core 保存，Edge 原生场景负责显示。需先启用并设置已授权的 Edge 节点 ID。",
  "enabledByDefault": true,
  "env": [{"name":"EDGE_SCENE_NODE_ID","description":"已授权设备空间中的 Edge 节点 ID","required":false}],
  "tools": [
    {"name":"test","description":"查询原生场景协议、屏幕及缓存能力，不申请显示区域","parameters":[]},
    {"name":"start_pet","description":"显示桌宠及道具，租约 60 秒；renew_pet 可单独续约。点击宠物或道具后由 Core 更新互动次数。","parameters":[{"name":"node_id","description":"明确的 Edge 节点 ID，缺省使用环境变量","type":"string","required":false}]},
    {"name":"renew_pet","description":"只续显示租约，不查询事件；已关闭时停止","parameters":[{"name":"node_id","description":"Edge 节点 ID","type":"string","required":false}]},
    {"name":"pet_status","description":"只读取 Core 已保存的互动次数和关闭状态，不访问 Edge","parameters":[{"name":"node_id","description":"Edge 节点 ID","type":"string","required":false}]},
    {"name":"close_pet","description":"释放桌宠区域，恢复系统界面，素材保留在有限缓存","parameters":[{"name":"node_id","description":"Edge 节点 ID","type":"string","required":false}]}
  ]
}
*/
// Existing same-package IPC keeps config + event cursor in one Core main actor.
function invoke(operation, params = {}) {
    return ToolPkg.ipc.call('edge.pet', { operation, params });
}
function test() { return invoke('test'); }
function start_pet(params = {}) { return invoke('start_pet', params); }
function renew_pet(params = {}) { return invoke('renew_pet', params); }
function pet_status(params = {}) { return invoke('pet_status', params); }
function close_pet(params = {}) { return invoke('close_pet', params); }
