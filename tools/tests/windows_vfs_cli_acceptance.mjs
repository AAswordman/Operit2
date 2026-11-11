import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createServer } from 'node:http';
import { mkdtemp, mkdir, readFile, writeFile, copyFile, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

const binary = resolve(process.argv[2] ?? 'apps/cli/target/debug/operit2.exe');
const root = await mkdtemp(join(tmpdir(), 'operit-vfs-cli-'));
const cwd = join(root, '中文 input space');
await mkdir(cwd, { recursive: true });
await mkdir(join(root, 'config'));
await writeFile(join(root, 'config/storage.json'), JSON.stringify({
  runtimeRoot: join(root, 'runtime'), workspaceRoot: join(root, 'workspaces'),
  activeIdentityId: 'vfs-test', identities: [{ id: 'vfs-test', name: 'VFS acceptance', createdAt: 1700000000000 }],
}));
const env = { ...process.env, OPERIT_CLI_CONFIG_DIR: join(root, 'config'), OPERIT_CLI_SECRET_STORE: 'file' };
const results = [];

/** Verifies portable file metadata without interpreting user text or settings display paths. */
function validateVirtualMetadata(value) {
  if (!value || typeof value !== 'object') return;
  for (const [key, child] of Object.entries(value)) {
    if (/(path|directory|dir|root)$/i.test(key) && key !== 'displayPath'
        && typeof child === 'string' && child.startsWith('/')) {
      assert.ok(!child.includes('\\'), `${key} contains a native separator: ${child}`);
    }
    validateVirtualMetadata(child);
  }
}

/** Runs the built Windows executable with an isolated identity and explicit working directory. */
async function command(label, args, expectSuccess = true) {
  const output = await new Promise((done, reject) => {
    const child = spawn(binary, ['cli', ...args, '--json'], { cwd, env, windowsHide: true });
    let stdout = '', stderr = '';
    child.stdout.on('data', data => { stdout += data; });
    child.stderr.on('data', data => { stderr += data; });
    child.on('error', reject);
    const timer = setTimeout(() => { child.kill(); reject(new Error(`${label}: command timed out`)); }, 45000);
    child.on('close', code => { clearTimeout(timer); done({ code, stdout, stderr }); });
  });
  let json;
  try { json = JSON.parse(output.stdout.trim()); }
  catch { throw new Error(`${label}: invalid JSON output: ${output.stdout.slice(0, 800)} ${output.stderr}`); }
  results.push({ label, args, ...output, json });
  await writeFile(join(root, 'results.json'), JSON.stringify(results, null, 2));
  assert.equal(output.code === 0, expectSuccess, `${label}: ${output.stdout} ${output.stderr}`);
  if (expectSuccess) validateVirtualMetadata(json);
  console.log(`PASS ${label}`);
  return json;
}

/** Executes one file tool through the public CLI invocation pipeline. */
async function tool(label, name, parameters, expectSuccess = true) {
  const result = await command(label, ['tool', 'exec', name, JSON.stringify(parameters)], expectSuccess);
  if (expectSuccess) assert.equal(result.success, true, label);
  return result;
}

const payload = 'VFS 中文内容 123\nsecond line';
const virtualRoot = '/operitapp/data/验收 空格';
const physicalRoot = join(root, 'runtime/identities/vfs-test/验收 空格');
const server = createServer((request, response) => {
  response.writeHead(200, { 'content-type': 'application/octet-stream' });
  response.end(payload);
});
await new Promise(done => server.listen(0, '127.0.0.1', done));
try {
  const locations = await command('storage virtual roots', ['storage', 'paths']);
  assert.equal(locations.runtimeRoot, '/operitapp/data');
  assert.equal(locations.workspaceRoot, '/operitapp/workspaces');
  const dataLocation = locations.locations.find(item => item.vfsPath === '/operitapp/data');
  assert.equal(resolve(dataLocation.displayPath), resolve(root, 'runtime/identities/vfs-test'));
  assert.ok(dataLocation.locationVfsPath.startsWith('/virtual/windows/'));
  await command('local model virtual roots', ['local-models', 'paths']);
  const catalog = await command('native chat model catalog remains available', ['local-models', 'catalog']);
  assert.ok(catalog.some(item => item.manifest.id === 'qwen3-0.6b-gguf'));
  await tool('write virtual Unicode path', 'write_file', { path: `${virtualRoot}/a.txt`, content: payload });
  assert.equal(await readFile(join(physicalRoot, 'a.txt'), 'utf8'), payload);
  await tool('storage alias reads the same file', 'read_file_full', { path: `${dataLocation.locationVfsPath}/验收 空格/a.txt` });
  await tool('write relative to CLI cwd', 'write_file', { path: '相对 文件.txt', content: payload });
  assert.equal(await readFile(join(cwd, '相对 文件.txt'), 'utf8'), payload);
  await tool('read relative', 'read_file_full', { path: '相对 文件.txt' });
  await tool('read virtual', 'read_file_full', { path: `${virtualRoot}/a.txt` });
  await tool('read native ingress', 'read_file_full', { path: join(cwd, '相对 文件.txt') });
  await tool('list virtual', 'list_files', { path: virtualRoot });
  await tool('file metadata', 'file_info', { path: `${virtualRoot}/a.txt` });
  await tool('copy virtual to relative', 'copy_file', { source: `${virtualRoot}/a.txt`, destination: 'copy.txt' });
  assert.equal(await readFile(join(cwd, 'copy.txt'), 'utf8'), payload);
  await tool('move relative to virtual', 'move_file', { source: 'copy.txt', destination: `${virtualRoot}/moved.txt` });
  assert.equal(await readFile(join(physicalRoot, 'moved.txt'), 'utf8'), payload);
  await tool('compress virtual source', 'zip_files', { source: virtualRoot, destination: 'archive.zip' });
  await tool('extract into virtual destination', 'unzip_files', { source: 'archive.zip', destination: `${virtualRoot}/unpacked` });
  await tool('download relative', 'download_file', { url: `http://127.0.0.1:${server.address().port}/file`, destination: '下载 文件.bin' });
  assert.equal(await readFile(join(cwd, '下载 文件.bin'), 'utf8'), payload);
  await tool('download virtual', 'download_file', { url: `http://127.0.0.1:${server.address().port}/file`, destination: `${virtualRoot}/download.bin` });
  assert.equal(await readFile(join(physicalRoot, 'download.bin'), 'utf8'), payload);
  await tool('unknown node rejected', 'read_file_full', { path: 'missing-vfs-node:/operitapp/data/a.txt' }, false);
  await tool('unresolved relative installer rejected', 'install_app', { path: 'missing-installer.apk' }, false);
  await tool('missing open file rejected', 'open_file', { path: 'missing-open.txt' }, false);
  await tool('missing shared file rejected', 'share_file', { path: 'missing-share.txt' }, false);
  await command('workspace create', ['workspace', 'create-default', 'vfs-workspace']);
  await command('workspace resolve', ['workspace', 'resolve-path', '/operitapp/workspaces/vfs-workspace']);
  const execution = await command('workspace native consumer receives cwd', ['workspace', 'exec', '/operitapp/workspaces/vfs-workspace', 'cmd', '/d', '/c', 'cd']);
  assert.equal(execution.exitCode, 0);
  assert.ok(JSON.stringify(execution).includes('vfs-workspace'));
  const forwarded = await command('program cwd argument is preserved', ['workspace', 'exec', '/operitapp/workspaces/vfs-workspace', 'cmd', '/d', '/c', 'echo', '--cwd', 'program-argument']);
  assert.equal(forwarded.exitCode, 0);
  assert.ok(forwarded.stdout.includes('--cwd program-argument'));
  await writeFile(join(cwd, 'skill-body.md'), '# CLI VFS fixture\nUse the virtual filesystem.');
  await command('skill content from relative file', ['skill', 'create', 'vfs-fixture', 'VFS fixture', '@skill-body.md', '相对 文件.txt']);
  await command('skill virtual directory', ['skill', 'dir']);
  await command('skill read', ['skill', 'show', 'vfs-fixture']);
  await mkdir(join(cwd, 'plugin source'));
  await writeFile(join(cwd, 'plugin source/manifest.json'), JSON.stringify({ schema_version: 1, toolpkg_id: 'vfs_fixture', api_version: '2.0.0', main: 'main.js' }));
  await writeFile(join(cwd, 'plugin source/main.js'), 'exports.registerToolPkg = function() { return true; };');
  await tool('package archive fixture', 'zip_files', { source: 'plugin source', destination: 'plugin.zip' });
  await copyFile(join(cwd, 'plugin.zip'), join(cwd, 'fixture.toolpkg'));
  await command('plugin import relative archive', ['plugin', 'import', 'fixture.toolpkg']);
  await command('plugin inspect virtual installation', ['plugin', 'show', 'vfs_fixture']);
  await command('plugin catalog reload', ['plugin', 'list']);
  await command('plugin remove', ['plugin', 'delete', 'vfs_fixture']);
  await tool('skill archive fixture', 'zip_files', { source: '/operitapp/data/extensions/device/skills/vfs-fixture', destination: 'skill.zip' });
  await command('skill remove', ['skill', 'delete', 'vfs-fixture']);
  await command('skill import relative archive', ['skill', 'import-zip', 'skill.zip']);
  await command('skill read after archive import', ['skill', 'show', 'vfs-fixture']);
  await command('imported skill remove', ['skill', 'delete', 'vfs-fixture']);
  await mkdir(join(cwd, 'mcp source'));
  await writeFile(join(cwd, 'mcp source/package.json'), JSON.stringify({ name: 'vfs-mcp-fixture', version: '1.0.0', main: 'server.js' }));
  await writeFile(join(cwd, 'mcp source/server.js'), 'process.stdin.resume();');
  await writeFile(join(cwd, 'mcp-config.json'), JSON.stringify({ mcpServers: { vfs_mcp: { command: 'node', args: ['mcp source/server.js'], disabled: true, env: {} } } }));
  await tool('MCP archive fixture', 'zip_files', { source: 'mcp source', destination: 'mcp.zip' });
  await command('MCP install relative archive and config', ['mcp', 'install-zip', 'vfs_mcp', 'mcp.zip', 'VFS MCP', 'VFS fixture', 'test', '1', '@mcp-config.json']);
  await command('MCP replace existing installation', ['mcp', 'install-zip', 'vfs_mcp', 'mcp.zip', 'VFS MCP', 'VFS fixture', 'test', '2', '@mcp-config.json']);
  await command('MCP virtual configuration path', ['mcp', 'dir']);
  await command('MCP persisted installation metadata', ['mcp', 'meta', 'vfs_mcp']);
  await command('MCP remove', ['mcp', 'remove', 'vfs_mcp']);
  await tool('delete relative', 'delete_file', { path: '相对 文件.txt' });
  await tool('delete virtual subtree', 'delete_file', { path: virtualRoot, recursive: true });
  await assert.rejects(stat(physicalRoot), { code: 'ENOENT' });
  await tool('storage migration marker', 'write_file', { path: '/operitapp/data/migration-marker.txt', content: payload });
  const configBeforeFailure = await readFile(join(root, 'config/storage.json'), 'utf8');
  await command('storage migration rejects an alias of its child', ['storage', 'migrate', '--runtime', `${dataLocation.locationVfsPath}/nested`, '--workspace', join(root, 'unused-workspaces')], false);
  assert.equal(await readFile(join(root, 'config/storage.json'), 'utf8'), configBeforeFailure);
  const selectedRuntime = join(root, 'selected storage/runtime');
  const selectedWorkspaces = join(root, 'selected storage/workspaces');
  await command('storage migration to user-selected disk locations', ['storage', 'migrate', '--runtime', selectedRuntime, '--workspace', selectedWorkspaces]);
  const movedLocations = await command('storage locations after restart', ['storage', 'paths']);
  const movedData = movedLocations.locations.find(item => item.vfsPath === '/operitapp/data');
  assert.equal(resolve(movedData.displayPath).toLowerCase(), resolve(selectedRuntime, 'identities/vfs-test').toLowerCase());
  assert.equal(await readFile(join(selectedRuntime, 'identities/vfs-test/migration-marker.txt'), 'utf8'), payload);
  await tool('stable alias after storage relocation', 'read_file_full', { path: '/operitapp/data/migration-marker.txt' });
  await tool('new mount location after storage relocation', 'read_file_full', { path: `${movedData.locationVfsPath}/migration-marker.txt` });
  console.log(`Verified ${results.length} commands. Evidence: ${join(root, 'results.json')}`);
} finally {
  server.close();
  console.log(`Isolated test directory: ${root}`);
}
