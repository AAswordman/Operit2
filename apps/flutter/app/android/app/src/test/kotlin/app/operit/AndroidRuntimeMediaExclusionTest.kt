package app.operit

import java.io.File
import java.io.IOException
import java.nio.file.Files
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertThrows
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class AndroidRuntimeMediaExclusionTest {
    @get:Rule
    val temporary = TemporaryFolder()

    @Test
    fun marksExistingAndNewIdentityDirectoriesThroughSdcardAlias() {
        val volume = temporary.newFolder("emulated")
        val alias = File(temporary.root, "sdcard")
        Files.createSymbolicLink(alias.toPath(), volume.toPath())
        val runtime = File(alias, "Download/identities/identity-runtime").apply { mkdirs() }
        val workspace = File(volume, "Documents/identities/identity-workspace").apply { mkdirs() }
        File(workspace, "existing.png").writeText("existing workspace data")

        for (root in listOf(runtime, workspace)) {
            AndroidRuntimeMediaExclusion.ensureMarker(root, listOf(volume))
            val marker = File(root, ".nomedia")
            assertTrue(marker.isFile)
            assertEquals(0L, marker.length())
            assertFalse(File(root.parentFile, ".nomedia").exists())
            assertFalse(File(root.parentFile.parentFile, ".nomedia").exists())
        }
        assertFalse(File(volume, ".nomedia").exists())
        assertEquals("existing workspace data", File(workspace, "existing.png").readText())
    }

    @Test
    fun preservesExistingMarker() {
        val volume = temporary.newFolder("ABCD-1234")
        val root = File(volume, "Operit/identities/identity-new").apply { mkdirs() }
        val marker = File(root, ".nomedia").apply { writeText("existing marker") }

        repeat(2) { AndroidRuntimeMediaExclusion.ensureMarker(root, listOf(volume)) }

        assertEquals("existing marker", marker.readText())
    }

    @Test
    fun skipsPublicParentsAndDirectoriesOutsideSharedStorage() {
        val volume = temporary.newFolder("sdcard")
        val outside = temporary.newFolder("sdcard-other")
        val roots = listOf(
            volume,
            File(volume, "DCIM").apply { mkdirs() },
            File(outside, "runtime/identities/identity-private").apply { mkdirs() },
        )
        for (root in roots) {
            AndroidRuntimeMediaExclusion.ensureMarker(root, listOf(volume))
            assertFalse(File(root, ".nomedia").exists())
        }
    }

    @Test
    fun doesNotFollowAnIdentitySymlinkIntoPublicStorage() {
        val volume = temporary.newFolder("sdcard")
        val public = File(volume, "Pictures/Family").apply { mkdirs() }
        val identities = File(volume, "Operit/identities").apply { mkdirs() }
        val root = File(identities, "identity-linked")
        Files.createSymbolicLink(root.toPath(), public.toPath())

        AndroidRuntimeMediaExclusion.ensureMarker(root, listOf(volume))

        assertFalse(File(public, ".nomedia").exists())
    }

    @Test
    fun reportsAnUnwritableMarkerWithoutReplacingIt() {
        val volume = temporary.newFolder("sdcard")
        val root = File(volume, "Operit/identities/identity-new").apply { mkdirs() }
        val conflict = File(root, ".nomedia").apply { mkdir() }

        assertThrows(IOException::class.java) {
            AndroidRuntimeMediaExclusion.ensureMarker(root, listOf(volume))
        }
        assertTrue(conflict.isDirectory)
    }
}
