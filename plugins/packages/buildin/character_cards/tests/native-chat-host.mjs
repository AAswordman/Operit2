import assert from "node:assert/strict";
import { plain } from "./runtime.mjs";

/** Controls generic owner-isolated native record namespaces only; all role interpretation and CRUD remain in the real plugin. */
export function createNativeChatHost(records) {
  assert.ok(records instanceof Map); const calls = [], faults = new Map(); let writeGate = null;
  /** Requires an explicitly declared native chat and an exact generic owner-isolated record target. */
  function recordFor(target) {
    assert.deepEqual(Object.keys(target).sort(), ["chatId", "kind"]); assert.equal(target.kind, "chat");
    assert.equal(records.has(target.chatId), true, "Undeclared controlled chat: " + target.chatId);
    const record = records.get(target.chatId); assert.equal(Object.hasOwn(record, "extension"), true); return record;
  }
  /** Audits the genuine boundary input and propagates a deliberately planned original host error. */
  function observe(method, ...args) {
    calls.push({ method, args: plain(args) });
    if (faults.has(method)) { const failure = faults.get(method); faults.delete(method); throw failure; }
  }
  const chat = {
    /** Reads the declared owner namespace without creating a binding or substituting any active selection. */
    async readExtension(target) { observe("readExtension", target); const record = recordFor(target); return record.extension === null ? null : plain(record.extension); },
    /** Stores exactly the actual plugin-submitted JSON after the explicit controlled native write barrier. */
    async writeExtension(target, value) {
      observe("writeExtension", target, value); const record = recordFor(target);
      if (writeGate !== null) await writeGate;
      record.extension = plain(value); return plain(record.extension);
    },
    /** Deletes only the executing owner's namespace without deleting chats or plugin metadata. */
    async deleteExtension(target) { observe("deleteExtension", target); const record = recordFor(target), existed = record.extension !== null; record.extension = null; return existed; },
  };
  return {
    records, calls, chat, observe,
    /** Schedules one explicit host failure without changing the real plugin's rejection behavior. */
    failNext(method, failure) { assert.equal(faults.has(method), false); faults.set(method, failure); },
    /** Holds one genuine native write so tests can audit disabled controls and stale action rejection. */
    holdWrite() {
      assert.equal(writeGate, null); let release;
      writeGate = new Promise(
        /** Captures only the external ABI barrier, not a synthetic successful binding response. */
        resolve => { release = resolve; },
      );
      return {
        /** Releases the actual pending host call once, without retrying any operation. */
        release() { assert.notEqual(writeGate, null); writeGate = null; release(); },
      };
    },
  };
}
