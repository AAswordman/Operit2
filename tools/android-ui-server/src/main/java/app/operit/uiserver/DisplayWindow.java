package app.operit.uiserver;

import android.animation.ValueAnimator;
import android.app.ActivityOptions;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.pm.ApplicationInfo;
import android.content.pm.PackageManager;
import android.graphics.Insets;
import android.graphics.PixelFormat;
import android.graphics.Rect;
import android.hardware.display.DisplayManager;
import android.hardware.display.VirtualDisplay;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.util.DisplayMetrics;
import android.util.Log;
import android.view.Gravity;
import android.view.KeyEvent;
import android.view.MotionEvent;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.view.VelocityTracker;
import android.view.View;
import android.view.ViewConfiguration;
import android.view.WindowManager;
import android.view.animation.PathInterpolator;
import android.widget.OverScroller;
import java.util.BitSet;
import java.util.concurrent.CompletableFuture;
import java.util.function.Consumer;
import java.util.function.IntConsumer;
import java.util.UUID;
import org.json.JSONObject;

/** Owns one native floating window and its direct virtual-display Surface. */
final class DisplayWindow implements SurfaceHolder.Callback {
    private static final int TYPE_APPLICATION_OVERLAY = 2038;
    private static final int SUPPORTS_TOUCH = 1 << 6;
    private static final int DESTROY_CONTENT_ON_REMOVAL = 1 << 8;
    private static final int TRUSTED = 1 << 10;
    private static final int OWN_DISPLAY_GROUP = 1 << 11;
    private static final int ALWAYS_UNLOCKED = 1 << 12;
    private static final int TOUCH_FEEDBACK_DISABLED = 1 << 13;
    private static final int OWN_FOCUS = 1 << 14;
    private static final int ASPECT_WIDTH = 2;
    private static final int ASPECT_HEIGHT = 3;
    private static final float ASPECT = ASPECT_HEIGHT / (float) ASPECT_WIDTH;
    final String packageName;
    final String windowId = UUID.randomUUID().toString();
    private final ShellContext context;
    private final DisplayInput input;
    private final String component;
    private final CompletableFuture<JSONObject> ready;
    private final IntConsumer removed;
    private final WindowManager manager;
    private final WindowManager.LayoutParams layout;
    private final WindowChrome chrome;
    private final SurfaceView surface;
    private final Rect screen;
    private final Rect area;
    private final Rect frame = new Rect();
    private final Rect restoredFrame = new Rect();
    private final Rect gestureFrame = new Rect();
    private final BitSet dockSlots;
    private final Consumer<DisplayWindow> foreground;
    private final OverScroller scroller;
    private final Runnable flingFrame = this::stepFling;
    private final Runnable layoutFrame = this::applyFrame;
    private final ViewConfiguration gestures;
    private final int displayWidth;
    private final int displayHeight;
    private final int density;
    private final Handler main = new Handler(Looper.getMainLooper());
    private VirtualDisplay display;
    private WindowInputReceiver receiver;
    private Runnable created;
    private boolean attached;
    private boolean closed;
    private boolean minimized;
    private boolean launched;
    private int dockSlot = -1;
    private int pointer = -1;
    private float dockFraction;
    private float downX;
    private float downY;
    private boolean moved;
    private boolean layoutScheduled;
    private VelocityTracker velocity;
    private ValueAnimator transition;

    /** Creates an interactive native chrome without allocating any image or video transport. */
    DisplayWindow(ShellContext context, DisplayInput input, String component,
            CompletableFuture<JSONObject> ready, IntConsumer removed, BitSet dockSlots,
            Consumer<DisplayWindow> foreground) throws PackageManager.NameNotFoundException, ReflectiveOperationException {
        this.context = context;
        this.input = input;
        this.component = component;
        this.ready = ready;
        this.removed = removed;
        this.dockSlots = dockSlots;
        this.foreground = foreground;
        packageName = ComponentName.unflattenFromString(component).getPackageName();
        Context viewContext = context.windowContext();
        manager = (WindowManager) viewContext.getSystemService(Context.WINDOW_SERVICE);
        DisplayMetrics metrics = new DisplayMetrics();
        context.getDisplay().getRealMetrics(metrics);
        screen = new Rect(0, 0, metrics.widthPixels, metrics.heightPixels);
        int displayUnit = Math.min(screen.width() / ASPECT_WIDTH, screen.height() / ASPECT_HEIGHT);
        displayWidth = displayUnit * ASPECT_WIDTH;
        displayHeight = displayUnit * ASPECT_HEIGHT;
        density = metrics.densityDpi;
        PackageManager packages = context.getPackageManager();
        ApplicationInfo application = packages.getApplicationInfo(packageName, 0);
        chrome = new WindowChrome(viewContext, application.loadLabel(packages), density, displayWidth,
                this::back, this::minimize, this::fullscreen, this::close, this::raise);
        chrome.grip.setOnTouchListener((view, event) -> gesture(event, 0));
        chrome.resizeLeft.setOnTouchListener((view, event) -> gesture(event, -1));
        chrome.resizeRight.setOnTouchListener((view, event) -> gesture(event, 1));
        scroller = new OverScroller(viewContext);
        gestures = ViewConfiguration.get(viewContext);
        int margin = chrome.dp(8);
        area = new Rect(screen);
        area.inset(margin, margin);

        surface = chrome.surface;
        surface.getHolder().setFixedSize(displayWidth, displayHeight);
        surface.getHolder().addCallback(this);
        surface.setOnTouchListener(this::touch);
        surface.setOnGenericMotionListener(this::touch);
        layout = new WindowManager.LayoutParams(0, 0,
                TYPE_APPLICATION_OVERLAY, WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE
                        | WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN
                        | WindowManager.LayoutParams.FLAG_LAYOUT_NO_LIMITS
                        | WindowManager.LayoutParams.FLAG_HARDWARE_ACCELERATED,
                PixelFormat.TRANSLUCENT);
        layout.setFitInsetsTypes(0);
        layout.layoutInDisplayCutoutMode = WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_ALWAYS;
        layout.gravity = Gravity.TOP | Gravity.LEFT;
        layout.setTitle("Operit: " + packageName);
        layout.packageName = viewContext.getPackageName();
        context.configureFloatingLayout(layout);
        positionInitially();
    }

    /** Centers the interactive card below the system bars on first open. */
    private void positionInitially() {
        int initialWidth = Math.min(Math.round(area.width() * 0.68f), maxWidth(area.width()));
        setFrame(area.centerX() - initialWidth / 2, area.top + chrome.dp(22), initialWidth, 0);
    }

    /** Preserves the virtual screen's aspect ratio during native window resizing. */
    private int contentHeight(int windowWidth) { return Math.round(windowWidth * ASPECT); }

    /** Limits a full-size card to the primary display's working area. */
    private int maxWidth(int horizontalSpace) {
        return Math.min(horizontalSpace, (int) (area.height() / ASPECT));
    }

    /** Applies one bounded card frame while keeping shadow and content geometry separate. */
    private void setFrame(int left, int top, int cardWidth, float dock) {
        int cardHeight = contentHeight(cardWidth);
        left = Math.max(area.left, Math.min(area.right - cardWidth, left));
        top = Math.max(area.top, Math.min(area.bottom - cardHeight, top));
        if (dockFraction != dock) chrome.update(dock);
        dockFraction = dock;
        if (frame.left == left && frame.top == top && frame.width() == cardWidth && frame.height() == cardHeight) return;
        frame.set(left, top, left + cardWidth, top + cardHeight);
        int shadow = chrome.shadow();
        layout.x = left - shadow;
        layout.y = top - shadow;
        layout.width = cardWidth + 2 * shadow;
        layout.height = cardHeight + 2 * shadow;
        if (attached && !layoutScheduled) {
            layoutScheduled = true;
            chrome.root.postOnAnimation(layoutFrame);
        }
    }

    /** Commits the latest geometry once per display frame instead of relayouts per input sample. */
    private void applyFrame() {
        layoutScheduled = false;
        if (attached && !closed) manager.updateViewLayout(chrome.root, layout);
    }

    /** Delivers Back to the hosted application. */
    private void back() {
        try { input.key(displayId(), KeyEvent.KEYCODE_BACK); }
        catch (Exception error) { report(error); }
    }

    /** Moves the live task onto the physical display before destroying its former virtual display. */
    private void fullscreen() {
        if (closed || transition != null) return;
        try {
            context.moveToPrimaryDisplay(displayId());
            close();
        } catch (Exception error) { report(error); }
    }

    /** Attaches the floating window and bounds asynchronous Surface initialization. */
    void open(Runnable created) {
        this.created = created;
        try {
            manager.addView(chrome.root, layout);
            attached = true;
            receiver = new WindowInputReceiver(chrome.root, context.takeInputChannel());
            Insets insets = context.floatingInsets();
            area.inset(insets);
            positionInitially();
            main.postDelayed(() -> {
                if (!ready.isDone() || ready.isCancelled()) {
                    ready.completeExceptionally(new IllegalStateException("Android did not create the window Surface"));
                    close();
                }
            }, 9000);
        } catch (RuntimeException error) {
            ready.completeExceptionally(error);
            close();
            throw error;
        }
    }

    /** Connects virtual-display output directly to the native window Surface. */
    @Override public void surfaceCreated(SurfaceHolder holder) {
        if (closed) return;
        if (ready.isCancelled()) { close(); return; }
        try {
            if (display == null) {
                int flags = DisplayManager.VIRTUAL_DISPLAY_FLAG_PUBLIC
                        | DisplayManager.VIRTUAL_DISPLAY_FLAG_OWN_CONTENT_ONLY
                        | SUPPORTS_TOUCH | DESTROY_CONTENT_ON_REMOVAL | TRUSTED;
                if (Build.VERSION.SDK_INT >= 33) flags |= OWN_DISPLAY_GROUP | ALWAYS_UNLOCKED | TOUCH_FEEDBACK_DISABLED;
                if (Build.VERSION.SDK_INT >= 34) flags |= OWN_FOCUS;
                DisplayManager displays = (DisplayManager) context.getSystemService(Context.DISPLAY_SERVICE);
                display = displays.createVirtualDisplay("Operit: " + packageName, displayWidth, displayHeight, density,
                        holder.getSurface(), flags);
                if (display == null) throw new IllegalStateException("Android rejected virtual display creation");
                Object windowService = Class.forName("android.view.WindowManagerGlobal")
                        .getMethod("getWindowManagerService").invoke(null);
                if (Build.VERSION.SDK_INT >= 31) {
                    Class.forName("android.view.IWindowManager").getMethod("setDisplayImePolicy", int.class, int.class)
                            .invoke(windowService, displayId(), 0);
                } else {
                    Class.forName("android.view.IWindowManager").getMethod("setShouldShowIme", int.class, boolean.class)
                            .invoke(windowService, displayId(), true);
                }
                created.run();
                launch(component);
                ready.complete(describe());
            } else {
                display.setSurface(holder.getSurface());
            }
        } catch (Exception error) {
            ready.completeExceptionally(error);
            report(error);
            close();
        }
    }

    /** Keeps buffer dimensions independent of the floating window's on-screen dimensions. */
    @Override public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) { }

    /** Disconnects production when the native Surface is destroyed. */
    @Override public void surfaceDestroyed(SurfaceHolder holder) {
        if (display != null) display.setSurface(null);
    }

    /** Starts or brings the requested application into this display's task namespace. */
    void launch(String target) throws Exception {
        requireOpen();
        ActivityOptions options = ActivityOptions.makeBasic().setLaunchDisplayId(displayId());
        ActivityOptions.class.getMethod("setLaunchWindowingMode", int.class).invoke(options, 1);
        Intent intent = new Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
                .setComponent(ComponentName.unflattenFromString(target))
                .addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        if (!launched) intent.addFlags(Intent.FLAG_ACTIVITY_MULTIPLE_TASK);
        context.launch(intent, options.toBundle());
        launched = true;
    }

    /** Restores a minimized window and raises it without recreating its virtual display. */
    void raise() {
        requireOpen();
        chrome.dismissActions();
        foreground.accept(this);
        if (minimized) {
            minimized = false;
            animateTo(restoredFrame, 0);
        }
    }

    /** Moves a covered overlay to the top using the existing delegated window session. */
    void restack() {
        try {
            receiver.dispose();
            manager.removeViewImmediate(chrome.root);
            attached = false;
            manager.addView(chrome.root, layout);
            attached = true;
            receiver = new WindowInputReceiver(chrome.root, context.takeInputChannel());
        } catch (RuntimeException error) {
            close();
            throw new IllegalStateException("Cannot raise the native window", error);
        }
    }

    /** Reserves a top-right dock slot and animates the live content into it. */
    private void minimize() {
        minimize(frame);
    }

    /** Preserves the last usable expanded size when an inward resize crosses the dock threshold. */
    private void minimize(Rect restoreFrame) {
        if (minimized || transition != null) return;
        int previewWidth = Math.min(chrome.dp(80), maxWidth(area.width()));
        int previewHeight = contentHeight(previewWidth);
        int gap = chrome.dp(10);
        int rows = Math.max(1, (area.height() + gap) / (previewHeight + gap));
        int columns = Math.max(1, (area.width() + gap) / (previewWidth + gap));
        int slot = dockSlots.nextClearBit(0);
        if (slot >= rows * columns) {
            chrome.showError("There is no space for another minimized window.");
            return;
        }
        minimized = true;
        restoredFrame.set(restoreFrame);
        dockSlot = slot;
        dockSlots.set(dockSlot);
        int right = area.right - dockSlot / rows * (previewWidth + gap);
        int top = area.top + dockSlot % rows * (previewHeight + gap);
        animateTo(new Rect(right - previewWidth, top, right, top + previewHeight), 1);
    }

    /** Animates position, scale and chrome together without replacing the Surface. */
    private void animateTo(Rect target, float dock) {
        stopFling();
        if (transition != null) transition.cancel();
        Rect origin = new Rect(frame);
        float startDock = dockFraction;
        ValueAnimator animator = ValueAnimator.ofFloat(0, 1);
        animator.setDuration(dock == 1 ? 370 : 300);
        animator.setInterpolator(dock == 1 ? new PathInterpolator(0.34f, 0, 0.2f, 1)
                : new PathInterpolator(0.17f, 0.75f, 0.27f, 1));
        animator.addUpdateListener(value -> {
            float fraction = (float) value.getAnimatedValue();
            int cardWidth = Math.round(origin.width() + (target.width() - origin.width()) * fraction);
            float horizontal = dock == 1 ? fraction * (0.86f + 0.14f * fraction) : fraction;
            int right = Math.round(origin.right + (target.right - origin.right) * horizontal);
            int top = Math.round(origin.top + (target.top - origin.top) * fraction);
            setFrame(right - cardWidth, top, cardWidth, startDock + (dock - startDock) * fraction);
            if (value.getAnimatedFraction() == 1) {
                transition = null;
                if (dock == 0) releaseDockSlot();
            }
        });
        transition = animator;
        animator.start();
    }

    /** Frees a dock slot only when its preview is gone. */
    private void releaseDockSlot() {
        if (dockSlot >= 0) {
            dockSlots.clear(dockSlot);
            dockSlot = -1;
        }
    }

    /** Handles a caption drag or a resize from either bottom corner in screen coordinates. */
    private boolean gesture(MotionEvent event, int side) {
        if (closed || minimized || transition != null) return true;
        int action = event.getActionMasked();
        if (action == MotionEvent.ACTION_DOWN) {
            stopFling();
            finishGesture();
            pointer = event.getPointerId(0);
            downX = event.getRawX();
            downY = event.getRawY();
            gestureFrame.set(frame);
            moved = false;
            if (side == 0) velocity = VelocityTracker.obtain();
        }
        if (action == MotionEvent.ACTION_CANCEL) {
            finishGesture();
            return true;
        }
        int index = event.findPointerIndex(pointer);
        if (index < 0) return true;
        if (velocity != null) {
            MotionEvent screenEvent = MotionEvent.obtain(event);
            screenEvent.offsetLocation(event.getRawX() - event.getX(), event.getRawY() - event.getY());
            velocity.addMovement(screenEvent);
            screenEvent.recycle();
        }
        boolean released = action == MotionEvent.ACTION_UP
                || action == MotionEvent.ACTION_POINTER_UP && event.getActionIndex() == index;
        if (action == MotionEvent.ACTION_MOVE || released) {
            float deltaX = event.getRawX(index) - downX;
            float deltaY = event.getRawY(index) - downY;
            moved |= Math.hypot(deltaX, deltaY) > gestures.getScaledTouchSlop();
            if (moved && side == 0) {
                setFrame(gestureFrame.left + Math.round(deltaX), gestureFrame.top + Math.round(deltaY), frame.width(), 0);
            } else if (moved) {
                resize(side, deltaX, deltaY);
            }
            if (released) {
                if (moved && velocity != null) startFling();
                if (!moved && side == 0 && action == MotionEvent.ACTION_UP) chrome.grip.performClick();
                finishGesture();
            }
        }
        return true;
    }

    /** Keeps the opposite top corner fixed and projects motion onto the aspect-ratio diagonal. */
    private void resize(int side, float deltaX, float deltaY) {
        int widthDelta = Math.round((side * deltaX + ASPECT * deltaY) / (1 + ASPECT * ASPECT));
        int horizontalSpace = side < 0 ? gestureFrame.right - area.left : area.right - gestureFrame.left;
        int verticalSpace = area.bottom - gestureFrame.top;
        int maximum = Math.min(horizontalSpace, (int) (verticalSpace / ASPECT));
        int minimum = Math.min(maximum, chrome.dp(184));
        int threshold = minimum - Math.min(chrome.dp(32), Math.max(1, minimum / 5));
        int requested = gestureFrame.width() + widthDelta;
        int cardWidth = Math.max(threshold, Math.min(maximum, requested));
        int left = side < 0 ? gestureFrame.right - cardWidth : gestureFrame.left;
        setFrame(left, gestureFrame.top, cardWidth, 0);
        if (requested <= threshold) {
            int restoreLeft = side < 0 ? gestureFrame.right - minimum : gestureFrame.left;
            minimize(new Rect(restoreLeft, gestureFrame.top, restoreLeft + minimum,
                    gestureFrame.top + contentHeight(minimum)));
            finishGesture();
        }
    }

    /** Continues a fast drag with bounded native inertial deceleration. */
    private void startFling() {
        velocity.computeCurrentVelocity(1000, chrome.dp(2400));
        int horizontal = Math.round(velocity.getXVelocity(pointer));
        int vertical = Math.round(velocity.getYVelocity(pointer));
        if (Math.hypot(horizontal, vertical) < chrome.dp(250)) return;
        scroller.fling(frame.left, frame.top, horizontal, vertical, area.left, area.right - frame.width(),
                area.top, area.bottom - frame.height());
        chrome.root.postOnAnimation(flingFrame);
    }

    /** Advances the inertial movement at the display refresh rate. */
    private void stepFling() {
        if (closed || !scroller.computeScrollOffset()) return;
        setFrame(scroller.getCurrX(), scroller.getCurrY(), frame.width(), 0);
        if (!scroller.isFinished()) chrome.root.postOnAnimation(flingFrame);
    }

    /** Stops at the current position so the user can grab a moving window. */
    private void stopFling() {
        scroller.forceFinished(true);
        chrome.root.removeCallbacks(flingFrame);
    }

    /** Releases the screen-space pointer tracker at the end of a chrome gesture. */
    private void finishGesture() {
        pointer = -1;
        if (velocity != null) {
            velocity.recycle();
            velocity = null;
        }
    }

    /** Forwards every pointer and scroll event in the virtual display's coordinate space. */
    private boolean touch(View view, MotionEvent event) {
        if (minimized || transition != null) return true;
        try {
            requireOpen();
            input.motion(event, displayId(), displayWidth / (float) view.getWidth(), displayHeight / (float) view.getHeight());
        } catch (Exception error) { report(error); }
        return true;
    }

    /** Provides native physical window bounds for the existing cross-platform indicator. */
    JSONObject describe() throws Exception {
        requireOpen();
        int[] location = new int[2];
        surface.getLocationOnScreen(location);
        Rect bounds = new Rect(location[0], location[1], location[0] + surface.getWidth(), location[1] + surface.getHeight());
        return new JSONObject().put("windowId", windowId).put("displayId", displayId()).put("title", packageName).put("packageName", packageName)
                .put("chromeWindowId", View.class.getMethod("getAccessibilityWindowId").invoke(chrome.root).toString())
                .put("bounds", bounds.toShortString()).put("isMinimized", minimized);
    }

    /** Returns the exact native display ID without assigning a primary-display default. */
    int displayId() {
        if (display == null) throw new IllegalStateException("Virtual display is not ready");
        return display.getDisplay().getDisplayId();
    }

    /** Rejects operations against closed or incomplete sessions. */
    private void requireOpen() {
        if (closed || display == null) throw new IllegalStateException("Android window is unavailable");
    }

    /** Makes native input and control failures visible without replacing the requested action. */
    private void report(Exception error) {
        Log.e("OperitUiServer", "Window operation failed", error);
        chrome.showError(error.toString());
    }

    /** Removes the window and destroys its tasks and Surface on explicit closure. */
    void close() {
        if (closed) return;
        closed = true;
        stopFling();
        chrome.root.removeCallbacks(layoutFrame);
        layoutScheduled = false;
        finishGesture();
        if (transition != null) transition.cancel();
        releaseDockSlot();
        ready.completeExceptionally(new IllegalStateException("Android window closed"));
        try {
            if (display != null) {
                removed.accept(displayId());
                VirtualDisplay releasing = display;
                display = null;
                releasing.release();
            }
        } finally {
            if (receiver != null) { receiver.dispose(); receiver = null; }
            if (attached) { attached = false; manager.removeViewImmediate(chrome.root); }
        }
    }
}
