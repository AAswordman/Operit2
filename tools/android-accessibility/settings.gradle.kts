pluginManagement {
    repositories { google(); mavenCentral(); gradlePluginPortal() }
    plugins {
        id("com.android.application") version "8.11.1"
        id("org.jetbrains.kotlin.android") version "2.2.20"
    }
}
dependencyResolutionManagement { repositories { google(); mavenCentral() } }
rootProject.name = "OperitAccessibility"
