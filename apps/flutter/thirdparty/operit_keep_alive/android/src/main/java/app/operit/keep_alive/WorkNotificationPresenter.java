package app.operit.keep_alive;

import android.app.Notification;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.app.Service;
import android.content.Intent;
import android.net.Uri;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;
import java.util.HashSet;
import java.util.Set;

/** Renders process-owned work on the existing foreground notification and Android Live Updates. */
final class WorkNotificationPresenter {
    static final String STOP_ACTION = "operit.keep_alive.STOP_WORK";
    static final String DISMISS_ACTION = "operit.keep_alive.DISMISS_WORK";
    static final String WORK_ID = "operit.keep_alive.workId";
    static final String ACTIVATION = "operit.keep_alive.activation";
    private static final long RESULT_LIFETIME_MS = 60_000L;
    private final Service service;
    private final NotificationManager manager;
    private final String channelId;
    private final int notificationId;
    private final int icon;
    private final Notification idle;
    private final Handler handler = new Handler(Looper.getMainLooper());
    private final Set<String> dismissed = new HashSet<>();
    private long revision = -1;
    private JSONObject latest;
    private Runnable expiry;

    /** Retains only the foreground service and its already-established idle notification. */
    WorkNotificationPresenter(Service service, String channelId, int notificationId, int icon, Notification idle) {
        this.service = service;
        this.manager = service.getSystemService(NotificationManager.class);
        if (manager == null) throw new IllegalStateException("Notification manager is unavailable");
        this.channelId = channelId;
        this.notificationId = notificationId;
        this.icon = icon;
        this.idle = idle;
    }

    /** Queries actual system permission without promising promotion by an OEM interface. */
    static boolean canPromote(Service service) {
        if (Build.VERSION.SDK_INT < 36) return false;
        NotificationManager manager = service.getSystemService(NotificationManager.class);
        if (manager == null) throw new IllegalStateException("Notification manager is unavailable");
        return manager.canPostPromotedNotifications();
    }

    /** Accepts complete revisions and prevents stale callbacks from replacing newer work. */
    void update(String encoded) throws JSONException {
        JSONObject snapshot = new JSONObject(encoded);
        if (snapshot.getInt("schemaVersion") != 1) throw new IllegalArgumentException("Unsupported work schema");
        long next = snapshot.getLong("revision");
        if (next <= revision) return;
        snapshot.getJSONArray("works");
        revision = next;
        latest = snapshot;
        render();
    }

    /** Reposts the retained foreground state when notification authorization changes. */
    void refresh() throws JSONException {
        if (latest == null) {
            manager.notify(notificationId, idle);
            return;
        }
        render();
    }

    /** Creates an immutable action intent whose identity includes the exact execution. */
    private PendingIntent serviceAction(String action, String workId) {
        Intent intent = new Intent(service, service.getClass()).setAction(action)
            .setData(new Uri.Builder().scheme("operit-work").authority(action).appendPath(workId).build())
            .putExtra(WORK_ID, workId);
        return PendingIntent.getService(service, 0, intent, PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
    }

    /** Opens the application's existing activity and lets the plugin route the work activation. */
    private PendingIntent openAction(JSONObject work) throws JSONException {
        Intent intent = service.getPackageManager().getLaunchIntentForPackage(service.getPackageName());
        if (intent == null) throw new IllegalStateException("Application has no launch activity");
        String id = work.getString("workId");
        intent.addFlags(Intent.FLAG_ACTIVITY_SINGLE_TOP | Intent.FLAG_ACTIVITY_CLEAR_TOP)
            .setData(new Uri.Builder().scheme("operit-work").authority("open").appendPath(id).build())
            .putExtra(WORK_ID, id).putExtra(ACTIVATION, work.getJSONObject("activation").toString());
        return PendingIntent.getActivity(service, 0, intent, PendingIntent.FLAG_UPDATE_CURRENT | PendingIntent.FLAG_IMMUTABLE);
    }

    /** Maps only documented phase identifiers to localized user-facing wording. */
    private String phase(JSONObject work) throws JSONException {
        switch (work.getString("status")) {
            case "waiting": return service.getString(R.string.operit_work_waiting);
            case "completed": return service.getString(R.string.operit_work_completed);
            case "failed": return service.getString(R.string.operit_work_failed);
            case "cancelled": return service.getString(R.string.operit_work_cancelled);
            case "running": break;
            default: throw new IllegalArgumentException("Unknown work status");
        }
        int resource;
        switch (work.getString("phase")) {
            case "processing": resource = R.string.operit_work_processing; break;
            case "connecting": resource = R.string.operit_work_connecting; break;
            case "receiving": resource = R.string.operit_work_receiving; break;
            case "executingTool": resource = R.string.operit_work_executing_tool; break;
            case "processingToolResult": resource = R.string.operit_work_processing_tool_result; break;
            case "summarizing": resource = R.string.operit_work_summarizing; break;
            case "executingPlan": resource = R.string.operit_work_executing_plan; break;
            default: throw new IllegalArgumentException("Unknown work phase");
        }
        return service.getString(resource);
    }

    /** Renders one stable focus, aggregate counts, real elapsed time, and supported actions. */
    private void render() throws JSONException {
        if (expiry != null) handler.removeCallbacks(expiry);
        JSONArray works = latest.getJSONArray("works");
        int active = 0;
        int waiting = 0;
        JSONObject focus = null;
        JSONObject completed = null;
        Set<String> present = new HashSet<>();
        for (int i = 0; i < works.length(); i++) {
            JSONObject work = works.getJSONObject(i);
            String id = work.getString("workId");
            present.add(id);
            if (work.isNull("endedAtMillis")) {
                active++;
                if (work.getString("status").equals("waiting")) waiting++;
            }
            if (work.isNull("endedAtMillis") && focus == null && !dismissed.contains(id)) focus = work;
            if (!work.isNull("endedAtMillis") && completed == null && !dismissed.contains(id)) completed = work;
        }
        dismissed.retainAll(present);
        if (focus == null && active == 0) focus = completed;
        if (focus == null) {
            if (active == 0 && works.length() == 0) manager.notify(notificationId, idle);
            return;
        }
        boolean ongoing = focus.isNull("endedAtMillis");
        long started = focus.getLong("startedAtMillis");
        long ended = ongoing ? 0 : focus.getLong("endedAtMillis");
        if (!ongoing && System.currentTimeMillis() - ended >= RESULT_LIFETIME_MS) {
            if (active == 0) manager.notify(notificationId, idle);
            return;
        }
        if (!ongoing && ended < started) throw new IllegalArgumentException("Work end precedes its start");
        String status = phase(focus);
        String detail = focus.getString("detail");
        String text = detail.isEmpty() ? status : status + " · " + detail;
        JSONObject progress = focus.isNull("progress") ? null : focus.getJSONObject("progress");
        if (progress != null) text += " · " + progress.getString("scope") + " " + Math.round(progress.getDouble("value")) + "%";
        PendingIntent open = openAction(focus);
        Notification.Builder builder = Build.VERSION.SDK_INT >= 26
            ? new Notification.Builder(service, channelId) : new Notification.Builder(service);
        builder.setSmallIcon(icon).setContentTitle(focus.getString("title")).setContentText(text)
            .setContentIntent(open).setOnlyAlertOnce(true).setOngoing(ongoing)
            .setCategory(Notification.CATEGORY_PROGRESS).setVisibility(Notification.VISIBILITY_PRIVATE)
            .setDeleteIntent(serviceAction(DISMISS_ACTION, focus.getString("workId")));
        if (active > 1) builder.setSubText(service.getString(R.string.operit_work_summary, active, waiting));
        if (ongoing) builder.setWhen(started).setShowWhen(true).setUsesChronometer(true);
        else {
            long duration = (ended - started) / 1000;
            builder.setContentText(text + " · " + String.format(java.util.Locale.ROOT, "%02d:%02d", duration / 60, duration % 60))
                .setWhen(ended).setShowWhen(true).setUsesChronometer(false);
        }
        builder.addAction(new Notification.Action.Builder(null,
            service.getString(focus.getString("status").equals("waiting") ? R.string.operit_work_review : R.string.operit_work_open), open).build());
        if (ongoing && focus.getBoolean("canStop")) builder.addAction(new Notification.Action.Builder(null,
            service.getString(R.string.operit_work_stop), serviceAction(STOP_ACTION, focus.getString("workId"))).build());
        if (Build.VERSION.SDK_INT >= 36 && ongoing) {
            builder.setShortCriticalText(service.getString(focus.getString("status").equals("waiting")
                ? R.string.operit_work_chip_waiting : R.string.operit_work_chip_running));
            Notification.ProgressStyle style = new Notification.ProgressStyle()
                .addProgressSegment(new Notification.ProgressStyle.Segment(100))
                .setStyledByProgress(false).setProgressTrackerIcon(null);
            style.setProgressIndeterminate(progress == null);
            if (progress != null) style.setProgress((int) Math.round(progress.getDouble("value")));
            builder.setStyle(style);
            if (manager.canPostPromotedNotifications()) {
                Bundle extras = new Bundle();
                extras.putBoolean("android.requestPromotedOngoing", true);
                builder.addExtras(extras);
            }
        } else if (ongoing && progress != null) {
            builder.setProgress(100, (int) Math.round(progress.getDouble("value")), false);
        }
        manager.notify(notificationId, builder.build());
        if (!ongoing) {
            expiry = () -> manager.notify(notificationId, idle);
            handler.postDelayed(expiry, RESULT_LIFETIME_MS - (System.currentTimeMillis() - ended));
        }
    }

    /** Honors a user's dismissal for this execution instead of reposting its Live Update. */
    void dismiss(String workId) { dismissed.add(workId); }

    /** Cancels pending UI expiry when the actual service owner is destroyed. */
    void close() { if (expiry != null) handler.removeCallbacks(expiry); }
}
