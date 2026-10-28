package app.operit

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

/**
 * JVM unit tests for the ABI that [AndroidRuntimeAssets] picks before installing
 * the packaged rootfs.
 *
 * The device ABI list is not enough on its own: an x86_64 build reports the arm64
 * translation ABI as a secondary entry, and an arm64-only APK reports arm64 while
 * an x86_64 APK reports both. The packaged asset listing decides, so these tests
 * pin selection order and the failure message. Installing the assets themselves
 * still needs a device.
 */
class AndroidRuntimeAssetsTest {
    // An x86_64 device also reports arm64-v8a through the translation layer.
    private val x86DeviceAbis = listOf("x86_64", "arm64-v8a")

    @Test
    fun selectsDeviceAbiWhoseRuntimeAssetsArePackaged() {
        assertEquals(
            "x86_64",
            AndroidRuntimeAssets.selectPackagedAbi(x86DeviceAbis, setOf("x86_64")),
        )
    }

    @Test
    fun fallsBackToTranslatedAbiWhenOnlyItsAssetsArePackaged() {
        assertEquals(
            "arm64-v8a",
            AndroidRuntimeAssets.selectPackagedAbi(x86DeviceAbis, setOf("arm64-v8a")),
        )
    }

    @Test
    fun followsDeviceAbiPreferenceWhenSeveralArePackaged() {
        assertEquals(
            "x86_64",
            AndroidRuntimeAssets.selectPackagedAbi(
                x86DeviceAbis,
                setOf("arm64-v8a", "x86_64"),
            ),
        )
    }

    @Test
    fun reportsSupportedAndPackagedAbisWhenNothingMatches() {
        val error = runCatching {
            AndroidRuntimeAssets.selectPackagedAbi(listOf("x86_64"), setOf("arm64-v8a"))
        }.exceptionOrNull()

        assertTrue("expected an ABI error, got $error", error is IllegalArgumentException)
        val message = error?.message.orEmpty()
        assertTrue(message, message.contains("Unsupported Android ABI: x86_64"))
        assertTrue(message, message.contains("packaged runtime assets: arm64-v8a"))
    }
}
