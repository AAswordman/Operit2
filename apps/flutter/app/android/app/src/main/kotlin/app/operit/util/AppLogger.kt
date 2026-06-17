package app.operit.util

import android.util.Log

object AppLogger {
    @JvmStatic
    fun d(tag: String, msg: String): Int = Log.d(tag, msg)

    @JvmStatic
    fun d(tag: String, msg: String, tr: Throwable): Int = Log.d(tag, msg, tr)

    @JvmStatic
    fun e(tag: String, msg: String): Int = Log.e(tag, msg)

    @JvmStatic
    fun e(tag: String, msg: String, tr: Throwable): Int = Log.e(tag, msg, tr)
}
