package com.operit.edgephone;

import android.content.Context;
import android.graphics.Canvas;
import android.graphics.Color;
import android.graphics.Paint;
import android.graphics.RectF;
import android.view.View;

/** Resolution-independent face with a brief blink, rather than a continuous render loop. */
final class FaceView extends View {
    private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final RectF bounds = new RectF();
    private String expression = "happy";
    private boolean blinking;
    private final Runnable blink = new Runnable() {
        /** Draws two frames per blink and stops when the view is hidden. */
        @Override public void run() {
            if (!isShown() || getWindowVisibility() != VISIBLE) return;
            blinking = !blinking;
            invalidate();
            postDelayed(this, blinking ? 130 : 4200);
        }
    };

    /** Creates one allocation-light renderer for the phone's entire display. */
    FaceView(Context context) { super(context); setContentDescription("机器人表情"); }

    /** Invalidates only when local or authenticated remote expression state changes. */
    void expression(String value) {
        if (!expression.equals(value)) { expression = value; invalidate(); }
    }

    /** Begins infrequent animation only while the screen is visible. */
    @Override protected void onWindowVisibilityChanged(int visibility) {
        super.onWindowVisibilityChanged(visibility);
        removeCallbacks(blink);
        blinking = false;
        if (visibility == VISIBLE) postDelayed(blink, 4200);
    }

    /** Ensures hidden settings screens do not retain animation callbacks. */
    @Override protected void onDetachedFromWindow() { removeCallbacks(blink); super.onDetachedFromWindow(); }

    /** Paints eyes and mouth using a fixed normalized coordinate system. */
    @Override protected void onDraw(Canvas canvas) {
        super.onDraw(canvas);
        canvas.drawColor(Color.rgb(11, 18, 23));
        float scale = Math.min(getWidth(), getHeight()) / 240f;
        canvas.save();
        canvas.translate(getWidth() / 2f, getHeight() / 2f);
        canvas.scale(scale, scale);
        paint.setColor(Color.rgb(105, 228, 198));
        paint.setStrokeWidth(8);
        paint.setStrokeCap(Paint.Cap.ROUND);
        paint.setStyle(Paint.Style.FILL);
        boolean closed = blinking || "sleeping".equals(expression);
        float shift = "thinking".equals(expression) ? -9 : 0;
        for (int side = -1; side <= 1; side += 2) {
            float x = side * 49;
            if (closed) canvas.drawLine(x - 16, -27, x + 16, -27, paint);
            else {
                bounds.set(x - 13, -48 + shift, x + 13, -8 + shift);
                canvas.drawRoundRect(bounds, 13, 13, paint);
            }
        }
        paint.setStyle(Paint.Style.STROKE);
        if ("speaking".equals(expression)) {
            bounds.set(-20, 22, 20, 52); canvas.drawOval(bounds, paint);
        } else if ("neutral".equals(expression) || "thinking".equals(expression) || "sleeping".equals(expression)) {
            canvas.drawLine(-21, 35, 21, 35, paint);
        } else {
            bounds.set(-38, -1, 38, 49); canvas.drawArc(bounds, 15, 150, false, paint);
        }
        if ("listening".equals(expression)) {
            paint.setStrokeWidth(3); canvas.drawCircle(0, 0, 104, paint);
        }
        canvas.restore();
    }
}
