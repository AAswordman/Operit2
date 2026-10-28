import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile, access} from 'node:fs/promises';

const root = new URL('../../../../../', import.meta.url);
const read = path => readFile(new URL(path, root), 'utf8');

test('firmware has one standalone UI component and no retired widget dependency or backend marker', async () => {
  for (const path of ['edge/boards/esp32/Cargo.toml', 'edge/boards/esp32/.cargo/config.toml',
    'edge/boards/esp32/components_esp32.lock', 'edge/boards/esp32/sdkconfig.defaults',
    'edge/boards/esp32/build.rs', 'edge/boards/esp32/ui_port/CMakeLists.txt']) {
    const text = await read(path);
    assert(!/lvgl|CONFIG_LV_|minimal-ui|operit_mini_ui\)/i.test(text), path);
  }
  const component = await read('edge/boards/esp32/ui_port/CMakeLists.txt');
  assert(component.includes('operit_mini_ui.c'));
  assert(!component.includes('if(EXISTS'));
  await assert.rejects(access(new URL('edge/boards/esp32/lvgl_port/', root)), {code: 'ENOENT'});
  await assert.rejects(access(new URL('edge/boards/esp32/src/lvgl.rs', root)), {code: 'ENOENT'});
});

test('preview compiles local C sources without an ESP-IDF cache or library font extractor', async () => {
  const build = await read('edge/boards/esp32/editor/src/build.mts');
  assert(!/firmwareLvgl|managed_components|lvgl__|prepare-mini-font|lv_version|CONFIG_LV_|-DLV_/i.test(build));
  assert(build.includes("path.join(firmwareRoot, 'sdkconfig.defaults')"));
  assert(build.includes("renderer: 'mini'"));
  const bridge = await read('edge/boards/esp32/editor/wasm/bridge.c');
  assert(!/lv_mem_monitor|lvgl\.h|#ifdef OPERIT_MINIMAL_UI/.test(bridge));
  const font = await read('edge/boards/esp32/ui_port/mini_font.h');
  assert(font.includes('SIL Open Font License'));
  assert(font.includes('mini_text_glyphs'));
  assert(font.includes('mini_digits_glyphs'));
  assert(!/lv_font_t|lv_font_fmt_txt/.test(font));
});

test('one set of commands builds the standalone renderer without compatibility aliases', async () => {
  const {scripts} = JSON.parse(await read('edge/boards/esp32/editor/package.json'));
  for (const alias of ['build:mini', 'build:firmware:mini', 'dev:mini', 'test:mini']) assert.equal(scripts[alias], undefined);
  assert(!Object.values(scripts).some(command => command.includes('--minimal-ui') || command.includes('--lvgl')));
});


test('fixed runtime preview hides the editing overlay before and after Wasm initialization', async () => {
  const html = await read('edge/boards/esp32/editor/web/index.html');
  const app = await read('edge/boards/esp32/editor/web/app.ts');
  assert.match(html, /<div\s+id="edit-layer"\s+hidden\s*>/,
    'an uninitialized or disabled editor must not swallow real pointer clicks');
  assert(app.includes("query<HTMLElement>('#edit-layer').hidden = true;"),
    'runtime initialization must keep the retired editing overlay hidden');
});


test('state-driven expression selector does not advertise inactive preview options', async () => {
  const html = await read('edge/boards/esp32/editor/web/index.html');
  const app = await read('edge/boards/esp32/editor/web/app.ts');
  const selector = /<select id="expression">([\s\S]*?)<\/select>/.exec(html);
  assert(selector);
  assert(!/<option>(happy|sad|sleepy)<\/option>/.test(selector[1]));
  assert(app.includes('expressionSelect.disabled = true;'));
});

// The packed firmware table must retain every metric and address every bitmap pixel.
test('bundled glyphs fit their compact records without truncation', async () => {
  const font = await read('edge/boards/esp32/ui_port/mini_font.h');
  let count = 0;
  for (const table of ['text', 'digits']) {
    const bitmap = new RegExp(`mini_${table}_bitmap\\[\\] = \\{([\\s\\S]*?)\\};`).exec(font);
    const glyphs = new RegExp(`mini_${table}_glyphs\\[\\] = \\{([\\s\\S]*?)\\};`).exec(font);
    assert(bitmap && glyphs, `${table}: missing font table`);
    const bytes = bitmap[1].match(/\d+/g).map(Number);
    assert(bytes.every(byte => byte >= 0 && byte <= 255));
    let previous = -1;
    for (const match of glyphs[1].matchAll(/\{([\d,\-]+)\}/g)) {
      const values = match[1].split(',').map(Number);
      assert.equal(values.length, 10);
      const [cp, offset, advance, w, h, bpp, line, baseline, x, y] = values;
      assert(cp > previous && cp <= 0xffff, `${table}: ordered BMP codepoint ${cp}`);
      previous = cp;
      assert(offset >= 0 && offset < (1 << 17));
      assert([advance, w, h, bpp, line, baseline].every((v, i) =>
        v >= 0 && v < (1 << [5, 4, 5, 2, 5, 3][i])));
      assert([x, y].every(v => v >= -16 && v <= 15));
      assert(offset + Math.ceil(w * h * bpp / 8) <= bytes.length, `${table}: bitmap bounds for ${cp}`);
      count++;
    }
  }
  assert.equal(count, 4168, 'retain the complete text and digit character set');
});
