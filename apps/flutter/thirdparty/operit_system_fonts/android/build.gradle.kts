group = "app.operit.system_fonts"
version = "0.1.0"
plugins { id("com.android.library") }
android {
    namespace = "app.operit.system_fonts"
    compileSdk = flutter.compileSdkVersion
    defaultConfig { minSdk = 24 }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}
