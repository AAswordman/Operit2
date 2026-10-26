/* METADATA
{
  "name": "edge_pixel_pet",
  "display_name": "像素桌宠设备配置器",
  "description": "读取设备能力、上传资源并保存显示配置；显示和点击由设备本地处理。",
  "enabledByDefault": true,
  "env": [
    {
      "name": "EDGE_SCENE_NODE_ID",
      "description": "设备空间中的目标节点 ID",
      "required": false
    }
  ],
  "tools": [
    {
      "name": "inspect",
      "description": "读取设备协议、显示及资源能力",
      "parameters": [
        {
          "name": "node_id",
          "description": "目标设备 ID，缺省使用环境变量",
          "type": "string",
          "required": false
        }
      ]
    },
    {
      "name": "apply_configuration",
      "description": "上传桌宠资源并将显示配置保存到设备",
      "parameters": [
        {
          "name": "node_id",
          "description": "目标设备 ID，缺省使用环境变量",
          "type": "string",
          "required": false
        }
      ]
    },
    {
      "name": "clear_configuration",
      "description": "清除设备显示配置，恢复系统界面",
      "parameters": [
        {
          "name": "node_id",
          "description": "目标设备 ID，缺省使用环境变量",
          "type": "string",
          "required": false
        }
      ]
    }
  ]
}
*/// The package only exposes configuration commands; runtime state stays on Edge.
function invoke(operation: string, params: {
    node_id?: string;
} = {}): Promise<any> {
    return ToolPkg.ipc.call('edge.pet', { operation, params });
}
export function inspect(params: {
    node_id?: string;
} = {}): Promise<any> { return invoke('inspect', params); }
export function apply_configuration(params: {
    node_id?: string;
} = {}): Promise<any> { return invoke('apply_configuration', params); }
export function clear_configuration(params: {
    node_id?: string;
} = {}): Promise<any> { return invoke('clear_configuration', params); }
