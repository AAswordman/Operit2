import assert from "node:assert/strict";
import test from "node:test";
import { fixture } from "./fixtures.mjs";
import { loadModule, plain } from "./runtime.mjs";
function summaries() {
  return [
    { id: "ui-chat-one", title: "完整第一条对话", updatedAt: "2026-10-08T12:00:00Z", displayOrder: 0, workspaceId: "workspace-one", workspaceName: "工作区一", locked: false, pinned: true, group: null },
    { id: "ui-chat-two", title: "另一条锁定对话", updatedAt: "2026-10-08T12:00:01Z", displayOrder: 1, workspaceId: "workspace-one", workspaceName: "工作区一", locked: true, pinned: false, group: null },
  ];
}
/** Validates the exact neutral host-state shape and rejects plugin role fields rather than inspecting them. */
test("sidebar context parser requires explicit input and generic summary fields with no presentation inference", () => {
  const parser = loadModule("src/ui-sidebar.ts");
  const expected = { input: { view: "characters" }, chatSidebar: { chats: summaries(), currentChatId: "ui-chat-one", activeStreamingChatIds: ["ui-chat-one"] } };
  assert.deepEqual(plain(parser.parseCurrentSidebar(expected)), expected);
  for (const invalid of [null, {}, { input: { mode: "manage" }, chatSidebar: expected.chatSidebar }, { input: expected.input, chatSidebar: null }, { input: expected.input, chatSidebar: { ...expected.chatSidebar, chats: [{ ...summaries()[0], characterCardName: "old-native-card" }] } }]) assert.throws(
    /** Passes each exact malformed host boundary to the real parser. */
    () => parser.parseCurrentSidebar(invalid),
  );
  assert.deepEqual(plain(parser.parseChatActivateAction({ type: "toolpkg.chat.activate", chatId: "ui-chat-one" })), { type: "toolpkg.chat.activate", chatId: "ui-chat-one" });
  assert.throws(
    /** Rejects attaching opaque selection or owner fields to the generic host action. */
    () => parser.parseChatActivateAction({ type: "toolpkg.chat.activate", chatId: "ui-chat-one", selection: "card:foreign" }),
  );
});
/** Keeps every explicit namespace-derived selection state mandatory, including an explicitly unbound chat. */
test("character categories require explicit selection states and preserve actual empty role-group categories", () => {
  const projection = loadModule("src/ui-sidebar.ts"), directory = fixture().snapshot;
  const chats = summaries(), selections = [{ chatId: chats[0].id, selection: "card:travel" }, { chatId: chats[1].id, selection: null }];
  const sections = plain(projection.characterSidebarSections(directory, selections, chats));
  assert.equal(sections.find(
    /** Finds the actual persisted role group from the labelled directory input. */
    section => section.id === "group:group-one",
  ).chats.length, 0);
  assert.deepEqual(sections.find(
    /** Locates explicitly unbound state rather than assigning the active character. */
    section => section.id === "unbound",
  ).chats, [chats[1]]);
  assert.throws(
    /** Rejects a missing state instead of treating it as an implicitly unbound record. */
    () => projection.characterSidebarSections(directory, selections.slice(0, 1), chats), /no explicit namespace selection state/,
  );
});


test("folder projection reads native membership without plugin metadata and scopes repeated names", () => {
  const projection = loadModule("src/ui-sidebar.ts");
  const chats = summaries().map(chat => ({...chat, group: "Native folder"}));
  const section = {id:"card:travel",title:"Travel",selection:"card:travel",avatarUri:null,kind:"card",chats};
  const projected = plain(projection.conversationSidebarScope(section));
  assert.equal(projected.groups.length, 1);
  assert.equal(projected.groups[0].name, "Native folder");
  assert.deepEqual(projected.groups[0].chats.map(chat => chat.id), chats.map(chat => chat.id));
  assert.equal(projected.groups[0].pinned, false);
  assert.deepEqual(projected.ungrouped, []);
  const other = plain(projection.conversationSidebarScope({...section,id:"card:other"}));
  assert.notEqual(projected.groups[0].id, other.groups[0].id);
});
