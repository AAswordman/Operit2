group = "app.operit.keep_alive"
version = "1.0"

plugins { id("com.android.library") }

android {
    namespace = "app.operit.keep_alive"
    compileSdk = flutter.compileSdkVersion
    defaultConfig { minSdk = 24 }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.mockito:mockito-core:5.23.0")
    testImplementation("org.robolectric:robolectric:4.16")
}
