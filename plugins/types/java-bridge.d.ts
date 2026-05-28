/**
 * Java/Kotlin bridge type definitions.
 *
 * Rhino-like usage:
 *   const Cls = Java.type("java.lang.StringBuilder");
 *   const obj = Cls.newInstance();
 *   obj.append("hello");
 *   const out = obj.toString();
 */

/**
 * Opaque handle payload used when values cross the native bridge boundary.
 */
export interface JavaBridgeHandle {
    __javaHandle: string;
    __javaClass: string;
}

/**
 * Marker object produced by `Java.implement(...)`.
 * Pass this object to Java methods/constructors expecting interface arguments.
 * Runtime note: callbacks are marshalled back onto the QuickJS runtime thread.
 */
export interface JavaBridgeJsInterfaceMarker {
    __javaJsInterface: true;
    __javaJsObjectId: string;
    __javaInterfaces: string[];
}

/**
 * Primitive values that can cross the bridge directly.
 */
export type JavaBridgePrimitive =
    string |
    number |
    boolean |
    bigint |
    null |
    undefined;

/**
 * Structured object payload passed through bridge conversion.
 */
export interface JavaBridgeRecord {
    [key: string]: JavaBridgeValue;
}

/**
 * Dynamic value model for bridge inputs/outputs.
 * Keep this broad enough for runtime behavior while avoiding `any`.
 */
export type JavaBridgeValue =
    JavaBridgePrimitive |
    JavaBridgeRecord |
    JavaBridgeValue[] |
    JavaBridgeHandle |
    JavaBridgeJsInterfaceMarker |
    JavaBridgeInstance |
    JavaBridgeClass |
    JavaBridgePackage;

/**
 * Argument type accepted by Java bridge dynamic calls.
 */
export type JavaBridgeArg = JavaBridgeValue;

/**
 * JS implementation target for Java interface callbacks.
 * - function: single callable target (SAM-style usage)
 * - object: method-name based implementation
 */
export type JavaBridgeCallbackResult = JavaBridgeValue | void;

export type JavaBridgeJsMethod = (...args: JavaBridgeArg[]) => JavaBridgeCallbackResult;

export type JavaBridgeJsInterfaceImpl =
    JavaBridgeJsMethod |
    Record<string, JavaBridgeJsMethod | JavaBridgeValue>;

/**
 * Interface reference accepted by `Java.implement(...)` / `Java.proxy(...)`.
 * Supports both string class names and Java class proxies (e.g. `Java.java.lang.Runnable`).
 */
export type JavaBridgeInterfaceRef = string | JavaBridgeClass;

/**
 * Dynamic callable member returned from proxy fallbacks.
 */
export type JavaBridgeDynamicCallable = (...args: JavaBridgeArg[]) => JavaBridgeValue;

/**
 * Options accepted by `Java.loadDex(...)` / `Java.loadJar(...)`.
 */
export interface JavaBridgeExternalCodeLoadOptions {
    nativeLibraryDir?: string;
    /**
     * Package/class-name prefixes that should prefer classes from the loaded dex/jar
     * before delegating to the parent class loader.
     */
    childFirstPrefixes?: string[];
}

/**
 * One external dex/jar artifact registered into the Java bridge class loader chain.
 */
export interface JavaBridgeLoadedCodePath {
    index: number;
    type: 'dex' | 'jar';
    path: string;
    nativeLibraryDir: string | null;
    childFirstPrefixes: string[];
    alreadyLoaded: boolean;
}

/**
 * Dynamic proxy object for a Java/Kotlin instance.
 * - Unknown property reads prefer instance method callable, then fallback to instance field/property get.
 * - Unknown property writes are treated as instance field/property set.
 * - Prefer direct syntax sugar like `obj.methodName()`. Use `.call('methodName', ...)` only for rare ambiguity/debug cases.
 */
export interface JavaBridgeInstance extends JavaBridgeHandle {
    readonly className: string;
    readonly handle: string;

    call<T extends JavaBridgeValue = JavaBridgeValue>(methodName: string, ...args: JavaBridgeArg[]): T;
    callSuspend(methodName: string, ...args: JavaBridgeArg[]): Promise<JavaBridgeValue>;
    get<T extends JavaBridgeValue = JavaBridgeValue>(): T;
    get<T extends JavaBridgeValue = JavaBridgeValue>(fieldName: string): T;
    set<T extends JavaBridgeValue = JavaBridgeValue>(value: JavaBridgeArg): T;
    set<T extends JavaBridgeValue = JavaBridgeValue>(fieldName: string, value: JavaBridgeArg): T;

    toJSON(): JavaBridgeHandle;
    toString(): string;

    [member: string]: any;
}

/**
 * Dynamic proxy object for a Java/Kotlin class.
 * - Unknown property reads first try static field/property get, then fallback to static method callable.
 * - Unknown property writes are treated as static field/property set.
 */
export interface JavaBridgeClass {
    (...args: JavaBridgeArg[]): JavaBridgeInstance;
    new (...args: JavaBridgeArg[]): JavaBridgeInstance;

    readonly className: string;

    exists(): boolean;
    newInstance<T extends JavaBridgeInstance = JavaBridgeInstance>(...args: JavaBridgeArg[]): T;
    callStatic<T extends JavaBridgeValue = JavaBridgeValue>(methodName: string, ...args: JavaBridgeArg[]): T;
    callSuspend(methodName: string, ...args: JavaBridgeArg[]): Promise<JavaBridgeValue>;
    getStatic<T extends JavaBridgeValue = JavaBridgeValue>(fieldName: string): T;
    setStatic<T extends JavaBridgeValue = JavaBridgeValue>(fieldName: string, value: JavaBridgeArg): T;
    toString(): string;

    [member: string]: any;
}

/**
 * Dynamic package namespace proxy:
 * - e.g. `Java.java.lang.System`
 */
export interface JavaBridgePackage {
    (...args: JavaBridgeArg[]): JavaBridgeInstance;
    new (...args: JavaBridgeArg[]): JavaBridgeInstance;

    readonly path: string;
    toString(): string;

    [member: string]: JavaBridgeClass | JavaBridgePackage;
}

/**
 * Top-level Java/Kotlin bridge API injected by runtime.
 */
export interface JavaBridgeApi {
    type(className: string): JavaBridgeClass;
    use(className: string): JavaBridgeClass;
    importClass(className: string): JavaBridgeClass;
    package(packageName: string): JavaBridgePackage;
    implement(interfaceName: JavaBridgeInterfaceRef, impl: JavaBridgeJsInterfaceImpl): JavaBridgeJsInterfaceMarker;
    implement(interfaceNames: JavaBridgeInterfaceRef[], impl: JavaBridgeJsInterfaceImpl): JavaBridgeJsInterfaceMarker;
    implement(impl: JavaBridgeJsInterfaceImpl): JavaBridgeJsInterfaceMarker;
    proxy(interfaceName: JavaBridgeInterfaceRef, impl: JavaBridgeJsInterfaceImpl): JavaBridgeJsInterfaceMarker;
    proxy(interfaceNames: JavaBridgeInterfaceRef[], impl: JavaBridgeJsInterfaceImpl): JavaBridgeJsInterfaceMarker;
    proxy(impl: JavaBridgeJsInterfaceImpl): JavaBridgeJsInterfaceMarker;
    classExists(className: string): boolean;
    loadDex(path: string, options?: string | JavaBridgeExternalCodeLoadOptions): JavaBridgeLoadedCodePath;
    loadJar(path: string, options?: string | JavaBridgeExternalCodeLoadOptions): JavaBridgeLoadedCodePath;
    listLoadedCodePaths(): JavaBridgeLoadedCodePath[];
    callStatic<T extends JavaBridgeValue = JavaBridgeValue>(className: string, methodName: string, ...args: JavaBridgeArg[]): T;
    callSuspend(className: string, methodName: string, ...args: JavaBridgeArg[]): Promise<JavaBridgeValue>;
    newInstance<T extends JavaBridgeInstance = JavaBridgeInstance>(className: string, ...args: JavaBridgeArg[]): T;
    getApplicationContext<T extends JavaBridgeInstance = JavaBridgeInstance>(): T;
    getContext<T extends JavaBridgeInstance = JavaBridgeInstance>(): T;
    getCurrentActivity<T extends JavaBridgeInstance = JavaBridgeInstance>(): T;
    getActivity<T extends JavaBridgeInstance = JavaBridgeInstance>(): T;

    [member: string]: JavaBridgeClass | JavaBridgePackage;
}
