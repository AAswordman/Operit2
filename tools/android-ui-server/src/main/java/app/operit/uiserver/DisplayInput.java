package app.operit.uiserver;

import android.graphics.Matrix;
import android.os.Build;
import android.os.SystemClock;
import android.view.InputDevice;
import android.view.InputEvent;
import android.view.KeyEvent;
import android.view.KeyCharacterMap;
import android.view.MotionEvent;
import java.lang.reflect.Method;

/** Injects complete native event streams into one exact virtual display. */
final class DisplayInput {
    private final Object manager;
    private final Method inject;
    private final Method setDisplay;

    /** Resolves the input service API for the running Android release. */
    DisplayInput() throws Exception {
        Class<?> type = Class.forName(Build.VERSION.SDK_INT >= 34
                ? "android.hardware.input.InputManagerGlobal" : "android.hardware.input.InputManager");
        manager = type.getMethod("getInstance").invoke(null);
        inject = type.getMethod("injectInputEvent", InputEvent.class, int.class);
        setDisplay = InputEvent.class.getMethod("setDisplayId", int.class);
    }

    /** Retains pointer IDs, history, pressure, scroll axes and cancellation when scaling input. */
    void motion(MotionEvent original, int displayId, float scaleX, float scaleY) throws Exception {
        MotionEvent event = MotionEvent.obtain(original);
        try {
            Matrix matrix = new Matrix();
            matrix.setScale(scaleX, scaleY);
            event.transform(matrix);
            send(event, displayId);
        } finally { event.recycle(); }
    }

    /** Sends both key transitions to the selected display without global key actions. */
    void key(int displayId, int keyCode) throws Exception {
        if (keyCode == KeyEvent.KEYCODE_UNKNOWN) throw new IllegalArgumentException("Unknown Android key");
        long timestamp = SystemClock.uptimeMillis();
        for (int action : new int[]{KeyEvent.ACTION_DOWN, KeyEvent.ACTION_UP}) {
            KeyEvent event = new KeyEvent(timestamp, timestamp, action, keyCode, 0, 0,
                    KeyCharacterMap.VIRTUAL_KEYBOARD, 0, 0, InputDevice.SOURCE_KEYBOARD);
            send(event, displayId);
        }
    }

    /** Rejects failed injection instead of redirecting events onto another display. */
    private void send(InputEvent event, int displayId) throws Exception {
        setDisplay.invoke(event, displayId);
        if (!Boolean.TRUE.equals(inject.invoke(manager, event, 0))) {
            throw new IllegalStateException("Android rejected input for display " + displayId);
        }
    }
}
