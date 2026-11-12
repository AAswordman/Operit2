package com.operit.edgephone;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.Intent;
import android.os.Binder;
import android.os.Build;
import android.os.Handler;
import android.os.IBinder;
import android.os.Looper;
import org.json.JSONObject;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** Owns the node independently of Activity rotation and screen visibility. */
public final class EdgeService extends Service {
    interface Listener { void changed(JSONObject status, String error); }
    interface Callback { void complete(JSONObject data, String error); }
    public final class LocalBinder extends Binder {
        /** Returns the sole service owner to this app's bound Activity. */
        EdgeService service() { return EdgeService.this; }
    }
    private final LocalBinder binder = new LocalBinder();
    private final Handler main = new Handler(Looper.getMainLooper());
    private final ExecutorService worker = Executors.newSingleThreadExecutor();
    private Listener listener;
    private JSONObject status;
    private String failure;
    private boolean destroyed;
    private boolean running; // Accessed only on the serialized worker.
    private boolean pollPending;
    private final Runnable poll = new Runnable() {
        /** Polls local display state only while the UI is visible. */
        @Override public void run() {
            if (listener == null || destroyed || pollPending) return;
            pollPending = true;
            request(command("status"), (data, error) -> {
                pollPending = false;
                if (listener != null && !destroyed) main.postDelayed(poll, 1500);
            });
        }
    };

    /** Enters foreground promptly, then starts the native node away from the UI thread. */
    @Override public void onCreate() {
        super.onCreate();
        notification(false);
        worker.execute(() -> {
            try {
                JSONObject result = decode(EdgeNative.start(getFilesDir().getAbsolutePath() + "/edge", Build.MODEL));
                running = true;
                publish(result, null);
            } catch (Exception | LinkageError error) {
                publish(null, error.toString());
                main.post(() -> notification(true));
            }
        });
    }

    /** Requires an explicit app launch after Android terminates the process. */
    @Override public int onStartCommand(Intent intent, int flags, int startId) { return START_NOT_STICKY; }
    /** Shares one node with the visible screen without exposing IPC to other apps. */
    @Override public IBinder onBind(Intent intent) { return binder; }

    /** Attaches display observation without restarting native services. */
    void attach(Listener next) {
        listener = next;
        listener.changed(status, failure);
        main.removeCallbacks(poll);
        main.post(poll);
    }

    /** Suspends UI polling while keeping the Edge network listener alive. */
    void detach(Listener previous) {
        if (listener == previous) listener = null;
        main.removeCallbacks(poll);
    }

    /** Runs typed management commands in order, including orderly node shutdown. */
    void request(JSONObject command, Callback callback) {
        if (destroyed) { callback.complete(null, "节点服务已停止"); return; }
        worker.execute(() -> {
            JSONObject result = null;
            String error = null;
            try {
                if (!running) throw new IllegalStateException("边缘节点未运行，请查看启动错误");
                result = decode(EdgeNative.execute(command.toString()));
                if ("stop".equals(command.optString("action"))) running = false;
                else publish("status".equals(command.optString("action")) ? result
                    : decode(EdgeNative.execute(command("status").toString())), null);
            } catch (Exception | LinkageError problem) { error = problem.toString(); }
            JSONObject reply = result;
            String problem = error;
            main.post(() -> { if (!destroyed) callback.complete(reply, problem); });
        });
    }

    /** Releases the listener and executor when the foreground service ends. */
    @Override public void onDestroy() {
        destroyed = true;
        listener = null;
        main.removeCallbacks(poll);
        worker.execute(() -> {
            if (running) {
                try { decode(EdgeNative.execute(command("stop").toString())); }
                catch (Exception | LinkageError error) { android.util.Log.e("OperitEdge", "Node stop failed", error); }
                running = false;
            }
        });
        worker.shutdown();
        super.onDestroy();
    }

    /** Delivers native state on the Android main thread. */
    private void publish(JSONObject next, String error) {
        main.post(() -> {
            if (destroyed) return;
            status = next;
            failure = error;
            if (listener != null) listener.changed(next, error);
        });
    }

    /** Separates explicit native errors from successful empty management results. */
    private static JSONObject decode(String text) throws Exception {
        JSONObject reply = new JSONObject(text);
        if (!reply.getBoolean("ok")) throw new IllegalStateException(reply.getString("error"));
        return reply.getJSONObject("data");
    }

    /** Creates a command with a fixed SDK management action. */
    static JSONObject command(String action) {
        JSONObject value = new JSONObject();
        try { value.put("action", action); }
        catch (org.json.JSONException error) { throw new IllegalArgumentException(error); }
        return value;
    }

    /** Uses only framework notifications, including the Android 5 notification path. */
    private void notification(boolean failed) {
        NotificationManager manager = (NotificationManager) getSystemService(NOTIFICATION_SERVICE);
        Notification.Builder builder;
        if (Build.VERSION.SDK_INT >= 26) {
            manager.createNotificationChannel(new NotificationChannel("edge", getString(R.string.channel_name), NotificationManager.IMPORTANCE_LOW));
            builder = new Notification.Builder(this, "edge");
        } else builder = new Notification.Builder(this);
        PendingIntent open = PendingIntent.getActivity(this, 0, new Intent(this, MainActivity.class),
            PendingIntent.FLAG_UPDATE_CURRENT | (Build.VERSION.SDK_INT >= 23 ? PendingIntent.FLAG_IMMUTABLE : 0));
        Notification notification = builder.setSmallIcon(R.drawable.ic_edge).setContentTitle(getString(R.string.app_name))
            .setContentText(getString(failed ? R.string.notification_failed : R.string.notification_text))
            .setContentIntent(open).setOngoing(true).build();
        startForeground(1, notification);
    }
}
