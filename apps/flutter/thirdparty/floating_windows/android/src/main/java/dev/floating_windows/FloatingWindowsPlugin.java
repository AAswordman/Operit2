package dev.floating_windows;

import android.content.Context;
import android.content.Intent;
import android.graphics.Color;
import android.hardware.input.InputManager;
import android.graphics.PixelFormat;
import android.net.Uri;
import android.os.Build;
import android.os.Handler;
import android.os.Looper;
import android.provider.Settings;
import android.view.Gravity;
import android.view.MotionEvent;
import android.view.ViewConfiguration;
import android.view.WindowManager;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.UUID;
import io.flutter.embedding.android.FlutterTextureView;
import io.flutter.embedding.android.FlutterView;
import io.flutter.embedding.engine.FlutterEngine;
import io.flutter.embedding.engine.dart.DartExecutor;
import io.flutter.embedding.engine.loader.FlutterLoader;
import io.flutter.FlutterInjector;
import io.flutter.embedding.engine.plugins.FlutterPlugin;
import io.flutter.plugin.common.BinaryMessenger;
import io.flutter.plugin.common.MethodCall;
import io.flutter.plugin.common.MethodChannel;

/** Transparent live Android overlays; every surface owns an independent engine. */
public final class FloatingWindowsPlugin implements FlutterPlugin, MethodChannel.MethodCallHandler {
    public interface EngineConfigurer { void configure(FlutterEngine engine); }
    private static EngineConfigurer engineConfigurer;
    public static void setEngineConfigurer(EngineConfigurer configurer) { engineConfigurer = configurer; }
    private static final Map<String, Surface> surfaces = new LinkedHashMap<>();
    private static final Handler handler = new Handler(Looper.getMainLooper());
    private Context context;
    private BinaryMessenger messenger;
    private MethodChannel channel;

    private static final class Surface {
        String id;
        BinaryMessenger owner;
        FlutterEngine engine;
        FlutterView view;
        WindowManager manager;
        WindowManager.LayoutParams params;
        boolean attached, configured, closed, draggable, pointerDown, dragging;
        float downX, downY;
        int windowX, windowY;
        float opacity = 1.0f;
    }
    private final class DragFlutterView extends FlutterView {
        private final Surface surface;
        DragFlutterView(Context context, FlutterTextureView texture, Surface surface) {
            super(context, texture); this.surface = surface;
        }
        @Override public boolean dispatchTouchEvent(MotionEvent event) {
            Surface s = surface;
            if (!s.configured || !s.draggable) return super.dispatchTouchEvent(event);
            int action = event.getActionMasked();
            if (action == MotionEvent.ACTION_DOWN) {
                s.pointerDown = true; s.dragging = false;
                s.downX = event.getRawX(); s.downY = event.getRawY();
                s.windowX = s.params.x; s.windowY = s.params.y;
            }
            if (event.getPointerCount() != 1 || action == MotionEvent.ACTION_CANCEL) {
                boolean consumed = s.dragging;
                s.pointerDown = false; s.dragging = false;
                return consumed || super.dispatchTouchEvent(event);
            }
            if (s.pointerDown && (action == MotionEvent.ACTION_MOVE || action == MotionEvent.ACTION_UP)) {
                float dx = event.getRawX() - s.downX, dy = event.getRawY() - s.downY;
                if (!s.dragging && Math.hypot(dx, dy) >= ViewConfiguration.get(getContext()).getScaledTouchSlop()) {
                    s.dragging = true;
                    MotionEvent cancel = MotionEvent.obtain(event);
                    cancel.setAction(MotionEvent.ACTION_CANCEL);
                    super.dispatchTouchEvent(cancel); cancel.recycle();
                }
                if (s.dragging) {
                    s.params.x = s.windowX + Math.round(dx); s.params.y = s.windowY + Math.round(dy);
                    s.manager.updateViewLayout(s.view, s.params);
                }
                boolean consumed = s.dragging;
                if (action == MotionEvent.ACTION_UP) { s.pointerDown = false; s.dragging = false; }
                if (consumed) return true;
            }
            return super.dispatchTouchEvent(event);
        }
    }
    @Override public void onAttachedToEngine(FlutterPluginBinding binding) {
        context = binding.getApplicationContext(); messenger = binding.getBinaryMessenger();
        channel = new MethodChannel(messenger, "floating_windows/window");
        channel.setMethodCallHandler(this);
    }
    @Override public void onDetachedFromEngine(FlutterPluginBinding binding) {
        channel.setMethodCallHandler(null);
        for (Surface surface : new ArrayList<>(surfaces.values())) {
            if (surface.owner == messenger) destroy(surface);
        }
    }
    private Surface current() {
        for (Surface surface : surfaces.values()) {
            if (surface.engine.getDartExecutor().getBinaryMessenger() == messenger) return surface;
        }
        throw new IllegalStateException("Only an overlay engine can control its current surface");
    }
    private boolean permitted() { return Settings.canDrawOverlays(context); }
    private int pixels(double value) { return (int)Math.round(value * context.getResources().getDisplayMetrics().density); }
    private double number(Map<?, ?> args, String key, double fallback, boolean dimension) {
        Object raw = args.get(key);
        if (raw == null) return fallback;
        if (!(raw instanceof Number)) throw new IllegalArgumentException("Expected numeric " + key);
        double value = ((Number)raw).doubleValue();
        if (!Double.isFinite(value) || Math.abs(value) > 1000000 || (dimension && (value < 16 || value > 4096))) {
            throw new IllegalArgumentException("Invalid " + key);
        }
        return value;
    }
    private boolean flag(Map<?, ?> args, String key, boolean fallback) {
        Object value = args.get(key);
        if (value == null) return fallback;
        if (!(value instanceof Boolean)) throw new IllegalArgumentException("Expected boolean " + key);
        return (Boolean)value;
    }
    private void apply(Surface surface, Map<?, ?> args) {
        if (!flag(args,"alwaysOnTop",true)) throw new UnsupportedOperationException("Android overlays are always above applications");
        surface.draggable = flag(args, "draggable", surface.draggable);
        if (!surface.draggable) { surface.pointerDown = false; surface.dragging = false; }
        float density = context.getResources().getDisplayMetrics().density;
        WindowManager.LayoutParams next = new WindowManager.LayoutParams();
        next.copyFrom(surface.params);
        next.width = pixels(number(args,"width",next.width/density,true));
        next.height = pixels(number(args,"height",next.height/density,true));
        next.x = pixels(number(args,"x",next.x/density,false));
        next.y = pixels(number(args,"y",next.y/density,false));
        boolean focus = flag(args,"focusable",(next.flags & WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE)==0);
        boolean through = flag(args,"clickThrough",(next.flags & WindowManager.LayoutParams.FLAG_NOT_TOUCHABLE)!=0);
        next.flags = WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL | WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN
            | (focus ? 0 : WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE)
            | (through ? WindowManager.LayoutParams.FLAG_NOT_TOUCHABLE : 0);
        // Android 12+ blocks touch delivery through untrusted overlays whose
        // combined opacity is too high. Use the system maximum for pass-through.
        surface.opacity = through && Build.VERSION.SDK_INT >= 31
            ? ((InputManager)context.getSystemService(Context.INPUT_SERVICE)).getMaximumObscuringOpacityForTouch() : 1.0f;
        next.alpha = surface.configured ? surface.opacity : 0.0f;
        if (surface.attached) surface.manager.updateViewLayout(surface.view,next);
        surface.params = next;
    }
    private void show(Surface surface) {
        if (!permitted()) throw new IllegalStateException("Overlay permission is missing");
        surface.params.alpha=surface.opacity;
        if (!surface.attached) { surface.manager.addView(surface.view,surface.params); surface.attached=true; }
        else surface.manager.updateViewLayout(surface.view,surface.params);
        surface.engine.getLifecycleChannel().appIsResumed();
    }
    private static void destroy(Surface surface) {
        if (surface.closed) return;
        surface.closed=true; surfaces.remove(surface.id);
        if (surface.attached) { surface.manager.removeViewImmediate(surface.view); surface.attached=false; }
        surface.view.detachFromFlutterEngine(); surface.engine.destroy();
    }
    @Override public void onMethodCall(MethodCall call, MethodChannel.Result result) {
        try {
            switch (call.method) {
            case "capabilities": {
                Map<String,Object> caps = new HashMap<>();
                caps.put("supported",engineConfigurer != null); caps.put("permissionGranted",permitted());
                caps.put("positioning",true); caps.put("alwaysOnTop",false);
                caps.put("clickThrough",true); caps.put("focusable",true);
                result.success(caps); return;
            }
            case "requestPermission":
                if (!permitted()) context.startActivity(new Intent(Settings.ACTION_MANAGE_OVERLAY_PERMISSION,
                    Uri.parse("package:"+context.getPackageName())).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK));
                result.success(permitted()); return;
            case "create": {
                if (!permitted()) throw new IllegalStateException("Overlay permission is missing");
                if (engineConfigurer == null) throw new IllegalStateException("Overlay engine registration callback is missing");
                FlutterLoader loader = FlutterInjector.instance().flutterLoader();
                loader.startInitialization(context); loader.ensureInitializationComplete(context,null);
                Surface surface = new Surface(); surface.id=UUID.randomUUID().toString(); surface.owner=messenger;
                surface.engine=new FlutterEngine(context);
                surface.manager=(WindowManager)context.getSystemService(Context.WINDOW_SERVICE);
                FlutterTextureView texture = new FlutterTextureView(context);
                texture.setOpaque(false);
                surface.view=new DragFlutterView(context,texture,surface);
                surface.view.setBackgroundColor(Color.TRANSPARENT);
                surface.params=new WindowManager.LayoutParams(pixels(240),pixels(240),
                    Build.VERSION.SDK_INT >= 26 ? WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY : WindowManager.LayoutParams.TYPE_PHONE,
                    WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE,PixelFormat.TRANSLUCENT);
                surface.params.gravity=Gravity.TOP|Gravity.LEFT;
                surfaces.put(surface.id,surface);
                try {
                    engineConfigurer.configure(surface.engine);
                    surface.view.attachToFlutterEngine(surface.engine);
                    surface.engine.getDartExecutor().executeDartEntrypoint(
                        new DartExecutor.DartEntrypoint(loader.findAppBundlePath(),"main"),
                        java.util.Arrays.asList("floating_windows.window",call.argument("arguments")));
                    // Attach immediately so the first Flutter frame has real
                    // viewport metrics. Dart reveals the prepared surface.
                    surface.params.alpha=0;
                    surface.manager.addView(surface.view,surface.params); surface.attached=true;
                    surface.engine.getLifecycleChannel().appIsResumed();
                    result.success(surface.id);
                } catch (Exception error) { destroy(surface); throw error; }
                return;
            }
            case "list": result.success(new ArrayList<>(surfaces.keySet())); return;
            case "destroy": {
                Surface target=surfaces.get(call.<String>argument("id"));
                if (target != null && target.owner != messenger) throw new SecurityException("Surface owner mismatch");
                result.success(null); if (target != null) handler.post(() -> destroy(target)); return;
            }
            case "invoke": {
                Surface target=surfaces.get(call.<String>argument("id"));
                if (target == null || target.owner != messenger) throw new IllegalArgumentException("Unknown owned surface");
                new MethodChannel(target.engine.getDartExecutor().getBinaryMessenger(),"floating_windows/window")
                    .invokeMethod(call.argument("method"),call.argument("arguments"),result); return;
            }
            default: {
                Surface surface=current();
                if (call.method.equals("configure")) {
                    if (!"floating_windows.window".equals(call.argument("role"))) throw new IllegalArgumentException("Explicit floating window role required");
                    apply(surface,(Map<?,?>)call.arguments); surface.configured=true; result.success(null); return;
                }
                if (!surface.configured) throw new IllegalStateException("Configure the floating window first");
                switch (call.method) {
                case "getBounds": {
                    float density = context.getResources().getDisplayMetrics().density;
                    Map<String,Object> bounds = new HashMap<>();
                    bounds.put("x",surface.params.x / density); bounds.put("y",surface.params.y / density);
                    bounds.put("width",surface.params.width / density); bounds.put("height",surface.params.height / density);
                    result.success(bounds); return;
                }
                case "update": apply(surface,(Map<?,?>)call.arguments); result.success(null); return;
                case "show": show(surface); result.success(null); return;
                case "hide":
                    if (surface.attached) { surface.manager.removeViewImmediate(surface.view); surface.attached=false; }
                    result.success(null); return;
                case "close": result.success(null); handler.post(() -> destroy(surface)); return;
                default: result.notImplemented();
                }
            }
            }
        } catch (Exception error) { result.error("FLOATING_WINDOW_ERROR",error.getMessage(),null); }
    }
}
