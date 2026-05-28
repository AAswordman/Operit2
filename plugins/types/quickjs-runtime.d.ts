declare global {
    interface Console {
        log(...data: any[]): void;
        info(...data: any[]): void;
        warn(...data: any[]): void;
        error(...data: any[]): void;
        debug(...data: any[]): void;
    }

    var console: Console;

    type TimerHandler = ((...args: any[]) => any) | string;

    function setTimeout(handler: TimerHandler, timeout?: number, ...args: any[]): number;
    function clearTimeout(timerId?: number): void;
    function setInterval(handler: TimerHandler, timeout?: number, ...args: any[]): number;
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
