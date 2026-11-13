import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "app.operit.accessibility"
    compileSdk = 35
    defaultConfig {
        applicationId = "app.operit.accessibility"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "1.0.0"
    }
    buildFeatures { aidl = true }
    sourceSets.getByName("main").aidl.srcDir("contract/src/main/aidl")
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions { jvmTarget = "17" }
    val signingPath = providers.gradleProperty("operitSigningProperties").orNull
    if (signingPath != null) {
        val signing = Properties().apply { file(signingPath).inputStream().use { load(it) } }
        signingConfigs.create("operitRelease") {
            storeFile = file(requireNotNull(signing.getProperty("RELEASE_STORE_FILE")))
            storePassword = requireNotNull(signing.getProperty("RELEASE_STORE_PASSWORD"))
            keyAlias = requireNotNull(signing.getProperty("RELEASE_KEY_ALIAS"))
            keyPassword = requireNotNull(signing.getProperty("RELEASE_KEY_PASSWORD"))
        }
        buildTypes.getByName("release").signingConfig = signingConfigs.getByName("operitRelease")
    }
}
