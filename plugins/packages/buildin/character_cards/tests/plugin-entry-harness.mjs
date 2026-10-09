import { loadModule } from "./runtime.mjs";

/** Captures existing SDK registrations and loops only the package's real IPC callback; it supplies no business data. */
export function openPlugin(disk) {
  const commands = new Map(), apis = new Map(), channels = new Map(), routes = [], navigation = [], lifecycle = [], chatMessageHooks = [], hostEventHooks = [], chatLifecycleHooks = [], chatInputHooks = [], toolLifecycleHooks = [], toolPromptHooks = [];
  const registry = {
    ...disk.globals.ToolPkg,
    /** Records an actual command provider and rejects duplicate test registration. */
    registerCoreCommand(definition) {
      if (commands.has(definition.name)) throw new Error("Duplicate command registration: " + definition.name);
      commands.set(definition.name, definition);
    },
    /** Records the actual typed public provider without replacing its implementation. */
    registerApi(definition) {
      if (apis.has(definition.name)) throw new Error("Duplicate API registration: " + definition.name);
      apis.set(definition.name, definition);
    },
    /** Captures the actual UI route definition; no screen is launched by this registry. */
    registerUiRoute(definition) { routes.push(definition); },
    /** Captures the existing navigation contract without inventing a contribution framework. */
    registerNavigationEntry(definition) { navigation.push(definition); },
    /** Captures the lifecycle callback without invoking it during registration. */
    registerAppLifecycleHook(definition) { lifecycle.push(definition); },
    /** Records the actual chat-message callback definition without enqueuing messages or implementing a job service. */
    registerChatMessageHook(definition) { chatMessageHooks.push(definition); },
    /** Records the actual host-event trigger and callback without starting timers or invoking background business. */
    registerHostEventHook(definition) { hostEventHooks.push(definition); },
    /** Captures the genuine pre-create callback without initializing a chat or substituting a creation provider. */
    registerChatLifecycleHook(definition) { chatLifecycleHooks.push(definition); },
    /** Captures the genuine group input handler without invoking planning or accepting input during registration. */
    registerChatInputHook(definition) { chatInputHooks.push(definition); },
    /** Captures the real execution policy without running it or synthesizing its result. */
    registerToolLifecycleHook(definition) { toolLifecycleHooks.push(definition); },
    /** Captures the real prompt policy without manufacturing model-visible tool descriptors. */
    registerToolPromptComposeHook(definition) { toolPromptHooks.push(definition); },
    ipc: {
      /** Retains the real package request handler under its exact channel name. */
      on(name, handler) {
        if (channels.has(name)) throw new Error("Duplicate IPC channel: " + name);
        channels.set(name, handler);
      },
      /** Calls only an actually registered channel in the explicit main-runtime scope. */
      async call(name, payload, options) {
        if (options.targetRuntime !== "main") throw new Error("Entry harness requires the real main-runtime IPC target");
        const callback = channels.get(name);
        if (callback === undefined) throw new Error("Unregistered IPC channel: " + name);
        return callback(payload);
      },
    },
  };
  const main = loadModule("src/main.ts", { ...disk.globals, ToolPkg: registry });
  if (main.registerToolPkg() !== true) throw new Error("Actual plugin registration did not succeed");
  return {
    main, commands, apis, routes, navigation, lifecycle, chatMessageHooks, hostEventHooks, chatLifecycleHooks, chatInputHooks, toolLifecycleHooks, toolPromptHooks,
    /** Invokes the real registered public provider with its current SDK payload shape. */
    api(name, payload) {
      const definition = apis.get(name);
      if (definition === undefined) throw new Error("Unregistered public API: " + name);
      return definition.function({ callerPackage: "host", payload });
    },
    /** Invokes the real command provider using the actual CoreCommandHookEvent envelope. */
    command(name, args) {
      const definition = commands.get(name);
      if (definition === undefined) throw new Error("Unregistered command: " + name);
      return definition.function({ event: "core_command", eventName: "core_command", eventPayload: { commandId: name, commandName: name, args, json: true } });
    },
    /** Invokes the actual registered UI handler through the existing IPC loop, not a browser data fixture. */
    web(request) { return registry.ipc.call("character-memory.request", request, { targetRuntime: "main" }); },
    /** Calls the actual sidebar catalog handler with neutral host metadata. */
    sidebar(request) { return registry.ipc.call("character-sidebar.catalog", request, { targetRuntime: "main" }); },
    /** Calls only the genuine registered group-control IPC handler with its exact retained identity. */
    groupExecution(request) { return registry.ipc.call("character-memory.group-execution", request, { targetRuntime: "main" }); },
  };
}
