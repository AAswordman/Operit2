package app.operit.uiserver;

import app.operit.windowserver.IUiServer;

import android.content.Intent;
import android.os.Binder;
import android.os.Bundle;
import android.os.Handler;
import android.os.IBinder;
import android.os.Looper;
import android.os.Process;
import android.system.Os;
import android.util.Log;
import java.io.File;
import java.lang.reflect.InvocationTargetException;
import java.util.ArrayList;
import java.util.BitSet;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.ExecutionException;
import org.json.JSONArray;
import org.json.JSONObject;

/** Runs an uninstalled shell-owned window server with Binder control and direct Surfaces. */
public final class UiServerMain {
    private final Handler main = new Handler(Looper.getMainLooper());
    private final Map<Integer, DisplayWindow> windows = new LinkedHashMap<>();
    private final BitSet dockSlots = new BitSet();
    private final ShellContext context;
    private final DisplayInput input;
    private final int ownerUid;
    private IBinder owner;
    private boolean stopping;
    private final IBinder.DeathRecipient death = () -> main.post(this::shutdown);

    /** Uses the existing Android Host's terminal identity implementation for Root launches. */
    private static native void restartAsShell(String[] arguments);

    /** Enters shell identity before starting Android framework and Binder services. */
    public static void main(String[] args) throws Exception {
        if (args.length != 4) throw new IllegalArgumentException("Expected owner UID, package, nonce and native library");
        if (Process.myUid() == 0) {
            System.load(args[3]);
            restartAsShell(new String[]{new File("/proc/self/exe").getCanonicalPath(), "/",
                    "app.operit.uiserver.UiServerMain", args[0], args[1], args[2], args[3]});
            throw new IllegalStateException("Shell process exec unexpectedly returned");
        }
        if (Process.myUid() != 2000 || Os.getgid() != 2000) throw new SecurityException("UI server requires uid/gid 2000");
        Process.class.getMethod("setArgV0", String.class).invoke(null, ShellContext.PROCESS_NAME);
        Looper.prepare();
        java.lang.reflect.Field mainLooper = Looper.class.getDeclaredField("sMainLooper");
        mainLooper.setAccessible(true);
        mainLooper.set(null, Looper.myLooper());
        UiServerMain server = new UiServerMain(Integer.parseInt(args[0]), args[1]);
        Bundle endpoint = new Bundle();
        endpoint.putBinder("server", server.binder);
        server.context.handoff(new Intent(args[1] + ".UI_SERVER_READY")
                .setPackage(args[1]).putExtra("nonce", args[2]).putExtra("endpoint", endpoint));
        server.main.postDelayed(() -> { if (server.owner == null) server.shutdown(); }, 15000);
        try { Looper.loop(); }
        catch (Throwable error) {
            Log.e("OperitUiServer", "Native window event loop failed", error);
            throw error;
        }
        finally {
            server.shutdown();
            File payload = new File(System.getenv("CLASSPATH"));
            File directory = payload.getParentFile();
            if (!payload.getName().equals("server.jar") || directory == null
                    || !directory.getParentFile().getCanonicalPath().equals("/data/local/tmp")
                    || !directory.getName().startsWith("operit-ui-")) {
                throw new SecurityException("Unexpected UI server deployment directory");
            }
            for (String name : new String[]{"server.jar", "server.b64", "server.log"}) {
                File file = new File(directory, name);
                if (file.exists() && !file.delete()) Log.w("OperitUiServer", "Cannot remove " + file);
            }
            if (!directory.delete()) Log.w("OperitUiServer", "Cannot remove " + directory);
        }
        System.exit(0);
    }

    /** Constructs framework and display input after shell identity is verified. */
    private UiServerMain(int ownerUid, String ownerPackage) throws Exception {
        this.ownerUid = ownerUid;
        context = ShellContext.create(ownerUid, ownerPackage);
        input = new DisplayInput();
    }

    private final IUiServer.Stub binder = new IUiServer.Stub() {
        /** Exposes the wire version only to the process that launched this server. */
        @Override public int getProtocolVersion() { checkCaller(); return 2; }

        /** Ties every window to one live Android Host Binder token. */
        @Override public void attach(IBinder token, IBinder windowSession) {
            checkCaller();
            CompletableFuture<JSONObject> attached = new CompletableFuture<>();
            main.post(() -> {
                try {
                    if (attached.isCancelled()) return;
                    if (stopping || owner != null || token == null) throw new IllegalStateException("UI server owner is unavailable");
                    context.attachWindowSession(windowSession);
                    token.linkToDeath(death, 0);
                    owner = token;
                    if (!attached.complete(new JSONObject())) shutdown();
                } catch (Exception error) { attached.completeExceptionally(error); }
            });
            await(attached);
        }

        /** Serializes control operations on the thread that owns the native Views. */
        @Override public String execute(String encoded) {
            checkCaller();
            CompletableFuture<JSONObject> result = new CompletableFuture<>();
            main.post(() -> {
                try {
                    if (result.isCancelled()) return;
                    if (stopping || owner == null || !owner.isBinderAlive()) throw new IllegalStateException("UI server owner disconnected");
                    dispatch(new JSONObject(encoded), result);
                } catch (Exception error) {
                    Log.e("OperitUiServer", "Native window request failed", error);
                    result.completeExceptionally(error);
                }
            });
            try { return new JSONObject().put("success", true).put("data", await(result)).toString(); }
            catch (Exception error) {
                try { return new JSONObject().put("success", false).put("error", error.toString()).toString(); }
                catch (Exception encodingError) { throw new IllegalStateException(encodingError); }
            }
        }
    };

    /** Rejects callers from other application UIDs even if they obtain the Binder. */
    private void checkCaller() {
        if (Binder.getCallingUid() != ownerUid) throw new SecurityException("UI server belongs to another Host");
    }

    /** Bounds Binder waits and cancels pending creation before reporting a timeout. */
    private static JSONObject await(CompletableFuture<JSONObject> result) {
        try { return result.get(10, TimeUnit.SECONDS); }
        catch (Exception error) {
            result.cancel(false);
            if (error instanceof InterruptedException) Thread.currentThread().interrupt();
            Throwable cause = error;
            while ((cause instanceof ExecutionException || cause instanceof InvocationTargetException) && cause.getCause() != null) {
                cause = cause.getCause();
            }
            throw new IllegalStateException("UI server operation failed: " + cause, cause);
        }
    }

    /** Handles only native Android transport operations behind the unified Host boundary. */
    private void dispatch(JSONObject request, CompletableFuture<JSONObject> result) throws Exception {
        String operation = request.getString("operation");
        if (operation.equals("launch")) {
            String component = request.getString("component");
            String packageName = android.content.ComponentName.unflattenFromString(component).getPackageName();
            for (DisplayWindow window : windows.values()) {
                if (window.packageName.equals(packageName)) {
                    window.launch(component);
                    result.complete(window.describe());
                    return;
                }
            }
            DisplayWindow window = new DisplayWindow(context, input, component, result, windows::remove,
                    dockSlots, this::bringToFront);
            window.open(() -> windows.put(window.displayId(), window));
            return;
        }
        if (operation.equals("windows")) {
            JSONArray entries = new JSONArray();
            for (DisplayWindow window : windows.values()) entries.put(window.describe());
            result.complete(new JSONObject().put("windows", entries));
            return;
        }
        if (operation.equals("close_windows")) {
            for (DisplayWindow window : new ArrayList<>(windows.values())) window.close();
        } else if (operation.equals("close_package")) {
            for (DisplayWindow window : new ArrayList<>(windows.values())) {
                if (window.packageName.equals(request.getString("packageName"))) window.close();
            }
        } else if (operation.equals("shutdown")) {
            shutdown();
        } else {
            String windowId = request.getString("windowId");
            DisplayWindow window = null;
            for (DisplayWindow candidate : windows.values()) {
                if (candidate.windowId.equals(windowId)) { window = candidate; break; }
            }
            if (window == null) throw new IllegalStateException("Selected Android window has closed; refresh list_windows");
            if (operation.equals("focus")) window.raise();
            else if (operation.equals("key")) {
                String key = request.getString("key").toUpperCase(java.util.Locale.ROOT);
                input.key(window.displayId(), android.view.KeyEvent.keyCodeFromString(
                        key.startsWith("KEYCODE_") ? key : "KEYCODE_" + key));
            } else throw new IllegalArgumentException("Unknown UI server operation: " + operation);
        }
        result.complete(new JSONObject());
    }

    /** Preserves the current Surface whenever the requested window is already on top. */
    private void bringToFront(DisplayWindow window) {
        DisplayWindow front = null;
        for (DisplayWindow candidate : windows.values()) front = candidate;
        if (front == window) return;
        window.restack();
        windows.remove(window.displayId());
        windows.put(window.displayId(), window);
    }

    /** Releases every Surface and display when the owning Android Host disconnects. */
    private void shutdown() {
        if (stopping) return;
        stopping = true;
        for (DisplayWindow window : new ArrayList<>(windows.values())) {
            try { window.close(); }
            catch (Exception error) { Log.e("OperitUiServer", "Window cleanup failed", error); }
        }
        if (owner != null) owner.unlinkToDeath(death, 0);
        Looper.getMainLooper().quitSafely();
    }
}
