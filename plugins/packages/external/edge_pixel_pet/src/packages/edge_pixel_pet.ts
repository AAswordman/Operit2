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
function invoke(operation: string, params: {
    node_id?: string;
} = {}): Promise<any> {
    return ToolPkg.ipc.call('edge.pet', { operation, params });
}
export function test(): Promise<any> { return invoke('test'); }
export function start_pet(params: {
    node_id?: string;
} = {}): Promise<any> { return invoke('start_pet', params); }
export function renew_pet(params: {
    node_id?: string;
} = {}): Promise<any> { return invoke('renew_pet', params); }
export function pet_status(params: {
    node_id?: string;
} = {}): Promise<any> { return invoke('pet_status', params); }
export function close_pet(params: {
    node_id?: string;
} = {}): Promise<any> { return invoke('close_pet', params); }
