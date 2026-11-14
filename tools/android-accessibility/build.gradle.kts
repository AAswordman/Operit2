import java.util.Properties
import com.android.build.api.artifact.SingleArtifact
import com.android.build.api.dsl.ApplicationExtension

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

val embedded = rootProject != project
val mainAndroid = if (embedded) project(":app").extensions.getByType<ApplicationExtension>() else null
val providerVersion = Properties().apply {
    layout.projectDirectory.file("version.properties").asFile.inputStream().use { load(it) }
}

android {
    namespace = "app.operit.accessibility"
    compileSdk = 35
    defaultConfig {
        applicationId = "app.operit.accessibility"
        minSdk = 26
        targetSdk = 35
        versionCode = providerVersion.getProperty("versionCode").toInt()
        versionName = providerVersion.getProperty("versionName")
    }
    buildFeatures { aidl = true }
    sourceSets.getByName("main").aidl.srcDir("contract/src/main/aidl")
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions { jvmTarget = "17" }
    if (embedded) {
        val main = requireNotNull(mainAndroid)
        signingConfigs.getByName("debug").initWith(requireNotNull(main.buildTypes.getByName("debug").signingConfig))
        signingConfigs.create("operitRelease").initWith(requireNotNull(main.buildTypes.getByName("release").signingConfig))
        buildTypes.getByName("release").signingConfig = signingConfigs.getByName("operitRelease")
    } else if (providers.gradleProperty("operitSigningProperties").isPresent) {
        val signing = Properties().apply {
            file(providers.gradleProperty("operitSigningProperties").get()).inputStream().use { load(it) }
        }
        signingConfigs.create("operitRelease") {
            storeFile = file(requireNotNull(signing.getProperty("RELEASE_STORE_FILE")))
            storePassword = requireNotNull(signing.getProperty("RELEASE_STORE_PASSWORD"))
            keyAlias = requireNotNull(signing.getProperty("RELEASE_KEY_ALIAS"))
            keyPassword = requireNotNull(signing.getProperty("RELEASE_KEY_PASSWORD"))
        }
        buildTypes.getByName("release").signingConfig = signingConfigs.getByName("operitRelease")
    }
}

dependencies {
    testImplementation("junit:junit:4.13.2")
}

androidComponents {
    onVariants(selector().all()) { variant ->
        configurations.create("${variant.name}Apk") {
            isCanBeConsumed = true
            isCanBeResolved = false
            outgoing.artifact(variant.artifacts.get(SingleArtifact.APK))
        }
    }
}
