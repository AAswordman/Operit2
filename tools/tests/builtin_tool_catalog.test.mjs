import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const configRoot = new URL('../../core/crates/provider/services/src/chat/config/', import.meta.url);
const publicPrompts = readFileSync(new URL('SystemToolPrompts.rs', configRoot), 'utf8');
const internalPrompts = readFileSync(new URL('SystemToolPromptsInternal.rs', configRoot), 'utf8');

/** Extracts a top-level Rust prompt declaration up to the next function. */
function declaration(source, name) {
  const start = source.indexOf(`\nfn ${name}(`);
  assert.notEqual(start, -1, `Missing prompt declaration: ${name}`);
  const end = source.indexOf('\nfn ', start + 1);
  return source.slice(start, end === -1 ? source.length : end);
}

/** Reads the literal tool names from a prompt category declaration. */
function toolNames(source) {
  return Array.from(source.matchAll(/\btool\(\s*"([a-z0-9_]+)"/g), match => match[1]);
}

for (const [language, suffix] of [['En', 'en'], ['Cn', 'cn']]) {
  /** Requires public and internal sources to form one unambiguous catalog. */
  test(`${language} builtin catalog declares each tool exactly once`, () => {
    const publicNames = ['basic_tools', 'file_system_tools', 'http_tools']
      .flatMap(name => toolNames(declaration(publicPrompts, `${name}_${suffix}`)));
    const internalNames = toolNames(declaration(internalPrompts, `internalToolCategories${language}Source`));
    const names = [...publicNames, ...internalNames];
    assert.ok(publicNames.length > 0);
    assert.ok(internalNames.length > 0);
    const seen = new Set();
    for (const name of names) {
      assert.ok(!seen.has(name), `Duplicate builtin tool catalog entry: ${name}`);
      seen.add(name);
    }
  });

  /** Keeps text writing AI-visible with its existing optional append contract. */
  test(`${language} write_file has one public schema including append`, () => {
    const files = declaration(publicPrompts, `file_system_tools_${suffix}`);
    const write = files.match(/\btool\("write_file",[^\n]+/);
    assert.ok(write, 'write_file must remain AI-visible');
    assert.match(write[0], /param\("path", "string", [^,]+, true, None\)/);
    assert.match(write[0], /param\("content", "string", [^,]+, true, None\)/);
    assert.match(write[0], /param\("append", "boolean", [^,]+, false, Some\("false"\)\)/);
    const internalNames = toolNames(declaration(internalPrompts, `internalToolCategories${language}Source`));
    assert.ok(!internalNames.includes('write_file'));
    assert.ok(internalNames.includes('write_file_binary'));
  });
}
