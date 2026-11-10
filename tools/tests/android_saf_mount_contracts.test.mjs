import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../../', import.meta.url);
const app = 'apps/flutter/app/android/app/src/main/kotlin/app/operit/';
const plugin = 'apps/flutter/thirdparty/operit_folder_access/';
const source = path => readFileSync(new URL(path, root), 'utf8');

/** Storage roots stay native while workspace selection retains tree capabilities. */
test('Android workspace picker persists actual grants without URI-to-path conversion', () => {
  const picker = source(plugin + 'android/src/main/java/app/operit/folder_access/OperitFolderAccessPlugin.java');
  assert.match(picker, /ACTION_OPEN_DOCUMENT_TREE/);
  assert.match(picker, /data\.getFlags\(\) &/);
  assert.match(picker, /takePersistableUriPermission\(uri, flags\)/);
  assert.match(picker, /source\.put\("root", uri\.toString\(\)\)/);
  assert.doesNotMatch(picker, /getPathFromUri|com\.termux|uri\.getPath\(\)/);
  const dart = source(plugin + 'lib/operit_folder_access.dart');
  assert.match(dart, /class OperitFolderAccessAndroid extends _FileSelectorFolderAccess/);
  assert.match(dart, /Future<FolderMountSource\?> pickWorkspaceDirectory/);
  const content = source('apps/flutter/app/lib/ui/features/chat/components/workspace/WorkspaceTabContent.dart');
  assert.match(content, /OperitFolderAccess\.pickWorkspaceDirectory\(\)/);
  const onboarding = source('apps/flutter/app/lib/ui/features/onboarding/OnboardingStartupRoute.dart');
  assert.match(onboarding, /OperitFolderAccess\.pickDirectory\(\)/);
});

/** Document IDs are opaque, every child stays inside the selected tree capability. */
test('document backend resolves children by display name and tree-scoped document ID', () => {
  const backend = source(app + 'AndroidDocumentFileSystem.kt') + source(app + 'DocumentTreeAccess.kt');
  for (const api of ['persistedUriPermissions', 'buildDocumentUriUsingTree', 'buildChildDocumentsUriUsingTree', 'COLUMN_DOCUMENT_ID', 'COLUMN_DISPLAY_NAME'])
    assert.ok(backend.includes(api));
  assert.match(backend, /isReadPermission/);
  assert.match(backend, /isWritePermission/);
  assert.match(backend, /part !=|it != "\.\."/);
  assert.doesNotMatch(backend, /com\.termux|ProcessBuilder|Runtime\.getRuntime|\/data\/data\/com/);
});

/** Revoked permission errors must not masquerade as empty lists or missing files. */
test('JNI backend preserves failures and runs directly on runtime workers', () => {
  const backend = source(app + 'AndroidDocumentFileSystem.kt');
  assert.match(backend, /\.put\("ok", false\)/);
  assert.match(backend, /catch \(_: Missing\)/);
  const bridge = source('hosts/android/src/document_filesystem.rs');
  assert.match(bridge, /crate::attachCurrentThread\(&bridge\.vm\)/);
  assert.match(bridge, /exception_clear\(\)/);
  assert.match(bridge, /fileSystemResourceOperation/);
  const host = source(app + 'AndroidRuntimeHost.kt');
  assert.match(host, /fun fileSystemResourceOperation\(request: String\)/);
});
