# AndroidRuntimeHost methods are resolved by Rust through JNI GetMethodID.
# Their Java names and descriptors are an ABI shared with the native bridge.
-keepclassmembers class app.operit.AndroidRuntimeHost {
    public *** *(...);
}
