package com.operit.edgephone;

import android.content.Context;
import android.content.res.ColorStateList;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.graphics.drawable.RippleDrawable;
import android.view.Gravity;
import android.widget.Button;
import android.widget.TextView;

/** Visual tokens shared with the Operit watch companion panel. */
final class PetStyle {
    static final int NIGHT = 0xff10131d;
    static final int PANEL = 0xff1c2231;
    static final int LINE = 0xff343e50;
    static final int INK = 0xfff4f2e9;
    static final int MUTED = 0xffb5bdce;
    static final int MINT = 0xffa5e9cb;
    static final int PEACH = 0xffffbb9e;
    static final int LILAC = 0xffc6bcff;

    /** Converts display-independent sizing to physical pixels. */
    static int dp(Context context, int value) {
        return Math.round(context.getResources().getDisplayMetrics().density * value);
    }

    /** Resolves one device-owned appearance setting. */
    static int accent(String value) {
        switch (value) {
            case "mint": return MINT;
            case "peach": return PEACH;
            case "lilac": return LILAC;
            default: throw new IllegalArgumentException("Unknown pet accent: " + value);
        }
    }

    /** Draws one bounded surface with rounded geometry. */
    static GradientDrawable surface(Context context, int fill, int radius, int outline) {
        GradientDrawable shape = new GradientDrawable();
        shape.setColor(fill);
        shape.setCornerRadius(dp(context, radius));
        if (outline != Color.TRANSPARENT) shape.setStroke(dp(context, 1), outline);
        return shape;
    }

    /** Styles labels consistently on round and rectangular displays. */
    static TextView label(Context context, String text, int size, int color, boolean bold) {
        TextView label = new TextView(context);
        label.setText(text);
        label.setTextSize(size);
        label.setTextColor(color);
        label.setGravity(Gravity.CENTER);
        label.setTypeface(Typeface.create(bold ? "sans-serif-medium" : "sans-serif", Typeface.NORMAL));
        return label;
    }

    /** Styles the compact home entry as an unfilled icon and label. */
    static Button homeAction(Context context) {
        Button button = new Button(context);
        button.setAllCaps(false);
        button.setTextSize(12);
        button.setTextColor(new ColorStateList(new int[][] {
            new int[] {-android.R.attr.state_enabled}, new int[] {}
        }, new int[] {0xff727c90, MUTED}));
        button.setTypeface(Typeface.create("sans-serif-medium", Typeface.NORMAL));
        button.setGravity(Gravity.CENTER);
        button.setPadding(dp(context, 12), 0, dp(context, 12), 0);
        button.setCompoundDrawablePadding(dp(context, 6));
        button.setStateListAnimator(null);
        button.setBackgroundTintList(null);
        button.setBackground(new RippleDrawable(ColorStateList.valueOf(0x22b5bdce), null,
            surface(context, Color.WHITE, 24, Color.TRANSPARENT)));
        return button;
    }

    /** Creates a pill action with a 48dp touch target and input feedback. */
    static Button action(Context context, String text, boolean primary) {
        Button button = new Button(context);
        button.setText(text);
        button.setAllCaps(false);
        button.setTextSize(14);
        button.setTextColor(new ColorStateList(new int[][] {
            new int[] {-android.R.attr.state_enabled}, new int[] {}
        }, new int[] {primary ? LINE : MUTED, primary ? NIGHT : INK}));
        button.setTypeface(Typeface.create("sans-serif-medium", Typeface.NORMAL));
        button.setMinHeight(dp(context, 48));
        button.setMinimumHeight(dp(context, 48));
        button.setPadding(dp(context, 8), 0, dp(context, 8), 0);
        button.setStateListAnimator(null);
        button.setBackgroundTintList(null);
        button.setBackground(new RippleDrawable(ColorStateList.valueOf(primary ? 0x3310131d : 0x40a5e9cb),
            surface(context, primary ? MINT : PANEL, 24, primary ? Color.TRANSPARENT : LINE), null));
        return button;
    }

    /** Keeps the visual constants independent of any screen. */
    private PetStyle() {}
}
