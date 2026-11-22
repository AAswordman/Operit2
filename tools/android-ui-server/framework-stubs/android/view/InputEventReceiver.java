package android.view;

import android.os.Looper;

/** Compile-only declarations; Android supplies the implementation at runtime. */
public abstract class InputEventReceiver {
    /** Attaches the receiver to one native channel and its owning Looper. */
    public InputEventReceiver(InputChannel channel, Looper looper) { throw new UnsupportedOperationException(); }
    /** Receives one native event for dispatch and acknowledgement. */
    public void onInputEvent(InputEvent event) { throw new UnsupportedOperationException(); }
    /** Schedules consumption of native pointer batches. */
    public void onBatchedInputEventPending(int source) { throw new UnsupportedOperationException(); }
    /** Consumes native samples through the requested frame timestamp. */
    public final boolean consumeBatchedInputEvents(long frameTimeNanos) { throw new UnsupportedOperationException(); }
    /** Acknowledges and recycles exactly one received event. */
    public final void finishInputEvent(InputEvent event, boolean handled) { throw new UnsupportedOperationException(); }
    /** Unregisters the native receiver and closes its channel. */
    public void dispose() { throw new UnsupportedOperationException(); }
}
