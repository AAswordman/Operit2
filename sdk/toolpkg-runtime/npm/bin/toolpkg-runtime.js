#!/usr/bin/env node
'use strict';

const { spawnSync } = require('node:child_process');

const runtimeBin = process.env.OPERIT_TOOLPKG_RUNTIME_BIN;
if (!runtimeBin || !runtimeBin.trim()) {
  console.error('OPERIT_TOOLPKG_RUNTIME_BIN is required');
  process.exit(1);
}

const result = spawnSync(runtimeBin, process.argv.slice(2), {
  stdio: 'inherit'
});

if (result.error) {
  console.error(result.error.message);
  process.exit(1);
}

process.exit(result.status == null ? 1 : result.status);
