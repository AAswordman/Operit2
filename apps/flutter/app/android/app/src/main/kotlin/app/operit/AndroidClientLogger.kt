package app.operit

import android.content.Context
import java.io.File

object AndroidClientLogger {
    fun d(context: Context, tag: String, message: String) {
        write(context, "D", tag, message)
    }

    fun i(context: Context, tag: String, message: String) {
        write(context, "I", tag, message)
    }

    fun w(context: Context, tag: String, message: String) {
        write(context, "W", tag, message)
    }

    fun e(context: Context, tag: String, message: String) {
        write(context, "E", tag, message)
    }

    @Synchronized
    private fun write(context: Context, level: String, tag: String, message: String) {
        val logsDir = File(context.filesDir, "client/logs")
        logsDir.mkdirs()
        val logFile = File(logsDir, "client.log")
        logFile.appendText("${System.currentTimeMillis()} $level/$tag: $message\n", Charsets.UTF_8)
    }
}
