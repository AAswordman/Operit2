"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.inspect = inspect;
exports.apply_configuration = apply_configuration;
exports.clear_configuration = clear_configuration;
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
*/ // The package only exposes configuration commands; runtime state stays on Edge.
function invoke(operation, params = {}) {
    return ToolPkg.ipc.call('edge.pet', { operation, params });
}
function inspect(params = {}) { return invoke('inspect', params); }
function apply_configuration(params = {}) { return invoke('apply_configuration', params); }
function clear_configuration(params = {}) { return invoke('clear_configuration', params); }
