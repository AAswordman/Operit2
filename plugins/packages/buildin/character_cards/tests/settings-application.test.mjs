import assert from "node:assert/strict";
import test from "node:test";
import path from "node:path";
import { readFile } from "node:fs/promises";
import { createDiskHarness } from "./disk-files.mjs";
import { selectorEnvironment } from "./selector-environment.mjs";
import { keyedNode, mountRegisteredSelector } from "./selector-render.mjs";
import { createSelectionSettingsFixture } from "./selection-settings-fixture.mjs";
import { importedCallCount, loadModule, plain } from "./runtime.mjs";

const { createChatInput } = loadModule("src/chat-lifecycle.ts");
const themeId = "independent-theme-two", speechId = "tts-two";
const cardCalls = ["listThemeConfigs", "listTtsConfigs", "applyThemeConfig", "setCurrentTtsConfigId", "getCurrentTtsConfigId"];

/** Audits genuine host writes without supplying any role, selection or persistence implementation. */
function writes(runtime) {
  return runtime.calls.filter(
    /** Counts only the actual generic native namespace write. */
    call => call.method === "writeExtension",
  );
}

/** Reads the actual published plugin file rather than a test-provided active selection. */
function stateBytes(runtime) { return runtime.disk.stateBytes(); }

/** Builds an explicit selector presentation with a real global-null or native-chat target. */
function presentation(requestId, chatId, kind = "card") { return { requestId, input: { mode: "select", chatId, kind, selected: null } }; }

/** Loads the production registered route using the real Compose runtime, callbacks and main IPC service. */
async function ready(runtime, value) { const ui = mountRegisteredSelector(runtime, value); await ui.load(); return ui; }

/** Opens actual plugin registrations and creates genuine file-owned records, mocking only independent SDK configuration owners. */
async function environment(t) {
  const settings = createSelectionSettingsFixture(), runtime = await selectorEnvironment(t, settings.settings);
  const card = await runtime.api("character.create", { values: { name: "Registered Settings role", themeConfigId: themeId, ttsConfigId: speechId } });
  const group = await runtime.api("group.create", { values: { name: "Registered Settings group", themeConfigId: "independent-theme-one", members: [{ characterCardId: card.id, orderIndex: 0 }] } });
  settings.clearCalls(); runtime.calls.length = 0; runtime.disk.clearCalls();
  return { runtime, settings, card, group };
}

/** Asserts every directory validation observes the exact pre-selection file and namespace before any real mutation. */
async function auditPrevalidation(runtime) {
  const before = await stateBytes(runtime), extension = plain(runtime.records.get("selector-chat").extension);
  for (const method of ["listThemeConfigs", "listTtsConfigs"]) {
    const original = runtime.tools.SoftwareSettings[method];
    /** Inspects actual native and file state at the SDK read boundary, not a plugin-provided success flag. */
    runtime.tools.SoftwareSettings[method] = async (...args) => {
      assert.equal(Buffer.compare(await stateBytes(runtime), before), 0, method + " must precede file publication");
      assert.deepEqual(runtime.records.get("selector-chat").extension, extension, method + " must precede native selection writes");
      return original(...args);
    };
  }
}

/** Requires the exact formal SDK call sequence and independently confirmed requested IDs. */
function assertCardApplication(settings) {
  assert.deepEqual(settings.calls.map(
    /** Preserves each actual SDK method name for strict ordering assertions. */
    call => call.method,
  ), cardCalls);
  assert.deepEqual(settings.calls.map(
    /** Preserves complete actual argument lists, including parameterless reads. */
    call => call.args,
  ), [[], [], [themeId], [speechId], []]);
  assert.deepEqual(settings.current, { theme: themeId, tts: speechId });
}

/** Proves the actual bootstrap installs the lazy Settings adapter once and never reads directories, files or prefs during registration. */
test("main registers one lazy real Settings access and main IPC reads actual named Theme configs", async t => {
  const settings = createSelectionSettingsFixture(), runtime = await selectorEnvironment(t, settings.settings);
  assert.equal(importedCallCount("../src/main.ts", "./selection-settings", "registerSelectionSettingsAccess"), 1);
  assert.deepEqual(settings.calls, []); assert.deepEqual(runtime.disk.calls, []); assert.deepEqual(runtime.calls, []);
  const choices = await runtime.web({ action: "listThemeChoices" });
  assert.deepEqual(plain(choices), [
    { id: "independent-theme-one", label: "Controlled prior independent theme" },
    { id: themeId, label: "Controlled independent theme" },
  ]);
  assert.deepEqual(settings.calls, [{ method: "listThemeConfigs", args: [] }]);
  assert.deepEqual(settings.current, { theme: "independent-theme-one", tts: "tts-one" });
});

/** Exercises every real public card-selection alias through registration, exact prevalidation and awaited independent owner acknowledgements. */
test("registered card selection APIs share the main Settings adapter and await exact Theme and TTS receipts", async t => {
  for (const operation of ["character.setActive", "activePrompt.setCard", "activePrompt.activateForChat", "chat.configuration.binding.write"]) {
    /** Tests a distinct actual registered provider with fresh native namespaces and real files. */
    await t.test(operation, async s => {
      const { runtime, settings, card } = await environment(s); await auditPrevalidation(runtime);
      const payload = operation === "activePrompt.activateForChat" ? { characterCardName: card.name, characterGroupId: null }
        : operation === "chat.configuration.binding.write" ? { chatId: "selector-chat", selection: "card:" + card.id } : { id: card.id };
      await runtime.api(operation, payload); assertCardApplication(settings);
      const actual = JSON.parse((await stateBytes(runtime)).toString("utf8"));
      if (operation === "chat.configuration.binding.write") {
        assert.deepEqual(actual.active, { CharacterCard: { id: "default" } });
        assert.deepEqual(runtime.records.get("selector-chat").extension, { version: 1, selection: "card:" + card.id });
        assert.equal(writes(runtime).length, 1);
      } else {
        assert.deepEqual(actual.active, { CharacterCard: { id: card.id } }); assert.equal(writes(runtime).length, 0);
      }
    });
  }
});

/** Covers group-own references at every real registered public entrypoint without borrowing a member's TTS or Theme. */
test("registered group selection aliases use only the group own Theme and never apply first-member TTS", async t => {
  for (const operation of ["group.setActive", "activePrompt.setGroup", "activePrompt.activateForChat", "chat.configuration.binding.write"]) {
    /** Runs the real group provider with a participant that deliberately has explicit independent speech. */
    await t.test(operation, async s => {
      const { runtime, settings, group } = await environment(s); await auditPrevalidation(runtime);
      const payload = operation === "activePrompt.activateForChat" ? { characterCardName: null, characterGroupId: group.id }
        : operation === "chat.configuration.binding.write" ? { chatId: "selector-chat", selection: "group:" + group.id } : { id: group.id };
      await runtime.api(operation, payload);
      assert.notEqual(group.themeConfigId, settings.themes.get(themeId).id);
      assert.deepEqual(settings.calls, [{ method: "listThemeConfigs", args: [] }, { method: "applyThemeConfig", args: [group.themeConfigId] }]);
      assert.deepEqual(settings.current, { theme: group.themeConfigId, tts: "tts-one" });
      if (operation === "chat.configuration.binding.write") assert.equal(runtime.records.get("selector-chat").extension.selection, "group:" + group.id);
      else assert.deepEqual(JSON.parse((await stateBytes(runtime)).toString("utf8")).active, { CharacterGroup: { id: group.id } });
    });
  }
});

/** Preserves the Web management and sidebar IPC entrypoints on the same real adapter, not a second config-apply path. */
test("real Web activate and writeChatBinding IPC requests await the same central Settings application", async t => {
  for (const action of ["activate", "writeChatBinding"]) {
    /** Calls the existing main-runtime request channel rather than a browser presentation fixture. */
    await t.test(action, async s => {
      const { runtime, settings, card } = await environment(s);
      const request = action === "activate" ? { action, type: "card", id: card.id } : { action, chatId: "selector-chat", selection: "card:" + card.id };
      await runtime.web(request);
      const expected = action === "activate" ? ["listThemeConfigs", "listTtsConfigs", "listTtsConfigs", "applyThemeConfig", "setCurrentTtsConfigId", "getCurrentTtsConfigId"] : cardCalls;
      assert.deepEqual(settings.calls.map(
        /** Includes the management snapshot's genuine TTS directory read instead of hiding it from the audit. */
        call => call.method,
      ), expected);
      assert.deepEqual(settings.current, { theme: themeId, tts: speechId });
      if (action === "activate") assert.deepEqual(JSON.parse((await stateBytes(runtime)).toString("utf8")).active, { CharacterCard: { id: card.id } });
      else { assert.equal(writes(runtime).length, 1); assert.equal(runtime.records.get("selector-chat").extension.selection, "card:" + card.id); }
    });
  }
});

/** Null means no configuration-change request, including groups whose participants have explicit speech or Theme references. */
test("registered null-reference selections do not read or mutate independent Settings", async t => {
  const { runtime, settings, card } = await environment(t);
  const group = await runtime.api("group.create", { values: { name: "Explicit null group Theme", themeConfigId: null, members: [{ characterCardId: card.id, orderIndex: 0 }] } });
  await runtime.api("activePrompt.setCard", { id: "default" });
  await runtime.api("chat.configuration.binding.write", { chatId: "selector-chat", selection: "group:" + group.id });
  assert.deepEqual(settings.calls, []); assert.deepEqual(settings.current, { theme: "independent-theme-one", tts: "tts-one" });
  assert.equal(runtime.records.get("selector-chat").extension.selection, "group:" + group.id);
});

/** Deleted references reject before any published selection, native namespace write or preference application. */
test("deleted Theme or TTS IDs reject registered active, binding and native selector operations with zero selection writes", async t => {
  for (const missing of ["theme", "tts"]) {
    for (const entry of ["active", "binding", "selector"]) {
      /** Audits exact native and disk state for a real registered user operation with an invalid independent reference. */
      await t.test(missing + ":" + entry, async s => {
        const { runtime, settings, card } = await environment(s);
        const ui = entry === "selector" ? await ready(runtime, presentation("deleted-" + missing, "selector-chat")) : null;
        if (missing === "theme") settings.themes.delete(themeId); else settings.speech.delete(speechId);
        settings.clearCalls(); runtime.disk.clearCalls(); const before = await stateBytes(runtime);
        const operation = entry === "active" ? runtime.api("activePrompt.setCard", { id: card.id })
          : entry === "binding" ? runtime.api("chat.configuration.binding.write", { chatId: "selector-chat", selection: "card:" + card.id })
          : ui.dispatch(keyedNode(ui.render().tree, "card:" + card.id).props.onClick);
        await assert.rejects(operation, missing === "theme" ? /主题配置不存在/ : /TTS 配置不存在/);
        assert.equal(Buffer.compare(await stateBytes(runtime), before), 0); assert.equal(writes(runtime).length, 0);
        assert.equal(runtime.records.get("selector-chat").extension, null);
        assert.deepEqual(settings.calls.map(
          /** Requires only the actual preceding validation reads, never an apply or setter invocation. */
          call => call.method,
        ), missing === "theme" ? ["listThemeConfigs"] : ["listThemeConfigs", "listTtsConfigs"]);
        assert.equal(runtime.disk.calls.filter(
          /** Requires zero real file writes, not merely unchanged displayed state. */
          call => call.method === "write" || call.method === "move",
        ).length, 0);
        if (ui !== null) {
          const tree = ui.render().tree; assert.equal(tree.type, "Dialog"); assert.equal(keyedNode(tree, "selector-close").props.enabled, true);
          assert.match(keyedNode(tree, "selector-commit-error").props.text, /配置不存在/);
          const cancelled = await ui.dispatch(keyedNode(tree, "selector-close").props.onClick);
          assert.equal(cancelled.actionResult.type, "toolpkg.presentation.cancel");
        }
      });
    }
  }
});

/** A genuine owner-directory failure is propagated unchanged instead of substituting current config IDs or treating validation as successful. */
test("registered Settings directory rejection preserves its original error and performs no selection commit", async t => {
  const { runtime, settings, card } = await environment(t), original = new Error("Controlled original Theme directory rejection"), before = await stateBytes(runtime);
  /** Rejects the formally declared directory read at the controlled host boundary. */
  runtime.tools.SoftwareSettings.listThemeConfigs = async () => { throw original; };
  await assert.rejects(runtime.api("chat.configuration.binding.write", { chatId: "selector-chat", selection: "card:" + card.id }),
    /** Requires the original boundary rejection identity, not a transformed success or retry response. */
    failure => failure === original,
  );
  assert.equal(Buffer.compare(await stateBytes(runtime), before), 0); assert.equal(writes(runtime).length, 0); assert.deepEqual(settings.calls, []);
});

/** Verifies UI completion remains pending and every interactive control is disabled until the actual independent apply acknowledgement arrives. */
test("registered selector awaits Theme and TTS before V1 completion for both chat and explicit global targets", async t => {
  for (const chatId of ["selector-chat", null]) {
    for (const method of ["applyThemeConfig", "setCurrentTtsConfigId", "getCurrentTtsConfigId"]) {
    /** Uses the actual registered native modal and callback dispatcher for each real acknowledgement and target mode. */
    await t.test((chatId === null ? "global:" : "chat:") + method, async s => {
      const { runtime, settings, card } = await environment(s), requestId = chatId === null ? "settings-global" : "settings-chat";
      const ui = await ready(runtime, presentation(requestId, chatId)), tree = ui.render().tree;
      const row = keyedNode(tree, "card:" + card.id), close = keyedNode(tree, "selector-close"), gate = settings.hold(method);
      let resolved = false;
      const pending = ui.dispatch(row.props.onClick).then(
        /** Records real SDK completion only after the returned action-result promise resolves. */
        result => { resolved = true; return result; },
      );
      try {
        await gate.reached; assert.equal(resolved, false);
        if (chatId === null) assert.deepEqual(JSON.parse((await stateBytes(runtime)).toString("utf8")).active, { CharacterCard: { id: card.id } });
        else assert.equal(runtime.records.get(chatId).extension.selection, "card:" + card.id);
        const busy = ui.render().tree;
        assert.equal(keyedNode(busy, "selector-close").props.enabled, false); assert.equal(keyedNode(busy, "card:" + card.id).props.onClick, undefined);
        await assert.rejects(ui.dispatch(row.props.onClick), /already running/);
        assert.throws(
          /** Requires a retained busy cancellation action to reject synchronously without completing or cancelling. */
          () => ui.dispatch(close.props.onClick), /already running/,
        );
        assert.deepEqual(settings.calls.map(
          /** Confirms no subsequent step or V1 completion can precede this exact outstanding acknowledgement. */
          call => call.method,
        ), cardCalls.slice(0, method === "applyThemeConfig" ? 3 : method === "setCurrentTtsConfigId" ? 4 : 5));
      } finally { gate.release(); }
      const result = await pending; assertCardApplication(settings);
      assert.deepEqual(plain(result.actionResult), { type: "toolpkg.presentation.complete", requestId, value: { selection: "card:" + card.id, contextKey: "card:" + card.id } });
      assert.equal(writes(runtime).length, chatId === null ? 0 : 1);
    });
    }
  }
});

/** Application errors expose true acknowledged partial progress, preserve the original error and leave the same modal open without retry. */
test("registered selector apply failures retain partial selection state and do not emit successful completion", async t => {
  for (const method of ["applyThemeConfig", "setCurrentTtsConfigId"]) {
    /** Injects a real post-write SDK rejection rather than pretending cross-store selection and preferences are atomic. */
    await t.test(method, async s => {
      const { runtime, settings, card } = await environment(s), ui = await ready(runtime, presentation("partial-" + method, "selector-chat"));
      const original = new Error("Controlled post-write rejection: " + method); settings.failAfterWrite(method, original);
      const row = keyedNode(ui.render().tree, "card:" + card.id);
      await assert.rejects(ui.dispatch(row.props.onClick),
        /** Requires the precise confirmed progress and original SDK cause from the real application coordinator. */
        failure => {
          assert.equal(failure.name, "SelectionApplicationFailure"); assert.equal(failure.cause, original);
          assert.deepEqual(plain(failure.completed), method === "applyThemeConfig" ? [{ type: "selection", selection: "card:" + card.id }]
            : [{ type: "selection", selection: "card:" + card.id }, { type: "theme", id: themeId }]);
          assert.deepEqual(plain(failure.failedStep), { type: method === "applyThemeConfig" ? "theme" : "tts", id: method === "applyThemeConfig" ? themeId : speechId });
          return true;
        },
      );
      assert.equal(runtime.records.get("selector-chat").extension.selection, "card:" + card.id); assert.equal(writes(runtime).length, 1);
      assert.equal(settings.current.theme, themeId); assert.equal(settings.current.tts, method === "applyThemeConfig" ? "tts-one" : speechId);
      const tree = ui.render().tree; assert.equal(tree.type, "Dialog"); assert.equal(keyedNode(tree, "selector-close").props.enabled, true);
      assert.match(keyedNode(tree, "selector-commit-error").props.text, /选择已提交.*实际存储状态待核对.*未自动撤销或重试.*Controlled post-write rejection/);
      const audit = plain(settings.calls); ui.render(); assert.deepEqual(settings.calls, audit);
      const result = await ui.dispatch(keyedNode(tree, "selector-close").props.onClick);
      assert.equal(result.actionResult.type, "toolpkg.presentation.cancel"); assert.deepEqual(settings.calls, audit); assert.equal(writes(runtime).length, 1);
    });
  }
});

/** Exact-ID receipts are required even when the independent owner has genuinely changed its preference state. */
test("registered binding application rejects contradictory Theme, TTS setter and current-ID receipts", async t => {
  for (const method of ["applyThemeConfig", "setCurrentTtsConfigId", "getCurrentTtsConfigId"]) {
    /** Tests each formal acknowledgement separately through the real registered binding provider. */
    await t.test(method, async s => {
      const { runtime, settings, card } = await environment(s);
      settings.returnReceipt(method, method === "applyThemeConfig" ? settings.themes.get("independent-theme-one") : "tts-one");
      await assert.rejects(runtime.api("chat.configuration.binding.write", { chatId: "selector-chat", selection: "card:" + card.id }),
        /** Rejects exact receipt mismatch while retaining confirmed selection and every genuinely acknowledged earlier apply. */
        failure => {
          assert.equal(failure.name, "SelectionApplicationFailure"); assert.match(String(failure.cause), /ID 与请求不一致|未确认请求的 ID/);
          assert.equal(failure.failedStep.type, method === "applyThemeConfig" ? "theme" : "tts"); return true;
        },
      );
      assert.equal(writes(runtime).length, 1); assert.equal(runtime.records.get("selector-chat").extension.selection, "card:" + card.id);
      assert.equal(settings.current.theme, themeId); assert.equal(settings.current.tts, method === "applyThemeConfig" ? "tts-one" : speechId);
      assert.equal(settings.calls.length, method === "applyThemeConfig" ? 3 : method === "setCurrentTtsConfigId" ? 4 : 5);
    });
  }
});

/** Explicit cancellation stays write-free for chat and global selection, even when listed records carry nonnull independent references. */
test("registered selector cancellation never writes selection or applies Theme and TTS", async t => {
  for (const chatId of ["selector-chat", null]) {
    /** Exercises the genuine SDK cancellation result rather than an injected production receiver. */
    await t.test(chatId === null ? "global" : "chat", async s => {
      const { runtime, settings } = await environment(s), ui = await ready(runtime, presentation("cancel-settings", chatId)), before = await stateBytes(runtime);
      settings.clearCalls(); runtime.disk.clearCalls(); const response = await ui.dispatch(keyedNode(ui.render().tree, "selector-close").props.onClick);
      assert.deepEqual(plain(response.actionResult), { type: "toolpkg.presentation.cancel", requestId: "cancel-settings" });
      assert.equal(Buffer.compare(await stateBytes(runtime), before), 0); assert.equal(writes(runtime).length, 0); assert.deepEqual(settings.calls, []);
      assert.equal(runtime.records.get("selector-chat").extension, null);
    });
  }
});

/** A real file publication error cannot be reported as a successful selection or trigger independent preference mutations. */
test("registered file selection commit failure preserves the original IO rejection and makes zero apply calls", async t => {
  const { runtime, settings, card } = await environment(t), original = new Error("Controlled selection file move failure"), before = await stateBytes(runtime);
  runtime.disk.failNext("storage.commit", original);
  await assert.rejects(runtime.api("activePrompt.setCard", { id: card.id }),
    /** Preserves the actual original IO error identity through the registered provider and central coordinator. */
    failure => failure === original,
  );
  assert.equal(Buffer.compare(await stateBytes(runtime), before), 0);
  assert.deepEqual(settings.calls, [{ method: "listThemeConfigs", args: [] }, { method: "listTtsConfigs", args: [] }]);
  assert.deepEqual(settings.current, { theme: "independent-theme-one", tts: "tts-one" });
});

/** Pre-create hooks only initialize this package's namespace and never apply global preferences before native chat creation commits. */
test("actual registered before-create hook never applies configs for explicit, source-free or branch initialization", async t => {
  const { runtime, settings, card } = await environment(t); await runtime.api("activePrompt.setCard", { id: card.id });
  settings.clearCalls(); runtime.calls.length = 0;
  assert.equal(runtime.chatLifecycleHooks.length, 1); assert.equal(runtime.chatLifecycleHooks[0].function, runtime.main.beforeChatCreate);
  for (const creation of ["explicit", "source-free", "branch"]) {
    const branch = creation === "branch", payload = {
      eventName: "before_create", creationKind: branch ? "branch" : "new",
      chat: { id: "uncommitted-" + creation, title: "Controlled uncommitted creation", workspaceId: null, parentChatId: branch ? "selector-chat" : null },
      sourceChatId: branch ? "selector-chat" : null, sourceMessageTimestamp: branch ? 123 : null,
      input: creation === "explicit" ? createChatInput("card:" + card.id) : null,
      sourceExtension: branch ? { version: 1, selection: "card:" + card.id } : null,
    };
    const initialized = await runtime.chatLifecycleHooks[0].function({ eventName: "before_create", eventPayload: payload });
    assert.deepEqual(plain(initialized), { extension: { version: 1, selection: "card:" + card.id } });
  }
  assert.deepEqual(settings.calls, []); assert.deepEqual(runtime.calls, []);
});

/** Executes the same main module as an ordinary engine owner without invoking the registration-only export. */
test("execution runtime can read named themes before registerToolPkg is invoked", async t => {
  const disk = await createDiskHarness(t), settings = createSelectionSettingsFixture(), channels = new Map();
  loadModule("src/main.ts", {
    ...disk.globals,
    Tools: { ...disk.globals.Tools, SoftwareSettings: settings.settings },
    ToolPkg: { ...disk.globals.ToolPkg, ipc: {
      /** Retains the real main-module handler without invoking package registration or emulating business operations. */
      on(name, handler) { assert.equal(channels.has(name), false); channels.set(name, handler); },
    } },
  });
  assert.deepEqual(settings.calls, []); assert.deepEqual(disk.calls, []);
  const choices = await channels.get("character-memory.request")({ action: "listThemeChoices" });
  assert.deepEqual(plain(choices), [...settings.themes.values()].map(
    /** Compares exact public catalog names and IDs without inventing plugin configuration state. */
    config => ({ id: config.id, label: config.name }),
  ));
});
