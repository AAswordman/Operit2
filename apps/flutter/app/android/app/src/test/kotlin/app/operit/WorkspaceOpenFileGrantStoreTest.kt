package app.operit

import java.io.File
import java.io.IOException
import java.nio.file.Files
import java.security.SecureRandom
import java.util.concurrent.Callable
import java.util.concurrent.Executors
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeFalse
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class WorkspaceOpenFileGrantStoreTest {
    @get:Rule
    val temporary = TemporaryFolder()

    private fun source(directory: File = temporary.newFolder("source"), name: String = "archive.zip"): File =
        File(directory, name).apply { writeBytes(byteArrayOf(0, 1, 2, 127, -1)) }.canonicalFile

    // Store tests inject an opened-FD identity; actual fstat/seek and permissions are device tests.
    private fun identity(file: File, inode: Long = 7): WorkspaceOpenFileIdentity =
        WorkspaceOpenFileIdentity(3, inode, file.length(), 11, 12)

    private fun grant(store: WorkspaceOpenFileGrantStore, file: File, inode: Long = 7): WorkspaceOpenFileGrant =
        store.register(file, file.name, "application/zip", identity(file, inode))

    @Test
    fun persistsOpaqueGrantWithoutCopyingTheSource() {
        val file = source()
        val bytes = file.readBytes()
        val directory = temporary.newFolder("grants")
        val store = WorkspaceOpenFileGrantStore(directory, now = { 1_000L })
        val permission = grant(store, file)
        assertTrue(permission.token.matches(Regex("[0-9a-f]{32}")))
        assertEquals(WorkspaceOpenFileGrantStore.TTL_MILLIS, permission.expiresAt - permission.issuedAt)
        assertEquals(permission, WorkspaceOpenFileGrantStore(directory, now = { 1_001L }).find(permission.token))
        val metadata = directory.listFiles()!!.single()
        assertEquals("${permission.token}.json", metadata.name)
        assertTrue(metadata.length() < 16 * 1024)
        assertArrayEquals(bytes, file.readBytes())
    }

    @Test
    fun reloadKeepsIdenticalNamesAndLongMetadataSeparate() {
        val first = source(temporary.newFolder("first"), "项目 bundle.zip")
        val second = source(temporary.newFolder("second"), first.name)
        val directory = temporary.newFolder("grants")
        val store = WorkspaceOpenFileGrantStore(directory, now = { 1_000L })
        val firstGrant = grant(store, first, 1)
        // This checks JSON Long preservation, not a large/sparse ZIP performance fixture.
        val longIdentity = identity(second, 2).copy(size = 2L * 1024 * 1024 * 1024 + 19)
        val secondGrant = store.register(second, second.name, "application/zip", longIdentity)
        assertNotEquals(firstGrant.token, secondGrant.token)
        val restored = WorkspaceOpenFileGrantStore(directory, now = { 1_001L })
        assertEquals(first.path, restored.find(firstGrant.token)!!.canonicalPath)
        assertEquals(second.path, restored.find(secondGrant.token)!!.canonicalPath)
        assertEquals(longIdentity.size, restored.find(secondGrant.token)!!.identity.size)
        assertEquals(first.name, restored.find(secondGrant.token)!!.displayName)
    }

    @Test
    fun expiryAndClockRollbackRemoveOnlyMetadata() {
        val file = source()
        val bytes = file.readBytes()
        val directory = temporary.newFolder("grants")
        var clock = 1_000L
        val store = WorkspaceOpenFileGrantStore(directory, now = { clock })
        val permission = grant(store, file)
        clock = permission.expiresAt
        assertNull(store.find(permission.token))
        assertFalse(File(directory, "${permission.token}.json").exists())
        clock = 1_000L
        val next = grant(store, file)
        clock--
        assertNull(store.find(next.token))
        assertArrayEquals(bytes, file.readBytes())
        assertTrue(file.isFile)
    }

    @Test
    fun revocationDoesNotTouchTheSelectedFile() {
        val file = source()
        val directory = temporary.newFolder("grants")
        val store = WorkspaceOpenFileGrantStore(directory, now = { 1_000L })
        val permission = grant(store, file)
        store.revoke(permission.token)
        assertNull(WorkspaceOpenFileGrantStore(directory, now = { 1_001L }).find(permission.token))
        assertArrayEquals(byteArrayOf(0, 1, 2, 127, -1), file.readBytes())
    }

    @Test
    fun activeLimitIs128AndExpiredSlotsCanBeReused() {
        val file = source()
        val directory = temporary.newFolder("grants")
        var clock = 1_000L
        val store = WorkspaceOpenFileGrantStore(directory, now = { clock })
        val permissions = (0 until 128).map { grant(store, file) }
        assertEquals(128, permissions.map { it.token }.toSet().size)
        assertThrows(WorkspaceOpenFileGrantLimitException::class.java) { grant(store, file) }
        assertEquals(128, directory.listFiles()!!.size)
        clock = permissions.first().expiresAt
        val next = grant(store, file)
        assertNotNull(store.find(next.token))
        assertEquals(1, directory.listFiles()!!.size)
        assertTrue(file.isFile)
    }

    @Test
    fun separateConcurrentInstancesShareTheLimit() {
        val file = source()
        val directory = temporary.newFolder("grants")
        val workers = Executors.newFixedThreadPool(4)
        try {
            val results = workers.invokeAll((0 until 132).map {
                Callable {
                    try {
                        grant(WorkspaceOpenFileGrantStore(directory, now = { 1_000L }), file)
                        true
                    } catch (_: WorkspaceOpenFileGrantLimitException) {
                        false
                    }
                }
            }).map { it.get() }
            assertEquals(128, results.count { it })
            assertEquals(128, directory.listFiles()!!.size)
        } finally {
            workers.shutdownNow()
        }
    }

    @Test
    fun malformedTokensAndCorruptMetadataCannotEscapeTheStore() {
        val file = source()
        val bytes = file.readBytes()
        val directory = temporary.newFolder("grants")
        val store = WorkspaceOpenFileGrantStore(directory, now = { 1_000L })
        for (token in listOf("", "..", "../source/archive.zip", "a".repeat(31), "a".repeat(33),
            "A".repeat(32), "g".repeat(32), "%2e%2e", "/" + "a".repeat(32))) {
            assertNull(store.find(token))
            assertThrows(IllegalArgumentException::class.java) { store.revoke(token) }
        }
        val permission = grant(store, file)
        val metadata = File(directory, "${permission.token}.json")
        metadata.writeText("{")
        assertNull(store.find(permission.token))
        assertFalse(metadata.exists())
        val oversized = grant(store, file)
        File(directory, "${oversized.token}.json").writeText("x".repeat(16 * 1024 + 1))
        assertNull(store.find(oversized.token))
        assertArrayEquals(bytes, file.readBytes())
    }

    @Test
    fun metadataSymlinkCleanupNeverDeletesItsTarget() {
        assumeFalse(System.getProperty("os.name").lowercase().startsWith("windows"))
        val file = source()
        val bytes = file.readBytes()
        val directory = temporary.newFolder("grants")
        val store = WorkspaceOpenFileGrantStore(directory, now = { 1_000L })
        val permission = grant(store, file)
        val metadata = File(directory, "${permission.token}.json")
        assertTrue(metadata.delete())
        Files.createSymbolicLink(metadata.toPath(), file.toPath())
        assertNull(store.find(permission.token))
        assertFalse(Files.exists(metadata.toPath(), java.nio.file.LinkOption.NOFOLLOW_LINKS))
        assertArrayEquals(bytes, file.readBytes())
    }

    @Test
    fun interruptedMetadataIsCleanedAndMetadataCannotBeGranted() {
        val file = source()
        val directory = temporary.newFolder("grants")
        val interrupted = File(directory, "grant-interrupted.tmp").apply { writeText("partial metadata") }
        val store = WorkspaceOpenFileGrantStore(directory, now = { 1_000L })
        grant(store, file)
        assertFalse(interrupted.exists())
        val metadata = directory.listFiles()!!.single().canonicalFile
        assertThrows(IllegalArgumentException::class.java) { grant(store, metadata) }
        assertTrue(metadata.isFile)
        assertTrue(file.isFile)
    }

    @Test
    fun collisionRetriesAreBounded() {
        val file = source()
        val directory = temporary.newFolder("grants")
        val predictable = object : SecureRandom() {
            override fun nextBytes(bytes: ByteArray) { bytes.fill(0) }
        }
        val store = WorkspaceOpenFileGrantStore(directory, now = { 1_000L }, random = predictable)
        grant(store, file)
        assertThrows(IOException::class.java) { grant(store, file) }
        assertEquals(1, directory.listFiles()!!.size)
    }
}
