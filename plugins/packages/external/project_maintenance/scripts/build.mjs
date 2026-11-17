import { createRequire } from 'node:module';
import { readdir, readFile, writeFile, mkdir, rm } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
const root = new URL('../', import.meta.url);
const localRequire = createRequire(new URL('../package.json', import.meta.url));
const {zipSync, unzipSync} = localRequire('fflate');
const {build} = localRequire('esbuild');
const tsc = localRequire.resolve('typescript/bin/tsc');
for (const config of ['tsconfig.json', 'tsconfig.web.json']) {
  const check = spawnSync(process.execPath, [tsc, '-p', fileURLToPath(new URL(config, root)), '--noEmit'], {stdio:'inherit'});
  if (check.status !== 0) process.exit(check.status ?? 1);
}
const web = await build({entryPoints:[fileURLToPath(new URL('web/app.tsx',root))], bundle:true, minify:true, write:false, outfile:'control.js', format:'iife', target:['es2020'], jsx:'automatic', legalComments:'none', define:{'process.env.NODE_ENV':'"production"'}});
const js=web.outputFiles.find(f=>f.path.endsWith('.js')).text;
const css=web.outputFiles.find(f=>f.path.endsWith('.css')).text;
const html=`<!doctype html><html lang="zh-CN"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><meta http-equiv="Content-Security-Policy" content="default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data:; connect-src 'none'; base-uri 'none'; form-action 'none'"><meta name="color-scheme" content="light"><title>Operit 维护中控</title><style>${css.replaceAll('</style','<\\/style')}</style></head><body><div id="root"></div><script>${js.replaceAll('</script','<\\/script')}</script></body></html>`;
await mkdir(new URL('resources/',root),{recursive:true});
await writeFile(new URL('resources/control.html',root),html);

await rm(new URL('dist/', root), {recursive:true, force:true});
const compile = spawnSync(process.execPath, [tsc, '-p', fileURLToPath(new URL('tsconfig.json', root)), '--noEmit', 'false'], {stdio:'inherit'});
if (compile.status !== 0) process.exit(compile.status ?? 1);
const files = { 'manifest.json': new Uint8Array(await readFile(new URL('manifest.json', root))) };
async function add(dir) {
 for (const item of await readdir(new URL(dir+'/',root),{withFileTypes:true})) {
  const name=dir+'/'+item.name;
  if(item.isDirectory()) await add(name);
  else if(/\.(js|mjs|ts|tsx|css)$/.test(name)) files[name]=new Uint8Array(await readFile(new URL(name,root)));
 }
}
await add('dist'); await add('src'); await add('web'); await add('scripts');
for (const name of ['package.json', 'package-lock.json', 'pnpm-lock.yaml', 'tsconfig.json', 'tsconfig.web.json', 'README.md', 'DEMO.md']) {
  files[name] = new Uint8Array(await readFile(new URL(name,root)));
}
const notices = [];
for (const dependency of ['react', 'react-dom', 'scheduler']) {
  const file = fileURLToPath(new URL(`node_modules/${dependency}/LICENSE`, root));
  notices.push(`${dependency}\n${await readFile(file,'utf8')}`);
}
await writeFile(new URL('resources/THIRD_PARTY_NOTICES.txt',root), notices.join('\n\n'));
files['resources/THIRD_PARTY_NOTICES.txt'] = new Uint8Array(await readFile(new URL('resources/THIRD_PARTY_NOTICES.txt',root)));
files['resources/control.html'] = new Uint8Array(await readFile(new URL('resources/control.html',root)));
const manifest=JSON.parse(Buffer.from(files['manifest.json']));
for(const entry of [manifest.main,...manifest.subpackages.map(x=>x.entry)]) if(!files[entry]) throw new Error('Missing '+entry);
const archive=zipSync(files,{level:6,mtime:new Date(1980,0,1)});
const decoded=unzipSync(archive); for(const [name,value] of Object.entries(files)) if(!Buffer.from(decoded[name]).equals(Buffer.from(value))) throw new Error('Archive mismatch '+name);
await writeFile(new URL('dist/project_maintenance.toolpkg',root),archive);
console.log(`Packed ${Object.keys(files).length} files into project_maintenance.toolpkg`);
