package app.operit.uiserver;

import android.content.Context;
import android.content.res.ColorStateList;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.graphics.Region;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.graphics.drawable.RippleDrawable;
import android.os.Build;
import android.view.Gravity;
import android.view.SurfaceControl;
import android.view.SurfaceView;
import android.view.View;
import android.view.ViewTreeObserver;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.TextView;
import java.lang.reflect.Method;
import java.lang.reflect.Proxy;

/** Composes a live, rounded Surface with only local handles over the app. */
final class WindowChrome {
    private static final int INK = 0xfff1f3f8;
    private static final int PANEL = 0xff252a33;
    private static final int HANDLE = 0xd9262b34;
    final FrameLayout root;
    final SurfaceView surface;
    final View grip;
    final View resizeLeft;
    final View resizeRight;
    private final FrameLayout card;
    private final FrameLayout actions;
    private final View restore;
    private final TextView errorLabel;
    private final GradientDrawable background;
    private final Method setViewportRadius;
    private final Method getSurfaceControl;
    private final Method setLayerRadius;
    private final float density;
    private final int bufferWidth;
    private final int shadow;
    private final ViewTreeObserver.OnPreDrawListener radiusFrame = this::applyLayerRadius;
    private float cornerRadius;
    private boolean updateLayerRadius;

    /** Creates native window affordances without reserving a title or footer bar. */
    WindowChrome(Context context, CharSequence name, int densityDpi, int bufferWidth, Runnable back, Runnable minimize,
            Runnable fullscreen, Runnable close, Runnable expand) throws ReflectiveOperationException {
        this.bufferWidth = bufferWidth;
        density = densityDpi / 160f;
        shadow = dp(32);
        root = new FrameLayout(context);
        root.setClipChildren(false);
        card = new FrameLayout(context);
        background = rounded(PANEL, 22);
        card.setBackground(background);
        card.setClipToOutline(true);
        card.setElevation(dp(9));
        FrameLayout.LayoutParams frame = new FrameLayout.LayoutParams(-1, -1);
        frame.setMargins(shadow, shadow, shadow, shadow);
        root.addView(card, frame);

        surface = new SurfaceView(context);
        setViewportRadius = SurfaceView.class.getMethod("setCornerRadius", float.class);
        getSurfaceControl = SurfaceView.class.getMethod("getSurfaceControl");
        setLayerRadius = SurfaceControl.Transaction.class.getMethod("setCornerRadius", SurfaceControl.class, float.class);
        card.addView(surface, new FrameLayout.LayoutParams(-1, -1));
        surface.addOnAttachStateChangeListener(new View.OnAttachStateChangeListener() {
            /** Runs after SurfaceView's own pre-draw update so its raw radius cannot overwrite ours. */
            @Override public void onViewAttachedToWindow(View view) {
                view.getViewTreeObserver().addOnPreDrawListener(radiusFrame);
                updateLayerRadius = true;
            }
            /** Removes the listener before a restack attaches this same SurfaceView again. */
            @Override public void onViewDetachedFromWindow(View view) {
                view.getViewTreeObserver().removeOnPreDrawListener(radiusFrame);
            }
        });
        surface.addOnLayoutChangeListener((view, left, top, right, bottom, oldLeft, oldTop, oldRight, oldBottom) -> {
            if (right - left != oldRight - oldLeft) updateLayerRadius = true;
        });
        grip = new Mark(context, Glyph.GRIP);
        grip.setContentDescription("Window actions");
        grip.setOnClickListener(view -> openActions());
        grip.setFocusable(true);
        card.addView(grip, new FrameLayout.LayoutParams(dp(68), dp(20), Gravity.TOP | Gravity.CENTER_HORIZONTAL));
        resizeLeft = new Mark(context, Glyph.LEFT);
        resizeRight = new Mark(context, Glyph.RIGHT);
        card.addView(resizeLeft, new FrameLayout.LayoutParams(dp(34), dp(31), Gravity.BOTTOM | Gravity.LEFT));
        card.addView(resizeRight, new FrameLayout.LayoutParams(dp(34), dp(31), Gravity.BOTTOM | Gravity.RIGHT));

        actions = new FrameLayout(context);
        actions.setOnClickListener(view -> dismissActions());
        actions.setImportantForAccessibility(View.IMPORTANT_FOR_ACCESSIBILITY_NO);
        card.addView(actions, new FrameLayout.LayoutParams(-1, -1));
        LinearLayout menu = new LinearLayout(context);
        menu.setOrientation(LinearLayout.VERTICAL);
        menu.setPadding(dp(5), dp(5), dp(5), dp(5));
        menu.setBackground(rounded(PANEL, 14));
        menu.setElevation(dp(6));
        FrameLayout.LayoutParams menuFrame = new FrameLayout.LayoutParams(dp(156), -2,
                Gravity.TOP | Gravity.CENTER_HORIZONTAL);
        menuFrame.topMargin = dp(14);
        actions.addView(menu, menuFrame);
        addAction(menu, "Minimize", minimize);
        addAction(menu, "Fullscreen", fullscreen);
        addAction(menu, "Back", back);
        addAction(menu, "Close", close);
        dismissActions();

        errorLabel = new TextView(context);
        errorLabel.setTextColor(INK);
        errorLabel.setPadding(dp(12), dp(8), dp(12), dp(8));
        errorLabel.setBackground(rounded(PANEL, 12));
        FrameLayout.LayoutParams errorFrame = new FrameLayout.LayoutParams(-2, -2, Gravity.TOP | Gravity.CENTER_HORIZONTAL);
        errorFrame.topMargin = dp(30);
        card.addView(errorLabel, errorFrame);
        errorLabel.setVisibility(View.GONE);
        errorLabel.setOnClickListener(view -> errorLabel.setVisibility(View.GONE));

        restore = new Mark(context, Glyph.RESTORE);
        restore.setContentDescription("Restore " + name);
        restore.setOnClickListener(view -> expand.run());
        restore.setFocusable(true);
        card.addView(restore, new FrameLayout.LayoutParams(-1, -1));
        update(0);
        installTouchableRegion();
    }

    /** Restricts input to the card, excluding the transparent gutter used for its shadow. */
    private void installTouchableRegion() throws ReflectiveOperationException {
        Class<?> listenerType = Class.forName("android.view.ViewTreeObserver$OnComputeInternalInsetsListener");
        Class<?> infoType = Class.forName("android.view.ViewTreeObserver$InternalInsetsInfo");
        var insets = infoType.getMethod("setTouchableInsets", int.class);
        var region = infoType.getField("touchableRegion");
        int regionMode = infoType.getField("TOUCHABLE_INSETS_REGION").getInt(null);
        Object listener = Proxy.newProxyInstance(listenerType.getClassLoader(), new Class<?>[] {listenerType},
                (proxy, method, arguments) -> {
                    insets.invoke(arguments[0], regionMode);
                    ((Region) region.get(arguments[0])).set(card.getLeft(), card.getTop(), card.getRight(), card.getBottom());
                    return null;
                });
        ViewTreeObserver.class.getMethod("addOnComputeInternalInsetsListener", listenerType)
                .invoke(root.getViewTreeObserver(), listener);
    }

    /** Converts a native chrome measurement into physical pixels. */
    int dp(int value) { return Math.round(value * density); }

    /** Returns the gutter required for the native elevation shadow. */
    int shadow() { return shadow; }

    /** Creates a rounded native background in physical display coordinates. */
    private GradientDrawable rounded(int color, int radius) {
        GradientDrawable shape = new GradientDrawable();
        shape.setColor(color);
        shape.setCornerRadius(dp(radius));
        return shape;
    }

    /** Shows the actions only when the user taps the move handle. */
    private void openActions() {
        actions.setAlpha(0);
        actions.setVisibility(View.VISIBLE);
        actions.animate().alpha(1).setDuration(125).start();
    }

    /** Dismisses actions before changing geometry or delivering an app command. */
    void dismissActions() {
        actions.animate().cancel();
        actions.setVisibility(View.GONE);
    }

    /** Adds one clearly labeled action with press feedback. */
    private void addAction(LinearLayout menu, String label, Runnable action) {
        TextView item = new TextView(menu.getContext());
        item.setText(label);
        item.setTextColor(INK);
        item.setTextSize(14);
        item.setTypeface(Typeface.create("sans-serif-medium", Typeface.NORMAL));
        item.setPadding(dp(13), 0, dp(13), 0);
        item.setGravity(Gravity.CENTER_VERTICAL);
        item.setBackground(new RippleDrawable(ColorStateList.valueOf(0x33ffffff), null, rounded(Color.WHITE, 9)));
        item.setOnClickListener(view -> {
            dismissActions();
            action.run();
        });
        item.setFocusable(true);
        menu.addView(item, new LinearLayout.LayoutParams(-1, dp(43)));
    }

    /** Displays a native failure over the app so it is visible without permanent chrome. */
    void showError(String error) {
        errorLabel.setText(error);
        errorLabel.setVisibility(View.VISIBLE);
    }

    /** Rounds both the actual Surface and its reveal, and fades affordances as the card docks. */
    void update(float dock) {
        cornerRadius = density * (18 - 5 * dock);
        background.setCornerRadius(cornerRadius);
        card.invalidateOutline();
        try {
            setViewportRadius.invoke(surface, cornerRadius);
        } catch (ReflectiveOperationException error) {
            throw new IllegalStateException("Cannot round the SurfaceView viewport", error);
        }
        updateLayerRadius = true;
        root.invalidate();
        float controls = Math.max(0, 1 - 2 * dock);
        grip.setAlpha(controls);
        grip.setVisibility(controls > 0 ? View.VISIBLE : View.INVISIBLE);
        resizeLeft.setAlpha(controls);
        resizeRight.setAlpha(controls);
        restore.setAlpha(Math.max(0, 2 * dock - 1));
        restore.setVisibility(dock > 0 ? View.VISIBLE : View.GONE);
        restore.setImportantForAccessibility(dock >= 0.5f
                ? View.IMPORTANT_FOR_ACCESSIBILITY_YES : View.IMPORTANT_FOR_ACCESSIBILITY_NO);
    }

    /** Matches the compositor's buffer-space rounding to the outline drawn in view pixels. */
    private boolean applyLayerRadius() {
        if (!updateLayerRadius || !surface.getHolder().getSurface().isValid()) return true;
        try (SurfaceControl.Transaction transaction = new SurfaceControl.Transaction()) {
            SurfaceControl layer = (SurfaceControl) getSurfaceControl.invoke(surface);
            setLayerRadius.invoke(transaction, layer, cornerRadius * bufferWidth / surface.getWidth());
            if (Build.VERSION.SDK_INT >= 31) root.getRootSurfaceControl().applyTransactionOnDraw(transaction);
            else transaction.apply();
            updateLayerRadius = false;
            return true;
        } catch (ReflectiveOperationException error) {
            throw new IllegalStateException("Cannot round the native display layer", error);
        }
    }

    /** Identifies a localized handle drawn over the app. */
    private enum Glyph { GRIP, LEFT, RIGHT, RESTORE }

    /** Draws the minimal move, resize and restore controls over the live Surface. */
    private final class Mark extends View {
        private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
        private final Glyph glyph;

        /** Initializes the vector mark with rounded strokes. */
        Mark(Context context, Glyph glyph) {
            super(context);
            this.glyph = glyph;
            paint.setStrokeCap(Paint.Cap.ROUND);
        }

        /** Draws a compact, high-contrast affordance at its native display density. */
        @Override protected void onDraw(Canvas canvas) {
            canvas.save();
            canvas.translate(getWidth() / 2f, getHeight() / 2f);
            canvas.scale(density, density);
            paint.setStrokeWidth(1.6f);
            paint.setColor(isPressed() ? PANEL : HANDLE);
            switch (glyph) {
                case GRIP -> {
                    paint.setColor(isPressed() ? 0xffbec5d0 : 0xffe1e5ec);
                    canvas.drawRoundRect(-15, -1.5f, 15, 1.5f, 2, 2, paint);
                }
                case LEFT, RIGHT -> {
                    canvas.scale(glyph == Glyph.LEFT ? -1 : 1, 1);
                    canvas.translate(4, 5);
                    canvas.drawCircle(0, 0, 6.5f, paint);
                    paint.setColor(INK);
                    canvas.drawLine(-2.5f, 2.5f, 2.5f, -2.5f, paint);
                    canvas.drawLine(1, 3, 3, 1, paint);
                }
                case RESTORE -> {
                    canvas.translate(0, getHeight() / (2 * density) - 16);
                    canvas.drawRoundRect(-12, -8, 12, 8, 8, 8, paint);
                    paint.setColor(INK);
                    canvas.drawLine(-3, 3, 3, -3, paint);
                    canvas.drawLine(-1, -3, 3, -3, paint);
                    canvas.drawLine(3, -3, 3, 1, paint);
                }
            }
            canvas.restore();
        }
    }
}
