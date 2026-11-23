plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    id("dev.rikka.tools.refine")
    id("dev.rikka.tools.materialthemebuilder")
}

apply(from = "ui-theme.gradle")

android {
    namespace = "app.operit.embeddedshizuku"
    compileSdk = 36
    defaultConfig {
        minSdk = 24
        consumerProguardFiles("consumer-rules.pro")
        externalNativeBuild.cmake.arguments += "-DANDROID_STL=none"
    }
    buildFeatures { prefab = true; viewBinding = true }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_21
        targetCompatibility = JavaVersion.VERSION_21
    }
    kotlinOptions { jvmTarget = "21" }
    sourceSets.getByName("main") {
        java.srcDirs("upstream/server/java", "upstream/starter", "upstream/server-shared", "upstream/adb")
        assets.srcDir("licenses")
        res.srcDir("upstream/manager-ui/res")
    }
    externalNativeBuild.cmake {
        path = file("src/main/cpp/CMakeLists.txt")
        version = "3.22.1"
    }
}

dependencies {
    api("dev.rikka.shizuku:api:13.1.5")
    api("dev.rikka.shizuku:provider:13.1.5")
    api("com.google.android.material:material:1.12.0")
    api("dev.rikka.rikkax.appcompat:appcompat:1.6.1")
    api("dev.rikka.rikkax.material:material:2.7.2")
    api("dev.rikka.rikkax.core:core-ktx:1.4.1")
    api("dev.rikka.rikkax.insets:insets:1.3.0")
    api("dev.rikka.rikkax.widget:borderview:1.1.0")
    api("dev.rikka.rikkax.compatibility:compatibility:2.0.0")
    api("androidx.recyclerview:recyclerview:1.4.0")
    implementation("androidx.core:core-ktx:1.16.0")
    implementation("androidx.lifecycle:lifecycle-livedata-core:2.9.0")
    implementation("dev.rikka.hidden:compat:4.4.0")
    compileOnly("dev.rikka.hidden:stub:4.4.0")
    implementation("dev.rikka.tools.refine:runtime:4.4.0")
    implementation("org.bouncycastle:bcpkix-jdk18on:1.80")
    implementation("io.github.vvb2060.ndk:boringssl:20250114")
    implementation("org.lsposed.libcxx:libcxx:27.0.12077973")
}

configurations.configureEach { exclude(group = "androidx.appcompat", module = "appcompat") }
