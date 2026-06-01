import java.io.FileInputStream
import java.util.Properties

plugins {
    id("com.android.application")
    id("kotlin-android")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

val localProperties = Properties()
val localPropertiesFile = rootProject.file("local.properties")
localProperties.load(FileInputStream(localPropertiesFile))

fun requiredLocalProperty(name: String): String =
    localProperties.getProperty(name)
        ?: throw GradleException("Missing Android release signing property: $name")

android {
    namespace = "com.ai.assistance.operit2"
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
        applicationId = "com.ai.assistance.operit2"
        // You can update the following values to match your application needs.
        // For more information, see: https://flutter.dev/to/review-gradle-config.
        minSdk = flutter.minSdkVersion
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName
    }

    buildTypes {
        release {
            signingConfig = signingConfigs.getByName("release")
        }
    }

    packaging {
        jniLibs {
            useLegacyPackaging = true
        }
    }
}

flutter {
    source = "../.."
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
val operitLibclangDir = operitRepoRoot
    .resolve("target/operit-build-tools/libclang.runtime.win-x64.21.1.8/runtimes/win-x64/native")
fun File.clangPath(): String = absolutePath.replace('\\', '/')
val operitRustTargets = listOf(
    Triple("arm64-v8a", "aarch64-linux-android", "AARCH64_LINUX_ANDROID"),
    Triple("armeabi-v7a", "armv7-linux-androideabi", "ARMV7_LINUX_ANDROIDEABI"),
    Triple("x86_64", "x86_64-linux-android", "X86_64_LINUX_ANDROID"),
)

val syncOperitPlugins = tasks.register<Exec>("syncOperitPlugins") {
    workingDir = operitRepoRoot
    commandLine(
        operitPluginSyncPython.absolutePath,
        operitPluginSyncScript.absolutePath,
        "--source",
        "buildin",
    )
}

val cargoBuildOperitFlutterBridgeTasks = operitRustTargets.map { (abi, rustTarget, envTarget) ->
    tasks.register<Exec>("cargoBuildOperitFlutterBridge${abi.replace("-", "").replace("_", "")}") {
        dependsOn(syncOperitPlugins)
        val clangPrefix = rustTarget
        val apiLevel = 23
        val ndkToolchain = android.ndkDirectory
            .resolve("toolchains")
            .resolve("llvm")
            .resolve("prebuilt")
            .resolve("windows-x86_64")
            .resolve("bin")
        val linkerPrefix = if (rustTarget == "armv7-linux-androideabi") {
            "armv7a-linux-androideabi"
        } else {
            clangPrefix
        }
        val linker = ndkToolchain.resolve("${linkerPrefix}${apiLevel}-clang.cmd")
        val ar = ndkToolchain.resolve("llvm-ar.exe")
        val clangResourceDir = ndkToolchain
            .parentFile
            .resolve("lib")
            .resolve("clang")
            .listFiles()
            ?.single { it.isDirectory }
            ?: throw GradleException("Android NDK clang resource dir not found")
        val bindgenClangArgs =
            "--target=$rustTarget --sysroot=${ndkToolchain.parentFile.resolve("sysroot").clangPath()} -resource-dir=${clangResourceDir.clangPath()}"
        val ccEnvTarget = rustTarget.replace("-", "_")
        environment("CC_$ccEnvTarget", linker.absolutePath)
        environment("AR_$ccEnvTarget", ar.absolutePath)
        environment("CARGO_TARGET_${envTarget}_LINKER", linker.absolutePath)
        environment("CARGO_TARGET_${envTarget}_AR", ar.absolutePath)
        environment("LIBCLANG_PATH", operitLibclangDir.absolutePath)
        environment("BINDGEN_EXTRA_CLANG_ARGS_$ccEnvTarget", bindgenClangArgs)
        commandLine(
            "cargo",
            "build",
            "--manifest-path",
            operitBridgeCrate.resolve("Cargo.toml").absolutePath,
            "--target",
            rustTarget,
        )
        doLast {
            copy {
                from(operitBridgeCrate.resolve("target/$rustTarget/debug/liboperit_flutter_bridge.so"))
                into(operitBridgeJniLibs.resolve(abi))
            }
        }
    }
}

val cargoBuildOperitFlutterBridge = tasks.register("cargoBuildOperitFlutterBridge") {
    dependsOn(cargoBuildOperitFlutterBridgeTasks)
}

tasks.named("preBuild") {
    dependsOn(syncOperitPlugins)
    dependsOn(cargoBuildOperitFlutterBridge)
}
