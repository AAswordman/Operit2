package app.operit.accessibility

import android.content.Context
import android.system.Os
import java.io.File
import java.io.FileOutputStream

/** Keeps the latest crash report in app-private storage across process restarts. */
internal class CrashReportStore(context: Context) {
    private val file = File(context.noBackupFilesDir, "companion-crash.txt")

    /** Indicates whether a completed report is available for review. */
    fun exists(): Boolean = file.isFile

    /** Commits the report before attempting to present it. */
    fun write(thread: Thread, error: Throwable) {
        val details = "Thread: ${thread.name}\n\n${error.stackTraceToString()}".toByteArray(Charsets.UTF_8)
        val pending = File(file.parentFile, "companion-crash.tmp")
        try {
            FileOutputStream(pending).use { output ->
                output.write(details)
                output.fd.sync()
            }
            Os.rename(pending.absolutePath, file.absolutePath)
        } finally {
            pending.delete()
        }
    }

    /** Reads only the committed report while the companion may be writing its replacement. */
    fun read(): String = file.readText(Charsets.UTF_8)

    /** Removes the reviewed report without touching an in-progress write. */
    fun clear() {
        file.delete()
    }
}
