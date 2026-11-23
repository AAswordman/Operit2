package app.operit

import android.content.Context
import android.os.Build
import android.os.Process
import android.system.Os
import android.util.Log
import java.io.File

data class AndroidRuntimePaths(
    val abi: String,
    val runtimeDir: File,
    val rootfsDir: File,
    val busybox: File,
    val bash: File,
    val proot: File,
    val loader: File,
    val nativeLibraryDir: File,
    val runtimeRoot: File,
    val workspaceRoot: File,
    val internalRoot: File,
    val tmpDir: File,
)

object AndroidRuntimeAssets {
    private const val runtimeAssetsDirectory = "android-runtime"
    private const val perUserRange = 100000
    private const val TAG = "OperitRuntimeAssets"

    @Synchronized
    fun prepare(
        context: Context,
        runtimeRoot: File,
        workspaceRoot: File,
    ): AndroidRuntimePaths {
        val startedAtMillis = System.currentTimeMillis()
        // Gradle stages runtime assets for the requested ABIs only, so this listing
        // is the authoritative set of ABIs this APK can actually run.
        val packagedAbis = context.assets.list(runtimeAssetsDirectory)?.toSet() ?: emptySet()
        val abi = selectPackagedAbi(Build.SUPPORTED_ABIS.toList(), packagedAbis)
        val runtimeDir = File(context.filesDir, "android-runtime/$abi")
        val rootfsDir = File(runtimeDir, "rootfs")
        val nativeLibraryDir = File(context.applicationInfo.nativeLibraryDir)
        runtimeDir.mkdirs()

        val nativeBusybox = File(nativeLibraryDir, "liboperit_busybox.so")
        val busybox = File(runtimeDir, "busybox")
        val nativeBash = File(nativeLibraryDir, "libbash.so")
        val bash = File(runtimeDir, "bash")
        val nativeProot = File(nativeLibraryDir, "liboperit_proot.so")
        val proot = File(runtimeDir, "proot")
        val nativeLoader = File(nativeLibraryDir, "liboperit_loader.so")
        val loader = File(runtimeDir, "loader")
        val rootfsArchive = File(runtimeDir, "rootfs.tar.gz")

        require(nativeBusybox.isFile) { "Android runtime busybox is missing: ${nativeBusybox.absolutePath}" }
        require(nativeBash.isFile) { "Android runtime bash is missing: ${nativeBash.absolutePath}" }
        require(nativeProot.isFile) { "Android runtime proot is missing: ${nativeProot.absolutePath}" }
        require(nativeLoader.isFile) { "Android runtime loader is missing: ${nativeLoader.absolutePath}" }
        createExecutableLink(nativeBusybox, busybox)
        createExecutableLink(nativeBash, bash)
        createExecutableLink(nativeProot, proot)
        createExecutableLink(nativeLoader, loader)

        val packagedSha = readAssetText(
            context,
            "$runtimeAssetsDirectory/$abi/rootfs.tar.gz.bin.sha256",
        ).trim().substringBefore(' ')

        val installedShaFile = File(runtimeDir, "rootfs.installed.sha256")
        val installedSha = when {
            installedShaFile.isFile -> installedShaFile.readText().trim()
            else -> ""
        }

        if (!rootfsDir.isDirectory || installedSha != packagedSha) {
            Log.i(TAG, "runtime rootfs install start")
            copyAsset(context, "$runtimeAssetsDirectory/$abi/rootfs.tar.gz.bin", rootfsArchive)
            Log.i(TAG, "runtime rootfs copy done elapsedMs=" + (System.currentTimeMillis() - startedAtMillis))
            rootfsDir.deleteRecursively()
            rootfsDir.mkdirs()
            runBusybox(
                busybox,
                listOf("tar", "-xzf", rootfsArchive.absolutePath, "-C", rootfsDir.absolutePath),
            )
            Log.i(TAG, "runtime rootfs extract done elapsedMs=" + (System.currentTimeMillis() - startedAtMillis))
            installedShaFile.writeText(packagedSha)
        } else {
            Log.i(TAG, "runtime rootfs already installed elapsedMs=" + (System.currentTimeMillis() - startedAtMillis))
        }

        ensureRootfsAbsolutePath(rootfsDir, context.filesDir.absolutePath)
        ensureRootfsAbsolutePath(rootfsDir, context.applicationInfo.dataDir)
        ensureRootfsAbsolutePath(rootfsDir, "/data/data/${context.packageName}")
        ensureRootfsAbsolutePath(rootfsDir, "/data/local/tmp")
        ensureRootfsAbsolutePath(rootfsDir, runtimeRoot.absolutePath)
        ensureRootfsAbsolutePath(rootfsDir, workspaceRoot.absolutePath)
        File(rootfsDir, "dev/pts").mkdirs()
        File(rootfsDir, "storage").mkdirs()
        File(rootfsDir, "sdcard").mkdirs()

        val tmpDir = File(runtimeDir, "tmp")
        tmpDir.mkdirs()
        Log.i(TAG, "runtime assets prepare done elapsedMs=" + (System.currentTimeMillis() - startedAtMillis))

        Os.setenv("OPERIT_ANDROID_RUNTIME_DIR", runtimeDir.absolutePath, true)
        Os.setenv("OPERIT_ANDROID_NATIVE_LIBRARY_DIR", nativeLibraryDir.absolutePath, true)
        Os.setenv("OPERIT_ANDROID_BUSYBOX", busybox.absolutePath, true)
        Os.setenv("OPERIT_ANDROID_BASH", bash.absolutePath, true)
        Os.setenv("OPERIT_ANDROID_PROOT", proot.absolutePath, true)
        Os.setenv("OPERIT_ANDROID_LOADER", loader.absolutePath, true)
        Os.setenv("OPERIT_ANDROID_ROOTFS_DIR", rootfsDir.absolutePath, true)
        Os.setenv("OPERIT_ANDROID_RUNTIME_ROOT", runtimeRoot.absolutePath, true)
        Os.setenv("OPERIT_ANDROID_WORKSPACE_ROOT", workspaceRoot.absolutePath, true)
        Os.setenv("OPERIT_ANDROID_INTERNAL_ROOT", context.filesDir.absolutePath, true)
        Os.setenv("OPERIT_ANDROID_RUNTIME_TMP", tmpDir.absolutePath, true)

        return AndroidRuntimePaths(
            abi = abi,
            runtimeDir = runtimeDir,
            rootfsDir = rootfsDir,
            busybox = busybox,
            bash = bash,
            proot = proot,
            loader = loader,
            nativeLibraryDir = nativeLibraryDir,
            runtimeRoot = runtimeRoot,
            workspaceRoot = workspaceRoot,
            internalRoot = context.filesDir,
            tmpDir = tmpDir,
        )
    }

    /**
     * Selects the most preferred device ABI whose runtime assets are packaged in
     * this APK. Device ABI lists can contain secondary ABIs (for example the arm64
     * translation ABI reported on x86_64 devices) that this build ships no assets
     * for, so only ABIs present in the packaged listing are eligible.
     */
    internal fun selectPackagedAbi(
        supportedAbis: List<String>,
        packagedAbis: Set<String>,
    ): String {
        val abi = supportedAbis.firstOrNull { packagedAbis.contains(it) }
        require(abi != null) {
            "Unsupported Android ABI: ${supportedAbis.joinToString(", ")} " +
                "(packaged runtime assets: ${packagedAbis.sorted().joinToString(", ")})"
        }
        return abi
    }

    /// Reads one packaged text asset without duplicating its binary to app storage.
    private fun readAssetText(context: Context, assetPath: String): String {
        return context.assets.open(assetPath).use { input ->
            input.bufferedReader().use { reader ->
                reader.readText()
            }
        }
    }

    /// Copies one packaged asset to the app-private destination.
    private fun copyAsset(context: Context, assetPath: String, target: File) {
        target.parentFile?.mkdirs()
        context.assets.open(assetPath).use { input ->
            target.outputStream().use { output ->
                input.copyTo(output)
            }
        }
    }

    private fun ensureRootfsAbsolutePath(rootfsDir: File, absolutePath: String) {
        require(absolutePath.startsWith("/")) { "Android runtime path must be absolute: $absolutePath" }
        File(rootfsDir, absolutePath.trimStart('/')).mkdirs()
    }

    private fun runBusybox(busybox: File, args: List<String>) {
        val command = mutableListOf(busybox.absolutePath)
        command.addAll(args)
        val process = ProcessBuilder(command)
            .redirectErrorStream(true)
            .start()
        val output = process.inputStream.bufferedReader().use { it.readText() }
        val exitCode = process.waitFor()
        check(exitCode == 0) {
            "Android runtime asset command failed ($exitCode): ${command.joinToString(" ")}\n$output"
        }
    }

    private fun createExecutableLink(target: File, link: File) {
        link.parentFile?.mkdirs()
        link.delete()
        Os.symlink(target.absolutePath, link.absolutePath)
    }
}
