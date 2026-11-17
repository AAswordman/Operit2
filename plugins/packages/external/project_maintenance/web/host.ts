import type {Request, Response} from '../src/api';

declare global {
  interface Window {
    MaintenanceHost?: {request:(payload:Request)=>Promise<unknown>};
  }
}
/** Calls the interface registered by the Compose WebView controller on every platform. */
export async function request<R extends Request>(payload:R):Promise<Response<R>> {
  if(typeof window.MaintenanceHost?.request!=='function') {
    throw Error('维护接口尚未就绪，正在等待宿主连接，请稍后重试。');
  }
  const result = await window.MaintenanceHost.request(payload) as {ok: boolean; data?: Response<R>; error?: string};
  if (!result || typeof result.ok !== 'boolean') throw Error('维护接口返回了无效结果，请重新打开维护中控。');
  if (!result.ok) throw Error(result.error || '维护操作失败，请重试。');
  return result.data as Response<R>;
}
