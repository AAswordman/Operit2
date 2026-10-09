import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdirSync } from 'node:fs';
import { createHash } from 'node:crypto';

const root = new URL('../../', import.meta.url);
const output = new URL('target/dsl-performance/', root);

/** Extracts uniquely delimited production code and fails when its source contract changes. */
function between(source, start, end) {
  assert.equal(source.split(start).length, 2, 'Expected exactly one source start marker: ' + start);
  assert.equal(source.split(end).length, 2, 'Expected exactly one source end marker: ' + end);
  const first = source.indexOf(start);
  const last = source.indexOf(end, first + start.length);
  assert.ok(last > first, 'End marker must follow the start marker');
  return source.slice(first, last);
}

/** Generates a package-free probe from the actual parser, never a reimplemented parser. */
function main() {
  const modelPath = 'apps/flutter/app/lib/ui/features/packages/screens/compose_dsl/render_models.dart';
  const helpersPath = 'apps/flutter/app/lib/ui/features/packages/screens/compose_dsl/value_parsers.dart';
  const models = readFileSync(new URL(modelPath, root), 'utf8');
  const helpers = readFileSync(new URL(helpersPath, root), 'utf8');
  const template = readFileSync(new URL('tools/performance/compose_dsl_parser_probe.dart.template', root), 'utf8');
  const parser = [
    between(models, 'class _ParsedComposeDslActionEvent {', 'class _NoUiView extends StatelessWidget {'),
    between(helpers, 'Map<String, Object?> _stringMap(Object? raw) {', '/// Resolves canvas commands'),
    between(helpers, '/// Resolves node list', '/// Evaluates bool'),
    between(helpers, '/// Resolves normalize token', 'double? _number(Object? raw) {'),
  ].join('\n');
  mkdirSync(output, { recursive: true });
  writeFileSync(new URL('dart_parser_probe.dart', output), template + '\n' + parser);
  writeFileSync(new URL('empty-package-config.json', output), JSON.stringify({ configVersion: 2, packages: [] }));
  writeFileSync(new URL('dart-parser-source.json', output), JSON.stringify({
    scope: 'Exact source extraction; no production parser changes and no Flutter native-assets build hook',
    files: [modelPath, helpersPath],
    modelSha256: createHash('sha256').update(models).digest('hex'),
    helpersSha256: createHash('sha256').update(helpers).digest('hex'),
  }, null, 2));
  console.log('Generated target/dsl-performance/dart_parser_probe.dart from the checked-out production parser');
}

main();
