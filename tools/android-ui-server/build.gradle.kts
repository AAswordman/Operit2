import com.android.build.api.artifact.SingleArtifact

plugins {
    id("com.android.application")
}

android {
    namespace = "app.operit.uiserver"
    compileSdk = 35
    defaultConfig {
        applicationId = "app.operit.uiserver"
        minSdk = 30
        targetSdk = 35
        versionCode = 1
        versionName = "1"
    }
    buildFeatures { aidl = true }
    sourceSets.getByName("main").aidl.srcDir("contract/src/main/aidl")
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    buildTypes.getByName("release") {
        isMinifyEnabled = false
    }
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
