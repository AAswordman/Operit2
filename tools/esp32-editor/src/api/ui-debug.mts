/** Requests are executed by the browser's actual self-drawn UI instance. The Rust
 * simulator owns transport state only; it must never invent a second UI tree. */
let sequence = 0;
const queue: {id: number; command: string; input: unknown}[] = [];
const pending = new Map<number, {resolve: (value: unknown) => void; reject: (error: Error) => void}>();

export function requestUi(command: string, input: unknown = {}): Promise<unknown> {
  const id = ++sequence;
  return new Promise((resolve, reject) => {
    const timeout = setTimeout(() => {
      pending.delete(id);
      const index = queue.findIndex(item => item.id === id);
      if (index >= 0) queue.splice(index, 1);
      reject(new Error('请打开浏览器中的 ESP32 运行界面后重试'));
    }, 5000);
    pending.set(id, {
      resolve: value => { clearTimeout(timeout); resolve(value); },
      reject: error => { clearTimeout(timeout); reject(error); },
    });
    queue.push({id, command, input});
  });
}
export function takeUiCommands(): unknown[] { return queue.splice(0); }
export function completeUiCommand(id: number, value: unknown, error?: string): void {
  const request = pending.get(id); pending.delete(id);
  if (error) request?.reject(new Error(error)); else request?.resolve(value);
}
