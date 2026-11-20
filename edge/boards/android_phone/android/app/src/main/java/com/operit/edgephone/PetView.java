package com.operit.edgephone;

import android.animation.ValueAnimator;
import android.content.Context;
import android.graphics.Canvas;
import android.graphics.LinearGradient;
import android.graphics.Paint;
import android.graphics.Path;
import android.graphics.RadialGradient;
import android.graphics.Shader;
import android.view.View;

/** Draws the watch pet at any display density with brief, local reactions. */
final class PetView extends View {
    private final Paint paint = new Paint(Paint.ANTI_ALIAS_FLAG);
    private final Path body = new Path();
    private final Path tail = new Path();
    private final Path head = new Path();
    private final Path ears = new Path();
    private final Path muzzle = new Path();
    private final Path nose = new Path();
    private final Path whiskers = new Path();
    private final Path smile = new Path();
    private final Path spark = new Path();
    private final Shader fur = new LinearGradient(-31, -54, 19, 30, 0xff85c8ed, 0xff4d91cb, Shader.TileMode.CLAMP);
    private String expression = "happy";
    private int accent = PetStyle.MINT;
    private Shader halo;
    private boolean blinking;
    private float reaction;
    private ValueAnimator greeting;
    private final Runnable blink = new Runnable() {
        /** Alternates two blink frames with an idle pause. */
        @Override public void run() {
            if (!isShown() || getWindowVisibility() != VISIBLE) return;
            blinking = !blinking;
            invalidate();
            postDelayed(this, blinking ? 120 : 4600);
        }
    };

    /** Creates the cat silhouette and a touch-accessible target. */
    PetView(Context context) {
        super(context);
        setContentDescription("小猫桌宠，轻点打招呼，长按打开设备空间");
        setFocusable(true);
        shapeBody();
        shapeFace();
        updateHalo();
    }

    /** Defines the sitting pose and curling tail. */
    private void shapeBody() {
        body.moveTo(-34, 14);
        body.cubicTo(-47, 31, -49, 52, -36, 61);
        body.cubicTo(-25, 68, 14, 67, 25, 59);
        body.cubicTo(38, 49, 33, 26, 18, 12);
        body.close();
        tail.moveTo(19, 52);
        tail.cubicTo(49, 69, 83, 53, 75, 29);
        tail.cubicTo(71, 16, 56, 18, 57, 29);
    }

    /** Builds the triangular ears, muzzle and whiskers once for all expressions. */
    private void shapeFace() {
        head.moveTo(-56, -23);
        head.cubicTo(-59, -35, -62, -50, -59, -60);
        head.quadTo(-58, -65, -53, -62);
        head.cubicTo(-43, -58, -33, -49, -27, -44);
        head.cubicTo(-17, -48, -2, -49, 11, -45);
        head.cubicTo(20, -53, 29, -59, 39, -62);
        head.quadTo(44, -64, 45, -59);
        head.cubicTo(47, -50, 44, -33, 41, -24);
        head.cubicTo(55, -15, 55, 3, 49, 14);
        head.cubicTo(40, 31, 18, 37, -8, 37);
        head.cubicTo(-35, 37, -55, 29, -62, 12);
        head.cubicTo(-67, 0, -66, -14, -56, -23);
        head.close();
        ears.moveTo(-53, -55);
        ears.quadTo(-43, -53, -35, -43);
        ears.quadTo(-46, -40, -51, -34);
        ears.quadTo(-54, -46, -53, -55);
        ears.close();
        ears.moveTo(38, -54);
        ears.quadTo(40, -46, 35, -34);
        ears.quadTo(30, -39, 21, -43);
        ears.quadTo(30, -51, 38, -54);
        ears.close();
        muzzle.moveTo(-43, 12);
        muzzle.cubicTo(-38, 1, -22, 3, -8, 10);
        muzzle.cubicTo(4, 3, 22, 1, 28, 11);
        muzzle.cubicTo(36, 22, 22, 33, -8, 34);
        muzzle.cubicTo(-34, 34, -50, 24, -43, 12);
        muzzle.close();
        nose.moveTo(-13, 11);
        nose.quadTo(-8, 9, -3, 11);
        nose.quadTo(-3, 14, -8, 17);
        nose.quadTo(-13, 14, -13, 11);
        nose.close();
        smile.moveTo(-8, 17);
        smile.lineTo(-8, 21);
        smile.cubicTo(-11, 25, -17, 24, -19, 20);
        smile.moveTo(-8, 21);
        smile.cubicTo(-5, 25, 1, 24, 3, 20);
        whiskers.moveTo(-39, 12);
        whiskers.lineTo(-60, 8);
        whiskers.moveTo(-39, 18);
        whiskers.lineTo(-58, 22);
        whiskers.moveTo(23, 12);
        whiskers.lineTo(44, 8);
        whiskers.moveTo(23, 18);
        whiskers.lineTo(42, 22);
    }

    /** Updates the authoritative expression independently of tap animations. */
    void appearance(String nextExpression, int nextAccent) {
        if (expression.equals(nextExpression) && accent == nextAccent) return;
        expression = nextExpression;
        if (accent != nextAccent) {
            accent = nextAccent;
            updateHalo();
        }
        invalidate();
    }

    /** Performs a local hop without a network round trip. */
    void greet() {
        if (greeting != null) greeting.cancel();
        greeting = ValueAnimator.ofFloat(0f, 1f);
        greeting.setDuration(620);
        greeting.addUpdateListener(animation -> {
            reaction = (float) animation.getAnimatedValue();
            invalidate();
        });
        greeting.start();
    }

    /** Colors the ambient light while keeping the cat's coat blue. */
    private void updateHalo() {
        int tint = accent & 0x00ffffff;
        halo = new RadialGradient(0, -13, 124, 0x3d000000 | tint, tint, Shader.TileMode.CLAMP);
    }

    /** Runs the blink only while the app surface is visible. */
    @Override protected void onWindowVisibilityChanged(int visibility) {
        super.onWindowVisibilityChanged(visibility);
        removeCallbacks(blink);
        blinking = false;
        if (visibility == VISIBLE) postDelayed(blink, 4600);
        else if (greeting != null) greeting.cancel();
        reaction = 0;
    }

    /** Releases transient animation when the user opens a device page. */
    @Override protected void onDetachedFromWindow() {
        removeCallbacks(blink);
        if (greeting != null) greeting.cancel();
        super.onDetachedFromWindow();
    }

    /** Paints the sitting cat and its short greeting reaction. */
    @Override protected void onDraw(Canvas canvas) {
        super.onDraw(canvas);
        float stageHeight = getHeight() - getPaddingTop() - getPaddingBottom();
        if (stageHeight <= 0) return;
        float scale = Math.min(getWidth() / 188f, stageHeight / 132f);
        float hop = (float) Math.sin(Math.PI * reaction);
        canvas.save();
        canvas.translate(getWidth() / 2f, getPaddingTop() + stageHeight / 2f);
        canvas.scale(scale, scale);
        paint.setStyle(Paint.Style.FILL);
        paint.setColor(0xffffffff);
        paint.setShader(halo);
        canvas.drawCircle(0, -13, 124, paint);
        paint.setShader(null);
        paint.setColor(0x99090c13);
        canvas.drawOval(-52 + hop * 6, 59, 68 - hop * 6, 71, paint);
        canvas.save();
        canvas.translate(0, -12 * hop);
        drawBody(canvas, hop);
        drawFace(canvas, hop > 0.25f);
        canvas.restore();
        if (hop > 0.01f) {
            drawSpark(canvas, -73 - hop * 5, -21 - hop * 18, 5 * hop, PetStyle.PEACH);
            drawSpark(canvas, 67 + hop * 5, -20 - hop * 15, 7 * hop, accent);
        }
        canvas.restore();
    }

    /** Draws the tail behind the body and keeps the paws visible at small sizes. */
    private void drawBody(Canvas canvas, float hop) {
        canvas.save();
        canvas.rotate(-12 * hop, 19, 52);
        paint.setColor(0xff508fc5);
        paint.setStyle(Paint.Style.STROKE);
        paint.setStrokeCap(Paint.Cap.ROUND);
        paint.setStrokeWidth(15);
        canvas.drawPath(tail, paint);
        canvas.restore();
        paint.setStyle(Paint.Style.FILL);
        paint.setColor(0xff477db0);
        canvas.drawPath(body, paint);
        paint.setColor(0xff9ec8e5);
        canvas.drawOval(-26, 25, 7, 61, paint);
        paint.setColor(0xffc5e3f4);
        canvas.drawOval(-37, 54, -10, 66, paint);
        canvas.drawOval(-5, 54, 22, 66, paint);
    }

    /** Draws the coat, ears and expression in front of the sitting pose. */
    private void drawFace(Canvas canvas, boolean greeting) {
        paint.setColor(0xffffffff);
        paint.setShader(fur);
        canvas.drawPath(head, paint);
        paint.setShader(null);
        paint.setColor(0xffdf9eac);
        canvas.drawPath(ears, paint);
        paint.setColor(0xffd9edf7);
        canvas.drawPath(muzzle, paint);
        drawExpression(canvas, greeting);
        paint.setStyle(Paint.Style.STROKE);
        paint.setStrokeWidth(1.8f);
        paint.setColor(0xff325671);
        canvas.drawPath(whiskers, paint);
        paint.setStyle(Paint.Style.FILL);
        paint.setColor(0xffb7607f);
        canvas.drawPath(nose, paint);
    }

    /** Keeps all existing remote expressions legible on a small screen. */
    private void drawExpression(Canvas canvas, boolean greeting) {
        paint.setStrokeWidth(3.4f);
        paint.setStrokeCap(Paint.Cap.ROUND);
        boolean eyesClosed = blinking || "sleeping".equals(expression) || greeting;
        int lift = "thinking".equals(expression) ? -4 : 0;
        for (int center = -30; center <= 14; center += 44) {
            paint.setColor(0xff182d45);
            if (eyesClosed) {
                paint.setStyle(Paint.Style.STROKE);
                canvas.drawArc(center - 7, -10, center + 7, 2, 195, 150, false, paint);
            } else {
                paint.setStyle(Paint.Style.FILL);
                canvas.drawOval(center - 6, -13 + lift, center + 6, 4 + lift, paint);
                paint.setColor(0xfff4fbfd);
                canvas.drawCircle(center - 2, -8 + lift, 1.8f, paint);
            }
        }
        paint.setColor(0xff365570);
        paint.setStrokeWidth(2.1f);
        paint.setStyle(Paint.Style.STROKE);
        if ("speaking".equals(expression) && !greeting) {
            canvas.drawOval(-13, 18, -3, 28, paint);
        } else if (greeting || "happy".equals(expression) || "listening".equals(expression)) {
            canvas.drawPath(smile, paint);
        } else {
            canvas.drawLine(-13, 22, -3, 22, paint);
        }
        if ("listening".equals(expression)) {
            paint.setColor(accent);
            paint.setStrokeWidth(2);
            canvas.drawArc(-77, -44, 61, 32, 161, 45, false, paint);
            canvas.drawArc(-77, -44, 61, 32, -26, 45, false, paint);
        }
    }

    /** Draws a four-point accent star without loading a bitmap. */
    private void drawSpark(Canvas canvas, float centerX, float centerY, float size, int color) {
        spark.reset();
        spark.moveTo(centerX, centerY - size);
        spark.quadTo(centerX + size * .18f, centerY - size * .18f, centerX + size, centerY);
        spark.quadTo(centerX + size * .18f, centerY + size * .18f, centerX, centerY + size);
        spark.quadTo(centerX - size * .18f, centerY + size * .18f, centerX - size, centerY);
        spark.quadTo(centerX - size * .18f, centerY - size * .18f, centerX, centerY - size);
        spark.close();
        paint.setStyle(Paint.Style.FILL);
        paint.setColor(color);
        canvas.drawPath(spark, paint);
    }
}
