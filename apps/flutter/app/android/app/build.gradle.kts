plugins {
    id("com.android.application")
    id("kotlin-android")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

android {
    namespace = "com.operit.operit2"
    compileSdk = flutter.compileSdkVersion
    ndkVersion = flutter.ndkVersion

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = JavaVersion.VERSION_17.toString()
    }

    defaultConfig {
        // TODO: Specify your own unique Application ID (https://developer.android.com/studio/build/application-id.html).
        applicationId = "com.operit.operit2"
        // You can update the following values to match your application needs.
        // For more information, see: https://flutter.dev/to/review-gradle-config.
        minSdk = flutter.minSdkVersion
        targetSdk = flutter.targetSdkVersion
        versionCode = flutter.versionCode
        versionName = flutter.versionName
        ndk {
            abiFilters += listOf("arm64-v8a", "x86_64")
        }
    }

    buildTypes {
        release {
            // TODO: Add your own signing config for the release build.
            // Signing with the debug keys for now, so `flutter run --release` works.
            signingConfig = signingConfigs.getByName("debug")
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
val operitRustTargets = listOf(
    Triple("arm64-v8a", "aarch64-linux-android", "AARCH64_LINUX_ANDROID"),
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
        val linker = ndkToolchain.resolve("${clangPrefix}${apiLevel}-clang.cmd")
        val ar = ndkToolchain.resolve("llvm-ar.exe")
        val ccEnvTarget = rustTarget.replace("-", "_")
        environment("CC_$ccEnvTarget", linker.absolutePath)
        environment("AR_$ccEnvTarget", ar.absolutePath)
        environment("CARGO_TARGET_${envTarget}_LINKER", linker.absolutePath)
        environment("CARGO_TARGET_${envTarget}_AR", ar.absolutePath)
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
