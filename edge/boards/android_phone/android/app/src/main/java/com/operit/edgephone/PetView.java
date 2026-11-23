package com.operit.edgephone;

import android.content.Context;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.Canvas;
import android.graphics.Paint;
import android.graphics.RadialGradient;
import android.graphics.Rect;
import android.graphics.RectF;
import android.graphics.Shader;
import android.os.SystemClock;
import android.view.View;
import java.io.ByteArrayOutputStream;
import java.io.InputStream;
import org.json.JSONArray;
import org.json.JSONObject;

/** Plays the original Task Status Cat frames on API 21 without a WebView. */
final class PetView extends View {
    private final Paint paint = new Paint();
    private final Rect source = new Rect();
    private final RectF destination = new RectF();
    private Bitmap atlas;
    private int[] durations;
    private int duration;
    private String expression = "idle";
    private String mood = "";
    private int accent = PetStyle.MINT;
    private long started;
    private long expressionStarted;
    private long greetingUntil;
    private boolean completionFinished;
    private Shader halo;

    /** Creates a touch-accessible pet and loads only its current animation. */
    PetView(Context context) {
        super(context);
        setFocusable(true);
        setContentDescription("任务状态猫，轻点互动，长按打开设备空间");
        select("idle");
    }

    /** Keeps the six existing robot-face commands as explicit animation aliases. */
    private static String moodFor(String expression) {
        switch (expression) {
            case "working": case "speaking": return "working";
            case "thinking": return "thinking";
            case "approval": case "listening": return "approval";
            case "completed": case "happy": return "completed";
            case "error": return "error";
            case "idle": case "neutral": case "sleeping": return "idle";
            default: throw new IllegalArgumentException("Unknown pet expression: " + expression);
        }
    }

    /** Applies remote state without restarting the animation on every UI snapshot. */
    void appearance(String nextExpression, int nextAccent) {
        if (!expression.equals(nextExpression)) {
            expression = nextExpression;
            expressionStarted = SystemClock.uptimeMillis();
            completionFinished = false;
            greetingUntil = 0;
            select(moodFor(expression));
        }
        if (accent != nextAccent) { accent = nextAccent; updateHalo(); }
        invalidate();
    }

    /** Briefly raises a paw, then resumes the authoritative task animation. */
    void greet() {
        greetingUntil = SystemClock.uptimeMillis() + 1200;
        select("approval");
        started = SystemClock.uptimeMillis();
        invalidate();
    }

    /** Decodes one compact lossless atlas and the exact original frame durations. */
    private void select(String next) {
        if (next.equals(mood)) return;
        try {
            JSONObject info;
            try (InputStream stream = getContext().getAssets().open("task-cat/" + next + ".json")) {
                ByteArrayOutputStream bytes = new ByteArrayOutputStream();
                byte[] buffer = new byte[1024];
                int count;
                while ((count = stream.read(buffer)) != -1) bytes.write(buffer, 0, count);
                info = new JSONObject(bytes.toString("UTF-8"));
            }
            Bitmap image;
            try (InputStream stream = getContext().getAssets().open("task-cat/" + next + ".png")) {
                image = BitmapFactory.decodeStream(stream);
            }
            if (image == null) throw new IllegalStateException("Invalid task cat atlas: " + next);
            JSONArray timing = info.getJSONArray("durations");
            durations = new int[timing.length()];
            duration = 0;
            for (int index = 0; index < durations.length; index++) {
                durations[index] = timing.getInt(index);
                duration += durations[index];
            }
            if (atlas != null) atlas.recycle();
            atlas = image;
            mood = next;
            started = SystemClock.uptimeMillis();
        } catch (Exception error) {
            throw new IllegalStateException("Cannot load task cat animation: " + next, error);
        }
    }

    /** Stops scheduling frames while the home surface is hidden. */
    @Override protected void onWindowVisibilityChanged(int visibility) {
        super.onWindowVisibilityChanged(visibility);
        if (visibility == VISIBLE) invalidate();
    }

    /** Rebuilds the ambient gradient only when the screen geometry changes. */
    @Override protected void onSizeChanged(int width, int height, int oldWidth, int oldHeight) {
        super.onSizeChanged(width, height, oldWidth, oldHeight);
        updateHalo();
    }

    /** Caches background lighting separately from the original pixel artwork. */
    private void updateHalo() {
        float height = getHeight() - getPaddingTop() - getPaddingBottom();
        float scale = Math.min(getWidth() / 90f, height / 88f);
        if (scale <= 0) return;
        halo = new RadialGradient(getWidth() / 2f, getPaddingTop() + height / 2f, 90 * scale * .65f,
            0x26000000 | (accent & 0xffffff), accent & 0xffffff, Shader.TileMode.CLAMP);
    }

    /** Fits the full pixel silhouette inside the available round-screen stage. */
    @Override protected void onDraw(Canvas canvas) {
        super.onDraw(canvas);
        long now = SystemClock.uptimeMillis();
        if (!completionFinished && "completed".equals(moodFor(expression)) && now - expressionStarted >= 3000) {
            completionFinished = true;
            if (greetingUntil == 0) select("idle");
        }
        if (greetingUntil != 0 && now >= greetingUntil) {
            greetingUntil = 0;
            select(completionFinished ? "idle" : moodFor(expression));
        }
        long elapsed = SystemClock.uptimeMillis() - started;
        int position = (int) (elapsed % duration), frame = 0;
        while (frame < durations.length - 1 && position >= durations[frame]) position -= durations[frame++];
        float height = getHeight() - getPaddingTop() - getPaddingBottom();
        if (height <= 0) return;
        float scale = Math.min(getWidth() / 90f, height / 88f);
        float width = 90 * scale, drawnHeight = 88 * scale;
        float centerX = getWidth() / 2f, centerY = getPaddingTop() + height / 2f;
        paint.setShader(halo);
        canvas.drawCircle(centerX, centerY, Math.max(width, drawnHeight) * .65f, paint);
        paint.setShader(null);
        source.set((frame % 8) * 90, (frame / 8) * 88, (frame % 8 + 1) * 90, (frame / 8 + 1) * 88);
        destination.set(centerX - width / 2, centerY - drawnHeight / 2, centerX + width / 2, centerY + drawnHeight / 2);
        canvas.drawBitmap(atlas, source, destination, paint);
        if (isShown() && getWindowVisibility() == VISIBLE) postInvalidateDelayed(durations[frame] - position);
    }
}
