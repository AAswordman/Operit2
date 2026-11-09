import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const root = fileURLToPath(new URL('../../', import.meta.url));

test('iOS selected-file access and persistent directory grants', { skip: process.platform !== 'darwin' }, () => {
  const temp = mkdtempSync(join(tmpdir(), 'operit-ios-file-access-'));
  try {
    const binary = join(temp, 'file-access-tests');
    execFileSync('xcrun', ['swiftc',
      join(root, 'apps/flutter/thirdparty/file_selector_ios/ios/file_selector_ios/Sources/file_selector_ios/IOSFolderAccessStore.swift'),
      join(root, 'tools/tests/fixtures/ios_file_selector_access/main.swift'),
      '-o', binary,
    ], { encoding: 'utf8', timeout: 60000 });
    const output = execFileSync(binary, { encoding: 'utf8', timeout: 30000 });
    assert.match(output, /Executed 6 tests, with 0 failures/);
  } finally {
    rmSync(temp, { recursive: true, force: true });
  }
});

test('ToolPkg picker UTI is exported by the iOS application', { skip: process.platform !== 'darwin' }, () => {
  const plist = JSON.parse(execFileSync('plutil', ['-convert', 'json', '-o', '-',
    join(root, 'apps/flutter/app/ios/Runner/Info.plist'),
  ], { encoding: 'utf8' }));
  const declaration = plist.UTExportedTypeDeclarations.find(type =>
    type.UTTypeTagSpecification['public.filename-extension'].includes('toolpkg'));
  assert.equal(declaration.UTTypeIdentifier, 'app.operit.toolpkg');
  assert.ok(declaration.UTTypeConformsTo.includes('public.zip-archive'));
  assert.ok(plist.CFBundleDocumentTypes.some(type => type.LSItemContentTypes.includes(declaration.UTTypeIdentifier)));
  assert.equal(plist.LSSupportsOpeningDocumentsInPlace, true);
});
