package app.operit

import android.content.Context
import android.os.Build
import android.os.Process
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
    private const val perUserRange = 100000

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
        ensureRootfsAbsolutePath(rootfsDir, context.applicationInfo.dataDir)
        ensureRootfsAbsolutePath(rootfsDir, "/data/data/${context.packageName}")
        ensureRootfsAbsolutePath(rootfsDir, "/data/local/tmp")
        ensureRootfsAbsolutePath(rootfsDir, storageRoot.absolutePath)
        File(rootfsDir, "dev/pts").mkdirs()
        File(rootfsDir, "storage").mkdirs()
        File(rootfsDir, "sdcard").mkdirs()
        writeCommonScript(
            target = File(context.filesDir, "common.sh"),
            runtimeDir = runtimeDir,
            rootfsDir = rootfsDir,
            storageRoot = storageRoot,
            internalRoot = context.filesDir,
            appDataDir = File(context.applicationInfo.dataDir),
            packageName = context.packageName,
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
        appDataDir: File,
        packageName: String,
    ) {
        val emulatedStoragePath = "/storage/emulated/${Process.myUid() / perUserRange}"
        val userDataPackagePath = "/data/user/${Process.myUid() / perUserRange}/$packageName"
        val legacyDataPackagePath = "/data/data/$packageName"
        val prootBindSetup = listOf(
            "/dev" to "/dev",
            "/proc" to "/proc",
            "/sys" to "/sys",
            "/dev/pts" to "/dev/pts",
            "/proc/self/fd" to "/dev/fd",
            "/proc/self/fd/0" to "/dev/stdin",
            "/proc/self/fd/1" to "/dev/stdout",
            "/proc/self/fd/2" to "/dev/stderr",
            emulatedStoragePath to "/sdcard",
            emulatedStoragePath to emulatedStoragePath,
            "/data/local/tmp" to "/data/local/tmp",
            appDataDir.absolutePath to userDataPackagePath,
            appDataDir.absolutePath to legacyDataPackagePath,
            internalRoot.absolutePath to internalRoot.absolutePath,
            storageRoot.absolutePath to storageRoot.absolutePath,
        ).joinToString(separator = "\n") { (source, target) ->
            "              probe_and_append_bind_arg \"$source\" \"$target\""
        }
        val content = """
            export BIN=${runtimeDir.absolutePath}
            export HOME=${internalRoot.absolutePath}
            export TMPDIR=${File(runtimeDir, "tmp").absolutePath}
            export PROOT_TMP_DIR=${File(runtimeDir, "tmp").absolutePath}
            export PROOT_LOADER=${File(runtimeDir, "loader").absolutePath}
            export UBUNTU_PATH=${rootfsDir.absolutePath}
            export OPERIT_STORAGE_ROOT=${storageRoot.absolutePath}
            export OPERIT_INTERNAL_ROOT=${internalRoot.absolutePath}
            can_access_bind_source(){
              bind_source="${'$'}1"
              if [ -z "${'$'}bind_source" ]; then
                return 1
              fi
              if [ ! -e "${'$'}bind_source" ] && [ ! -L "${'$'}bind_source" ]; then
                return 1
              fi
              "${'$'}BIN/busybox" ls -Ld "${'$'}bind_source" >/dev/null 2>&1
            }
            append_proot_bind_arg(){
              bind_source="${'$'}1"
              bind_target="${'$'}2"
              if ! can_access_bind_source "${'$'}bind_source"; then
                return 0
              fi
              if [ -z "${'$'}bind_target" ] || [ "${'$'}bind_source" = "${'$'}bind_target" ]; then
                PROOT_BIND_ARGS="${'$'}PROOT_BIND_ARGS -b ${'$'}bind_source"
              else
                PROOT_BIND_ARGS="${'$'}PROOT_BIND_ARGS -b ${'$'}bind_source:${'$'}bind_target"
              fi
            }
            run_proot_binary(){
              LD_LIBRARY_PATH= "${'$'}BIN/proot" "${'$'}@"
            }
            exec_proot_binary(){
              LD_LIBRARY_PATH= exec "${'$'}BIN/proot" "${'$'}@"
            }
            LAST_PROOT_PROBE_STATUS=0
            LAST_PROOT_PROBE_OUTPUT=""
            LAST_PROOT_PROBE_ARGS=""
            run_proot_probe(){
              LAST_PROOT_PROBE_ARGS="${'$'}*"
              if [ "${'$'}PROOT_LINK2SYMLINK" = "1" ]; then
                LAST_PROOT_PROBE_OUTPUT="${'$'}(
                  run_proot_binary \
                    -v 1 \
                    -0 \
                    -r "${'$'}UBUNTU_PATH" \
                    --link2symlink \
                    "${'$'}@" \
                    -w /root \
                    /usr/bin/env -i \
                      HOME=/root \
                      TERM=xterm-256color \
                      LANG=en_US.UTF-8 \
                      PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
                      /bin/bash --noprofile --norc -c 'exit 0' 2>&1
                )"
                LAST_PROOT_PROBE_STATUS="${'$'}?"
              else
                LAST_PROOT_PROBE_OUTPUT="${'$'}(
                  run_proot_binary \
                    -v 1 \
                    -0 \
                    -r "${'$'}UBUNTU_PATH" \
                    "${'$'}@" \
                    -w /root \
                    /usr/bin/env -i \
                      HOME=/root \
                      TERM=xterm-256color \
                      LANG=en_US.UTF-8 \
                      PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
                      /bin/bash --noprofile --norc -c 'exit 0' 2>&1
                )"
                LAST_PROOT_PROBE_STATUS="${'$'}?"
              fi
              return "${'$'}LAST_PROOT_PROBE_STATUS"
            }
            print_last_proot_probe_failure(){
              echo "PRoot startup probe failed."
              echo "exit_code=${'$'}LAST_PROOT_PROBE_STATUS"
              echo "ubuntu_path=${'$'}UBUNTU_PATH"
              echo "proot_loader=${'$'}{PROOT_LOADER:-<unset>}"
              echo "proot_no_seccomp=${'$'}{PROOT_NO_SECCOMP:-<unset>}"
              echo "ld_library_path=${'$'}{LD_LIBRARY_PATH:-<unset>}"
              echo "proot_exec_ld_library_path=<empty>"
              echo "proot_link2symlink=${'$'}PROOT_LINK2SYMLINK"
              if [ -n "${'$'}PROOT_BIND_ARGS" ]; then
                echo "bind_args=${'$'}PROOT_BIND_ARGS"
              else
                echo "bind_args=<none>"
              fi
              if [ -n "${'$'}LAST_PROOT_PROBE_ARGS" ]; then
                echo "extra_args=${'$'}LAST_PROOT_PROBE_ARGS"
              else
                echo "extra_args=<none>"
              fi
              echo "--- proot stdout/stderr begin ---"
              if [ -n "${'$'}LAST_PROOT_PROBE_OUTPUT" ]; then
                printf '%s\n' "${'$'}LAST_PROOT_PROBE_OUTPUT"
              else
                echo "<empty>"
              fi
              echo "--- proot stdout/stderr end ---"
            }
            probe_proot_link2symlink(){
              run_proot_binary \
                -v 1 \
                -0 \
                -r "${'$'}UBUNTU_PATH" \
                --link2symlink \
                -w /root \
                /usr/bin/env -i \
                  HOME=/root \
                  TERM=xterm-256color \
                  LANG=en_US.UTF-8 \
                  PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
                  /bin/bash --noprofile --norc -c 'exit 0' >/dev/null 2>&1
            }
            probe_and_append_bind_arg(){
              bind_source="${'$'}1"
              bind_target="${'$'}2"
              if ! can_access_bind_source "${'$'}bind_source"; then
                return 0
              fi

              candidate_bind_args="-b ${'$'}bind_source"
              if [ -n "${'$'}bind_target" ] && [ "${'$'}bind_source" != "${'$'}bind_target" ]; then
                candidate_bind_args="-b ${'$'}bind_source:${'$'}bind_target"
              fi

              if [ -n "${'$'}PROOT_BIND_ARGS" ]; then
                set -- ${'$'}PROOT_BIND_ARGS ${'$'}candidate_bind_args
              else
                set -- ${'$'}candidate_bind_args
              fi

              if run_proot_probe "${'$'}@"; then
                append_proot_bind_arg "${'$'}bind_source" "${'$'}bind_target"
              fi
            }
            resolve_proot_runtime(){
              PROOT_LINK2SYMLINK=0
              PROOT_BIND_ARGS=""

              if ! run_proot_probe; then
                print_last_proot_probe_failure
                return 1
              fi

              if probe_proot_link2symlink; then
                PROOT_LINK2SYMLINK=1
              fi
              return 0
            }
            login_ubuntu(){
              COMMAND_TO_EXEC="${'$'}1"
              if [ -z "${'$'}COMMAND_TO_EXEC" ]; then
                COMMAND_TO_EXEC="/bin/bash -il"
              fi
              mkdir -p "${'$'}UBUNTU_PATH/dev/pts" 2>/dev/null
              mkdir -p "${'$'}UBUNTU_PATH/dev/fd" 2>/dev/null
              mkdir -p "${'$'}UBUNTU_PATH/sdcard" 2>/dev/null
              mkdir -p "${'$'}UBUNTU_PATH${emulatedStoragePath}" 2>/dev/null
              mkdir -p "${'$'}UBUNTU_PATH${userDataPackagePath}" 2>/dev/null
              mkdir -p "${'$'}UBUNTU_PATH${legacyDataPackagePath}" 2>/dev/null
              mkdir -p "${'$'}UBUNTU_PATH/data/local/tmp" 2>/dev/null
              mkdir -p "${'$'}UBUNTU_PATH${internalRoot.absolutePath}" 2>/dev/null
              mkdir -p "${'$'}UBUNTU_PATH${storageRoot.absolutePath}" 2>/dev/null
              if ! resolve_proot_runtime; then
                return 1
              fi
$prootBindSetup
              if [ -n "${'$'}PROOT_BIND_ARGS" ]; then
                set -- ${'$'}PROOT_BIND_ARGS
              else
                set --
              fi
              if [ "${'$'}PROOT_LINK2SYMLINK" = "1" ]; then
                exec_proot_binary \
                  -0 \
                  -r "${'$'}UBUNTU_PATH" \
                  --link2symlink \
                  "${'$'}@" \
                  -w /root \
                  /usr/bin/env -i \
                    HOME=/root \
                    TERM=xterm-256color \
                    LANG=en_US.UTF-8 \
                    PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
                    PROMPT_COMMAND="${'$'}PROMPT_COMMAND" \
                    PS1="${'$'}PS1" \
                    OPERIT_WORKING_DIR="${'$'}OPERIT_WORKING_DIR" \
                    COMMAND_TO_EXEC="${'$'}COMMAND_TO_EXEC" \
                    /bin/bash -lc 'eval "${'$'}COMMAND_TO_EXEC"'
              else
                exec_proot_binary \
                  -0 \
                  -r "${'$'}UBUNTU_PATH" \
                  "${'$'}@" \
                  -w /root \
                  /usr/bin/env -i \
                    HOME=/root \
                    TERM=xterm-256color \
                    LANG=en_US.UTF-8 \
                    PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin \
                    PROMPT_COMMAND="${'$'}PROMPT_COMMAND" \
                    PS1="${'$'}PS1" \
                    OPERIT_WORKING_DIR="${'$'}OPERIT_WORKING_DIR" \
                    COMMAND_TO_EXEC="${'$'}COMMAND_TO_EXEC" \
                    /bin/bash -lc 'eval "${'$'}COMMAND_TO_EXEC"'
              fi
            }
            start_shell(){
              login_ubuntu 'cd "${'$'}OPERIT_WORKING_DIR" && /bin/bash -il'
            }
        """.trimIndent()
        target.writeText(content.replace("\r\n", "\n").replace("\r", "\n"))
        target.setReadable(true, false)
        target.setExecutable(true, false)
    }
}
