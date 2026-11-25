package app.operit.windowsmoke;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.Context;
import android.content.Intent;
import android.os.Bundle;
import android.os.SystemClock;
import java.io.File;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Proxy;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.HashMap;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.TimeUnit;
import org.json.JSONObject;

/** Exercises the live app's actual Flutter owner channel without adding a production endpoint. */
public final class OwnerProbe extends Instrumentation {
    /** Starts only when explicitly launched through Android's test instrumentation. */
    @Override public void onCreate(Bundle arguments) { super.onCreate(arguments); start(); }

    /** Keeps one real activity and owner alive across correlated device-test requests. */
    @Override public void onStart() {
        Bundle result = new Bundle();
        try {
            android.util.Log.i("OperitOwnerProbe", "Starting actual MainActivity");
            Context context = getTargetContext();
            context.startActivity(new Intent().setClassName("app.operit", "app.operit.MainActivity")
                    .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
            Class<?> main = context.getClassLoader().loadClass("app.operit.MainActivity");
            Object companion = main.getField("Companion").get(null);
            Activity activity = null;
            long startDeadline = SystemClock.uptimeMillis() + 30000;
            while (activity == null && SystemClock.uptimeMillis() < startDeadline) {
                activity = (Activity) companion.getClass().getMethod("currentActivity").invoke(companion);
                if (activity == null) SystemClock.sleep(50);
            }
            if (activity == null) throw new IllegalStateException("MainActivity did not start");
            android.util.Log.i("OperitOwnerProbe", "Actual MainActivity is ready");
            Object owner = field(activity, "ownerSystem");
            File request = new File(context.getFilesDir(), "window-probe-request.json");
            File response = new File(context.getFilesDir(), "window-probe-response.json");
            request.delete();
            write(response, new JSONObject().put("ready", true));
            long deadline = SystemClock.uptimeMillis() + 600000;
            while (SystemClock.uptimeMillis() < deadline) {
                if (!request.exists()) { SystemClock.sleep(50); continue; }
                JSONObject input = new JSONObject(Files.readString(request.toPath(), StandardCharsets.UTF_8));
                request.delete();
                JSONObject output = new JSONObject().put("sequence", input.getString("sequence"));
                try {
                    String operation = input.getString("operation");
                    if (operation.equals("stop")) { write(response, output.put("success", true)); break; }
                    Object value;
                    if (operation.equals("activate")) {
                        Class<?> type = Class.forName("app.operit.shizuku.EmbeddedShizuku");
                        Object embedded = type.getField("INSTANCE").get(null);
                        type.getMethod("selectComputerActivation", Context.class).invoke(embedded, context);
                        value = type.getMethod("command", Context.class).invoke(embedded, context);
                    } else if (operation.equals("windows")) {
                        Object server = field(owner, "uiServer");
                        value = server.getClass().getMethod("windows").invoke(server);
                    } else if (operation.equals("host")) {
                        value = host(activity, owner, input.getString("hostOperation"), input.getJSONObject("parameters"));
                    } else throw new IllegalArgumentException("Unknown probe operation: " + operation);
                    output.put("success", true).put("data", value);
                } catch (Exception error) {
                    Throwable cause = error;
                    while (cause instanceof InvocationTargetException && cause.getCause() != null) cause = cause.getCause();
                    output.put("success", false).put("error", android.util.Log.getStackTraceString(cause));
                }
                write(response, output);
            }
            result.putString("result", "Owner probe finished");
            finish(Activity.RESULT_OK, result);
        } catch (Exception error) {
            result.putString("error", android.util.Log.getStackTraceString(error));
            finish(Activity.RESULT_CANCELED, result);
        }
    }

    /** Sends the same ownerSystemOperation MethodCall used by the running Flutter application. */
    private Object host(Activity activity, Object owner, String operation, JSONObject parameters) throws Exception {
        Class<?> callType = Class.forName("io.flutter.plugin.common.MethodCall");
        Class<?> resultType = Class.forName("io.flutter.plugin.common.MethodChannel$Result");
        HashMap<String, Object> payload = new HashMap<>();
        payload.put("operation", operation);
        payload.put("paramsJson", parameters.toString());
        Object call = callType.getConstructor(String.class, Object.class).newInstance("ownerSystemOperation", payload);
        CompletableFuture<Object> reply = new CompletableFuture<>();
        Object callback = Proxy.newProxyInstance(resultType.getClassLoader(), new Class<?>[]{resultType},
                (proxy, method, arguments) -> {
                    if (method.getName().equals("success")) reply.complete(new JSONObject((java.util.Map<?, ?>) arguments[0]));
                    else if (method.getName().equals("error")) reply.completeExceptionally(new IllegalStateException(arguments[0] + ": " + arguments[1]));
                    else if (method.getName().equals("notImplemented")) reply.completeExceptionally(new IllegalStateException("Owner operation not implemented"));
                    return null;
                });
        activity.runOnUiThread(() -> {
            try { owner.getClass().getMethod("handle", callType, resultType).invoke(owner, call, callback); }
            catch (Exception error) { reply.completeExceptionally(error); }
        });
        return reply.get(45, TimeUnit.SECONDS);
    }

    /** Inspects the real activity's existing owner without constructing a parallel Host. */
    private static Object field(Object target, String name) throws Exception {
        Field field = target.getClass().getDeclaredField(name);
        field.setAccessible(true);
        return field.get(target);
    }

    /** Publishes a UTF-8 response for the debug app's run-as test driver. */
    private static void write(File target, JSONObject value) throws Exception {
        Files.writeString(target.toPath(), value.toString(), StandardCharsets.UTF_8);
    }
}
