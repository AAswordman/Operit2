package app.operit

import android.app.Activity
import android.content.ComponentName
import android.content.pm.PackageInfo
import android.content.pm.PackageManager
import org.junit.Assert.*
import org.junit.Test
import org.mockito.Mockito.*

/** Exercises provider version decisions using the Android host's actual PackageManager adapter. */
class AndroidAccessibilitySetupTest {
    /** Supplies an installed provider version without opening any Android authorization UI. */
    private fun withInstalledVersion(version: Int?, test: (AndroidAccessibilitySetup) -> Unit) {
        mockConstruction(ComponentName::class.java).use {
            val activity = mock(Activity::class.java)
            val manager = mock(PackageManager::class.java)
            `when`(activity.packageManager).thenReturn(manager)
            if (version == null) {
                `when`(manager.getPackageInfo("app.operit.accessibility", 0))
                    .thenThrow(mock(PackageManager.NameNotFoundException::class.java))
            } else {
                val info = mock(PackageInfo::class.java)
                info.versionCode = version
                `when`(manager.getPackageInfo("app.operit.accessibility", 0)).thenReturn(info)
            }
            test(AndroidAccessibilitySetup(activity))
        }
    }

    /** Requires installation when the independent provider is absent. */
    @Test
    fun missingProviderRequiresInstallation() = withInstalledVersion(null) {
        assertFalse(it.hasCurrentCompanion())
    }

    /** Requires an update even when the old provider version exceeds the main app's base version. */
    @Test
    fun olderProviderUsesProviderRequirementInsteadOfMainAppVersion() =
        withInstalledVersion(BuildConfig.OPERIT_ACCESSIBILITY_VERSION_CODE - 1) {
            assertFalse(it.hasCurrentCompanion())
        }

    /** Allows the exact required provider version without reinstalling. */
    @Test
    fun currentProviderIsAccepted() = withInstalledVersion(BuildConfig.OPERIT_ACCESSIBILITY_VERSION_CODE) {
        assertTrue(it.hasCurrentCompanion())
    }

    /** Preserves a newer independently installed provider instead of forcing a downgrade. */
    @Test
    fun newerProviderIsAccepted() = withInstalledVersion(BuildConfig.OPERIT_ACCESSIBILITY_VERSION_CODE + 1) {
        assertTrue(it.hasCurrentCompanion())
    }
}
