/* METADATA
{"name":"device_speech_processor","enabledByDefault":true,"display_name":"设备语音处理","description":"在当前宿主识别设备语音、合成语音，并通过通用双向通道返回结果。","tools":[{"name":"start","description":"为已配对设备提供语音识别和合成","parameters":[{"name":"node_id","type":"string","required":true,"description":"设备节点 ID"}]},{"name":"stop","description":"停止语音处理","parameters":[]},{"name":"status","description":"读取处理状态","parameters":[]}]}
*/
export function start(params: {node_id: string}): Promise<any> { return ToolPkg.ipc.call('device.speech.processor', {operation:'start', ...params}); }
export function stop(): Promise<any> { return ToolPkg.ipc.call('device.speech.processor', {operation:'stop'}); }
export function status(): Promise<any> { return ToolPkg.ipc.call('device.speech.processor', {operation:'status'}); }
