import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";

const chat = readFileSync(new URL("../src/commands/chat.rs", import.meta.url), "utf8");
const root = readFileSync(new URL("../src/commands/mod.rs", import.meta.url), "utf8");
const core = readFileSync(new URL("../../../runtime/application/src/services/ChatServiceCore.rs", import.meta.url), "utf8");

/** Extracts one production Rust function without executing or reimplementing its business logic. */
function functionSource(name) {
  const pattern = new RegExp(`(?:async )?fn ${name}\\([^]*?(?=\\n(?:///|#\\[|(?:pub )?(?:async )?fn )|$)`, "u");
  const match = chat.match(pattern);
  assert.notEqual(match, null, `missing mounted chat function ${name}`);
  return match[0];
}

/** Verifies the mounted command catalog and handler have no retired role or manual-group routes. */
test("static: mounted Core chat owns only neutral chat actions", () => {
  assert.match(root, /mod chat;/u);
  assert.match(root, /BuiltinRoot::Chat => chat::run_chat_command/u);
  assert.doesNotMatch(chat, /bind-character|bind-group|set-group|--character|--group-card|--group|updateChatCharacterCard|updateChatCharacterGroup|updateChatGroup|CharacterCards|CharacterManager|CharacterGroupManager/u);
  assert.doesNotMatch(chat, /characterCardName|characterGroupId|chat\.group\b/u);
  assert.match(functionSource("run_chat_command"), /_ => Err\(format!\("unknown chat command:/u);
});

/** Verifies real source/input options are forwarded to the actual awaited creation signature. */
test("static: generic creation uses the current awaited lifecycle signature", () => {
  assert.match(core, /pub async fn createNewChat\(\s*&mut self,\s*setAsCurrentChat: bool,\s*sourceChatId: Option<String>,\s*input: Option<serde_json::Value>,/u);
  const create = functionSource("create_chat");
  assert.match(create, /EnhancedAIService::new\(\s*application\.toolHandler\.clone\(\),\s*application\.providerRuntimeContext\.clone\(\)/u);
  assert.match(create, /core\.createNewChat\(\s*options\.setAsCurrentChat,\s*options\.sourceChatId\.clone\(\),\s*options\.input\.clone\(\),\s*\)\.await/u);
  assert.doesNotMatch(create, /currentChatIdFlow|callApi|exec\(|Character/u);
  assert.match(create, /"chatId": chatId/u);
});

/** Checks strict parsing branches against production source without supplying a second parser. */
test("static: creation parser rejects duplicate, malformed and legacy arguments", () => {
  const parser = functionSource("parse_chat_new_args");
  for (const flag of ["--set-current", "--source", "--input"]) assert.match(parser, new RegExp(`"${flag}" =>`, "u"));
  assert.match(parser, /if hasSetCurrent \{ return Err/u);
  assert.match(parser, /if options\.sourceChatId\.is_some\(\) \{ return Err/u);
  assert.match(parser, /if options\.input\.is_some\(\) \{ return Err/u);
  assert.match(parser, /serde_json::from_str\(value\)\.map_err/u);
  assert.match(parser, /if !input\.is_object\(\) \{ return Err/u);
  assert.match(parser, /_ => return Err\(usage\.to_string\(\)\)/u);
  assert.match(parser, /value\.trim\(\)\.is_empty\(\) \|\| value\.trim\(\) != value/u);
});

/** Verifies every command call affected by the async lifecycle refactor awaits its result. */
test("static: switch, deletion and branching await the real async host methods", () => {
  assert.match(core, /pub async fn deleteChatHistory\(/u);
  assert.match(functionSource("delete_chat"), /deleteChatHistory\(chatId\.clone\(\)\)\.await/u);
  assert.match(functionSource("show_chat"), /switchChat\(chatId\.clone\(\)\)\.await\?/u);
  assert.match(functionSource("switch_chat_command"), /switchChat\(chatId\.clone\(\)\)\.await/u);
  assert.match(functionSource("create_chat_branch"), /createBranch\(upToMessageTimestamp\)\.await/u);
});

/** Verifies neutral statistics no longer obtain data from retired character/group managers. */
test("static: stats and help expose only neutral canonical chat data", () => {
  const stats = functionSource("show_chat_stats");
  assert.match(stats, /manager\.getTotalChatCount\(\)/u);
  assert.match(stats, /manager\.getTotalMessageCount\(\)/u);
  assert.match(stats, /json!\(\{ "totalChats": totalChats, "totalMessages": totalMessages \}\)/u);
  assert.doesNotMatch(stats, /character|group|plugin/u);
  assert.match(functionSource("print_chat_usage"), /--source <chat-id>.*--input <json-object>/u);
});

/** Verifies sends use the actual originating receipt rather than searching current history or wall-clock timestamps. */
test("static: CLI send preserves the generic originating-turn outcome without a latest-AI heuristic", () => {
  const send = functionSource("send_chat_message_with_application");
  assert.match(send, /RuntimeChatSendRequest \{/u);
  assert.match(send, /participantId: None/u);
  assert.match(send, /replyToMessageTimestamp: sendArgs\.replyToTimestamp/u);
  assert.match(send, /runtimeSupport\(\)\.sendChatMessage\(request\)\.await\?/u);
  assert.match(send, /serde_json::to_value\(&receipt\)/u);
  assert.doesNotMatch(chat, /beforeLastAiTimestamp|wait_for_committed_ai_message|dispatch_chat_message_with_application|RecvTimeoutError/u);
  assert.match(functionSource("print_chat_send_result"), /output\.setJsonStdout\(result\.clone\(\)\)/u);
});

console.log("SCOPE: mounted Core chat source assertions only; no Rust compilation or runtime execution");
