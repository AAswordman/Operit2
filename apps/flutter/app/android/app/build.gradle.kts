import java.io.FileInputStream
import java.util.Properties
import javax.inject.Inject
import org.gradle.api.DefaultTask
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.file.FileSystemOperations
import org.gradle.api.provider.ListProperty
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputDirectory
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.PathSensitivity
import org.gradle.api.tasks.TaskAction

plugins {
    id("com.android.application")
    id("kotlin-android")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

/** Stages ordinary assets and only the runtime ABIs selected for this build. */
abstract class StageOperitAndroidAssets : DefaultTask() {
    @get:InputDirectory
    @get:PathSensitive(PathSensitivity.RELATIVE)
    abstract val sourceDirectory: DirectoryProperty

    @get:Input
    abstract val selectedAbis: ListProperty<String>

    @get:OutputDirectory
    abstract val outputDirectory: DirectoryProperty

    @get:Inject
    abstract val fileSystemOperations: FileSystemOperations

    /** Synchronizes the complete asset output so previous build ABIs cannot remain. */
    @TaskAction
    fun stageAssets() {
        fileSystemOperations.sync {
            from(sourceDirectory) {
                exclude("android-runtime/**")
            }
            selectedAbis.get().forEach { abi ->
                from(sourceDirectory.dir("android-runtime/$abi")) {
                    into("android-runtime/$abi")
                }
            }
            into(outputDirectory)
        }
    }
}

/** Publishes variant-isolated JNI libraries through Android's generated sources API. */
abstract class StageOperitAndroidJniLibraries : Sync() {
    @get:OutputDirectory
    abstract val outputDirectory: DirectoryProperty
}

val localProperties = Properties()
val localPropertiesFile = rootProject.file("local.properties")
localProperties.load(FileInputStream(localPropertiesFile))

// Reads a required signing property from local.properties.
fun requiredLocalProperty(name: String): String =
    localProperties.getProperty(name)
        ?: throw GradleException("Missing Android release signing property: $name")

data class OperitRustTarget(
    val flutterPlatform: String,
    val abi: String,
    val rustTarget: String,
    val envTarget: String,
)

// Reads Flutter's requested Android target platforms from Gradle properties.
fun requestedFlutterTargetPlatforms(): Set<String> {
    val raw = providers.gradleProperty("target-platform").orNull
        ?: throw GradleException("Missing Flutter target-platform Gradle property")
    return raw.split(',')
        .map { it.trim() }
        .filter { it.isNotEmpty() }
        .toSet()
}

// Selects only the Rust targets required by Flutter's Android build.
fun selectedOperitRustTargets(targets: List<OperitRustTarget>): List<OperitRustTarget> {
    val requestedPlatforms = requestedFlutterTargetPlatforms()
    val selectedTargets = targets.filter { requestedPlatforms.contains(it.flutterPlatform) }
    val selectedPlatforms = selectedTargets.map { it.flutterPlatform }.toSet()
    val unsupportedPlatforms = requestedPlatforms.subtract(selectedPlatforms)
    if (unsupportedPlatforms.isNotEmpty()) {
        throw GradleException("Unsupported Android Rust target-platform values: $unsupportedPlatforms")
    }
    if (selectedTargets.isEmpty()) {
        throw GradleException("No supported Android Rust targets selected for target-platform=$requestedPlatforms")
    }
    return selectedTargets
}

val operitRustTargets = listOf(
    OperitRustTarget("android-arm64", "arm64-v8a", "aarch64-linux-android", "AARCH64_LINUX_ANDROID"),
    OperitRustTarget("android-x64", "x86_64", "x86_64-linux-android", "X86_64_LINUX_ANDROID"),
    OperitRustTarget("android-arm", "armeabi-v7a", "armv7-linux-androideabi", "ARMV7_LINUX_ANDROIDEABI"),
)
val selectedOperitRustTargets = selectedOperitRustTargets(operitRustTargets)

android {
    namespace = "app.operit"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    signingConfigs {
        create("release") {
            storeFile = file(requiredLocalProperty("RELEASE_STORE_FILE"))
            storePassword = requiredLocalProperty("RELEASE_STORE_PASSWORD")
            keyAlias = requiredLocalProperty("RELEASE_KEY_ALIAS")
            keyPassword = requiredLocalProperty("RELEASE_KEY_PASSWORD")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = JavaVersion.VERSION_17.toString()
    }

    defaultConfig {
        // TODO: Specify your own unique Application ID (https://developer.android.com/studio/build/application-id.html).
        applicationId = "app.operit"
        // You can update the following values to match your application needs.
        // For more information, see: https://flutter.dev/to/review-gradle-config.
        minSdk = flutter.minSdkVersion
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName
    }

    sourceSets.getByName("main").assets.setSrcDirs(emptyList<String>())
    sourceSets.getByName("main").jniLibs.setSrcDirs(emptyList<String>())

    buildTypes {
        configureEach {
            // Flutter owns ABI splits; filter native libraries only for non-split builds.
            if (providers.gradleProperty("split-per-abi").orNull != "true") {
                ndk.abiFilters.clear()
                ndk.abiFilters.addAll(selectedOperitRustTargets.map { it.abi })
            }
        }

        release {
            signingConfig = signingConfigs.getByName("release")
            proguardFiles("proguard-rules.pro")
        }
    }

    packaging {
        jniLibs {
            useLegacyPackaging = true
        }
    }

}

androidComponents {
    onVariants(selector().all()) { variant ->
        val stagedAssets = tasks.register<StageOperitAndroidAssets>(
            "stageOperitAndroidAssets" + variant.name.replaceFirstChar { it.uppercase() },
        ) {
            sourceDirectory.set(layout.projectDirectory.dir("src/main/assets"))
            selectedAbis.set(selectedOperitRustTargets.map { it.abi })
        }
        val assets = variant.sources.assets
            ?: throw GradleException("Android variant ${variant.name} has no asset sources")
        assets.addGeneratedSourceDirectory(stagedAssets) { it.outputDirectory }
    }
}

flutter {
    source = "../.."
}

dependencies {
    // Match image_picker_android; the metadata-only attachment picker uses these contracts directly.
    implementation("androidx.activity:activity:1.12.4")
    implementation("com.google.mlkit:text-recognition:16.0.0")
    implementation("com.google.mlkit:text-recognition-chinese:16.0.0")
    implementation("com.google.mlkit:text-recognition-japanese:16.0.0")
    implementation("com.google.mlkit:text-recognition-korean:16.0.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.8.0")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.8.0")
    implementation("com.github.topjohnwu.libsu:core:6.0.0")
    implementation("dev.rikka.shizuku:api:13.1.5")
    implementation("dev.rikka.shizuku:provider:13.1.5")
}

val operitBridgeCrate = project.layout.projectDirectory
    .dir("../../../native/operit-flutter-bridge")
    .asFile
val operitRepoRoot = project.layout.projectDirectory
    .dir("../../../../..")
    .asFile
val operitPluginSyncScript = operitRepoRoot
    .resolve("plugins/tools/sync_plugin_packages.py")
val operitPluginSyncPython = if (System.getProperty("os.name").lowercase().contains("windows")) {
    operitRepoRoot.resolve(".venv/Scripts/python.exe")
} else {
    operitRepoRoot.resolve(".venv/bin/python")
}
val operitBridgeJniLibs = project.layout.projectDirectory.dir("src/main/jniLibs").asFile
val operitIsWindows = System.getProperty("os.name").lowercase().contains("windows")
val operitLibclangDir = when {
    System.getenv("OPERIT_LIBCLANG_DIR") != null ->
        file(System.getenv("OPERIT_LIBCLANG_DIR"))
    operitIsWindows ->
        operitRepoRoot
            .resolve("target/operit-build-tools/libclang.runtime.win-x64.21.1.8/runtimes/win-x64/native")
    else ->
        file("/usr/lib")
}
// Converts a file path into the clang-compatible slash format.
fun File.clangPath(): String = absolutePath.replace('\\', '/')


val syncOperitPlugins = tasks.register<Exec>("syncOperitPlugins") {
    workingDir = operitRepoRoot
    commandLine(
        operitPluginSyncPython.absolutePath,
        operitPluginSyncScript.absolutePath,
        "--source",
        "runtime",
        "--no-hot-reload",
    )
}

androidComponents {
    onVariants(selector().all()) { variant ->
        val variantTaskSuffix = variant.name.replaceFirstChar { it.uppercase() }
        val rustProfile = when (variant.buildType) {
            "debug" -> "debug"
            "profile", "release" -> "release"
            else -> throw GradleException("Unsupported Android Rust build type: ${variant.buildType}")
        }
        val cargoBuildOperitFlutterBridgeTasks = selectedOperitRustTargets.map { target ->
            val abiTaskSuffix = target.abi.replace("-", "").replace("_", "")
            val bridgeLibrary = operitBridgeCrate.resolve(
                "target/${target.rustTarget}/$rustProfile/liboperit_flutter_bridge.so",
            )
            tasks.register<Exec>("cargoBuildOperitFlutterBridge$variantTaskSuffix$abiTaskSuffix") {
                dependsOn(syncOperitPlugins)
                val clangPrefix = target.rustTarget
                val apiLevel = 23
                val ndkToolchain = android.ndkDirectory
                    .resolve("toolchains")
                    .resolve("llvm")
                    .resolve("prebuilt")
                    .resolve(if (operitIsWindows) "windows-x86_64" else "linux-x86_64")
                    .resolve("bin")
                val linkerPrefix = if (target.rustTarget == "armv7-linux-androideabi") {
                    "armv7a-linux-androideabi"
                } else {
                    clangPrefix
                }
                val linker = ndkToolchain.resolve("${linkerPrefix}${apiLevel}-clang${if (operitIsWindows) ".cmd" else ""}")
                val ar = ndkToolchain.resolve(if (operitIsWindows) "llvm-ar.exe" else "llvm-ar")
                val clangResourceDir = ndkToolchain
                    .parentFile
                    .resolve("lib")
                    .resolve("clang")
                    .listFiles()
                    ?.single { it.isDirectory }
                    ?: throw GradleException("Android NDK clang resource dir not found")
                val bindgenClangArgs =
                    "--target=${target.rustTarget} --sysroot=${ndkToolchain.parentFile.resolve("sysroot").clangPath()} -resource-dir=${clangResourceDir.clangPath()}"
                val ccEnvTarget = target.rustTarget.replace("-", "_")
                environment("CC_$ccEnvTarget", linker.absolutePath)
                environment("AR_$ccEnvTarget", ar.absolutePath)
                environment("CARGO_TARGET_${target.envTarget}_LINKER", linker.absolutePath)
                environment("CARGO_TARGET_${target.envTarget}_AR", ar.absolutePath)
                environment("LIBCLANG_PATH", operitLibclangDir.absolutePath)
                environment("BINDGEN_EXTRA_CLANG_ARGS_$ccEnvTarget", bindgenClangArgs)
                environment("RUSTFLAGS", "-Awarnings")
                val cargoArguments = mutableListOf(
                    "cargo",
                    "build",
                    "--manifest-path",
                    operitBridgeCrate.resolve("Cargo.toml").absolutePath,
                    "--target",
                    target.rustTarget,
                )
                if (rustProfile == "release") {
                    cargoArguments.add("--release")
                }
                commandLine(cargoArguments)
                doLast {
                    if (!bridgeLibrary.isFile) {
                        throw GradleException("Missing Android Rust $rustProfile bridge: $bridgeLibrary")
                    }
                }
            }
        }
        val stagedJniLibraries = tasks.register<StageOperitAndroidJniLibraries>(
            "stageOperitAndroidJniLibraries$variantTaskSuffix",
        ) {
            dependsOn(cargoBuildOperitFlutterBridgeTasks)
            selectedOperitRustTargets.forEach { target ->
                into(target.abi) {
                    from(operitBridgeJniLibs.resolve(target.abi)) {
                        exclude("liboperit_flutter_bridge.so")
                    }
                    from(operitBridgeCrate.resolve(
                        "target/${target.rustTarget}/$rustProfile/liboperit_flutter_bridge.so",
                    ))
                }
            }
            into(outputDirectory)
        }
        val jniLibraries = variant.sources.jniLibs
            ?: throw GradleException("Android variant ${variant.name} has no JNI library sources")
        jniLibraries.addGeneratedSourceDirectory(stagedJniLibraries) { it.outputDirectory }
    }
}

val requiredOperitAndroidRuntimeLibraries = listOf(
    "libbash.so",
    "liboperit_busybox.so",
    "liboperit_loader.so",
    "liboperit_proot.so",
)
val requiredOperitAndroidRuntimeAssets = listOf(
    "rootfs.tar.gz.bin",
    "rootfs.tar.gz.bin.sha256",
)
val verifyOperitAndroidRuntimeArtifacts = tasks.register("verifyOperitAndroidRuntimeArtifacts") {
    doLast {
        val missingArtifacts = selectedOperitRustTargets.flatMap { target ->
            val libraryDirectory = operitBridgeJniLibs.resolve(target.abi)
            val assetDirectory = project.layout.projectDirectory
                .dir("src/main/assets/android-runtime/${target.abi}")
                .asFile
            requiredOperitAndroidRuntimeLibraries
                .map { libraryName -> libraryDirectory.resolve(libraryName) }
                .plus(
                    requiredOperitAndroidRuntimeAssets.map { assetName ->
                        assetDirectory.resolve(assetName)
                    },
                )
                .filterNot(File::isFile)
        }
        if (missingArtifacts.isNotEmpty()) {
            throw GradleException(
                "Missing required Operit Android runtime artifacts:\n" +
                    missingArtifacts.joinToString(separator = "\n") { it.absolutePath },
            )
        }
    }
}

tasks.named("preBuild") {
    dependsOn(syncOperitPlugins)
    dependsOn(verifyOperitAndroidRuntimeArtifacts)
}
