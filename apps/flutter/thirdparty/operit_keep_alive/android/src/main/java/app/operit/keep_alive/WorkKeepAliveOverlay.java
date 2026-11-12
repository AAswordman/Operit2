package app.operit.keep_alive;

import android.app.Service;
import android.graphics.Color;
import android.graphics.PixelFormat;
import android.os.Build;
import android.provider.Settings;
import android.util.Log;
import android.view.Gravity;
import android.view.View;
import android.view.WindowManager;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

/** Holds a noninteractive one-pixel service window only while Core has unfinished work. */
final class WorkKeepAliveOverlay {
    private static final String TAG = "WorkKeepAliveOverlay";
    private final Service service;
    private final WindowManager manager;
    private boolean active;
    private boolean permissionMissingLogged;
    private View view;

    /** Uses the service context so destroying a Flutter activity cannot remove the window. */
    WorkKeepAliveOverlay(Service service) {
        this.service = service;
        manager = service.getSystemService(WindowManager.class);
        if (manager == null) throw new IllegalStateException("Window manager is unavailable");
    }

    /** Retains protection until every parallel work item, including confirmations, has ended. */
    void update(JSONObject snapshot) throws JSONException {
        JSONArray works = snapshot.getJSONArray("works");
        boolean unfinished = false;
        for (int index = 0; index < works.length(); index++) {
            if (works.getJSONObject(index).isNull("endedAtMillis")) unfinished = true;
        }
        active = unfinished;
        refresh();
    }

    /** Rechecks system authorization when work changes or the app returns from permission settings. */
    void refresh() {
        if (!active) {
            remove();
            return;
        }
        if (!Settings.canDrawOverlays(service)) {
            remove();
            if (!permissionMissingLogged) {
                Log.w(TAG, "Work keep-alive overlay requires the overlay permission from onboarding");
                permissionMissingLogged = true;
            }
            return;
        }
        permissionMissingLogged = false;
        if (view != null) return;
        View next = new View(service);
        next.setBackgroundColor(Color.BLACK);
        next.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
        int type = Build.VERSION.SDK_INT >= Build.VERSION_CODES.O
            ? WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY : WindowManager.LayoutParams.TYPE_PHONE;
        WindowManager.LayoutParams params = new WindowManager.LayoutParams(1, 1, type,
            WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE
                | WindowManager.LayoutParams.FLAG_NOT_TOUCHABLE
                | WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN
                | WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS,
            PixelFormat.TRANSLUCENT);
        params.gravity = Gravity.TOP | Gravity.START;
        params.x = 0;
        params.y = 0;
        // Keep a nonzero window alpha so Android does not classify it as fully invisible.
        params.alpha = 0.01f;
        try {
            manager.addView(next, params);
            view = next;
        } catch (RuntimeException error) {
            reportError("Could not attach work keep-alive overlay", error);
        }
    }

    /** Releases the service-owned window immediately when its owner is destroyed. */
    void close() {
        active = false;
        remove();
    }

    /** Removes the actual window without waiting for a notification result to expire. */
    private void remove() {
        if (view == null) return;
        View current = view;
        view = null;
        try {
            manager.removeViewImmediate(current);
        } catch (RuntimeException error) {
            reportError("Could not remove work keep-alive overlay", error);
        }
    }

    /** Exposes platform failures through both Android logs and the existing work error channel. */
    private void reportError(String message, RuntimeException error) {
        Log.e(TAG, message, error);
        KeepAliveServiceHost.reportWorkError(message + ": " + error);
    }
}
