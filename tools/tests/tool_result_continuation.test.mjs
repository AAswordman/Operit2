import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../../', import.meta.url);

/** Reads one production source file for non-compiling architecture contract checks. */
function source(path) {
  return readFileSync(new URL(path, root), 'utf8');
}

const tools = 'core/crates/tool/services/src/';
const providers = 'core/crates/provider/services/src/chat/';

/** A large batch must retain all result slots, using one unit for every single-result budget. */
test('tool results are bounded independently without a batch-wide admission gate', () => {
  const markup = source(tools + 'ConversationMarkupManager.rs');
  const limits = source(tools + 'tools/ToolExecutionLimits.rs');
  const buildStart = markup.indexOf('pub fn buildToolResultMessage(');
  const buildEnd = markup.indexOf('pub fn createToolNotAvailableError(', buildStart);
  assert.ok(buildStart >= 0 && buildEnd > buildStart);
  const build = markup.slice(buildStart, buildEnd);
  assert.match(build, /\.map\(Self::formatToolResultForMessage\)/u);
  assert.match(build, /\.join\("\\n"\)/u);
  assert.doesNotMatch(build, /\bbreak\b|MAX_|\.take\(/u);
  assert.match(limits, /MAX_SINGLE_TOOL_RESULT_MESSAGE_CHARS: usize = Self::MAX_FILE_READ_BYTES \* 2/u);
  assert.match(markup, /saturating_sub\(wrap\(""\)\.chars\(\)\.count\(\)\)/u);
  assert.match(markup, /TOOL_RESULT_TRUNCATION_SUFFIX\.chars\(\)\.count\(\)/u);
  assert.doesNotMatch(markup, /MAX_FINAL_TOOL_RESULT_MESSAGE_CHARS|buildBoundedToolResultMessage/u);
});

/** Execution produces one finalized batch; consumers never format the same tool results again. */
test('display and model history share the finalized invocation batch message', () => {
  const manager = source(tools + 'ToolExecutionManager.rs');
  const enhanced = source(providers + 'EnhancedAIService.rs');
  const execute = manager.slice(manager.indexOf('pub async fn executeInvocations('), manager.indexOf('fn routeChangeIntentForResult('));
  assert.match(execute, /Result<ToolExecutionBatch, String>/u);
  assert.match(execute, /ToolExecutionBatch::new\(results, routeChangeIntent\)/u);
  assert.doesNotMatch(execute, /formatToolResultForMessage|let mut emitted/u);
  assert.match(enhanced, /message: toolResultMessage/u);
  assert.match(enhanced, /format!\("\\n\{toolResultMessage\}\\n"\)/u);
  assert.match(enhanced, /content: toolResultMessage/u);
  assert.match(enhanced, /stream,\s+continuation\.message,\s+callbacks\.clone\(\)/u);
  assert.doesNotMatch(enhanced, /buildToolResultMessage|toolResultMessageOverride|toolResultOverrideMessage/u);
});

/** Warning-only continuations cannot enter a synthetic empty tool batch or duplicate history. */
test('pure-thinking warnings continue directly and actual dispatch rejects empty invocations', () => {
  const enhanced = source(providers + 'EnhancedAIService.rs');
  const start = enhanced.indexOf('let pureThinkingWarning =');
  const end = enhanced.indexOf('let finalContent =', start);
  const warning = enhanced.slice(start, end);
  assert.match(warning, /ResponseRoundAction::Continue\(ToolResultContinuation\s*\{\s*results: Vec::new\(\),\s*message: pureThinkingWarning,/u);
  assert.doesNotMatch(warning, /handleToolInvocation|conversationHistory\.push/u);
  const dispatch = enhanced.slice(enhanced.indexOf('async fn handleToolInvocation('));
  assert.match(dispatch, /if toolInvocations\.is_empty\(\) \{\s+return Err/u);
  assert.doesNotMatch(enhanced, /chat\.response\.raw/u);
});

/** Extracts a service method body at its implementation-level closing brace. */
function serviceMethod(enhanced, name) {
  const start = enhanced.indexOf(`fn ${name}(`);
  const end = enhanced.indexOf('\n    }', start);
  assert.ok(start >= 0 && end > start, `missing method ${name}`);
  return enhanced.slice(start, end);
}

/** Any call cycle in the round driver would restore the native stack overflow. */
test('tool and warning continuations use an iterative driver with an acyclic call graph', () => {
  const enhanced = source(providers + 'EnhancedAIService.rs');
  const methods = new Map([
    'processStreamCompletion',
    'completeResponseRound',
    'handleToolInvocation',
    'processToolResults',
  ].map(name => [name, serviceMethod(enhanced, name)]));
  const edges = new Map([...methods].map(([name, body]) => [
    name,
    [...body.matchAll(/self\s*\.\s*(\w+)\s*\(/gu)]
      .map(match => match[1])
      .filter(callee => methods.has(callee)),
  ]));
  assert.deepEqual(new Set(edges.get('processStreamCompletion')), new Set([
    'completeResponseRound', 'handleToolInvocation', 'processToolResults',
  ]));
  assert.match(methods.get('processStreamCompletion'), /loop \{\s+if !self\.isExecutionContextActive\(context\)/u);
  for (const start of methods.keys()) {
    const pending = [...edges.get(start)];
    const visited = new Set();
    while (pending.length > 0) {
      const current = pending.pop();
      assert.notEqual(current, start, `recursive response round through ${start}`);
      if (!visited.has(current)) {
        visited.add(current);
        pending.push(...edges.get(current));
      }
    }
  }
});

/** Cancellation and route handoff must stop the driver before another model request. */
test('cancelled and rerouted tool batches end the iterative continuation', () => {
  const enhanced = source(providers + 'EnhancedAIService.rs');
  const driver = serviceMethod(enhanced, 'processStreamCompletion');
  const dispatch = serviceMethod(enhanced, 'handleToolInvocation');
  assert.match(driver, /let Some\(continuation\) = self[\s\S]*?\.handleToolInvocation\([\s\S]*?\.await\?\s+else \{\s+return Ok\(\(\)\);/u);
  assert.match(dispatch, /if !self\.isExecutionContextActive\(context\) \{\s+return Ok\(None\);/u);
  const routePause = dispatch.indexOf('if let Some(targetNodeId) = routeChangeTargetNodeId');
  const continuation = dispatch.indexOf('Ok(Some(ToolResultContinuation');
  assert.ok(routePause >= 0 && continuation > routePause);
  assert.match(dispatch.slice(routePause, continuation), /return Ok\(None\);/u);
  assert.match(dispatch, /pendingRouteChange = Some\(routeChangeIntent\)/u);
  const request = serviceMethod(enhanced, 'processToolResults');
  assert.match(request, /self\.invalidateExecutionContext\(\s+context,\s+"processToolResults\.tokenLimit"/u);
});

/** Both asynchronous OpenAI request paths must preserve the failure before channel closure. */
test('streaming and non-streaming request failures reach the existing terminal error channel', () => {
  const openai = source(providers + 'llmprovider/OpenAIProvider.rs');
  const start = openai.indexOf('pub async fn send_prepared_request(');
  const end = openai.indexOf('async fn sendNonStreamingPreparedRequest(', start);
  const prepared = openai.slice(start, end);
  assert.equal([...prepared.matchAll(/finish_response_stream\(&producer, result\)/gu)].length, 2);
  assert.doesNotMatch(prepared, /if let Ok\(chunks\)|readResponseText\(response\)\.await\.unwrap_or_default/u);
  assert.match(openai, /Err\(error\) => output\.set_terminal_failure\(error\.to_string\(\)\)/u);
  assert.match(openai, /Ok\(\(\)\) \| Err\(AiServiceError::RequestCancelled\) => \{\}/u);
  const stream = source('core/crates/foundation/util/src/stream/RevisableTextStream.rs');
  assert.match(stream, /fn terminal_failure\(&self\) -> Option<String>;/u);
  assert.match(stream, /\(\*\*self\)\.terminal_failure\(\)/u);
});

/** Failed responses must not persist successful usage or execute model-generated tools. */
test('both response rounds check producer completion before normal completion processing', () => {
  const enhanced = source(providers + 'EnhancedAIService.rs');
  const initial = enhanced.indexOf('ensure_response_completed(provider_stream.as_ref())');
  const usage = enhanced.indexOf('lifecycle.push(SendMessageLifecycleStage::PersistTokenUsage)');
  const continuation = enhanced.indexOf('ensure_response_completed(response.as_ref())?');
  const completed = enhanced.indexOf('chat.model_response.after_tool_result_complete');
  assert.ok(initial >= 0 && initial < usage);
  assert.ok(continuation >= 0 && continuation < completed);
  const service = source(providers + 'llmprovider/AIService.rs');
  assert.match(service, /match stream\.terminal_failure\(\)/u);
  assert.match(service, /Some\(error\) => Err\(AiServiceError::RequestFailed\(error\)\)/u);
});
