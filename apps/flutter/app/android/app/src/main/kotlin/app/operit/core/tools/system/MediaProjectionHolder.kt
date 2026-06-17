package app.operit.core.tools.system

import android.content.Context
import android.content.Intent
import android.media.projection.MediaProjection

object MediaProjectionHolder {
    var mediaProjection: MediaProjection? = null
    var permissionResultData: Intent? = null
    var permissionResultCode: Int = 0

    fun clear(context: Context) {
        try {
            mediaProjection?.stop()
        } catch (_: Exception) {
        }
        mediaProjection = null
        permissionResultData = null
        permissionResultCode = 0
        ScreenCaptureService.stop(context)
    }
}
