/**
 * Plain object used as the initial shape and values for a plugin config file.
 *
 * The object keys become typed config properties on the value returned by
 * {@link PluginConfigApi.use}. Values must be JSON-serializable because the
 * runtime persists the config as JSON under the current plugin config dir.
 */
type PluginConfigDefaults = Record<string, any>;

/**
 * Top-level plugin configuration API.
 *
 * `PluginConfig` gives plugin scripts a memory-backed config object scoped to
 * the current plugin package. Call {@link PluginConfigApi.use} once with a
 * config name and defaults, then read or assign properties directly:
 *
 * ```ts
 * const settings = await PluginConfig.use("settings", {
 *   enabled: false,
 *   limit: 3,
 * });
 *
 * const enabled = settings.enabled;
 * settings.enabled = true;
 * settings.limit = 5;
 * ```
 *
 * Assigning a property updates the in-memory config and schedules persistence
 * to the plugin config directory. Plugin authors do not need to call a separate
 * save method or use `Tools.Files` for normal config reads and writes.
 */
interface PluginConfigApi {
    /**
     * Open the default `config.json` for the current plugin package.
     *
     * @param defaults Initial config shape and values used for missing keys and
     * the first in-memory value.
     * @returns A typed config object whose properties can be read and assigned
     * directly. Property assignments are persisted by the runtime.
     */
    use<TConfig extends PluginConfigDefaults>(defaults: TConfig): Promise<TConfig>;

    /**
     * Open a named config file for the current plugin package.
     *
     * The name is mapped to `<name>.json` under the current plugin config dir.
     * For example, `PluginConfig.use("settings", defaults)` uses
     * `settings.json`.
     *
     * @param name Config name without the `.json` suffix.
     * @param defaults Initial config shape and values used for missing keys and
     * the first in-memory value.
     * @returns A typed config object whose properties can be read and assigned
     * directly. Property assignments are persisted by the runtime.
     */
    use<TConfig extends PluginConfigDefaults>(name: string, defaults: TConfig): Promise<TConfig>;
}

/**
 * Memory-backed config API for the current plugin package.
 *
 * Use this instead of manually calling `getPluginConfigDir()` and `Tools.Files`
 * for routine plugin settings.
 */
declare const PluginConfig: PluginConfigApi;
