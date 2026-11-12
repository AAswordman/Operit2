package com.operit.edgephone;

/** Local JNI entry points for the standalone Edge SDK product. */
final class EdgeNative {
    static { System.loadLibrary("operit_edge_phone"); }
    /** Starts the native owner using this application's private files directory. */
    static native String start(String root, String model);
    /** Executes a bounded typed local command on the native owner. */
    static native String execute(String command);
    /** Prevents creating a second native entry point instance. */
    private EdgeNative() {}
}
