import assert from 'node:assert/strict';
import vm from 'node:vm';
import { fileURLToPath } from 'node:url';
import { build } from 'esbuild';
import { readSdkScript, wrapComposeScript } from '../../../../../tools/runtime/compose_scripts.mjs';
import { composeStreamFixture } from '../../../../../tools/tests/support/compose_stream_fixture.mjs';

/** Bundles the actual plugin source without writing generated files into the package. */
export async function bundle(path) {
  const result = await build({ entryPoints: [fileURLToPath(new URL(path, import.meta.url))],
    bundle: true, write: false, platform: 'neutral', format: 'cjs', target: 'es2020' });
  return result.outputFiles[0].text;
}

/** Runs a bundled plugin module against the real Compose compiler and stream protocol. */
export async function screen(path, state, options = {}) {
  const errors = [], toasts = [];
  const context = vm.createContext({
    console: { ...console, error: error => { errors.push(String(error)); } },
    module: { exports: {} }, Tools: options.tools,
    toolCall: async (name, args) => { assert.equal(name, 'toast'); toasts.push(args.message); },
  });
  vm.runInContext(readSdkScript('ToolPkgComposeDslBridge.rs'), context);
  vm.runInContext(wrapComposeScript(await bundle(path)), context);
  const fixture = composeStreamFixture(context);
  fixture.adapt();
  await context.__operit_render_compose_dsl({ executionContextKey: 'review-test', theme: options.theme ?? { brightness: 'light', colors: {} }, state });
  return {
    context, fixture, errors, toasts,
    /** Reads all currently mounted nodes, including explicit named content slots. */
    nodes() {
      /** Walks the committed host tree, not the plugin's internal state. */
      function walk(node) {
        return [node, ...node.children.flatMap(walk), ...Object.values(node.slots).flatMap(children => children.flatMap(walk))];
      }
      return walk(fixture.latest.tree);
    },
    /** Invokes a live callback using the action address received by the host. */
    async click(node) {
      assert.ok(node.props.onClick?.__actionId, `Not clickable: ${node.type}`);
      return context.__operit_dispatch_compose_dsl_action({ actionId: node.props.onClick.__actionId });
    },
    /** Updates semantic host input without remounting local plugin state. */
    async update(input) {
      return context.__operit_render_compose_dsl({ executionContextKey: 'review-test', __operit_update_inputs: true, __operit_input_state: input });
    },
    /** Commits asynchronous effect state using the host's normal next-render entry point. */
    async settle() {
      for (let index = 0; index < 4; index++) {
        await new Promise(resolve => setImmediate(resolve));
        await context.__operit_render_compose_dsl({ executionContextKey: 'review-test' });
      }
    },
  };
}

/** Constructs the exact semantic tool-result contract consumed by the production plugin. */
export function toolResult(path, diffContent) {
  return { kind: 'tool_result', content: '', attributes: {
    display: JSON.stringify({ resultRenderer: 'fileDiff', label: { zh: '编辑文件', en: 'Edit file' } }),
    result_data: JSON.stringify({ operation: { path }, diffContent }),
  } };
}

/** Keeps selected answer revisions explicit in every test input. */
export function message(parts = [toolResult('src/a.ts', '@@ -1 +1 @@\n-old\n+new')]) {
  return { timestamp: 123, variantIndex: 1, variantCount: 2, sender: 'ai', parts };
}
