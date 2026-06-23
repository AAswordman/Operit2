/**
 * A function that can run inside another ToolPkg runtime context.
 * The function source is serialized and evaluated in the target context, so
 * values it uses should come from the `envs` object or from functions registered
 * with {@link RuntimeContextApi.register}.
 */
type RuntimeContextRunner<TResult extends object> = () => TResult | Promise<TResult>;

/**
 * Values passed into {@link withContext}. Each key becomes a local variable in
 * the target runner. Keys must be valid JavaScript identifiers.
 */
type RuntimeContextEnvs = Record<string, object | string | number | boolean | null | undefined>;

/**
 * Module exports whose functions can be used by a {@link RuntimeContextRunner}
 * after the module is registered in the current runtime context.
 */
type RuntimeContextModule = Record<string, unknown>;

/**
 * Options for selecting a concrete target context when a runtime kind can have
 * more than one active context, such as a specific UI route.
 */
interface RuntimeContextOptions {
    /**
     * Exact runtime context key to call, for example a `toolpkg_compose:...`
     * UI context key. Main runtime calls usually do not need this.
     */
    targetContextKey?: string;
}

/**
 * Utilities for moving small JSON-serializable tasks between ToolPkg runtime
 * contexts. Register module functions once, then call {@link withContext} with
 * a runtime kind, env values, and a runner.
 *
 * @example
 * RuntimeContext.register({ resolveName });
 * const result = await withContext("main", { packageName }, () => ({
 *   label: resolveName(packageName)
 * }));
 */
interface RuntimeContextApi {
    /**
     * Registers function exports that may be referenced by name inside runners
     * executed through {@link withContext}. Non-function exports are ignored.
     */
    register(moduleExports: RuntimeContextModule): void;

    /**
     * Runs a function in another ToolPkg runtime context and returns its result.
     * The result must be JSON-serializable.
     */
    withContext<TResult extends object>(
        kind: ToolPkg.RuntimeKind,
        envs: RuntimeContextEnvs,
        runner: RuntimeContextRunner<TResult>,
        options?: RuntimeContextOptions
    ): Promise<TResult>;
}

/** Runtime context helper API exposed by the Operit runtime. */
declare const RuntimeContext: RuntimeContextApi;

/**
 * Runs a JSON-serializable task in another ToolPkg runtime context.
 * Use `RuntimeContext.register()` for helper functions that the runner needs to
 * call after it moves to the target context.
 */
declare function withContext<TResult extends object>(
    kind: ToolPkg.RuntimeKind,
    envs: RuntimeContextEnvs,
    runner: RuntimeContextRunner<TResult>,
    options?: RuntimeContextOptions
): Promise<TResult>;
