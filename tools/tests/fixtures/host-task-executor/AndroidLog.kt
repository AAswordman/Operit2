package android.util
/** Supplies Android logging for the JVM-only Host scheduling contract. */
object Log {
    /** Records a task error without loading the Android runtime. */
    @JvmStatic fun e(tag: String, message: String, error: Throwable): Int = 0
}
