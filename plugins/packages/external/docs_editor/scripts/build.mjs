import { build } from 'esbuild';
import { zipSync } from 'fflate';
import { cp, mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const dist = join(root, 'dist');
await mkdir(join(dist, 'web'), { recursive: true });
const shared = { absWorkingDir: root, bundle: true, charset: 'utf8', legalComments: 'linked' };
await build({ ...shared, entryPoints: ['src/main.ts', 'src/screen.ui.ts'], outdir: dist,
  external: ['./screen.ui.js'],
  platform: 'neutral', format: 'cjs', target: 'es2020', legalComments: 'inline' });
const metadata = (await readFile(join(root, 'src/tools.ts'), 'utf8')).match(/\/\* METADATA\s*([\s\S]*?)\*\//);
if (!metadata) throw new Error('Missing Office tool metadata');
JSON.parse(metadata[1]);
await build({ ...shared, entryPoints: ['src/tools.ts'], outdir: dist,
  platform: 'neutral', format: 'cjs', target: 'es2020', banner: { js: metadata[0] }, legalComments: 'inline' });
await build({ ...shared, entryPoints: ['web/app.ts'], outfile: join(dist, 'web/app.js'),
  platform: 'browser', format: 'iife', target: 'es2022', minify: true,
  loader: { '.svg': 'dataurl', '.woff': 'dataurl', '.woff2': 'dataurl', '.ttf': 'dataurl', '.png': 'dataurl' },
  banner: { js: 'var officeBundleUrl = document.currentScript.src;' },
  define: { 'process.env.NODE_ENV': '"production"', 'import.meta.url': 'officeBundleUrl' },
  plugins: [{ name: 'disable-superdoc-optional-pdf', setup(builder) {
    builder.onResolve({ filter: /^pdfjs-dist\/build\/pdf\.mjs$/ }, () => ({ path: 'pdf', namespace: 'disabled' }));
    builder.onLoad({ filter: /.*/, namespace: 'disabled' }, () => ({ contents: 'throw new Error("Use the bundled PDF editor instead");' }));
  } }],
});
await cp(join(root, 'web/index.html'), join(dist, 'web/index.html'));
await cp(join(root, 'node_modules/@embedpdf/pdfium/dist/pdfium.wasm'), join(dist, 'web/pdfium.wasm'));

const entries = Object.create(null);
async function include(relative) {
  const path = join(root, relative);
  for (const entry of await readdir(path, { withFileTypes: true })) {
    const child = `${relative}/${entry.name}`;
    if (entry.isDirectory()) await include(child);
    else if (entry.isFile()) entries[child] = await readFile(join(root, child));
  }
}
for (const name of ['manifest.json', 'package.json', 'pnpm-lock.yaml', 'tsconfig.json', 'README.md', 'THIRD_PARTY_NOTICES.md', 'dist/main.js', 'dist/screen.ui.js', 'dist/tools.js']) entries[name] = await readFile(join(root, name));
for (const directory of ['src', 'web', 'scripts', 'dist/web']) await include(directory);
for (const [name, directory, files] of [
  ['superdoc', 'superdoc', ['LICENSE']],
  ['pptx-editor', '@office-kit/pptx-editor', ['LICENSE']],
  ['embedpdf', '@embedpdf/snippet', ['LICENSE']],
  ['pdfium', '@embedpdf/pdfium', ['LICENSE', 'LICENSE.pdfium']],
  ['lucide', 'lucide', ['LICENSE']],
]) {
  for (const file of files) entries[`licenses/${name}/${file}`] = await readFile(join(root, 'node_modules', directory, file));
}
for (const asset of JSON.parse(entries['manifest.json']).resources) {
  if (!entries[asset.path]) throw new Error(`Missing ${asset.path}`);
}
const archive = zipSync(entries, { level: 6, mtime: new Date(1980, 0, 1) });
await writeFile(join(dist, 'docs_editor.toolpkg'), archive);
const destination = resolve(root, '../../../../core/crates/runtime/application/assets/plugins/external/docs_editor.toolpkg');
await mkdir(resolve(destination, '..'), { recursive: true });
await writeFile(destination, archive);
console.log(`Packed Office documents (${archive.length} bytes)`);
