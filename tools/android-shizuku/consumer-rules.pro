-keep class rikka.shizuku.server.** { *; }
-keep class moe.shizuku.starter.** { *; }
-keep class moe.shizuku.manager.adb.PairingContext { *; }
-keep class app.operit.embeddedshizuku.** { *; }

# Android provides these hidden framework classes from the compile-only stub dependency.
-dontwarn com.android.internal.app.IAppOpsService
-dontwarn com.android.org.conscrypt.Conscrypt
