package app.operit

import java.io.File
import java.io.FileNotFoundException
import java.nio.file.Files
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

/** Exercises the filesystem boundary used for external SAF grants without a device or Core. */
class DocumentFileStoreTest {
    @get:Rule val temporary = TemporaryFolder()

    /** Supplies the desktop no-follow filesystem operation for the shared path policy. */
    private fun isSymbolicLink(file: File): Boolean = Files.isSymbolicLink(file.toPath())

    /** Checks real bytes, Unicode paths, renamed IDs, and recursive directory deletion. */
    @Test fun managesOriginalFilesAndDirectories() {
        val directory = temporary.newFolder("private").canonicalFile
        val store = DocumentFileStore("private", directory, "Data", ::isSymbolicLink)
        val folder = store.create(store.documentId, "项目 with spaces", true)
        val document = store.create(folder, "报告.txt", false)
        store.file(document).writeText("original")
        assertEquals("original", File(directory, "项目 with spaces/报告.txt").readText())
        val renamed = store.rename(document, "完成.txt")
        assertEquals("original", store.file(renamed).readText())
        assertThrows(FileNotFoundException::class.java) { store.file(document) }
        assertEquals(listOf("完成.txt"), store.children(folder).map { it.name })
        store.delete(folder)
        assertTrue(store.children(store.documentId).isEmpty())
    }

    /** Rejects escaping IDs, malformed path components, and destructive root operations. */
    @Test fun refusesTraversalUnknownRootsAndRootMutations() {
        val store = DocumentFileStore("private", temporary.newFolder("private"), "Data", ::isSymbolicLink)
        for (id in listOf("other:", "private:../outside", "private:/absolute", "private:a//b",
            "private:./file", "private:a/../b", "private:a\\b", "private:a\u0000b")) {
            assertThrows(Exception::class.java) { store.file(id) }
        }
        for (name in listOf("", ".", "..", "../outside", "a/b", "a\\b", "a\u0000b")) {
            assertThrows(IllegalArgumentException::class.java) { store.create(store.documentId, name, false) }
        }
        assertThrows(FileNotFoundException::class.java) { store.delete(store.documentId) }
        assertThrows(FileNotFoundException::class.java) { store.rename(store.documentId, "renamed") }
        assertTrue(store.directory.isDirectory)
    }

    /** Ensures create and rename cannot overwrite existing user data. */
    @Test fun refusesNameCollisionsWithoutChangingBytes() {
        val store = DocumentFileStore("private", temporary.newFolder("private"), "Data", ::isSymbolicLink)
        val first = store.create(store.documentId, "first.txt", false)
        val second = store.create(store.documentId, "second.txt", false)
        store.file(first).writeText("first")
        store.file(second).writeText("second")
        assertThrows(FileNotFoundException::class.java) { store.create(store.documentId, "first.txt", false) }
        assertThrows(FileNotFoundException::class.java) { store.rename(first, "second.txt") }
        assertEquals("first", store.file(first).readText())
        assertEquals("second", store.file(second).readText())
    }

    /** Binds document grants to a canonical storage location rather than a mutable role name. */
    @Test fun storageChangesDoNotRetargetOldGrants() {
        val firstDirectory = temporary.newFolder("first")
        val first = DocumentFileStore.configured("runtime", firstDirectory, "Runtime", ::isSymbolicLink)
        val same = DocumentFileStore.configured("runtime", File(firstDirectory, "."), "Runtime", ::isSymbolicLink)
        val second = DocumentFileStore.configured("runtime", temporary.newFolder("second"), "Runtime", ::isSymbolicLink)
        assertEquals(first.documentId, same.documentId)
        assertNotEquals(first.documentId, second.documentId)
        assertThrows(FileNotFoundException::class.java) { second.file(first.documentId) }
        assertThrows(IllegalArgumentException::class.java) {
            DocumentFileStore.configured("runtime", File("relative"), "Runtime", ::isSymbolicLink)
        }
    }

    /** Rejects symlink reads and removes nested links without deleting their external targets. */
    @Test fun refusesSymlinksAndDoesNotFollowThemDuringDeletion() {
        val directory = temporary.newFolder("private").canonicalFile
        val outside = temporary.newFolder("outside").canonicalFile
        val protected = File(outside, "keep.txt").apply { writeText("keep") }
        val nested = File(directory, "nested").apply { mkdir() }
        Files.createSymbolicLink(File(nested, "link").toPath(), outside.toPath())
        Files.createSymbolicLink(File(directory, "file-link").toPath(), protected.toPath())
        Files.createSymbolicLink(File(directory, "internal-link").toPath(), nested.toPath())
        Files.createSymbolicLink(File(directory, "dangling").toPath(), File(outside, "missing").toPath())
        val store = DocumentFileStore("private", directory, "Data", ::isSymbolicLink)
        for (id in listOf("private:nested/link", "private:nested/link/keep.txt", "private:file-link", "private:internal-link", "private:dangling")) {
            assertThrows(FileNotFoundException::class.java) { store.file(id) }
        }
        assertEquals(listOf("nested"), store.children(store.documentId).map { it.name })
        assertThrows(FileNotFoundException::class.java) { store.create(store.documentId, "dangling", false) }
        assertThrows(FileNotFoundException::class.java) { store.rename("private:nested", "dangling") }
        store.delete("private:nested")
        assertEquals("keep", protected.readText())
        assertFalse(nested.exists())
    }
}
