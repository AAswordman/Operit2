package app.operit.util

import android.content.Context
import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.net.Uri
import androidx.annotation.WorkerThread
import app.operit.R
import com.google.mlkit.vision.common.InputImage
import com.google.mlkit.vision.text.Text
import com.google.mlkit.vision.text.TextRecognition
import com.google.mlkit.vision.text.TextRecognizer
import com.google.mlkit.vision.text.chinese.ChineseTextRecognizerOptions
import com.google.mlkit.vision.text.japanese.JapaneseTextRecognizerOptions
import com.google.mlkit.vision.text.korean.KoreanTextRecognizerOptions
import com.google.mlkit.vision.text.latin.TextRecognizerOptions
import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import kotlin.coroutines.resume
import kotlin.coroutines.resumeWithException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.suspendCancellableCoroutine
import kotlinx.coroutines.withContext

object OCRUtils {
    private const val TAG = "OCRUtils"

    private var latinRecognizer: TextRecognizer? = null
    private var chineseRecognizer: TextRecognizer? = null
    private var japaneseRecognizer: TextRecognizer? = null
    private var koreanRecognizer: TextRecognizer? = null

    enum class Language {
        LATIN,
        CHINESE,
        JAPANESE,
        KOREAN
    }

    enum class Quality {
        LOW,
        HIGH
    }

    private fun getRecognizer(language: Language): TextRecognizer {
        return when (language) {
            Language.LATIN -> {
                if (latinRecognizer == null) {
                    latinRecognizer =
                        TextRecognition.getClient(TextRecognizerOptions.DEFAULT_OPTIONS)
                }
                latinRecognizer!!
            }
            Language.CHINESE -> {
                if (chineseRecognizer == null) {
                    chineseRecognizer =
                        TextRecognition.getClient(
                            ChineseTextRecognizerOptions.Builder().build()
                        )
                }
                chineseRecognizer!!
            }
            Language.JAPANESE -> {
                if (japaneseRecognizer == null) {
                    japaneseRecognizer =
                        TextRecognition.getClient(
                            JapaneseTextRecognizerOptions.Builder().build()
                        )
                }
                japaneseRecognizer!!
            }
            Language.KOREAN -> {
                if (koreanRecognizer == null) {
                    koreanRecognizer =
                        TextRecognition.getClient(KoreanTextRecognizerOptions.Builder().build())
                }
                koreanRecognizer!!
            }
        }
    }

    private fun preprocessBitmap(bitmap: Bitmap): Bitmap {
        val scaleFactor = 2.0f
        val maxDimension = 4096

        val newWidth = (bitmap.width * scaleFactor).toInt()
        val newHeight = (bitmap.height * scaleFactor).toInt()

        if (
            bitmap.width >= newWidth ||
                bitmap.height >= newHeight ||
                newWidth > maxDimension ||
                newHeight > maxDimension
        ) {
            AppLogger.d(TAG, "Bitmap already large enough, not upscaling for OCR.")
            return bitmap
        }

        AppLogger.d(
            TAG,
            "Upscaling bitmap from ${bitmap.width}x${bitmap.height} to ${newWidth}x${newHeight} for OCR."
        )
        return Bitmap.createScaledBitmap(
            bitmap,
            newWidth,
            newHeight,
            true
        )
    }

    @WorkerThread
    suspend fun recognizeTextFromBitmap(
        bitmap: Bitmap,
        language: Language = Language.LATIN,
        quality: Quality = Quality.LOW
    ): OCRResult {
        val processedBitmap =
            if (quality == Quality.HIGH) {
                preprocessBitmap(bitmap)
            } else {
                bitmap
            }

        return try {
            val image = InputImage.fromBitmap(processedBitmap, 0)
            val result = processImage(image, language)
            OCRResult.Success(result)
        } catch (e: Exception) {
            AppLogger.e(TAG, "Error recognizing text from bitmap: ${e.message}", e)
            OCRResult.Error(e.message ?: "Unknown error")
        } finally {
            if (processedBitmap !== bitmap) {
                processedBitmap.recycle()
            }
        }
    }

    @WorkerThread
    suspend fun recognizeTextFromUri(
        context: Context,
        uri: Uri,
        language: Language = Language.LATIN,
        quality: Quality = Quality.LOW
    ): OCRResult {
        if (quality == Quality.LOW) {
            return try {
                val image = InputImage.fromFilePath(context, uri)
                val result = processImage(image, language)
                OCRResult.Success(result)
            } catch (e: IOException) {
                AppLogger.e(TAG, "Error reading image: ${e.message}", e)
                OCRResult.Error(context.getString(R.string.ocr_cannot_read_image, e.message))
            } catch (e: Exception) {
                AppLogger.e(TAG, "Error recognizing text from uri: ${e.message}", e)
                OCRResult.Error(e.message ?: "Unknown error")
            }
        }

        return withContext(Dispatchers.IO) {
            try {
                context.contentResolver.openInputStream(uri)?.use { inputStream ->
                    val originalBitmap = BitmapFactory.decodeStream(inputStream)
                    if (originalBitmap != null) {
                        val result = recognizeTextFromBitmap(originalBitmap, language, quality)
                        originalBitmap.recycle()
                        result
                    } else {
                        OCRResult.Error(context.getString(R.string.ocr_cannot_decode_bitmap_from_uri))
                    }
                }
                    ?: OCRResult.Error(context.getString(R.string.ocr_cannot_open_uri_stream))
            } catch (e: Exception) {
                AppLogger.e(TAG, "Error recognizing text from uri (high quality): ${e.message}", e)
                OCRResult.Error(e.message ?: "Unknown error on high quality path")
            }
        }
    }

    private suspend fun processImage(image: InputImage, language: Language): Text =
        suspendCancellableCoroutine { continuation ->
            val recognizer = getRecognizer(language)
            recognizer
                .process(image)
                .addOnSuccessListener { text -> continuation.resume(text) }
                .addOnFailureListener { e ->
                    AppLogger.e(TAG, "Text recognition failed: ${e.message}", e)
                    continuation.resumeWithException(e)
                }
        }

    @WorkerThread
    suspend fun recognizeText(
        context: Context,
        bitmap: Bitmap,
        quality: Quality = Quality.LOW
    ): String {
        val latinResult = recognizeTextFromBitmap(bitmap, Language.LATIN, quality)
        val chineseResult = recognizeTextFromBitmap(bitmap, Language.CHINESE, quality)

        val latinText = if (latinResult is OCRResult.Success) latinResult.getFullText() else ""
        val chineseText = if (chineseResult is OCRResult.Success) chineseResult.getFullText() else ""

        return when {
            latinText.isEmpty() -> chineseText
            chineseText.isEmpty() -> latinText
            latinText == chineseText -> latinText
            else -> "$latinText\n$chineseText"
        }
    }

    @WorkerThread
    suspend fun recognizeText(
        context: Context,
        bitmap: Bitmap,
        language: Language,
        quality: Quality = Quality.LOW
    ): String {
        val result = recognizeTextFromBitmap(bitmap, language, quality)
        return when (result) {
            is OCRResult.Success -> result.getFullText()
            is OCRResult.Error -> {
                AppLogger.e(TAG, "Text recognition failed: ${result.message}")
                ""
            }
        }
    }

    @WorkerThread
    suspend fun recognizeText(context: Context, uri: Uri, quality: Quality = Quality.LOW): String {
        val latinResult = recognizeTextFromUri(context, uri, Language.LATIN, quality)
        val chineseResult = recognizeTextFromUri(context, uri, Language.CHINESE, quality)

        val latinText = if (latinResult is OCRResult.Success) latinResult.getFullText() else ""
        val chineseText = if (chineseResult is OCRResult.Success) chineseResult.getFullText() else ""

        return when {
            latinText.isEmpty() -> chineseText
            chineseText.isEmpty() -> latinText
            latinText == chineseText -> latinText
            else -> "$latinText\n$chineseText"
        }
    }

    @WorkerThread
    suspend fun extractTextBlocks(
        bitmap: Bitmap,
        languages: List<Language> = listOf(Language.LATIN, Language.CHINESE),
        quality: Quality = Quality.LOW
    ): List<String> {
        val textBlocks = mutableListOf<String>()

        for (language in languages) {
            val result = recognizeTextFromBitmap(bitmap, language, quality)
            if (result is OCRResult.Success) {
                result.getTextBlocks().forEach { block -> textBlocks.add(block.text) }
                if (textBlocks.isNotEmpty()) {
                    break
                }
            }
        }

        return textBlocks
    }

    @WorkerThread
    suspend fun saveBitmapToTempFile(context: Context, bitmap: Bitmap): File? =
        withContext(Dispatchers.IO) {
            val cacheDir = context.cacheDir
            val tempFile = File(cacheDir, "ocr_temp_${System.currentTimeMillis()}.jpg")

            try {
                FileOutputStream(tempFile).use { out ->
                    bitmap.compress(Bitmap.CompressFormat.JPEG, 100, out)
                }
                return@withContext tempFile
            } catch (e: IOException) {
                AppLogger.e(TAG, "Failed to save bitmap to temp file", e)
                return@withContext null
            }
        }

    fun closeRecognizers() {
        latinRecognizer?.close()
        latinRecognizer = null

        chineseRecognizer?.close()
        chineseRecognizer = null

        japaneseRecognizer?.close()
        japaneseRecognizer = null

        koreanRecognizer?.close()
        koreanRecognizer = null
    }

    sealed class OCRResult {
        data class Success(val text: Text) : OCRResult() {
            fun getFullText(): String = text.text

            fun getTextBlocks(): List<Text.TextBlock> = text.textBlocks

            fun getStructuredText(): String {
                val sb = StringBuilder()
                for (block in text.textBlocks) {
                    for (line in block.lines) {
                        sb.append(line.text).append("\n")
                    }
                    sb.append("\n")
                }
                return sb.toString().trim()
            }
        }

        data class Error(val message: String) : OCRResult()
    }
}
