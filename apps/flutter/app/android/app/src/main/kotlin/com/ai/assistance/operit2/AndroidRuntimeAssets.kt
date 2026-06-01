package com.ai.assistance.operit2

import android.content.Context
import android.os.Build
import android.system.Os
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
    val storageRoot: File,
    val internalRoot: File,
    val tmpDir: File,
)

object AndroidRuntimeAssets {
    private val packagedAbis = setOf("arm64-v8a", "armeabi-v7a", "x86_64")

    @Synchronized
    fun prepare(context: Context, storageRoot: File): AndroidRuntimePaths {
        val abi = selectPackagedAbi()
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
        val rootfsShaFile = File(runtimeDir, "rootfs.tar.gz.bin.sha256")

        require(nativeBusybox.isFile) { "Android runtime busybox is missing: ${nativeBusybox.absolutePath}" }
        require(nativeBash.isFile) { "Android runtime bash is missing: ${nativeBash.absolutePath}" }
        require(nativeProot.isFile) { "Android runtime proot is missing: ${nativeProot.absolutePath}" }
        require(nativeLoader.isFile) { "Android runtime loader is missing: ${nativeLoader.absolutePath}" }
        createExecutableLink(nativeBusybox, busybox)
        createExecutableLink(nativeBash, bash)
        createExecutableLink(nativeProot, proot)
        createExecutableLink(nativeLoader, loader)

        copyAsset(context, "android-runtime/$abi/rootfs.tar.gz.bin", rootfsArchive)
        copyAsset(context, "android-runtime/$abi/rootfs.tar.gz.bin.sha256", rootfsShaFile)

        val packagedSha = rootfsShaFile.readText().trim().substringBefore(' ')
        val installedShaFile = File(runtimeDir, "rootfs.installed.sha256")
        val installedSha = when {
            installedShaFile.isFile -> installedShaFile.readText().trim()
            else -> ""
        }

        if (!rootfsDir.isDirectory || installedSha != packagedSha) {
            rootfsDir.deleteRecursively()
            rootfsDir.mkdirs()
            runBusybox(
                busybox,
                listOf("tar", "-xzf", rootfsArchive.absolutePath, "-C", rootfsDir.absolutePath),
            )
            installedShaFile.writeText(packagedSha)
        }

        ensureRootfsAbsolutePath(rootfsDir, context.filesDir.absolutePath)
        ensureRootfsAbsolutePath(rootfsDir, storageRoot.absolutePath)
        File(rootfsDir, "storage").mkdirs()
        File(rootfsDir, "sdcard").mkdirs()
        writeCommonScript(
            target = File(context.filesDir, "common.sh"),
            runtimeDir = runtimeDir,
            rootfsDir = rootfsDir,
            storageRoot = storageRoot,
            internalRoot = context.filesDir,
        )

        val tmpDir = File(runtimeDir, "tmp")
        tmpDir.mkdirs()

        Os.setenv("OPERIT_ANDROID_RUNTIME_DIR", runtimeDir.absolutePath, true)
        Os.setenv("OPERIT_ANDROID_NATIVE_LIBRARY_DIR", nativeLibraryDir.absolutePath, true)
        Os.setenv("OPERIT_ANDROID_BUSYBOX", busybox.absolutePath, true)
        Os.setenv("OPERIT_ANDROID_BASH", bash.absolutePath, true)
        Os.setenv("OPERIT_ANDROID_PROOT", proot.absolutePath, true)
        Os.setenv("OPERIT_ANDROID_LOADER", loader.absolutePath, true)
        Os.setenv("OPERIT_ANDROID_ROOTFS_DIR", rootfsDir.absolutePath, true)
        Os.setenv("OPERIT_ANDROID_STORAGE_ROOT", storageRoot.absolutePath, true)
        Os.setenv("OPERIT_ANDROID_INTERNAL_ROOT", context.filesDir.absolutePath, true)
        Os.setenv("OPERIT_ANDROID_RUNTIME_TMP", tmpDir.absolutePath, true)
        Os.setenv("PROOT_NO_SECCOMP", "1", true)

        return AndroidRuntimePaths(
            abi = abi,
            runtimeDir = runtimeDir,
            rootfsDir = rootfsDir,
            busybox = busybox,
            bash = bash,
            proot = proot,
            loader = loader,
            nativeLibraryDir = nativeLibraryDir,
            storageRoot = storageRoot,
            internalRoot = context.filesDir,
            tmpDir = tmpDir,
        )
    }

    private fun selectPackagedAbi(): String {
        val abi = Build.SUPPORTED_ABIS.firstOrNull { packagedAbis.contains(it) }
        require(abi != null) {
            "Unsupported Android ABI: ${Build.SUPPORTED_ABIS.joinToString(", ")}"
        }
        return abi
    }

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
        target.setExecutable(true, false)
        Os.symlink(target.absolutePath, link.absolutePath)
    }

    private fun writeCommonScript(
        target: File,
        runtimeDir: File,
        rootfsDir: File,
        storageRoot: File,
        internalRoot: File,
    ) {
        val content = """
            export BIN=${runtimeDir.absolutePath}
            export HOME=${internalRoot.absolutePath}
            export TMPDIR=${File(runtimeDir, "tmp").absolutePath}
            export PROOT_TMP_DIR=${File(runtimeDir, "tmp").absolutePath}
            export PROOT_LOADER=${File(runtimeDir, "loader").absolutePath}
            export PROOT_NO_SECCOMP=1
            export UBUNTU_PATH=${rootfsDir.absolutePath}
            export OPERIT_STORAGE_ROOT=${storageRoot.absolutePath}
            export OPERIT_INTERNAL_ROOT=${internalRoot.absolutePath}
            login_ubuntu(){
              COMMAND_TO_EXEC="${'$'}1"
              if [ -z "${'$'}COMMAND_TO_EXEC" ]; then
                COMMAND_TO_EXEC="/bin/bash -il"
              fi
              mkdir -p "${'$'}UBUNTU_PATH${internalRoot.absolutePath}" 2>/dev/null
              mkdir -p "${'$'}UBUNTU_PATH${storageRoot.absolutePath}" 2>/dev/null
              "${'$'}BIN/proot" \
                -0 \
                -r "${'$'}UBUNTU_PATH" \
                -b /proc \
                -b /dev \
                -b /sys \
                -b /sdcard \
                -b /storage \
                -b "${internalRoot.absolutePath}:${internalRoot.absolutePath}" \
                -b "${storageRoot.absolutePath}:${storageRoot.absolutePath}" \
                -w "${'$'}OPERIT_WORKING_DIR" \
                /usr/bin/env -i \
                  HOME=/home/operit \
                  TERM=xterm-256color \
                  LANG=C.UTF-8 \
                  PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
                  COMMAND_TO_EXEC="${'$'}COMMAND_TO_EXEC" \
                  /bin/bash -lc 'eval "${'$'}COMMAND_TO_EXEC"'
            }
            start_shell(){
              login_ubuntu
            }
        """.trimIndent()
        target.writeText(content.replace("\r\n", "\n").replace("\r", "\n"))
        target.setReadable(true, false)
        target.setExecutable(true, false)
    }
}
