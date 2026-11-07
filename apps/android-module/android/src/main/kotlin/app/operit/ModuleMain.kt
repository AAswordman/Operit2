package app.operit

import android.os.Process
import java.io.File
import kotlin.system.exitProcess

/** Boots the shared Core library in an Android process owned by the privileged Host deployment. */
object ModuleMain {
    /** Keeps process startup independent of Activities, Flutter engines and application initialization. */
    @JvmStatic
    fun main(arguments: Array<String>) {
        try {
            check(Process.myUid() == 0) { "The privileged Android Host must run as Root" }
            require(arguments.size == 5) { "Expected deployment, data root, model, language and client UID" }
            val deployment = File(arguments[0]).canonicalFile
            check(deployment.isDirectory) { "Android Host deployment directory is missing" }
            val library = File(deployment, "system/app/Operit/lib/arm64/liboperit_flutter_bridge.so")
            check(library.isFile) { "Shared Android Host library is missing" }
            System.load(library.absolutePath)
            runRuntime(deployment.path, arguments[1], arguments[2], arguments[3], arguments[4].toInt())
        } catch (error: Throwable) {
            System.err.println("Privileged Android Host failed: ${error.message}")
            error.printStackTrace(System.err)
            exitProcess(1)
        }
    }

    /** Runs the same Core implementation used by the application with privileged Host assembly. */
    @JvmStatic
    external fun runRuntime(deployment: String, dataRoot: String, model: String, language: String, clientUid: Int)
}
