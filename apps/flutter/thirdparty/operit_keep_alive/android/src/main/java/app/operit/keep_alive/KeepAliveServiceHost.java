package app.operit.keep_alive;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.Context;
import android.content.Intent;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.os.PowerManager;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Set;
import java.util.LinkedHashMap;
import java.util.Map;

import io.flutter.plugin.common.MethodChannel;

/** Owns Android protection in the Core service, independently of any Flutter activity. */
public final class KeepAliveServiceHost {
    private static final Map<String, String> LEASES = new LinkedHashMap<>();
    private static final Map<String, MethodChannel.Result> PENDING = new LinkedHashMap<>();
    private static final Handler MAIN = new Handler(Looper.getMainLooper());
    private static final Set<MethodChannel> OBSERVERS = new HashSet<>();
    private static Service service;
    private static PowerManager.WakeLock cpuLock;
    private static WorkNotificationPresenter workPresenter;
    private static String workSnapshot;
    public static final String STOP_WORK_ACTION = WorkNotificationPresenter.STOP_ACTION;
    public static final String DISMISS_WORK_ACTION = WorkNotificationPresenter.DISMISS_ACTION;
    public static final String WORK_ID_EXTRA = WorkNotificationPresenter.WORK_ID;

    /** Retains the latest Core revision independently of service and Flutter attachment. */
    public static void publishWorkSnapshot(String encoded) {
        requireMainThread();
        try {
            org.json.JSONObject snapshot = new org.json.JSONObject(encoded);
            if (snapshot.getInt("schemaVersion") != 1) throw new IllegalArgumentException("Unsupported work schema");
            if (workSnapshot != null && snapshot.getLong("revision") <= new org.json.JSONObject(workSnapshot).getLong("revision")) return;
            workSnapshot = encoded;
            if (workPresenter != null) workPresenter.update(encoded);
        } catch (org.json.JSONException error) {
            throw new IllegalArgumentException("Invalid Core work snapshot", error);
        }
    }

    /** Reports a work delivery or action failure to every attached application receiver. */
    public static void reportWorkError(String message) {
        requireMainThread();
        Map<String, Object> error = new LinkedHashMap<>();
        error.put("message", message);
        for (MethodChannel observer : OBSERVERS) observer.invokeMethod("workPresentationError", error);
    }

    /** Stops promoting an execution explicitly dismissed by the user. */
    public static void dismissWork(String workId) {
        requireMainThread();
        if (workPresenter == null) throw new IllegalStateException("Work notification owner is unavailable");
        workPresenter.dismiss(workId);
    }

    /** Reposts current work after authorization changes and publishes actual notification visibility. */
    static void refreshNotifications() {
        requireMainThread();
        if (workPresenter != null) {
            try { workPresenter.refresh(); }
            catch (org.json.JSONException error) {
                throw new IllegalArgumentException("Invalid retained Core work snapshot", error);
            }
        }
        publish();
    }

    /** Reports native ownership and the actual Android promotion permission. */
    static Map<String, Object> workCapabilities() {
        Map<String, Object> value = new LinkedHashMap<>();
        value.put("updatesOwner", "host");
        value.put("surface", "notification");
        value.put("liveUpdates", service != null && WorkNotificationPresenter.canPromote(service));
        return value;
    }

    /** Prevents construction of the process-owned service controller. */
    private KeepAliveServiceHost() {}

    /** Establishes the foreground notification and CPU lease before acknowledging work. */
    public static void attach(Service owner, String channelId, int notificationId,
                              String title, String text, int icon) {
        requireMainThread();
        if (service != null && service != owner) {
            throw new IllegalStateException("Another Core service already owns keep-alive");
        }
        try {
            NotificationManager manager = owner.getSystemService(NotificationManager.class);
            if (manager == null) throw new IllegalStateException("Notification manager is unavailable");
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                manager.createNotificationChannel(new NotificationChannel(
                    channelId, title, NotificationManager.IMPORTANCE_LOW));
            }
            Intent launch = owner.getPackageManager().getLaunchIntentForPackage(owner.getPackageName());
            if (launch == null) throw new IllegalStateException("Application has no launch activity");
            launch.addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP | Intent.FLAG_ACTIVITY_CLEAR_TOP);
            PendingIntent open = PendingIntent.getActivity(owner, 0, launch,
                PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
            Notification.Builder builder;
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                builder = new Notification.Builder(owner, channelId);
            } else {
                builder = new Notification.Builder(owner);
            }
            Notification notification = builder.setContentTitle(title).setContentText(text)
                .setSmallIcon(icon).setContentIntent(open).setOngoing(true)
                .setCategory(Notification.CATEGORY_SERVICE).build();
            owner.startForeground(notificationId, notification);
            PowerManager power = owner.getSystemService(PowerManager.class);
            if (power == null) throw new IllegalStateException("Power manager is unavailable");
            if (cpuLock != null) throw new IllegalStateException("Core CPU lease is already attached");
            PowerManager.WakeLock acquired = power.newWakeLock(
                PowerManager.PARTIAL_WAKE_LOCK, owner.getPackageName() + ":core-runtime");
            acquired.setReferenceCounted(false);
            acquired.acquire();
            cpuLock = acquired;
            service = owner;
            workPresenter = new WorkNotificationPresenter(owner, channelId, notificationId, icon, notification);
            if (workSnapshot != null) {
                try { workPresenter.update(workSnapshot); }
                catch (org.json.JSONException error) { throw new IllegalArgumentException("Invalid retained work snapshot", error); }
            }
            for (MethodChannel.Result result : new ArrayList<>(PENDING.values())) {
                result.success(report());
            }
            PENDING.clear();
            publish();
        } catch (RuntimeException error) {
            for (String id : new ArrayList<>(PENDING.keySet())) {
                failPending(id, error.getMessage());
            }
            throw error;
        }
    }

    /** Releases the CPU resource when the actual Core service is destroyed. */
    public static void detach(Service owner) {
        requireMainThread();
        if (service != owner) return;
        if (cpuLock != null) {
            cpuLock.release();
            cpuLock = null;
        }
        if (workPresenter != null) workPresenter.close();
        workPresenter = null;
        service = null;
        publish();
    }

    /** Observes service ownership for one initialized engine only. */
    static void observe(MethodChannel channel) {
        requireMainThread();
        OBSERVERS.add(channel);
    }

    /** Releases client accounting while the actual Core service retains its native lease. */
    static void forget(MethodChannel channel, Set<String> ids) {
        requireMainThread();
        OBSERVERS.remove(channel);
        for (String id : ids) {
            failPending(id, "The owning Flutter engine detached during service startup");
            LEASES.remove(id);
        }
        publish();
    }

    /** Publishes service acquisition and destruction through the common protection event. */
    private static void publish() {
        for (MethodChannel channel : new ArrayList<>(OBSERVERS)) {
            channel.invokeMethod("protectionChanged", report());
        }
    }

    /** Starts the declared Core service and acknowledges only an attached protection host. */
    static void acquire(Context context, Class<? extends Service> serviceType,
                        String id, String reason, MethodChannel.Result result) {
        requireMainThread();
        if (LEASES.containsKey(id)) throw new IllegalArgumentException("Duplicate lease: " + id);
        LEASES.put(id, reason);
        if (service != null) {
            result.success(report());
            return;
        }
        PENDING.put(id, result);
        try {
            Intent intent = new Intent(context, serviceType);
            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                context.startForegroundService(intent);
            } else {
                context.startService(intent);
            }
            MAIN.postDelayed(() -> failPending(id, "Core foreground protection did not attach within 10 seconds"), 10000);
        } catch (RuntimeException error) {
            failPending(id, error.getMessage());
        }
    }

    /** Removes a client lease while the service retains protection for its running Core. */
    static Map<String, Object> release(String id) {
        requireMainThread();
        if (!LEASES.containsKey(id)) throw new IllegalArgumentException("Unknown lease: " + id);
        LEASES.remove(id);
        return report();
    }

    /** Reports the native service lease separately from Flutter client leases. */
    static Map<String, Object> report() {
        Map<String, Object> value = new HashMap<>();
        boolean attached = service != null && cpuLock != null && cpuLock.isHeld();
        value.put("protection", attached ? "continuous" : "unavailable");
        value.put("closeToTray", false);
        value.put("activeLeases", LEASES.size() + (attached ? 1 : 0));
        boolean notificationVisible = false;
        if (attached) {
            NotificationManager manager = service.getSystemService(NotificationManager.class);
            notificationVisible = manager != null && manager.areNotificationsEnabled();
        }
        value.put("detail", attached
            ? (notificationVisible
                ? "Core foreground service and partial CPU lock are active; Android retains final process control."
                : "Core foreground service and CPU lock are active, but Android notification permission is disabled; enable notifications to show the status-bar entry.")
            : "Core foreground service has not attached; no execution protection is active.");
        return value;
    }

    /** Completes one rejected handoff without keeping an unacknowledged lease. */
    private static void failPending(String id, String reason) {
        MethodChannel.Result result = PENDING.remove(id);
        if (result == null) return;
        LEASES.remove(id);
        result.error("KEEP_ALIVE_START_FAILED", reason, null);
    }

    /** Serializes service and engine callbacks on Android's application thread. */
    private static void requireMainThread() {
        if (Looper.myLooper() != Looper.getMainLooper()) {
            throw new IllegalStateException("Keep-alive service operations require the main thread");
        }
    }
}
