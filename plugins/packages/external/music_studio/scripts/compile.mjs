import { readdir, rm, access } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { createRequire } from 'node:module';
// tsc does not remove outputs for deleted templates. Prune those before packaging.
async function prune(dir) {
  for (const entry of await readdir(dir, { withFileTypes: true }).catch(e => { if (e.code === 'ENOENT') return []; throw e; })) {
    const path = `${dir}/${entry.name}`;
    if (entry.isDirectory()) await prune(path);
    else if (entry.name.endsWith('.js')) {
      try { await access(path.replace(/^dist\//, 'src/').replace(/\.js$/, '.ts')); }
      catch (e) { if (e.code !== 'ENOENT') throw e; await rm(path); }
    }
  }
}
await prune('dist');
const require = createRequire(import.meta.url);
const result = spawnSync(process.execPath, [require.resolve('typescript/bin/tsc'), '-p', 'tsconfig.json'], { stdio: 'inherit' });
if (result.error) throw result.error;
process.exitCode = result.status ?? 1;
