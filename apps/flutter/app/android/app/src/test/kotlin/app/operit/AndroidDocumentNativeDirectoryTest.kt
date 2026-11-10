package app.operit

import java.io.File
import java.nio.file.Files
import org.junit.Assert.assertEquals
import org.junit.Assert.assertThrows
import org.junit.Rule
import org.junit.Test
import org.junit.rules.TemporaryFolder

class AndroidDocumentNativeDirectoryTest {
    @get:Rule
    val temporary = TemporaryFolder()
    private val authority = "com.android.externalstorage.documents"

    @Test
    fun resolvesPrimaryTreeAndNestedUnicodeDirectories() {
        val volume = temporary.newFolder("primary")
        val root = File(volume, "Projects").apply { mkdirs() }
        val child = File(root, "项目 with spaces").apply { mkdirs() }
        val volumes = mapOf("primary" to volume)
        assertEquals(root.canonicalPath,
            AndroidDocumentNativeDirectory.resolve(authority, "primary:Projects", "primary:Projects", volumes))
        assertEquals(child.canonicalPath,
            AndroidDocumentNativeDirectory.resolve(authority, "primary:Projects", "primary:Projects/项目 with spaces", volumes))
    }

    @Test
    fun resolvesRemovableStorageAndVolumeRoot() {
        val card = temporary.newFolder("card")
        val child = File(card, "Projects").apply { mkdirs() }
        val volumes = mapOf("ABCD-1234" to card)
        assertEquals(card.canonicalPath,
            AndroidDocumentNativeDirectory.resolve(authority, "ABCD-1234:", "ABCD-1234:", volumes))
        assertEquals(child.canonicalPath,
            AndroidDocumentNativeDirectory.resolve(authority, "ABCD-1234:", "abcd-1234:Projects", volumes))
    }

    @Test
    fun refusesOpaqueProvidersMissingVolumesAndNonDirectories() {
        val volume = temporary.newFolder("primary")
        File(volume, "file").writeText("data")
        val volumes = mapOf("primary" to volume)
        for (provider in listOf<String?>(null, "cloud.documents", "com.android.externalstorage.documents.other")) {
            assertThrows(IllegalArgumentException::class.java) {
                AndroidDocumentNativeDirectory.resolve(provider, "primary:", "primary:", volumes)
            }
        }
        for (id in listOf("missing:Projects", "opaque-id", "primary:missing", "primary:file")) {
            assertThrows(IllegalArgumentException::class.java) {
                AndroidDocumentNativeDirectory.resolve(authority, "primary:", id, volumes)
            }
        }
    }

    @Test
    fun refusesTraversalSiblingsAndDifferentVolumes() {
        val volume = temporary.newFolder("primary")
        File(volume, "Projects").mkdirs()
        File(volume, "Projects-other").mkdirs()
        val card = temporary.newFolder("card")
        val volumes = mapOf("primary" to volume, "ABCD-1234" to card)
        for (id in listOf("primary:Projects/../Projects-other", "primary:/Projects", "primary:Projects//child",
            "primary:Projects/./child", "primary:Projects-other", "primary:", "ABCD-1234:")) {
            assertThrows(IllegalArgumentException::class.java) {
                AndroidDocumentNativeDirectory.resolve(authority, "primary:Projects", id, volumes)
            }
        }
    }

    @Test
    fun refusesSymlinksEscapingTreeAndVolume() {
        val volume = temporary.newFolder("primary")
        val root = File(volume, "Projects").apply { mkdirs() }
        val sibling = File(volume, "Sibling").apply { mkdirs() }
        val outside = temporary.newFolder("outside")
        Files.createSymbolicLink(File(root, "sibling-link").toPath(), sibling.toPath())
        Files.createSymbolicLink(File(root, "outside-link").toPath(), outside.toPath())
        for (id in listOf("primary:Projects/sibling-link", "primary:Projects/outside-link")) {
            assertThrows(IllegalArgumentException::class.java) {
                AndroidDocumentNativeDirectory.resolve(authority, "primary:Projects", id, mapOf("primary" to volume))
            }
        }
    }
}
