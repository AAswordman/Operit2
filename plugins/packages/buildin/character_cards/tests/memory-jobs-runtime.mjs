import assert from "node:assert/strict";
import path from "node:path";
import { readFile } from "node:fs/promises";
import { createDiskHarness } from "./disk-files.mjs";
import { openPlugin } from "./plugin-entry-harness.mjs";
import { plain } from "./runtime.mjs";

export const CONTROLLED_CHAT_ID = "memory-jobs-controlled-chat";
export const CONTROLLED_OWNER = "character:default";
const SOURCE_TIME = 1760000000000;

/** Exposes only the explicitly supplied controlled host capabilities and rejects every missing method. */
function strictCapability(name, members) {
  return new Proxy(members, {
    /** Rejects unavailable SDK operations instead of inventing empty results or alternate providers. */
    get(target, property) {
      if (!Object.hasOwn(target, property)) throw new Error("Controlled memory-jobs fixture does not provide " + name + "." + String(property));
      return target[property];
    },
  });
}

/** Creates explicit Chat/AI fixtures around real temporary disk IO, never a substitute domain backend. */
export async function createMemoryJobsHarness(testContext) {
  const disk = await createDiskHarness(testContext), modelCalls = [], rangeCalls = [], findCalls = [], outcomes = [];
  const history = [];
  for (let index = 0; index < 32; index += 1) {
    history.push({ sender: index % 2 === 0 ? "user" : "ai", content: "Controlled conversation fact " + index,
      timestamp: SOURCE_TIME + index * 10, variantIndex: 0, variantCount: 1,
      provider: "controlled-fixture-provider", modelName: "controlled-fixture-model" });
  }
  const chat = { id: CONTROLLED_CHAT_ID, title: "Explicit controlled memory conversation", messageCount: history.length,
    createdAt: String(SOURCE_TIME), updatedAt: String(SOURCE_TIME + 1000), isCurrent: true, inputTokens: 0, outputTokens: 0 };
  let nextRangeEffect = null, chatExtension = null;
  const extensionCalls = [], messageExtensions = new Map();
  /** Supplies one explicitly controlled complete send snapshot for the fixture's original assistant identity. */
  function messageMarker() {
    const profile = { id: "default", name: "Controlled historical participant", avatarUri: null, introPrompt: "Controlled role prompt", userPreferencesText: "Controlled USER snapshot", openingStatement: "Controlled opening",
      modelBinding: { providerId: "controlled-fixture-provider", modelId: "controlled-fixture-model" }, ttsConfigId: "controlled-fixture-voice",
      toolAccess: { enabled: true, allowedBuiltinTools: ["read_file"], allowedPackages: [], allowedSkills: [], allowedMcpServers: [] },
      resources: [{ key: CONTROLLED_OWNER, readable: true, writable: true }] };
    return { version: 1, selection: "card:default", promptFunctionType: "CHAT", primaryOwnerKey: CONTROLLED_OWNER, profile, participants: [plain(profile)] };
  }
  for (const message of history) if (message.sender === "ai") messageExtensions.set(JSON.stringify([message.timestamp, message.variantIndex]), messageMarker());
  /** Requires an exact existing generic record revision rather than fabricating an absent record's namespace. */
  function extensionTarget(target) {
    assert.equal(target.chatId, chat.id);
    switch (target.kind) {
      case "chat": assert.deepEqual(Object.keys(target).sort(), ["chatId", "kind"]); return null;
      case "message": {
        assert.deepEqual(Object.keys(target).sort(), ["chatId", "kind", "messageTimestamp", "variantIndex"]);
        const matches = history.filter(
          /** Identifies the actual explicitly persisted fixture revision. */
          message => message.timestamp === target.messageTimestamp && target.variantIndex >= 0 && target.variantIndex < message.variantCount,
        );
        assert.equal(matches.length, 1, "Controlled message revision must really exist");
        return JSON.stringify([target.messageTimestamp, target.variantIndex]);
      }
      default: throw new Error("Unknown controlled extension target");
    }
  }

  /** Advances actual fixture history metadata for a deliberate source edit. */
  function historyChanged() { chat.messageCount = history.length; chat.updatedAt = String(Number(chat.updatedAt) + 1); }

  const controlledChat = strictCapability("Tools.Chat", {
    /** Enumerates the sole explicit generic host conversation without a file-backed association mirror. */
    async listAll() {
      return { totalCount: 1, currentChatId: chat.id, chats: [plain(chat)],
        /** Formats only the controlled generic directory response. */
        toString() { return "Controlled chat directory"; },
      };
    },
    /** Reads only the explicitly present authenticated fixture namespace on an existing record. */
    async readExtension(target) {
      extensionCalls.push({ method: "readExtension", target: plain(target) });
      const key = extensionTarget(target);
      if (key === null) return chatExtension === null ? null : plain(chatExtension);
      return messageExtensions.has(key) ? plain(messageExtensions.get(key)) : null;
    },
    /** Writes the exact supplied namespace object without implementing any plugin-domain association rules. */
    async writeExtension(target, value) {
      extensionCalls.push({ method: "writeExtension", target: plain(target), value: plain(value) });
      const key = extensionTarget(target);
      if (key === null) chatExtension = plain(value); else messageExtensions.set(key, plain(value));
      return plain(value);
    },
    /** Deletes only the controlled owner namespace and does not synthesize a missing-record success. */
    async deleteExtension(target) {
      extensionCalls.push({ method: "deleteExtension", target: plain(target) });
      const key = extensionTarget(target);
      if (key !== null) return messageExtensions.delete(key);
      const existed = chatExtension !== null; chatExtension = null; return existed;
    },
    /** Implements only the exact ID lookup used by production, with the current real SDK result fields. */
    async findChat(options) {
      assert.deepEqual(Object.keys(options).sort(), ["index", "match", "query"]);
      assert.equal(options.match, "exact"); assert.equal(options.index, 0); findCalls.push(plain(options));
      if (options.query !== chat.id) throw new Error("Controlled host chat does not exist: " + options.query);
      return { matchedCount: 1, chat: plain(chat),
        /** Formats this explicitly controlled identity lookup result. */
        toString() { return "Controlled chat: " + chat.id; },
      };
    },
    /** Enforces the audited mandatory inclusive i32 range and returns only explicitly present source messages. */
    async getMessagesRange(chatId, options) {
      assert.equal(chatId, chat.id); assert.deepEqual(Object.keys(options).sort(), ["end", "order", "start"]);
      assert.equal(options.order, "asc"); assert.equal(options.start, 0);
      assert.ok(Number.isSafeInteger(options.end) && options.end >= 0 && options.end < 2147483647, "Host range requires an explicit inclusive i32 end");
      const selected = [];
      for (const message of history.slice(options.start, options.end + 1)) if (message.sender !== "summary") selected.push(plain(message));
      rangeCalls.push({ chatId, ...plain(options) });
      const result = { chatId, order: "asc", start: options.start, end: options.end, limit: options.end - options.start + 1, messages: selected };
      const effect = nextRangeEffect; nextRangeEffect = null;
      if (effect !== null) effect();
      return result;
    },
    /** Consumes one scheduled controlled functional-model result; unscheduled AI calls always fail. */
    async call(options) {
      assert.deepEqual(Object.keys(options).sort(), ["enableThinking", "functionType", "recordTokenUsage", "turns"]);
      assert.equal(options.functionType, "MEMORY"); assert.equal(options.recordTokenUsage, true); assert.equal(options.enableThinking, false);
      assert.equal(options.turns.length, 2); assert.equal(options.turns[0].kind, "SYSTEM"); assert.equal(options.turns[1].kind, "USER");
      modelCalls.push(plain(options));
      if (outcomes.length === 0) throw new Error("Unscheduled controlled MEMORY model call");
      const outcome = outcomes.shift();
      switch (outcome.kind) {
        case "error": throw outcome.error;
        case "error-and-status-write": disk.failNext("write", outcome.statusError); throw outcome.error;
        case "output": return { text: outcome.text, turns: [{ kind: "ASSISTANT", content: outcome.text, metadata: { fixture: "controlled" } }],
          finishReason: "stop", metadata: { fixture: "controlled-chat-ai" }, receivedAt: SOURCE_TIME + 5000,
          /** Formats only the explicitly queued controlled response. */
          toString() { return outcome.text; },
        };
        default: throw new Error("Invalid scheduled controlled model outcome");
      }
    },
  });
  const tools = strictCapability("Tools", { Files: disk.files, Chat: controlledChat });
  const statePath = path.join(disk.directory, "character-memory", "state.json");

  /** Opens a fresh production main module against the same real disk after the previous runtime is retired. */
  function openRuntime() {
    const plugin = openPlugin({ ...disk, globals: { ...disk.globals, Tools: tools } });
    assert.equal(plugin.toolLifecycleHooks.length, 1); assert.equal(plugin.toolPromptHooks.length, 1);
    return {
      ...plugin,
      /** Invokes the actual registered lifecycle rather than directly constructing a repository. */
      async initialize() {
        assert.equal(plugin.lifecycle.length, 1); assert.equal(plugin.lifecycle[0].id, "service-initialize");
        assert.equal(plugin.lifecycle[0].event, "application_on_create");
        await plugin.lifecycle[0].function({ event: "application_on_create", eventName: "application_on_create", eventPayload: {} });
      },
      /** Writes the real plugin chat selection through its registered typed API. */
      async bind() { return plugin.api("chat.configuration.binding.write", { chatId: chat.id, selection: "card:default" }); },
      /** Maps the selected controlled source revision to the real Core persistence-hook payload field. */
      async persistMessage(message) {
        assert.equal(plugin.chatMessageHooks.length, 1);
        const { variantIndex, variantCount, ...record } = plain(message);
        assert.ok(Number.isInteger(variantIndex) && variantIndex >= 0 && variantIndex < variantCount);
        const eventPayload = { chatId: chat.id, ...record, selectedVariantIndex: variantIndex };
        assert.equal(Object.hasOwn(eventPayload, "variantIndex"), false);
        await plugin.chatMessageHooks[0].function({ event: "message_persisted", eventName: "message_persisted", eventPayload });
      },
      /** Invokes the registered interval callback, not a private scheduler or a second service. */
      async interval() {
        assert.equal(plugin.hostEventHooks.length, 1);
        await plugin.hostEventHooks[0].function({ event: "interval", eventName: "interval", eventPayload: {} });
      },
      /** Enqueues five explicitly stored assistant sources using the actual registered message hook. */
      async enqueueReplies() {
        const replies = [];
        for (const message of history) if (message.sender === "ai") replies.push(plain(message));
        const selected = replies.slice(-5); assert.equal(selected.length, 5);
        for (const message of selected) await this.persistMessage(message);
        return selected;
      },
      /** Makes scheduling due through the real settings API without a clock shim or a state-file edit. */
      async makeDue() {
        const settings = await plugin.api("memory.settings.read", { ownerKey: CONTROLLED_OWNER });
        await plugin.api("memory.settings.write", { ownerKey: CONTROLLED_OWNER, settings: { ...settings, nextAutoSaveRunAtMs: 1 } });
      },
      /** Starts a genuine persisted source plan through its public registered handler. */
      async startRebuild() {
        return plugin.api("memory.rebuild.start", { ownerKey: CONTROLLED_OWNER, rebuild: { chatIds: [chat.id], windowMessageCount: 8, fromInclusive: null, toInclusive: null } });
      },
    };
  }
  return {
    disk, tools, modelCalls, rangeCalls, findCalls, extensionCalls, statePath, openRuntime,
    /** Reads native authoritative bytes independently of retained plugin memory. */
    readStateText() { return readFile(statePath, "utf8"); },
    /** Parses native state bytes without repairing them or supplying missing records. */
    async readState() { return JSON.parse(await readFile(statePath, "utf8")); },
    /** Queues one explicitly supplied controlled JSON response, including the documented no-facts object. */
    queueAnalysis(value) { assert.notEqual(value, undefined); outcomes.push({ kind: "output", text: JSON.stringify(value) }); },
    /** Queues an exact error object whose identity must survive the production job pipeline. */
    queueExecutionError(error) { assert.ok(error instanceof Error); outcomes.push({ kind: "error", error }); },
    /** Schedules the exact AI rejection and the subsequent real status-publication IO failure. */
    queueExecutionAndStatusError(error, statusError) {
      assert.ok(error instanceof Error); assert.ok(statusError instanceof Error); outcomes.push({ kind: "error-and-status-write", error, statusError });
    },
    /** Requires every scheduled fixture result to have been consumed by the expected real callback. */
    assertOutputsConsumed() { assert.equal(outcomes.length, 0, "A controlled model response was scheduled but never invoked"); },
    /** Returns an explicitly present source message for registered callback input. */
    message(index) { assert.ok(Number.isInteger(index) && index >= 0 && index < history.length); return plain(history[index]); },
    /** Removes one explicitly saved assistant snapshot while leaving the actual message revision present. */
    removeMessageMarker(timestamp, variantIndex) { assert.equal(messageExtensions.delete(JSON.stringify([timestamp, variantIndex])), true); },
    /** Removes the current chat marker to prove background assistant work uses only immutable message snapshots. */
    removeChatMarker() { chatExtension = null; },
    /** Selects a genuinely present controlled variant and changes source metadata before the next scheduled read. */
    selectNewVariant(timestamp) {
      const messages = history.filter(
        /** Requires one explicitly present message to receive the controlled persisted variant. */
        message => message.timestamp === timestamp,
      );
      assert.equal(messages.length, 1); const message = messages[0];
      message.variantCount += 1; message.variantIndex = message.variantCount - 1; message.content += " [Controlled selected variant]";
      messageExtensions.set(JSON.stringify([timestamp, message.variantIndex]), messageMarker()); historyChanged();
    },
    /** Deletes exactly one claimed source and updates the controlled host's actual summary metadata. */
    deleteMessage(timestamp) {
      const indices = [];
      for (let index = 0; index < history.length; index += 1) if (history[index].timestamp === timestamp) indices.push(index);
      assert.equal(indices.length, 1); history.splice(indices[0], 1); historyChanged();
    },
    /** Appends one explicit source while a range is in flight to test source-count consistency. */
    appendMessageDuringNextRange() {
      assert.equal(nextRangeEffect, null);
      /** Mutates only the controlled host history after returning the planned range snapshot. */
      nextRangeEffect = () => {
        history.push({ sender: "user", content: "Controlled concurrent new source", timestamp: SOURCE_TIME + 10000,
          variantIndex: 0, variantCount: 1, provider: "controlled-fixture-provider", modelName: "controlled-fixture-model" });
        historyChanged();
      };
    },
    /** Edits one explicit source without changing count to test updated-at consistency during a range read. */
    editMessageDuringNextRange(timestamp) {
      assert.equal(nextRangeEffect, null);
      /** Updates the source content and revision only after the original range was captured. */
      nextRangeEffect = () => {
        const selected = [];
        for (const message of history) if (message.timestamp === timestamp) selected.push(message);
        assert.equal(selected.length, 1); selected[0].content = "Controlled concurrent content edit"; historyChanged();
      };
    },
  };
}
