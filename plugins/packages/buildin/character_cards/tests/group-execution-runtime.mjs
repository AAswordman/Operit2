import assert from "node:assert/strict";
import { createDiskHarness } from "./disk-files.mjs";
import { loadModule, plain } from "./runtime.mjs";
import { createSoftwareSettingsFixture } from "./software-settings-fixture.mjs";

const directoryInput = plain(loadModule("tests/software-settings-input.ts").softwareSettingsTestInput);
const owner = "com.operit.character_cards", otherOwner = "controlled.sibling_package";

/** Creates a deterministic test rendezvous without delays, timers or speculative polling. */
export function deferred() {
  let resolve, reject;
  const promise = new Promise(
    /** Exposes settlement only for the controlled asynchronous host/AI boundary used by the test. */
    (yes, no) => { resolve = yes; reject = no; },
  );
  return { promise, resolve, reject };
}

/** Constructs a fully correlated controlled receipt; these are not receipts from a configured Core host. */
export function controlledReceipt(input, userMessageTimestamp, assistant, status = "completed") {
  const initial = Object.hasOwn(input, "kind"), chatId = initial ? input.submission.chatId : input.chatId;
  return { chatId, message: initial ? input.submission.text : "", sentAt: 1700000000000, aiResponse: null, receivedAt: null,
    outcome: { type: "committed", status, userMessageTimestamp, assistant } };
}

/** Opens real disk domain records while injecting explicitly controlled, owner-isolated Chat, AI and turn-transport boundaries. */
export async function groupEnvironment(t, transport) {
  const disk = await createDiskHarness(t), settings = createSoftwareSettingsFixture(disk, directoryInput), chatId = "controlled-real-chat";
  const records = new Map(), planReplies = [], planningStarted = deferred(), calls = [], modelCalls = [], extensionCalls = [];
  const chat = {
    /** Requires an actual fixture chat and reads only the authenticated owner namespace; unknown targets are errors. */
    async readExtension(target) {
      if (target.kind !== "chat" || target.chatId !== chatId) throw new Error("Controlled chat record does not exist");
      const record = records.get(target.chatId); if (record === undefined) throw new Error("Controlled chat record does not exist");
      extensionCalls.push({ owner, target: plain(target) });
      return Object.hasOwn(record, owner) ? plain(record[owner]) : null;
    },
    /** Executes only the explicitly queued functional planner result and rejects all missing fixture capabilities. */
    async call(input) {
      assert.equal(input.functionType, "ROLE_RESPONSE_PLANNER"); modelCalls.push(plain(input)); planningStarted.resolve();
      if (planReplies.length === 0) throw new Error("No controlled AI result was scheduled");
      const response = await planReplies.shift();
      if (response instanceof Error) throw response;
      if (typeof response.text !== "string") throw new Error("Controlled AI result must provide explicit text");
      return { text: response.text, turns: [], finishReason: "stop", inputTokens: 1, outputTokens: 1 };
    },
  };
  const tools = new Proxy({ Files: disk.files, SoftwareSettings: settings.harness.globals.Tools.SoftwareSettings, Chat: chat }, {
    /** Rejects every unspecified host capability rather than emulating a working SDK. */
    get(target, property) {
      if (!Object.hasOwn(target, property)) throw new Error("Unprovided controlled Tools capability: " + String(property));
      return target[property];
    },
  });
  const globals = { ...disk.globals, Tools: tools };
  const repository = await loadModule("src/storage/files.ts", globals).FileCharacterRepository.open({
    /** Reads only the explicitly supplied complete controlled model directory. */
    listModels: () => tools.SoftwareSettings.listModelSummaries(),
    /** Reads only the explicitly supplied complete controlled voice directory. */
    listTtsConfigs: () => tools.SoftwareSettings.listTtsConfigs(),
    /** Reads only the explicitly supplied complete controlled tool-source catalog. */
    readToolCatalog: () => tools.SoftwareSettings.readToolSourceCatalog(),
  });
  const service = loadModule("src/service.ts", globals).createCharacterCardsService(repository);
  const a = await service.dispatchDomain("character.create", { values: { name: "Participant A", avatarUri: "a.png" } });
  const b = await service.dispatchDomain("character.create", { values: { name: "Participant B", avatarUri: "b.png" } });
  const group = await service.dispatchDomain("group.create", { values: { name: "Controlled saved group", members: [{ characterCardId: a.id, orderIndex: 1 }, { characterCardId: b.id, orderIndex: 0 }] } });
  const selection = "group:" + group.id;
  records.set(chatId, { [owner]: { version: 1, selection, audit: { preserved: true } }, [otherOwner]: { version: 37, selection: "not-this-owner" } });
  const wrapped = {
    /** Captures the exact full initial input before invoking the controlled turn boundary. */
    async submit(input) { calls.push({ method: "submit", input: plain(input) }); return transport.submit(input); },
    /** Captures a continuation without manufacturing another user message or newest-reply lookup. */
    async continue(input) { calls.push({ method: "continue", input: plain(input) }); return transport.continue(input); },
    /** Captures only the exact correlated execution requested for cancellation. */
    async cancel(input) { calls.push({ method: "cancel", input: plain(input) }); return transport.cancel(input); },
    /** Captures explicit plugin-local planning disposal; native sends own their cleanup independently. */
    async abandon(input) { calls.push({ method: "abandon", input: plain(input) }); return transport.abandon(input); },
    /** Captures explicit sequence finalization after the last planned participant receipt. */
    async finish(input) { calls.push({ method: "finish", input: plain(input) }); return transport.finish(input); },
  };
  const api = loadModule("src/group-execution/domain-adapter.ts", globals);
  const controller = api.createGroupExecutionController(service, wrapped);
  return { disk, service, controller, calls, modelCalls, extensionCalls, group, a, b, chatId, selection, planningStarted: planningStarted.promise,
    /** Schedules exactly one formal AI response, with no default response available to later calls. */
    setPlan(plan) { planReplies.push({ text: JSON.stringify(plan) }); },
    /** Schedules an explicit raw or delayed controlled functional-model result. */
    setRawPlan(result) { planReplies.push(result); },
    /** Constructs a complete original input including a real real reply timestamp and full attachment JSON. */
    input(submissionId = "controlled-submission") {
      return { submissionId, chatId, selection, runtime: "main", notifyReply: true, text: "Original request", attachments: [{ filePath: "vfs://source/document.pdf", nodeId: "resource-node", fileName: "document.pdf", mimeType: "application/pdf", fileSize: 405, content: "Actual inline attachment content" }],
        replyToMessageTimestamp: 1700000000100 };
    },
  };
}
