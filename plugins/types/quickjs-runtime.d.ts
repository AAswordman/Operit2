declare global {
    /** Alias of the standard global object, shared by all plugin API versions. */
    var global: typeof globalThis;

    /** Diagnostic depth shared with Rust AppLogger and Flutter ClientLogger. */
    type VerboseLogLevel = 1 | 2 | 3 | 4 | 5 | 6;

    interface Console {
        /** Writes lifecycle (V1), operation (V2), call (V3), resource (V4), internal (V5), or per-item (V6) diagnostics. */
        verbose(level: VerboseLogLevel, ...data: any[]): void;
        log(...data: any[]): void;
        info(...data: any[]): void;
        warn(...data: any[]): void;
        error(...data: any[]): void;
        debug(...data: any[]): void;
    }

    var console: Console;

    type QuickJsTimerHandler = ((...args: any[]) => any) | string;

    function setTimeout(handler: QuickJsTimerHandler, timeout?: number, ...args: any[]): number;
    function clearTimeout(timerId?: number): void;
    function setInterval(handler: QuickJsTimerHandler, timeout?: number, ...args: any[]): number;
    function clearInterval(timerId?: number): void;
    function queueMicrotask(callback: () => void): void;

    interface Storage {
        readonly length: number;
        clear(): void;
        getItem(key: string): string | null;
        key(index: number): string | null;
        removeItem(key: string): void;
        setItem(key: string, value: string): void;
    }

    interface Performance {
        now(): number;
    }

    var localStorage: Storage;
    var sessionStorage: Storage;
    var performance: Performance;
}

export {};
