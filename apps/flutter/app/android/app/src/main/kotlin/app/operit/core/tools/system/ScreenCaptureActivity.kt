package app.operit.core.tools.system

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.media.projection.MediaProjectionManager
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import app.operit.AndroidClientLogger

class ScreenCaptureActivity : Activity() {
    companion object {
        private const val TAG = "ScreenCaptureActivity"
        private const val REQUEST_CODE_CAPTURE = 1001

        fun cleanStart(context: Context) {
            val intent = Intent(context, ScreenCaptureActivity::class.java).apply {
                addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TOP)
            }
            context.startActivity(intent)
        }
    }

    private lateinit var mediaProjectionManager: MediaProjectionManager

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        mediaProjectionManager = getSystemService(Context.MEDIA_PROJECTION_SERVICE) as MediaProjectionManager
        startActivityForResult(mediaProjectionManager.createScreenCaptureIntent(), REQUEST_CODE_CAPTURE)
    }

    @Suppress("DEPRECATION")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode == REQUEST_CODE_CAPTURE) {
            if (resultCode == Activity.RESULT_OK && data != null) {
                AndroidClientLogger.d(applicationContext, TAG, "Screen capture permission granted")
                MediaProjectionHolder.permissionResultCode = resultCode
                MediaProjectionHolder.permissionResultData = data

                ScreenCaptureService.start(this)

                val handler = Handler(Looper.getMainLooper())
                val startAt = SystemClock.uptimeMillis()
                val checkIntervalMs = 30L
                val timeoutMs = 1500L

                val runnable = object : Runnable {
                    override fun run() {
                        val elapsed = SystemClock.uptimeMillis() - startAt
                        val isReady = ScreenCaptureService.isMediaProjectionForegroundReady

                        if (isReady || elapsed >= timeoutMs) {
                            try {
                                MediaProjectionHolder.mediaProjection =
                                    mediaProjectionManager.getMediaProjection(resultCode, data)
                            } catch (error: SecurityException) {
                                AndroidClientLogger.e(
                                    applicationContext,
                                    TAG,
                                    "Failed to obtain MediaProjection (FGS mediaProjection not ready): ${error.message.orEmpty()}",
                                )
                                MediaProjectionHolder.clear(this@ScreenCaptureActivity)
                            } catch (error: Exception) {
                                AndroidClientLogger.e(
                                    applicationContext,
                                    TAG,
                                    "Failed to obtain MediaProjection: ${error.message.orEmpty()}",
                                )
                                MediaProjectionHolder.clear(this@ScreenCaptureActivity)
                            } finally {
                                finish()
                            }
                            return
                        }

                        handler.postDelayed(this, checkIntervalMs)
                    }
                }

                handler.post(runnable)
                return
            } else {
                AndroidClientLogger.d(applicationContext, TAG, "Screen capture permission denied or cancelled")
                MediaProjectionHolder.clear(this)
            }
        }
        finish()
    }
}
