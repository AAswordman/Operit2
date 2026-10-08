import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

// These checks inspect authoritative source contracts; they do not execute the Rust backend.
const here = dirname(fileURLToPath(import.meta.url));
const repository = resolve(here, '../../../../../../..');
const manager = readFileSync(resolve(here, 'ThemeConfigManager.rs'), 'utf8');
const validator = readFileSync(resolve(here, 'ThemePreferenceSnapshot.rs'), 'utf8');
const rustTests = readFileSync(resolve(here, 'ThemeConfigManager.tests.rs'), 'utf8');
const fixture = JSON.parse(readFileSync(resolve(here, 'theme_snapshot_fixture.json'), 'utf8'));
const dart = readFileSync(resolve(repository, 'apps/flutter/app/lib/data/preferences/UserPreferencesManager.dart'), 'utf8');
const appearanceUi = readFileSync(resolve(repository, 'apps/flutter/app/lib/ui/features/settings/appearance/AppearanceSettingsPanel.dart'), 'utf8');
const application = readFileSync(resolve(repository, 'core/crates/runtime/application/src/core/application/OperitApplication.rs'), 'utf8');
const modules = readFileSync(resolve(here, 'mod.rs'), 'utf8');
const store = readFileSync(resolve(repository, 'core/crates/persistence/store/src/PreferencesDataStore.rs'), 'utf8');
const paths = readFileSync(resolve(repository, 'core/crates/foundation/util/src/RuntimeStorageLayout.rs'), 'utf8');
const factoryScanner = readFileSync(resolve(repository, 'core/crates/proxy/scan/src/build_scanner.rs'), 'utf8');

const jsonStart = dart.indexOf('Map<String, Object?> toJson()');
const jsonEnd = dart.indexOf('factory ThemePreferenceSnapshot.fromJson');
assert.ok(jsonStart >= 0 && jsonEnd > jsonStart);
const dartKeys = [...dart.slice(jsonStart, jsonEnd).matchAll(/'([A-Za-z]+)'\s*:/g)].map(match => match[1]);
const dartTypes = new Map([...dart.slice(0, jsonStart).matchAll(/final (\w+\??) (\w+);/g)].map(match => [match[2], match[1]]));
const dartConstants = new Map([...dart.matchAll(/static const String (\w+)\s*=\s*'([^']*)';/g)].map(match => [match[1], match[2]]));
const fields = [...validator.matchAll(/SnapshotField\s*\{\s*json_key:\s*"([^"]+)",\s*preference_key:\s*"([^"]+)",\s*kind:\s*(FieldKind::[\s\S]*?)\n\s*\},/g)].map(match => ({
  name: match[1],
  preferenceKey: match[2],
  rule: match[3],
}));

/// Extracts one Rust function body without treating quoted strings or comments as block delimiters.
function body(source, name) {
  const match = new RegExp(`\\bfn\\s+${name}(?:<[^\\n]*>)?\\s*\\(`).exec(source);
  assert.ok(match, `missing function ${name}`);
  const start = source.indexOf('{', match.index + match[0].length);
  let depth = 0;
  let quoted = false;
  let comment = false;
  for (let index = start; index < source.length; index += 1) {
    const ch = source[index];
    if (comment) {
      if (ch === '\n') comment = false;
      continue;
    }
    if (quoted) {
      if (ch === '\\') index += 1;
      else if (ch === '"') quoted = false;
      continue;
    }
    if (ch === '/' && source[index + 1] === '/') { comment = true; index += 1; continue; }
    if (ch === '"') { quoted = true; continue; }
    if (ch === '{') depth += 1;
    if (ch === '}' && --depth === 0) return source.slice(start + 1, index);
  }
  throw new Error(`unclosed function ${name}`);
}

/// Reads an explicitly declared field rule from the Rust schema table.
function rule(field) {
  const kind = /^FieldKind::(\w+)/.exec(field.rule);
  assert.ok(kind, field.name);
  if (kind[1] === 'Number') {
    const range = /min:\s*([\d.]+),\s*max:\s*([\d.]+|f64::MAX)/.exec(field.rule);
    assert.ok(range, field.name);
    return { kind: kind[1], min: Number(range[1]), max: range[2] === 'f64::MAX' ? Number.MAX_VALUE : Number(range[2]) };
  }
  return { kind: kind[1], allowed: [...field.rule.matchAll(/"([^"]+)"/g)].map(match => match[1]) };
}

/// Verifies one fixture value against source-derived field rules without emulating the Rust repository.
function assertFixtureField(field) {
  assert.ok(Object.hasOwn(fixture, field.name), field.name);
  const value = fixture[field.name];
  const constraint = rule(field);
  switch (constraint.kind) {
    case 'Boolean': assert.equal(typeof value, 'boolean', field.name); break;
    case 'OptionalColor': assert.ok(value === null || (Number.isInteger(value) && value >= 0 && value <= 0xffffffff), field.name); break;
    case 'OptionalString': assert.ok(value === null || typeof value === 'string', field.name); break;
    case 'OptionalEnum': assert.ok(value === null || constraint.allowed.includes(value), field.name); break;
    case 'Enum': assert.ok(constraint.allowed.includes(value), field.name); break;
    case 'Number': assert.ok(typeof value === 'number' && Number.isFinite(value) && value >= constraint.min && value <= constraint.max, field.name); break;
    default: throw new Error(`unrecognized schema rule ${constraint.kind}`);
  }
}

/// Finds one exact field rather than inferring its meaning from partial key matches.
function field(name) {
  const result = fields.find(item => item.name === name);
  assert.ok(result, name);
  return result;
}

/// Checks English documentation on each new Rust function, including test host methods.
function assertRustFunctionDocs(source) {
  const declared = [...source.matchAll(/\bfn\s+(\w+)(?:<[^\n]*>)?\s*\(/g)].map(match => match[1]);
  const documented = [...source.matchAll(/(?:\/\/\/[^\n]*\n\s*)+(?:#\[[^\n]*\]\n\s*)*(?:pub(?:\([^)]*\))?\s+)?fn\s+(\w+)(?:<[^\n]*>)?\s*\(/g)].map(match => match[1]);
  assert.deepEqual(documented.sort(), declared.sort());
}

/// Keeps the validator field set exactly equal to the complete Flutter serialization shape.
test('Rust schema, Dart toJson and fixture have exactly 78 unique matching keys', () => {
  assert.equal(dartKeys.length, 78);
  assert.equal(fields.length, 78);
  assert.equal(new Set(dartKeys).size, 78);
  assert.equal(new Set(fields.map(item => item.name)).size, 78);
  assert.equal(new Set(fields.map(item => item.preferenceKey)).size, 78);
  assert.deepEqual(fields.map(item => item.name).sort(), [...dartKeys].sort());
  assert.deepEqual(Object.keys(fixture).sort(), [...dartKeys].sort());
});

/// Cross-checks ordinary preference mappings using the actual Flutter preference reader.
test('all 78 mappings match the ordinary Flutter reader and preserve legacy bubble key names', () => {
  const resolveStart = dart.indexOf('Future<ThemePreferenceSnapshot> resolveThemePreferenceSnapshot()');
  assert.ok(resolveStart >= 0);
  const constructorStart = dart.indexOf('return ThemePreferenceSnapshot(', resolveStart);
  const constructorEnd = dart.indexOf('\n    );', constructorStart);
  const constructor = dart.slice(constructorStart, constructorEnd + 7);
  const mappings = new Map();
  for (const match of constructor.matchAll(/\n\s{6}(\w+):\s*([\s\S]*?)(?=\n\s{6}\w+:\s*|\n\s{4}\);)/g)) {
    const constant = /\b(_[A-Z][A-Z0-9_]*)\b/.exec(match[2]);
    if (match[1] === 'themeMode') mappings.set(match[1], dartConstants.get('_THEME_MODE'));
    else {
      assert.ok(constant, match[1]);
      assert.ok(dartConstants.has(constant[1]), match[1]);
      mappings.set(match[1], dartConstants.get(constant[1]));
    }
  }
  assert.equal(mappings.size, 78);
  for (const item of fields) assert.equal(item.preferenceKey, mappings.get(item.name), item.name);
  assert.equal(field('bubbleUserRoundedCornersEnabled').preferenceKey, 'bubble_rounded_corners_enabled');
  assert.equal(field('bubbleUserContentPaddingLeft').preferenceKey, 'bubble_content_padding_left');
  assert.equal(field('bubbleUserContentPaddingRight').preferenceKey, 'bubble_content_padding_right');
});

/// Checks declared JSON types against the Flutter snapshot field declarations.
test('every field validation rule matches the Flutter declared wire type', () => {
  for (const item of fields) {
    const constraint = rule(item);
    const type = dartTypes.get(item.name);
    switch (type) {
      case 'bool': assert.equal(constraint.kind, 'Boolean', item.name); break;
      case 'int?': assert.equal(constraint.kind, 'OptionalColor', item.name); break;
      case 'double': assert.equal(constraint.kind, 'Number', item.name); break;
      case 'String?': assert.ok(['OptionalString', 'OptionalEnum'].includes(constraint.kind), item.name); break;
      case 'String': case 'ThemeMode': assert.equal(constraint.kind, 'Enum', item.name); break;
      default: throw new Error(`unclassified Dart type ${type}`);
    }
  }
});

/// Confirms that the existing ordinary default snapshot is valid under all source-derived constraints.
test('complete ordinary default fixture satisfies types, enums, ranges and repeat intervals', () => {
  for (const item of fields) assertFixtureField(item);
  for (const prefix of ['bubbleUser', 'bubbleAi']) {
    for (const axis of ['', 'Y']) {
      assert.ok(fixture[`${prefix}ImageRepeat${axis}End`] >= fixture[`${prefix}ImageRepeat${axis}Start`] + 0.01);
    }
  }
});

/// Locks enum choices to the current ordinary appearance constants rather than plugin-specific values.
test('enum validators match all current ordinary Flutter choices', () => {
  const expected = {
    themeMode: ['system', 'light', 'dark'],
    backgroundMediaType: ['MEDIA_TYPE_IMAGE', 'MEDIA_TYPE_VIDEO'].map(name => dartConstants.get(name)),
    inputStyle: ['INPUT_STYLE_CLASSIC', 'INPUT_STYLE_AGENT'].map(name => dartConstants.get(name)),
    chatStyle: ['CHAT_STYLE_CURSOR', 'CHAT_STYLE_BUBBLE'].map(name => dartConstants.get(name)),
    avatarShape: ['AVATAR_SHAPE_CIRCLE', 'AVATAR_SHAPE_SQUARE'].map(name => dartConstants.get(name)),
  };
  for (const [name, allowed] of Object.entries(expected)) assert.deepEqual(rule(field(name)).allowed, allowed);
  for (const prefix of ['', 'bubbleUser', 'bubbleAi']) {
    const fontType = prefix === '' ? 'fontType' : `${prefix}FontType`;
    const fontName = prefix === '' ? 'systemFontName' : `${prefix}SystemFontName`;
    assert.deepEqual(rule(field(fontType)).allowed, ['FONT_TYPE_SYSTEM', 'FONT_TYPE_FILE'].map(name => dartConstants.get(name)));
    assert.deepEqual(rule(field(fontName)).allowed, ['SYSTEM_FONT_DEFAULT', 'SYSTEM_FONT_SERIF', 'SYSTEM_FONT_SANS_SERIF', 'SYSTEM_FONT_MONOSPACE', 'SYSTEM_FONT_CURSIVE'].map(name => dartConstants.get(name)));
  }
  for (const name of ['bubbleUserImageRenderMode', 'bubbleAiImageRenderMode']) {
    assert.deepEqual(rule(field(name)).allowed, ['BUBBLE_IMAGE_RENDER_MODE_TILED_NINE_SLICE', 'BUBBLE_IMAGE_RENDER_MODE_NINE_PATCH'].map(name => dartConstants.get(name)));
  }
});

/// Cross-checks numeric limits against actual sliders and requires finite nonnegative pixel dimensions.
test('numeric limits include the Flutter sliders without silently clamping input', () => {
  assert.match(appearanceUi, /snapshot\.fontScale\.clamp\(0\.85,\s*1\.3\)/);
  assert.match(appearanceUi, /snapshot\.backgroundBlurRadius\.clamp\(0,\s*40\)/);
  assert.match(appearanceUi, /value:\s*imageScale,\s*min:\s*0\.2,\s*max:\s*3/);
  assert.match(appearanceUi, /value:\s*cropLeft,\s*min:\s*0,\s*max:\s*0\.45/);
  assert.match(appearanceUi, /value:\s*repeatStart,\s*min:\s*0\.05,\s*max:\s*0\.9/);
  for (const [name, min, max] of [['backgroundImageOpacity', 0, 1], ['backgroundBlurRadius', 0, 40], ['fontScale', 0.85, 1.3], ['bubbleAiImageScale', 0.2, 3], ['bubbleUserImageCropLeft', 0, 0.45], ['bubbleAiImageRepeatStart', 0.05, 0.9], ['bubbleUserImageRepeatEnd', 0.06, 0.95]]) {
    assert.deepEqual(rule(field(name)), {kind: 'Number', min, max});
  }
  assert.match(body(validator, 'validateField'), /number\.is_finite\(\)\s*&&\s*number\s*>=\s*min\s*&&\s*number\s*<=\s*max/);
  assert.doesNotMatch(validator, /\.clamp\(/);
  assert.match(body(validator, 'validateThemePreferenceSnapshot'), /end_value\s*<\s*start_value\s*\+\s*0\.01/);
});

/// Requires every mutation to read and validate the current canonical state inside one shared transaction.
test('all six mutators read and write current state inside one try_edit_result transaction', () => {
  for (const name of ['create', 'update', 'delete', 'apply', 'saveAppearance', 'useCustomAppearance']) {
    const code = body(manager, name);
    assert.equal([...code.matchAll(/\.try_edit_result\(/g)].length, 1, name);
    const transaction = code.indexOf('.try_edit_result(');
    const read = code.indexOf('readState(preferences)?');
    const write = code.indexOf('writeState(preferences, &state)');
    assert.ok(read > transaction && write > read, name);
    assert.doesNotMatch(code.slice(0, transaction), /data\(|readState\(/, name);
    assert.doesNotMatch(code, /\.edit\(|\.replace\(|\.migrate\(/, name);
  }
  assert.match(body(store, 'try_edit_result_internal'), /sharedState[\s\S]*transaction[\s\S]*lock\(/);
  assert.match(body(store, 'try_edit_result_internal'), /let result = transform\(&mut preferences\)\?/);
});

/// Requires active selection and full ordinary appearance to be committed together without a second ledger.
test('apply, active update and ordinary save commit the full snapshot and named selection together', () => {
  const apply = body(manager, 'apply');
  assert.match(apply, /writeThemePreferenceSnapshot\(preferences, &config\.snapshot\)\?/);
  assert.match(apply, /state\.activeThemeConfigId = Some\(config\.id\.clone\(\)\)/);
  assert.match(body(manager, 'update'), /state\.activeThemeConfigId\.as_deref\(\) == Some\(id\.as_str\(\)\)[\s\S]*writeThemePreferenceSnapshot/);
  assert.match(body(manager, 'saveAppearance'), /findConfigIndex\(&state, &id\)\?[\s\S]*snapshot: snapshot\.clone\(\)[\s\S]*state\.configs\[index\] = config\.clone\(\)/);
  assert.match(body(manager, 'saveAppearance'), /writeThemePreferenceSnapshot\(preferences, &snapshot\)\?/);
  assert.equal([...manager.matchAll(/const\s+[A-Z_]+_KEY:/g)].length, 2);
  assert.doesNotMatch(manager, /active_snapshot|activeSnapshot|role_theme|group_theme/);
});

/// Checks deletion refusal and explicit custom selection without silent reassignment.
test('active deletion is refused and custom selection is explicit', () => {
  const deletion = body(manager, 'delete');
  assert.ok(deletion.indexOf('Cannot delete active theme configuration') < deletion.indexOf('state.configs.remove(index)'));
  assert.match(deletion, /return Err\(/);
  assert.doesNotMatch(deletion, /state\.activeThemeConfigId\s*=/);
  const custom = body(manager, 'useCustomAppearance');
  assert.match(custom, /state\.activeThemeConfigId = None/);
  assert.doesNotMatch(custom, /\.create\(|\.apply\(/);
});

/// Verifies strict stored state errors and ordinary consistency without manufactured success values.
test('corrupt catalog, dangling selection and active ordinary divergence remain errors', () => {
  const read = body(manager, 'readState');
  assert.match(read, /Some\(value\) => serde_json::from_str::<Vec<ThemeConfig>>\(value\)\?/);
  assert.match(read, /Some\(value\) => serde_json::from_str::<Option<String>>\(value\)\?/);
  assert.match(read, /validateState\(&state\)\?/);
  assert.match(read, /assertThemePreferenceSnapshot\(preferences,[\s\S]*findConfig\(&state, id\)\?/);
  assert.match(body(manager, 'validateState'), /Duplicate theme configuration identifier/);
  assert.match(body(manager, 'findConfigIndex'), /Theme configuration not found/);
  assert.match(body(validator, 'assertThemePreferenceSnapshot'), /active ordinary preference differs/);
  assert.match(body(validator, 'assertThemePreferenceSnapshot'), /\(Value::Null, Some\(_\)\) =>[\s\S]*active ordinary preference must be absent/);
  assert.match(body(validator, 'validateThemePreferenceSnapshot'), /unknown field/);
  assert.match(body(validator, 'validateThemePreferenceSnapshot'), /missing field/);
  assert.doesNotMatch(manager + validator, /\.unwrap\w*\(|\.catch\(|defaultRuntimeStorageHost|RuntimeStorePaths::default|CharacterCardManager|HostProvider|PluginStore|target_arch|exec\(/);
  assert.match(manager, /#\[serde\(deny_unknown_fields\)\]/);
});

/// Requires the application factory and all manager instances to share the owning runtime host and ordinary path.
test('factory and manager use the actual runtime-bound ordinary preference host', () => {
  const factory = body(application, 'themeConfigManager');
  assert.match(factory, /self\.hostManager\.runtimeStorageHost\.clone\(\)\.ok_or_else/);
  assert.match(factory, /Ok\(ThemeConfigManager::new\(storageHost\)\)/);
  assert.doesNotMatch(factory, /defaultRuntimeStorageHost|RuntimeStorePaths|UserPreferencesManager::getInstance/);
  assert.match(body(manager, 'new'), /PreferencesDataStore::newWithStorage\(\s*storageHost,\s*OperitPaths::USER_PREFERENCES_PATH/);
  assert.match(paths, /USER_PREFERENCES_PATH:\s*&str\s*=\s*"runtime\/config\/preferences\/user_preferences\.preferences\.json"/);
  assert.match(modules, /#\[path = "ThemeConfigManager.rs"\]\s*pub mod ThemeConfigManager/);
  assert.match(modules, /pub use ThemeConfigManager::\*/);
  assert.match(factoryScanner, /fn factory_returned_object_type[\s\S]*generic_args\(return_type, "Result"\)/);
});

/// Keeps watch backed by strict shared preference observation rather than an ad hoc catalog event bus.
test('watch derives coherent strict state from the shared dataFlow', () => {
  assert.match(body(manager, 'watch'), /self\.dataStore\s*\.dataFlow\(\)\s*\.mapResult\(\|preferences\| readState\(&preferences\)\)/);
  assert.match(body(store, 'dataFlow'), /preferencesDataStoreFlowObservation/);
  assert.match(body(store, 'try_edit_result_internal'), /self\.notifyChanged\(\)/);
  assert.match(body(validator, 'writeThemePreferenceSnapshot'), /preferences\.remove\(&stringPreferencesKey\("use_system_theme"\)\)/);
});

/// Ensures all newly added functions are documented and the real-host Rust tests stay available for later execution.
test('new functions have English docs and all 18 unexecuted Rust integration tests are present', () => {
  for (const source of [manager, validator, rustTests]) assertRustFunctionDocs(source);
  assert.equal([...rustTests.matchAll(/^#\[test\]/gm)].length, 18);
  assert.match(rustTests, /NativeRuntimeStorageHost::new/);
  assert.match(rustTests, /host_write_failure_preserves_committed_state/);
  assert.match(rustTests, /watch_observes_cross_context_commits_once/);
  assert.match(rustTests, /actual_runtime_hosts_isolate_catalogs_and_selection/);
  assert.doesNotMatch(rustTests, /setDefaultRuntimeStorageHost|setDefaultRuntimeStoreRootConfig/);
});
