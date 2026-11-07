# The module application starts through app_process and JNI rather than manifest discovery.
-keep class app.operit.ModuleMain {
    *;
}
