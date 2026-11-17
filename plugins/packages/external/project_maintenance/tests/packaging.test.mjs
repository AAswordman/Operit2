import test from 'node:test';
import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {readFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import path from 'node:path';
const require = createRequire(import.meta.url);
const packageRoot = fileURLToPath(new URL('../../../', import.meta.url));

test('the maintenance archive includes the exact self-contained client and Bot copies', () => {
  const {unzipSync} = require('../../../buildin/workflow/node_modules/fflate');
  const archive = unzipSync(readFileSync(path.join(packageRoot, 'external/project_maintenance/dist/project_maintenance.toolpkg')));
  const manifest = JSON.parse(Buffer.from(archive['manifest.json']).toString());
  for (const file of [manifest.public_api, 'src/dependencies/qqbot/api.ts', 'src/dependencies/wechat/api.ts']) {
    assert.ok(archive[file], `Missing archive source ${file}`);
    assert.equal(Buffer.from(archive[file]).toString(), readFileSync(path.join(packageRoot, 'external/project_maintenance', file), 'utf8'));
  }
  assert.equal(Object.keys(archive).some(file => /maintenance_(qq|wechat)_bridge/.test(file)), false);
});


test('external maintenance participates in the standard pnpm packaging pipeline', () => {
  const pkg = JSON.parse(readFileSync(new URL('../package.json', import.meta.url), 'utf8'));
  assert.equal(pkg.scripts['pack:toolpkg'], 'node scripts/build.mjs');
  assert.equal(pkg.packageManager, 'pnpm@10.7.0');
  assert.match(readFileSync(new URL('../pnpm-lock.yaml', import.meta.url), 'utf8'), /lockfileVersion:/);
});
