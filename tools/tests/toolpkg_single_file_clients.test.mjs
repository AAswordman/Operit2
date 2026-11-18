import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {mkdtempSync, readFileSync, writeFileSync, rmSync} from 'node:fs';
import {tmpdir} from 'node:os';
import path from 'node:path';
import vm from 'node:vm';

const require = createRequire(import.meta.url);
const ts = require('../../plugins/packages/buildin/workflow/node_modules/typescript');
const packages = [
  'buildin/workflow', 'buildin/character_cards', 'buildin/goal_mode',
  'external/subagents', 'external/waifu', 'external/qqbot', 'external/wechat',
  'external/feishu', 'external/connector_hub',
  'external/music_studio', 'external/project_maintenance',
];
const packageRoot = path.resolve('plugins/packages');
const sdk = path.resolve('plugins/types/index.d.ts');

for (const name of packages) {
  test(`${name}: the manifest client compiles after copying only that file`, () => {
    const manifest = JSON.parse(readFileSync(path.join(packageRoot, name, 'manifest.json'), 'utf8'));
    const source = readFileSync(path.join(packageRoot, name, manifest.public_api), 'utf8');
    const ast = ts.createSourceFile('api.ts', source, ts.ScriptTarget.Latest, true);
    const references = [];
    function visit(node) {
      if (ts.isImportDeclaration(node) || ts.isImportEqualsDeclaration(node) || ts.isImportTypeNode(node)
        || (ts.isExportDeclaration(node) && node.moduleSpecifier)
        || (ts.isCallExpression(node) && (node.expression.kind === ts.SyntaxKind.ImportKeyword || node.expression.getText(ast) === 'require'))) {
        references.push(node.getText(ast));
      }
      ts.forEachChild(node, visit);
    }
    visit(ast);
    assert.deepEqual(references, [], 'A published client must not import any sibling, implementation, or npm file');
    assert.equal(ast.referencedFiles.length, 0, 'Triple slash file dependencies also break a copied client');
    const dir = mkdtempSync(path.join(tmpdir(), 'operit-copied-client-'));
    try {
      const client = path.join(dir, 'api.ts');
      writeFileSync(client, source);
      const roots = [client, sdk, path.resolve('plugins/types/quickjs-runtime.d.ts')];
      let fixture;
      if (name === 'external/qqbot' || name === 'external/wechat') {
        fixture = `import {request, peek, type ChannelMessage, type ChannelRequest, type ChannelStatus, type ActiveContext} from './api';
          const messages: Promise<{messages: ChannelMessage[]}> = peek(20, 'conversation');
          const status: Promise<ChannelStatus> = request({action:'status'});
          const context: Promise<ActiveContext | null> = request({action:'active_context', chatId:'chat'});
          const ack: Promise<{acknowledged:boolean}> = request({action:'ack', key:'key'});
          const union: ChannelRequest = Math.random() > 0.5 ? {action:'status'} : {action:'peek'};
          const unionResult: Promise<ChannelStatus | {messages:ChannelMessage[]}> = request(union);
          // @ts-expect-error Invalid action.
          request({action:'private'});
          // @ts-expect-error Identifier must be text.
          request({action:'ack', key:1});
          // @ts-expect-error A status response is not a message array.
          const wrong: Promise<ChannelMessage[]> = request({action:'status'});
          // @ts-expect-error Consumers cannot select an arbitrary response type.
          request<string>({action:'status'});
        `;
      } else if (name === 'external/connector_hub') {
        fixture = `import {request, type Dashboard, type History} from './api';
          const dashboard: Promise<Dashboard> = request({action:'dashboard'});
          const history: Promise<History> = request({action:'history',channel:'feishu',conversationKey:'conversation'});
          // @ts-expect-error Attachment export is console-only.
          request({action:'attachment',channel:'feishu',conversationKey:'conversation',messageId:'id',index:0});
          // @ts-expect-error Credential management is console-only.
          request({action:'manage',channel:'feishu',command:'connect',appId:'id',appSecret:'secret'});
        `;
      } else if (name === 'external/project_maintenance') {
        fixture = `import {request, type Dashboard, type Ticket, type Inspection, type Request, type ChannelReport, type Note, type Stage, type AvailableChat, type CapturedMessage} from './api';
          const dashboard: Promise<Dashboard> = request({action:'dashboard'});
          const ticket: Promise<Ticket> = request({action:'get', id:'ticket'});
          const inspection: Promise<Inspection> = request({action:'inspect', id:'ticket'});
          const channels: Promise<ChannelReport[]> = request({action:'dashboard'}).then(value=>value.channels);
          const prepared: Promise<{chatId:string}> = request({action:'prepare_channel_chat',channel:'qq',conversationKey:'canonical'});
          // @ts-expect-error Invalid channel.
          request({action:'prepare_channel_chat', channel:'other', conversationKey:'canonical'});
          // @ts-expect-error Missing ticket identifier.
          request({action:'start'});
          // @ts-expect-error Response is inferred from the action.
          const wrong: Promise<Inspection> = request({action:'get', id:'ticket'});
        `;
      }
      if (fixture) { const file = path.join(dir, 'consumer.ts'); writeFileSync(file, fixture); roots.push(file); }
      const program = ts.createProgram(roots, {target:ts.ScriptTarget.ES2020, module:ts.ModuleKind.CommonJS,
        strict:true, noEmit:true, skipLibCheck:true, lib:['lib.es2020.d.ts'], types:[]});
      assert.deepEqual(ts.getPreEmitDiagnostics(program).map(d => ts.flattenDiagnosticMessageText(d.messageText, '\n')), []);
      assert.equal(program.getSourceFiles().some(f => f.fileName.startsWith(packageRoot) && !f.fileName.includes('/node_modules/')), false,
        'The isolated client must never resolve provider source files');
    } finally { rmSync(dir, {recursive:true, force:true}); }
  });
}

for (const [name, provider, method] of [
  ['external/qqbot', 'com.operit.qqbot_bundle', 'channel.v1'],
  ['external/wechat', 'com.operit.wechat_bot', 'channel.v1'],
  ['external/feishu', 'com.operit.feishu_bot', 'channel.v1'],
  ['external/connector_hub', 'com.operit.connector_hub', 'connectors.v1'],
  ['external/project_maintenance', 'com.operit.project_maintenance', 'maintenance.request'],
]) {
  test(`${name}: copied wrapper routes once and preserves transport failures`, async () => {
    const manifest = JSON.parse(readFileSync(path.join(packageRoot, name, 'manifest.json'), 'utf8'));
    const source = readFileSync(path.join(packageRoot, name, manifest.public_api), 'utf8');
    const calls = [], failure = new Error('provider unavailable');
    let fail = false;
    const context = vm.createContext({exports:{}, ToolPkg:{callDependency(...args) {
      calls.push(args); return fail ? Promise.reject(failure) : Promise.resolve({sentinel:true});
    }}});
    vm.runInContext(ts.transpileModule(source, {compilerOptions:{module:ts.ModuleKind.CommonJS, target:ts.ScriptTarget.ES2020}}).outputText, context);
    const payload = {action: method === 'channel.v1' ? 'status' : 'dashboard'};
    assert.deepEqual(await context.exports.request(payload), {sentinel:true});
    assert.deepEqual(calls, [[provider, method, payload]]);
    fail = true;
    await assert.rejects(context.exports.request(payload), error => error === failure);
    assert.equal(calls.length, 2);
  });
}

test('maintenance copies the exact published connector client', () => {
  assert.equal(readFileSync(path.join(packageRoot, 'external/project_maintenance/src/dependencies/connector_hub/api.ts'), 'utf8'),
    readFileSync(path.join(packageRoot, 'external/connector_hub/src/api.ts'), 'utf8'));
});
