package app.operit

import java.io.File
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class StorageRootAccessTest {
    @get:Rule
    val temporary = TemporaryFolder()

    /** Keeps application-private runtime roots exempt, including new identity directories. */
    @Test
    fun applicationOwnedRootsDoNotRequirePermission() {
        val owned = listOf(
            File(temporary.root, "data/user/0/app.operit"),
            File(temporary.root, "storage/emulated/0/Android/data/app.operit/files"),
            File(temporary.root, "storage/emulated/0/Android/data/app.operit/cache"),
            File(temporary.root, "storage/emulated/0/Android/media/app.operit"),
        )
        for (root in owned) {
            assertFalse(StorageRootAccess.requiresPermission(root.path, owned))
            assertFalse(StorageRootAccess.requiresPermission(File(root, "runtime/identities/new").path, owned))
        }
    }

    /** Requires authorization without relying on the OS enumerating a matching storage volume. */
    @Test
    fun sharedStorageAliasesAndOtherUsersRequirePermission() {
        val owned = listOf(File(temporary.root, "data/user/0/app.operit"))
        for (path in listOf(
            "storage/emulated/0/Operit",
            "sdcard/Operit",
            "mnt/sdcard/Operit",
            "storage/emulated/999/Operit",
            "storage/ABCD-1234/Operit",
        )) {
            val candidate = File(temporary.root, path)
            assertTrue(path, StorageRootAccess.requiresPermission(candidate.path, owned))
        }
    }

    /** Keeps sibling names and another package's directories outside the private boundary. */
    @Test
    fun matchingNamePrefixesDoNotExemptOtherDirectories() {
        val owned = File(temporary.root, "storage/emulated/0/Android/data/app.operit/files")
        for (path in listOf("files-other/runtime", "files/../other/runtime", "../app.other/files")) {
            val candidate = File(owned.parentFile, path)
            assertTrue(path, StorageRootAccess.requiresPermission(candidate.path, listOf(owned)))
        }
    }

    /** Resolves parent traversal before deciding that a path is private. */
    @Test
    fun pathsEscapingOwnedRootsRequirePermission() {
        val owned = temporary.newFolder("private")
        assertTrue(StorageRootAccess.requiresPermission(File(owned, "../shared/runtime").path, listOf(owned)))
        assertFalse(StorageRootAccess.requiresPermission(File(owned, "runtime/../workspaces").path, listOf(owned)))
    }

    /** Prevents missing private-directory metadata from exempting arbitrary paths. */
    @Test
    fun noOwnedRootsRequiresPermission() {
        assertTrue(StorageRootAccess.requiresPermission(File(temporary.root, "shared").path, emptyList()))
    }
}
