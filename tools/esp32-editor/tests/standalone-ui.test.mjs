import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile, access} from 'node:fs/promises';

const root = new URL('../../../', import.meta.url);
const read = path => readFile(new URL(path, root), 'utf8');

test('firmware has one standalone UI component and no retired widget dependency or backend marker', async () => {
  for (const path of ['apps/esp32/Cargo.toml', 'apps/esp32/.cargo/config.toml',
    'apps/esp32/components_esp32.lock', 'apps/esp32/sdkconfig.defaults',
    'apps/esp32/build.rs', 'apps/esp32/ui_port/CMakeLists.txt']) {
    const text = await read(path);
    assert(!/lvgl|CONFIG_LV_|minimal-ui|operit_mini_ui\)/i.test(text), path);
  }
  const component = await read('apps/esp32/ui_port/CMakeLists.txt');
  assert(component.includes('operit_mini_ui.c'));
  assert(!component.includes('if(EXISTS'));
  await assert.rejects(access(new URL('apps/esp32/lvgl_port/', root)), {code: 'ENOENT'});
  await assert.rejects(access(new URL('apps/esp32/src/lvgl.rs', root)), {code: 'ENOENT'});
});

test('preview compiles local C sources without an ESP-IDF cache or library font extractor', async () => {
  const build = await read('tools/esp32-editor/src/build.mts');
  assert(!/firmwareLvgl|managed_components|lvgl__|prepare-mini-font|lv_version|CONFIG_LV_|-DLV_/i.test(build));
  assert(build.includes("path.join(firmwareRoot, 'sdkconfig.defaults')"));
  assert(build.includes("renderer: 'mini'"));
  const bridge = await read('tools/esp32-editor/wasm/bridge.c');
  assert(!/lv_mem_monitor|lvgl\.h|#ifdef OPERIT_MINIMAL_UI/.test(bridge));
  const font = await read('apps/esp32/ui_port/mini_font.h');
  assert(font.includes('SIL Open Font License'));
  assert(font.includes('mini_text_glyphs'));
  assert(font.includes('mini_digits_glyphs'));
  assert(!/lv_font_t|lv_font_fmt_txt/.test(font));
});

test('one set of commands builds the standalone renderer without compatibility aliases', async () => {
  const {scripts} = JSON.parse(await read('tools/esp32-editor/package.json'));
  for (const alias of ['build:mini', 'build:firmware:mini', 'dev:mini', 'test:mini']) assert.equal(scripts[alias], undefined);
  assert(!Object.values(scripts).some(command => command.includes('--minimal-ui') || command.includes('--lvgl')));
});


test('fixed runtime preview hides the editing overlay before and after Wasm initialization', async () => {
  const html = await read('tools/esp32-editor/web/index.html');
  const app = await read('tools/esp32-editor/web/app.ts');
  assert.match(html, /<div\s+id="edit-layer"\s+hidden\s*>/,
    'an uninitialized or disabled editor must not swallow real pointer clicks');
  assert(app.includes("query<HTMLElement>('#edit-layer').hidden = true;"),
    'runtime initialization must keep the retired editing overlay hidden');
});


test('state-driven expression selector does not advertise inactive preview options', async () => {
  const html = await read('tools/esp32-editor/web/index.html');
  const app = await read('tools/esp32-editor/web/app.ts');
  const selector = /<select id="expression">([\s\S]*?)<\/select>/.exec(html);
  assert(selector);
  assert(!/<option>(happy|sad|sleepy)<\/option>/.test(selector[1]));
  assert(app.includes('expressionSelect.disabled = true;'));
});
