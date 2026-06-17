package app.operit.core.tools.system

import android.content.Context
import android.graphics.Bitmap
import android.graphics.PixelFormat
import android.hardware.display.DisplayManager
import android.hardware.display.VirtualDisplay
import android.media.Image
import android.media.ImageReader
import android.media.projection.MediaProjection
import android.os.Handler
import android.os.Looper
import android.util.DisplayMetrics
import android.view.WindowManager
import app.operit.AndroidClientLogger
import java.io.File
import java.io.FileOutputStream

class MediaProjectionCaptureManager(
    private val context: Context,
    private val mediaProjection: MediaProjection,
) {
    companion object {
        private const val TAG = "MediaProjectionCapture"
    }

    private var virtualDisplay: VirtualDisplay? = null
    private var imageReader: ImageReader? = null

    private val callbackHandler = Handler(Looper.getMainLooper())
    private var projectionCallback: MediaProjection.Callback? = null

    fun setupDisplay() {
        if (virtualDisplay != null) return

        try {
            ensureProjectionCallbackRegistered()

            val windowManager = context.getSystemService(Context.WINDOW_SERVICE) as WindowManager
            val metrics = DisplayMetrics()
            @Suppress("DEPRECATION")
            windowManager.defaultDisplay.getRealMetrics(metrics)

            val width = metrics.widthPixels
            val height = metrics.heightPixels
            val densityDpi = metrics.densityDpi

            val reader = ImageReader.newInstance(width, height, PixelFormat.RGBA_8888, 2)
            imageReader = reader

            virtualDisplay =
                mediaProjection.createVirtualDisplay(
                    "OperitScreenCapture",
                    width,
                    height,
                    densityDpi,
                    DisplayManager.VIRTUAL_DISPLAY_FLAG_AUTO_MIRROR,
                    reader.surface,
                    null,
                    null,
                )

            AndroidClientLogger.d(
                context.applicationContext,
                TAG,
                "Created MediaProjection virtual display: ${width}x$height",
            )
        } catch (error: Exception) {
            try {
                imageReader?.close()
            } catch (_: Exception) {
            }
            imageReader = null
            AndroidClientLogger.e(
                context.applicationContext,
                TAG,
                "Failed to create MediaProjection virtual display: ${error.message.orEmpty()}",
            )
        }
    }

    private fun ensureProjectionCallbackRegistered() {
        if (projectionCallback != null) return

        val callback =
            object : MediaProjection.Callback() {
                override fun onStop() {
                    AndroidClientLogger.w(context.applicationContext, TAG, "MediaProjection stopped")
                    try {
                        MediaProjectionHolder.clear(context)
                    } catch (_: Exception) {
                    }
                    release()
                }
            }

        projectionCallback = callback
        try {
            mediaProjection.registerCallback(callback, callbackHandler)
        } catch (error: Exception) {
            projectionCallback = null
            AndroidClientLogger.e(
                context.applicationContext,
                TAG,
                "Failed to register MediaProjection callback: ${error.message.orEmpty()}",
            )
        }
    }

    fun captureToBitmap(): Bitmap? {
        val reader = imageReader ?: return null
        var image: Image? = null
        return try {
            image = reader.acquireLatestImage() ?: return null

            val width = image.width
            val height = image.height
            if (width <= 0 || height <= 0) {
                return null
            }

            val plane = image.planes[0]
            val buffer = plane.buffer
            val pixelStride = plane.pixelStride
            val rowStride = plane.rowStride
            val rowPadding = rowStride - pixelStride * width

            val bitmap =
                Bitmap.createBitmap(
                    width + rowPadding / pixelStride,
                    height,
                    Bitmap.Config.ARGB_8888,
                )
            bitmap.copyPixelsFromBuffer(buffer)

            val cropped = Bitmap.createBitmap(bitmap, 0, 0, width, height)
            bitmap.recycle()

            cropped
        } catch (error: Exception) {
            AndroidClientLogger.e(
                context.applicationContext,
                TAG,
                "Error capturing frame from MediaProjection: ${error.message.orEmpty()}",
            )
            null
        } finally {
            image?.close()
        }
    }

    fun captureToFile(file: File): Boolean {
        val bitmap = captureToBitmap() ?: return false
        return try {
            FileOutputStream(file).use { out ->
                if (!bitmap.compress(Bitmap.CompressFormat.PNG, 100, out)) {
                    return false
                }
            }
            true
        } catch (error: Exception) {
            AndroidClientLogger.e(
                context.applicationContext,
                TAG,
                "Error writing MediaProjection capture to file: ${error.message.orEmpty()}",
            )
            false
        } finally {
            bitmap.recycle()
        }
    }

    fun release() {
        try {
            virtualDisplay?.release()
            imageReader?.close()
        } catch (error: Exception) {
            AndroidClientLogger.e(
                context.applicationContext,
                TAG,
                "Error releasing resources: ${error.message.orEmpty()}",
            )
        }
        virtualDisplay = null
        imageReader = null

        val callback = projectionCallback
        if (callback != null) {
            try {
                mediaProjection.unregisterCallback(callback)
            } catch (_: Exception) {
            }
        }
        projectionCallback = null
    }
}
