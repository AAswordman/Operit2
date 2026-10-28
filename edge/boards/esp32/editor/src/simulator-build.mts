import {fileURLToPath} from 'node:url';
import path from 'node:path';

// Editor startup and boundary probes must share the same Cargo output and cache.
const simulatorDirectory = fileURLToPath(new URL('../simulator/', import.meta.url));
export const simulatorExecutable = path.join(simulatorDirectory, 'target', 'release',
  'operit-esp32-simulator' + (process.platform === 'win32' ? '.exe' : ''));
export const simulatorBuildArguments = ['build', '--locked', '--release', '--manifest-path',
  path.join(simulatorDirectory, 'Cargo.toml'), '--target-dir', path.join(simulatorDirectory, 'target')];
