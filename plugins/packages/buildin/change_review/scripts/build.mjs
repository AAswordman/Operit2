import { build } from 'esbuild';
import { zipSync } from 'fflate';
import { readFile, readdir, mkdir, writeFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = fileURLToPath(new URL('../', import.meta.url));

/** Enumerates package source inputs so installed plugins retain readable editable modules. */
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

/** Bundles each executable with runtime-compatible Promise lowering and explicit module ownership. */
export async function bundles() {
  const result = {};
  for (const entry of ['main', 'ui/summary.ui', 'ui/workspace.ui', 'ui/home.ui']) {
    const bundle = await build({ absWorkingDir: root, entryPoints: [`src/${entry}.ts`], bundle: true,
      format: 'cjs', platform: 'neutral', target: 'es2020', supported: { 'async-await': false },
      external: entry === 'main' ? ['./ui/*.ui.js'] : [], write: false, charset: 'utf8' });
    if (bundle.outputFiles.length !== 1) throw new Error(`Expected one bundle for ${entry}`);
    result[`dist/${entry}.js`] = bundle.outputFiles[0].contents;
  }
  return result;
}

/** Packages the exact current sources and their executable Compose modules. */
export async function buildPackage() {
  const files = await bundles();
  for (const name of ['manifest.json', 'package.json', 'tsconfig.json', ...await sources('src'), ...await sources('scripts')]) {
    files[name] = new Uint8Array(await readFile(path.join(root, name)));
  }
  for (const [name, bytes] of Object.entries(files)) {
    if (!name.startsWith('dist/')) continue;
    await mkdir(path.dirname(path.join(root, name)), { recursive: true });
    await writeFile(path.join(root, name), bytes);
  }
  const archive = zipSync(files);
  await writeFile(path.join(root, 'dist/change_review.toolpkg'), archive);
  const assets = path.resolve(root, '../../../../core/crates/runtime/application/assets/plugins/buildin');
  await mkdir(assets, { recursive: true });
  await writeFile(path.join(assets, 'change_review.toolpkg'), archive);
  return files;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) await buildPackage();
