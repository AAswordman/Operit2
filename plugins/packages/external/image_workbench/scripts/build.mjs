import { build } from 'esbuild';
import { zipSync } from 'fflate';
import { readFile, readdir, mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = fileURLToPath(new URL('../', import.meta.url));

/** Enumerates editable package sources without including dependencies or previous artifacts. */
async function sources(directory) {
  const files = [];
  for (const entry of await readdir(path.join(root, directory), { withFileTypes: true })) {
    const name = `${directory}/${entry.name}`;
    if (entry.isDirectory()) files.push(...await sources(name));
    else if (entry.isFile()) files.push(name);
    else throw new Error(`Unsupported package entry: ${name}`);
  }
  return files.sort();
}

/** Bundles the registration, shared tool service and standalone native Compose page. */
export async function bundles() {
  const files = {};
  for (const entry of ['main', 'tools', 'ui/workbench.ui']) {
    const source = await readFile(path.join(root, `src/${entry}.ts`), 'utf8');
    const metadata = entry === 'tools' ? source.match(/^\/\* METADATA[\s\S]*?\*\//)?.[0] : undefined;
    if (entry === 'tools' && !metadata) throw new Error('Missing unified tool metadata');
    const result = await build({ absWorkingDir: root, entryPoints: [`src/${entry}.ts`], bundle: true, write: false, format: 'cjs', platform: 'neutral', target: 'es2020', charset: 'utf8',
      external: entry === 'main' ? ['./ui/*.ui.js'] : [], ...(metadata ? { banner: { js: metadata } } : {}) });
    files[`dist/${entry}.js`] = result.outputFiles[0].contents;
  }
  return files;
}

/** Packages one plugin owner with one public tool namespace and all six internal adapters. */
export async function buildPackage() {
  const files = await bundles();
  for (const name of ['manifest.json', 'package.json', 'pnpm-lock.yaml', 'tsconfig.json', 'README.md', ...await sources('src'), ...await sources('scripts'), ...await sources('resources')]) files[name] = new Uint8Array(await readFile(path.join(root, name)));
  for (const [name, bytes] of Object.entries(files)) {
    if (!name.startsWith('dist/')) continue;
    await mkdir(path.dirname(path.join(root, name)), { recursive: true });
    await writeFile(path.join(root, name), bytes);
  }
  const archive = zipSync(files, { mtime: new Date(1980, 0, 1) });
  await writeFile(path.join(root, 'dist/image_workbench.toolpkg'), archive);
  const assets = path.resolve(root, '../../../../core/crates/runtime/application/assets/plugins/external');
  await mkdir(assets, { recursive: true });
  await writeFile(path.join(assets, 'image_workbench.toolpkg'), archive);
  return files;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) await buildPackage();
