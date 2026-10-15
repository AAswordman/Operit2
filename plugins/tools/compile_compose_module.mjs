import { composeRoot, composeManifest } from '../../tools/runtime/compose_scripts.mjs';
import { readFileSync } from 'node:fs';
import vm from 'node:vm';

const parserInput = new URL(composeManifest.parserSource, composeRoot);
const artifactInput = new URL(composeManifest.artifact, composeRoot);
const compilerInput = new URL(composeManifest.compiler, composeRoot);

/** Build inputs are shared with the runtime, never a second implementation of DSL lowering. */
export const composeCompilerInputs = [
  parserInput,
  artifactInput,
  compilerInput,
  new URL(import.meta.url),
  new URL('sources.json', composeRoot),
  new URL('../../tools/runtime/compose_scripts.mjs', import.meta.url),
];
let compiler;
/** Compiles portable JavaScript ahead of page creation; does not execute plugin code. */
export function compileComposeModule(source, identity) {
  if (typeof source !== 'string' || typeof identity !== 'string' || !identity) {
    throw new TypeError('Compose compilation requires source and a stable module identity');
  }
  if (!compiler) {
    const context = vm.createContext({ module: { exports: {} } });
    context.exports = context.module.exports;
    vm.runInContext(readFileSync(parserInput, 'utf8'), context);
    context.__operitAcorn = context.module.exports;
    vm.runInContext(readFileSync(artifactInput, 'utf8'), context);
    vm.runInContext(readFileSync(compilerInput, 'utf8'), context);
    compiler = context.OperitComposeCompiler;
  }
  return compiler.artifact(source, identity);
}
