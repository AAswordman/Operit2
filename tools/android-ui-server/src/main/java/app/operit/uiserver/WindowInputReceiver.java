package app.operit.uiserver;

import android.os.Looper;
import android.view.Choreographer;
import android.view.InputChannel;
import android.view.InputEvent;
import android.view.InputEventReceiver;
import android.view.InputDevice;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.View;

/** Dispatches shell-window input directly without application-only ViewRoot input stages. */
final class WindowInputReceiver extends InputEventReceiver implements Choreographer.FrameCallback {
    private final View root;
    private final Choreographer frames = Choreographer.getInstance();
    private boolean scheduled;
    private boolean dispatching;
    private boolean disposed;

    /** Registers the session's native window channel on the same thread as its Views. */
    WindowInputReceiver(View root, InputChannel channel) {
        super(channel, Looper.getMainLooper());
        this.root = root;
    }

    /** Preserves native pointer streams and always acknowledges their completion. */
    @Override public void onInputEvent(InputEvent event) {
        boolean handled = false;
        dispatching = true;
        try {
            if (!disposed) {
                if (event instanceof MotionEvent motion) {
                    handled = motion.isFromSource(InputDevice.SOURCE_CLASS_POINTER)
                            && motion.getActionMasked() <= MotionEvent.ACTION_POINTER_UP
                            ? root.dispatchTouchEvent(motion) : root.dispatchGenericMotionEvent(motion);
                } else if (event instanceof KeyEvent key) {
                    handled = root.dispatchKeyEvent(key);
                }
            }
        } finally {
            finishInputEvent(event, handled);
            dispatching = false;
            if (disposed) super.dispose();
        }
    }

    /** Consumes accumulated movement at the display refresh rate. */
    @Override public void onBatchedInputEventPending(int source) {
        if (disposed || scheduled) return;
        scheduled = true;
        frames.postFrameCallback(this);
    }

    /** Retains event history while matching input work to the next display frame. */
    @Override public void doFrame(long frameTimeNanos) {
        scheduled = false;
        if (!disposed && consumeBatchedInputEvents(frameTimeNanos)) onBatchedInputEventPending(0);
    }

    /** Finishes an in-flight close-button event before disposing its native channel. */
    @Override public void dispose() {
        disposed = true;
        frames.removeFrameCallback(this);
        scheduled = false;
        if (!dispatching) super.dispose();
    }
}
