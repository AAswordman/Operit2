package app.operit

import android.content.ActivityNotFoundException
import android.content.ContentValues
import android.content.Context
import android.content.ContextWrapper
import android.content.Intent
import android.content.pm.PackageManager
import android.net.Uri
import android.os.Bundle
import android.os.ParcelFileDescriptor
import android.os.Process
import android.os.SystemClock
import android.provider.DocumentsContract
import android.provider.OpenableColumns
import android.system.ErrnoException
import android.system.Os
import android.system.OsConstants
import android.util.Log
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import java.io.File
import java.io.FileOutputStream
import java.io.OutputStream
import java.io.RandomAccessFile
import java.security.DigestOutputStream
import java.security.MessageDigest
import java.util.UUID
import java.util.zip.CRC32
import java.util.zip.ZipEntry
import java.util.zip.ZipFile
import java.util.zip.ZipOutputStream
import org.json.JSONObject
import org.junit.After
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith

/**
 * Run on a disposable device installation. Only this class's fixtures and grants are removed.
 * The default STORED ZIP is 1 MiB; use instrumentation argument workspaceZipMiB=64/512/2048/4096
 * to create real, fully written large archives (2 GiB and above explicitly use ZIP64).
 * SAF checks require safTreeUri/safDocumentId arguments and a pre-existing app read grant.
 * A reconstructed store proves persisted metadata; process death and external ZIP viewers
 * still need the manual lifecycle/regression matrix. The sparse fixture only tests FD offsets.
 */
@RunWith(AndroidJUnit4::class)
class WorkspaceOpenFileProviderTest {
    private lateinit var context: Context
    private lateinit var fixtures: File
    private val grants = mutableSetOf<Uri>()
    private val legacyDirectories = mutableSetOf<File>()

    @Before
    fun setUp() {
        context = InstrumentationRegistry.getInstrumentation().targetContext
        fixtures = File(context.cacheDir, "workspace-provider-test-${UUID.randomUUID()}").apply {
            check(mkdirs())
        }
    }

    @After
    fun tearDown() {
        for (uri in grants) {
            runCatching { context.revokeUriPermission(uri, Intent.FLAG_GRANT_READ_URI_PERMISSION) }
            runCatching { WorkspaceOpenFileProvider.revoke(context, uri) }
        }
        for (directory in legacyDirectories) directory.deleteRecursively()
        fixtures.deleteRecursively()
    }

    @Test
    fun realFileDescriptorsAreReadOnlyIndependentlySeekableAndKeepLongSizes() {
        // Sparse, non-ZIP data tests 64-bit descriptor offsets; it is not a performance fixture.
        val file = File(fixtures, "大文件 with spaces.bin")
        val offset = 2L * 1024 * 1024 * 1024 + 17
        val head = byteArrayOf(10, 11, 12, 13)
        val tail = byteArrayOf(20, 21, 22, 23)
        RandomAccessFile(file, "rw").use {
            it.setLength(offset + tail.size)
            it.write(head)
            it.seek(offset)
            it.write(tail)
        }
        val uri = register(file)
        context.contentResolver.query(uri, null, null, null, null).use { cursor ->
            assertNotNull(cursor)
            checkNotNull(cursor)
            assertTrue(cursor.moveToFirst())
            assertEquals(file.name, cursor.getString(cursor.getColumnIndexOrThrow(OpenableColumns.DISPLAY_NAME)))
            assertEquals(offset + tail.size, cursor.getLong(cursor.getColumnIndexOrThrow(OpenableColumns.SIZE)))
            assertFalse(cursor.moveToNext())
        }
        assertEquals("application/octet-stream", context.contentResolver.getType(uri))
        open(uri).use { first ->
            open(uri).use { second ->
                assertTrue(OsConstants.S_ISREG(Os.fstat(first.fileDescriptor).st_mode))
                assertEquals(OsConstants.O_RDONLY,
                    Os.fcntlInt(first.fileDescriptor, OsConstants.F_GETFL, 0) and OsConstants.O_ACCMODE)
                assertEquals(offset, Os.lseek(first.fileDescriptor, offset, OsConstants.SEEK_SET))
                assertArrayEquals(tail, readExactly(first, tail.size))
                assertEquals(0L, Os.lseek(second.fileDescriptor, 0, OsConstants.SEEK_CUR))
                assertArrayEquals(head, readExactly(second, head.size))
                Os.lseek(first.fileDescriptor, 0, OsConstants.SEEK_SET)
                assertArrayEquals(head, readExactly(first, head.size))
                val error = failure { Os.write(first.fileDescriptor, byteArrayOf(99), 0, 1) }
                assertTrue(error is ErrnoException)
            }
        }
        for (mode in listOf("w", "wa", "wt", "rw", "rwt")) {
            failure { context.contentResolver.openFileDescriptor(uri, mode)?.use { } }
        }
    }

    @Test
    fun exactUriReadGrantIsEnforcedForTheDifferentShellUid() {
        val firstBytes = "workspace-grant-first\n".toByteArray()
        val secondBytes = "workspace-grant-second\n".toByteArray()
        val first = register(File(fixtures, "first.txt").apply { writeBytes(firstBytes) })
        val second = register(File(fixtures, "second.txt").apply { writeBytes(secondBytes) })
        val shellUid = context.packageManager.getApplicationInfo("com.android.shell", 0).uid
        assertNotEquals(Process.myUid(), shellUid)
        assertFalse(shellRead(first).contentEquals(firstBytes))
        context.grantUriPermission("com.android.shell", first, Intent.FLAG_GRANT_READ_URI_PERMISSION)
        // A successful real shell read also establishes that the content CLI is available;
        // an unavailable CLI cannot make the denial checks below falsely pass this test.
        assertArrayEquals(firstBytes, shellRead(first))
        assertEquals(PackageManager.PERMISSION_GRANTED,
            context.checkUriPermission(first, -1, shellUid, Intent.FLAG_GRANT_READ_URI_PERMISSION))
        assertEquals(PackageManager.PERMISSION_DENIED,
            context.checkUriPermission(first, -1, shellUid, Intent.FLAG_GRANT_WRITE_URI_PERMISSION))
        assertFalse(shellRead(second).contentEquals(secondBytes))
        context.revokeUriPermission(first, Intent.FLAG_GRANT_READ_URI_PERMISSION)
        assertFalse(shellRead(first).contentEquals(firstBytes))
        assertArrayEquals(firstBytes, File(fixtures, "first.txt").readBytes())
        assertArrayEquals(secondBytes, File(fixtures, "second.txt").readBytes())
    }

    @Test
    fun rejectsDirectoriesMalformedUrisAndAllMutationApis() {
        failure { WorkspaceOpenFileProvider.register(context, fixtures, "application/octet-stream") }
        val file = File(fixtures, "single.bin").apply { writeBytes(byteArrayOf(1, 2, 3)) }
        val uri = register(file)
        val token = checkNotNull(uri.lastPathSegment)
        val prefix = "content://${WorkspaceOpenFileProvider.authority(context)}"
        val uppercaseToken = "F".repeat(32)
        for (bad in listOf(
            prefix, "$prefix/", "$prefix/..", "$prefix/$uppercaseToken",
            "$prefix/${token}a", "$prefix/${token}/other", "$prefix/%2e%2e/$token",
            "$prefix/%${token.take(2)}/${token.drop(2)}", "$prefix/$token?path=other", "$prefix/$token#other",
        )) {
            failure { context.contentResolver.openFileDescriptor(Uri.parse(bad), "r")?.use { } }
        }
        failure { context.contentResolver.insert(uri, ContentValues().apply { put("path", file.path) }) }
        failure { context.contentResolver.update(uri, ContentValues(), null, null) }
        failure { context.contentResolver.delete(uri, null, null) }
        failure { context.contentResolver.call(uri, "register", file.path, Bundle()) }
        assertArrayEquals(byteArrayOf(1, 2, 3), file.readBytes())
    }

    @Test
    fun persistentSameNamedGrantsStaySeparateAndExpiryOnlyRemovesMetadata() {
        val firstFile = File(File(fixtures, "one").apply { mkdir() }, "同名.zip").apply { writeText("first") }
        val secondFile = File(File(fixtures, "two").apply { mkdir() }, "同名.zip").apply { writeText("second") }
        val first = register(firstFile)
        val second = register(secondFile)
        val firstToken = checkNotNull(first.lastPathSegment)
        val secondToken = checkNotNull(second.lastPathSegment)
        val directory = File(context.noBackupFilesDir, "workspace_open_grants")
        val loaded = WorkspaceOpenFileGrantStore(directory)
        assertEquals(firstFile.canonicalPath, checkNotNull(loaded.find(firstToken)).canonicalPath)
        assertEquals(secondFile.canonicalPath, checkNotNull(loaded.find(secondToken)).canonicalPath)
        assertNotEquals(firstToken, secondToken)
        open(first).use { assertArrayEquals("first".toByteArray(), readExactly(it, 5)) }
        open(second).use { assertArrayEquals("second".toByteArray(), readExactly(it, 6)) }
        open(first).use { alreadyOpen ->
            val metadata = File(directory, "$firstToken.json")
            val now = System.currentTimeMillis()
            val expired = JSONObject(metadata.readText())
                .put("issuedAt", now - WorkspaceOpenFileGrantStore.TTL_MILLIS - 1)
                .put("expiresAt", now - 1)
            metadata.writeText(expired.toString())
            failure { context.contentResolver.openFileDescriptor(first, "r")?.use { } }
            assertFalse(metadata.exists())
            assertArrayEquals("first".toByteArray(), readExactly(alreadyOpen, 5))
        }
        assertTrue(firstFile.exists())
        assertTrue(secondFile.exists())
    }

    @Test
    fun replacingPathOrRevokingTokenRejectsNewOpensWithoutRecallingDeliveredFd() {
        val file = File(fixtures, "replace.bin").apply { writeText("original") }
        val uri = register(file)
        open(uri).use { delivered ->
            val retainedInode = File(fixtures, "original-inode.bin")
            assertTrue(file.renameTo(retainedInode))
            file.writeText("replacement")
            failure { context.contentResolver.openFileDescriptor(uri, "r")?.use { } }
            WorkspaceOpenFileProvider.revoke(context, uri)
            failure { context.contentResolver.openFileDescriptor(uri, "r")?.use { } }
            assertArrayEquals("original".toByteArray(), readExactly(delivered, 8))
            assertEquals("replacement", file.readText())
            assertEquals("original", retainedInode.readText())
        }
        val linked = File(fixtures, "link-source.bin").apply { writeText("selected") }
        val linkedUri = register(linked)
        val ungranted = File(fixtures, "ungranted.bin").apply { writeText("ungranted") }
        assertTrue(linked.renameTo(File(fixtures, "retained-link-source.bin")))
        Os.symlink(ungranted.path, linked.path)
        failure { context.contentResolver.openFileDescriptor(linkedUri, "r")?.use { } }
        assertEquals("ungranted", ungranted.readText())
    }

    @Test
    fun failedWorkspaceLaunchRevokesUndeliveredGrantAndRedactsIntentAndPhysicalPath() {
        val source = File(fixtures, "failure.zip").apply { writeText("test data") }
        val capture = CapturingContext(context, failLaunch = true)
        val error = failure { AndroidFileOpener(capture).openWorkspaceFile(source, null, null) }
        val firstUri = checkNotNull(capture.lastIntent?.data).also { grants.add(it) }
        assertNoCapability(error.toString(), firstUri, source)
        assertNull(error.cause)
        failure { context.contentResolver.openFileDescriptor(firstUri, "r")?.use { } }
        val filesystem = AndroidDocumentFileSystem(
            ContentResolverDocumentTreeAccess(context),
            fileOpener = AndroidFileOpener(capture)::openWorkspaceFile,
        )
        val result = JSONObject(filesystem.execute(JSONObject()
            .put("operation", "openFile").put("path", source.absolutePath).toString()))
        assertFalse(result.getBoolean("ok"))
        val secondUri = checkNotNull(capture.lastIntent?.data).also { grants.add(it) }
        assertNoCapability(result.getString("error"), secondUri, source)
        failure { context.contentResolver.openFileDescriptor(secondUri, "r")?.use { } }
        val missing = File(fixtures, "missing-sensitive-path.zip")
        val missingResult = JSONObject(filesystem.execute(JSONObject()
            .put("operation", "openFile").put("path", missing.absolutePath).toString()))
        assertFalse(missingResult.getString("error").contains(missing.absolutePath))
        assertEquals("test data", source.readText())
    }

    @Test
    fun validStoredZipHandoffDoesNotStageBytesAndTheGrantedFdOpensItsDirectory() {
        val argument = InstrumentationRegistry.getArguments().getString("workspaceZipMiB")
        val sizeMiB = if (argument == null) 1L else requireNotNull(argument.toLongOrNull()) { "Invalid workspaceZipMiB" }
        require(sizeMiB in setOf(1L, 64L, 512L, 2048L, 4096L)) { "Unsupported workspaceZipMiB" }
        val payloadBytes = sizeMiB * 1024 * 1024
        assumeTrue("Large fixtures require enough device storage for source and baseline staging",
            fixtures.usableSpace > payloadBytes * 2 + 64L * 1024 * 1024)
        val zip = File(fixtures, "stored-$sizeMiB.zip")
        val fixture = storedZip(zip, payloadBytes)
        val stagedBefore = openFilesSnapshot()
        val capture = CapturingContext(context)
        val start = SystemClock.elapsedRealtime()
        AndroidFileOpener(capture).openWorkspaceFile(zip, null, null)
        val handoffMs = SystemClock.elapsedRealtime() - start
        val intent = checkNotNull(capture.lastIntent)
        val uri = checkNotNull(intent.data).also { grants.add(it) }
        assertEquals(WorkspaceOpenFileProvider.authority(context), uri.authority)
        assertEquals(Intent.ACTION_VIEW, intent.action)
        assertEquals("application/zip", intent.type)
        assertEquals(uri, checkNotNull(intent.clipData).getItemAt(0).uri)
        assertEquals(1, checkNotNull(intent.clipData).itemCount)
        assertTrue(intent.flags and Intent.FLAG_GRANT_READ_URI_PERMISSION != 0)
        assertEquals(0, intent.flags and (Intent.FLAG_GRANT_WRITE_URI_PERMISSION or Intent.FLAG_GRANT_PREFIX_URI_PERMISSION))
        assertEquals(stagedBefore, openFilesSnapshot())
        assertTrue("URI handoff exceeded the approved 5 second target", handoffMs <= 5000)
        open(uri).use { descriptor ->
            ZipFile("/proc/self/fd/${descriptor.fd}").use { archive ->
                val entry = checkNotNull(archive.getEntry("payload.bin"))
                assertEquals(payloadBytes, entry.size)
                assertEquals(ZipEntry.STORED, entry.method)
                val crc = CRC32()
                val buffer = ByteArray(64 * 1024)
                archive.getInputStream(entry).use { input ->
                    while (true) {
                        val count = input.read(buffer)
                        if (count < 0) break
                        crc.update(buffer, 0, count)
                    }
                }
                assertEquals(fixture.first, crc.value)
                assertEquals(fixture.first, entry.crc)
            }
        }
        Log.i("WorkspaceOpenFileProviderTest", "fixtureBytes=${zip.length()} payloadBytes=$payloadBytes " +
            "crc=${fixture.first} sha256=${fixture.second} handoffMs=$handoffMs")
    }

    @Test
    fun legacyOpenAndShareRetainTheConfiguredOpenfilesAuthority() {
        val file = File(fixtures, "legacy.zip").apply { writeText("legacy snapshot") }
        val capture = CapturingContext(context)
        val opener = AndroidFileOpener(capture)
        opener.open(file, null, null)
        val openIntent = checkNotNull(capture.lastIntent)
        val openUri = checkNotNull(openIntent.data)
        trackLegacy(openUri)
        assertEquals("${context.packageName}.openfiles", openUri.authority)
        context.contentResolver.openInputStream(openUri).use {
            assertArrayEquals(file.readBytes(), checkNotNull(it).readBytes())
        }
        opener.share(file, null, null, "Share test")
        val chooser = checkNotNull(capture.lastIntent)
        assertEquals(Intent.ACTION_CHOOSER, chooser.action)
        @Suppress("DEPRECATION")
        val shareIntent = checkNotNull(chooser.getParcelableExtra<Intent>(Intent.EXTRA_INTENT))
        @Suppress("DEPRECATION")
        val sharedUri = checkNotNull(shareIntent.getParcelableExtra<Uri>(Intent.EXTRA_STREAM))
        trackLegacy(sharedUri)
        assertEquals(Intent.ACTION_SEND, shareIntent.action)
        assertEquals("${context.packageName}.openfiles", sharedUri.authority)
        assertEquals(sharedUri, checkNotNull(shareIntent.clipData).getItemAt(0).uri)
        context.contentResolver.openInputStream(sharedUri).use {
            assertArrayEquals(file.readBytes(), checkNotNull(it).readBytes())
        }
    }

    @Test
    fun safDocumentUriIsPreservedWhenAnExistingCapabilityIsProvided() {
        val arguments = InstrumentationRegistry.getArguments()
        val tree = arguments.getString("safTreeUri")
        val document = arguments.getString("safDocumentId")
        assumeTrue("SAF regression needs a real document and the app's existing tree read grant",
            tree != null && document != null)
        val expected = DocumentsContract.buildDocumentUriUsingTree(Uri.parse(checkNotNull(tree)), checkNotNull(document))
        val capture = CapturingContext(context)
        AndroidFileOpener(capture).openWorkspaceFile(null, tree, document)
        val intent = checkNotNull(capture.lastIntent)
        assertEquals(expected, intent.data)
        assertEquals(expected, checkNotNull(intent.clipData).getItemAt(0).uri)
        assertTrue(intent.flags and Intent.FLAG_GRANT_READ_URI_PERMISSION != 0)
        assertEquals(0, intent.flags and (Intent.FLAG_GRANT_WRITE_URI_PERMISSION or Intent.FLAG_GRANT_PREFIX_URI_PERMISSION))
    }

    private fun register(file: File): Uri =
        WorkspaceOpenFileProvider.register(context, file, "application/octet-stream").also { grants.add(it) }

    private fun open(uri: Uri): ParcelFileDescriptor =
        checkNotNull(context.contentResolver.openFileDescriptor(uri, "r"))

    private fun readExactly(descriptor: ParcelFileDescriptor, length: Int): ByteArray {
        val result = ByteArray(length)
        var offset = 0
        while (offset < length) {
            val count = Os.read(descriptor.fileDescriptor, result, offset, length - offset)
            check(count > 0) { "Unexpected descriptor EOF" }
            offset += count
        }
        return result
    }

    private fun shellRead(uri: Uri): ByteArray {
        val pipe = InstrumentationRegistry.getInstrumentation().uiAutomation
            .executeShellCommand("content read --uri $uri")
        return ParcelFileDescriptor.AutoCloseInputStream(pipe).use { it.readBytes() }
    }

    private fun failure(action: () -> Unit): Exception {
        try {
            action()
        } catch (error: Exception) {
            return error
        }
        throw AssertionError("Operation unexpectedly succeeded")
    }

    private fun assertNoCapability(message: String, uri: Uri, source: File) {
        assertFalse(message.contains(uri.toString()))
        assertFalse(message.contains(checkNotNull(uri.lastPathSegment)))
        assertFalse(message.contains(source.canonicalPath))
    }

    private fun openFilesSnapshot(): Set<String> =
        File(context.cacheDir, "open_files").list()?.toSet() ?: emptySet()

    private fun trackLegacy(uri: Uri) {
        val segments = uri.pathSegments
        check(segments.size == 3 && segments[0] == "open_files")
        UUID.fromString(segments[1])
        legacyDirectories.add(File(File(context.cacheDir, "open_files"), segments[1]))
    }

    private class CapturingContext(base: Context, private val failLaunch: Boolean = false) : ContextWrapper(base) {
        var lastIntent: Intent? = null
        override fun getApplicationContext(): Context = this
        override fun startActivity(intent: Intent) {
            lastIntent = intent
            if (failLaunch) throw ActivityNotFoundException("No Activity found to handle $intent")
        }
    }

    /** Returns entry CRC and SHA-256 of the complete, physically written ZIP file. */
    private fun storedZip(file: File, payloadBytes: Long): Pair<Long, String> {
        val block = ByteArray(1024 * 1024)
        val checksum = CRC32()
        var remaining = payloadBytes
        while (remaining > 0) {
            val count = minOf(remaining, block.size.toLong()).toInt()
            checksum.update(block, 0, count)
            remaining -= count
        }
        val digest = MessageDigest.getInstance("SHA-256")
        DigestOutputStream(FileOutputStream(file), digest).use { output ->
            if (payloadBytes >= 2L * 1024 * 1024 * 1024) {
                writeZip64(output, payloadBytes, block, checksum.value)
            } else {
                ZipOutputStream(output).use { archive ->
                    val entry = ZipEntry("payload.bin").apply {
                        method = ZipEntry.STORED
                        size = payloadBytes
                        compressedSize = payloadBytes
                        crc = checksum.value
                    }
                    archive.putNextEntry(entry)
                    writePayload(archive, payloadBytes, block)
                    archive.closeEntry()
                }
            }
        }
        val sha = digest.digest().joinToString("") { (it.toInt() and 0xff).toString(16).padStart(2, '0') }
        return Pair(checksum.value, sha)
    }

    /** Explicit ZIP64 STORED fixture; 2 GiB is below the automatic ZIP64 threshold of Java's writer. */
    private fun writeZip64(output: OutputStream, bytes: Long, block: ByteArray, crc: Long) {
        val name = "payload.bin".toByteArray(Charsets.UTF_8)
        fun short(value: Int) { repeat(2) { output.write(value ushr (it * 8) and 0xff) } }
        fun int(value: Long) { repeat(4) { output.write((value ushr (it * 8)).toInt() and 0xff) } }
        fun long(value: Long) { repeat(8) { output.write((value ushr (it * 8)).toInt() and 0xff) } }
        fun sizes() { short(1); short(16); long(bytes); long(bytes) }
        int(0x04034b50L); short(45); short(0); short(0); short(0); short(0)
        int(crc); int(0xffff_ffffL); int(0xffff_ffffL); short(name.size); short(20)
        output.write(name); sizes(); writePayload(output, bytes, block)
        val directoryOffset = 30L + name.size + 20 + bytes
        val directorySize = 46L + name.size + 20
        int(0x02014b50L); short(45); short(45); short(0); short(0); short(0); short(0)
        int(crc); int(0xffff_ffffL); int(0xffff_ffffL); short(name.size); short(20)
        short(0); short(0); short(0); int(0); int(0)
        output.write(name); sizes()
        int(0x06064b50L); long(44); short(45); short(45); int(0); int(0)
        long(1); long(1); long(directorySize); long(directoryOffset)
        int(0x07064b50L); int(0); long(directoryOffset + directorySize); int(1)
        int(0x06054b50L); short(0); short(0); short(0xffff); short(0xffff)
        int(0xffff_ffffL); int(0xffff_ffffL); short(0)
    }

    private fun writePayload(output: OutputStream, bytes: Long, block: ByteArray) {
        var remaining = bytes
        while (remaining > 0) {
            val count = minOf(remaining, block.size.toLong()).toInt()
            output.write(block, 0, count)
            remaining -= count
        }
    }
}
