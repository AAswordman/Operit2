package app.operit.uiserver;

import android.app.Application;
import android.app.Instrumentation;
import android.content.AttributionSource;
import android.content.Context;
import android.content.ContextWrapper;
import android.content.pm.ApplicationInfo;
import android.graphics.Insets;
import android.graphics.Rect;
import android.hardware.display.DisplayManager;
import android.os.Build;
import android.os.Bundle;
import android.os.IBinder;
import android.os.UserHandle;
import android.content.Intent;
import android.view.Display;
import android.view.DisplayCutout;
import android.view.View;
import android.view.WindowInsets;
import android.view.WindowManager;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.util.HashMap;
import java.util.Map;

/** Initializes the Android framework for an uninstalled app_process payload. */
final class ShellContext extends ContextWrapper {
    private final Map<String, Object> services = new HashMap<>();
    private final ApplicationInfo applicationInfo = new ApplicationInfo();
    private final int userId;
    private final Context windowContext;

    /** Wraps the framework's primary display with the shell attribution identity. */
    private ShellContext(Context base, int ownerUid, String ownerPackage) throws Exception {
        super(base);
        userId = (Integer) UserHandle.class.getMethod("getIdentifier").invoke(UserHandle.getUserHandleForUid(ownerUid));
        applicationInfo.packageName = "com.android.shell";
        applicationInfo.uid = 2000;
        applicationInfo.targetSdkVersion = Build.VERSION.SDK_INT;
        ApplicationInfo ownerInfo = base.getPackageManager().getApplicationInfo(ownerPackage, 0);
        if (ownerInfo.uid != ownerUid) throw new SecurityException("Window owner package UID mismatch");
        windowContext = new ContextWrapper(this) {
            private Object windowManager;
            /** Attributes the delegated window session to its registered Android owner. */
            @Override public String getPackageName() { return ownerPackage; }
            /** Uses the registered owner's window app-op attribution. */
            @Override public String getOpPackageName() { return ownerPackage; }
            /** Supplies metadata for the application that owns the window session. */
            @Override public ApplicationInfo getApplicationInfo() { return ownerInfo; }
            /** Keeps view-scoped lookups attached to the delegated owner context. */
            @Override public Context getApplicationContext() { return this; }
            /** Creates the native window manager with the registered owner's package. */
            @Override public Object getSystemService(String name) {
                if ("autofill".equals(name) || "content_capture".equals(name)) return null;
                if (!WINDOW_SERVICE.equals(name)) return super.getSystemService(name);
                if (windowManager == null) windowManager = createManager("android.view.WindowManagerImpl", this);
                return windowManager;
            }
        };
    }

    /** Builds ActivityThread, application and configuration state before creating Views. */
    static ShellContext create(int ownerUid, String ownerPackage) throws Exception {
        if (Build.VERSION.SDK_INT >= 35) {
            Class<?> fonts = Class.forName("android.graphics.fonts.SystemFonts");
            Class<?> configType = Class.forName("android.text.FontConfig");
            Object config = fonts.getMethod("getSystemPreinstalledFontConfigFromLegacyXml").invoke(null);
            Object families = fonts.getMethod("buildSystemFallback", configType).invoke(null, config);
            Object typefaces = fonts.getMethod("buildSystemTypefaces", configType, Map.class).invoke(null, config, families);
            android.graphics.Typeface.class.getMethod("setSystemFontMap", Map.class).invoke(null, typefaces);
        } else if (Build.VERSION.SDK_INT >= 31) {
            android.graphics.Typeface.class.getMethod("loadPreinstalledSystemFontMap").invoke(null);
        }
        Class<?> threadClass = Class.forName("android.app.ActivityThread");
        Constructor<?> constructor = threadClass.getDeclaredConstructor();
        constructor.setAccessible(true);
        Object thread = constructor.newInstance();
        set(threadClass, null, "sCurrentActivityThread", thread);
        set(threadClass, thread, "mSystemThread", true);
        if (Build.VERSION.SDK_INT >= 31) {
            Class<?> internal = Class.forName("android.app.ActivityThreadInternal");
            Constructor<?> controller = Class.forName("android.app.ConfigurationController").getDeclaredConstructor(internal);
            controller.setAccessible(true);
            set(threadClass, thread, "mConfigurationController", controller.newInstance(thread));
        }
        Class<?> bindClass = Class.forName("android.app.ActivityThread$AppBindData");
        Constructor<?> bindConstructor = bindClass.getDeclaredConstructor();
        bindConstructor.setAccessible(true);
        Object bind = bindConstructor.newInstance();
        ApplicationInfo info = new ApplicationInfo();
        info.packageName = "com.android.shell";
        info.uid = 2000;
        info.targetSdkVersion = Build.VERSION.SDK_INT;
        set(bindClass, bind, "appInfo", info);
        set(threadClass, thread, "mBoundApplication", bind);
        Context system = (Context) threadClass.getMethod("getSystemContext").invoke(thread);
        Display primary = system.getSystemService(DisplayManager.class).getDisplay(Display.DEFAULT_DISPLAY);
        if (primary == null) throw new IllegalStateException("Primary Android display is unavailable");
        ShellContext context = new ShellContext(system.createDisplayContext(primary), ownerUid, ownerPackage);
        Application application = new Application();
        set(ContextWrapper.class, application, "mBase", context);
        set(threadClass, thread, "mInitialApplication", application);
        set(threadClass, thread, "mInstrumentation", new Instrumentation());
        return context;
    }

    /** Reuses the registered owner's window session while this process owns all Views and Surfaces. */
    void attachWindowSession(IBinder binder) throws Exception {
        if (binder == null || !binder.isBinderAlive()) throw new IllegalStateException("Window owner session is unavailable");
        Object session = Class.forName("android.view.IWindowSession$Stub")
                .getMethod("asInterface", IBinder.class).invoke(null, binder);
        set(Class.forName("android.view.WindowManagerGlobal"), null, "sWindowSession", session);
    }

    /** Supplies the registered window owner's context only for native Views. */
    Context windowContext() { return windowContext; }

    /** Lets the payload use physical coordinates without adding system movement animations. */
    void configureFloatingLayout(WindowManager.LayoutParams layout) throws ReflectiveOperationException {
        Class<?> type = WindowManager.LayoutParams.class;
        Field flags = type.getField("privateFlags");
        flags.setInt(layout, flags.getInt(layout) | type.getField("PRIVATE_FLAG_NO_MOVE_ANIMATION").getInt(null));
    }

    /** Reads full-display bar insets from the attached window's inset state, independent of its size. */
    Insets floatingInsets(View root, Rect displayFrame) throws ReflectiveOperationException {
        Class<?> stateType = Class.forName("android.view.InsetsState");
        Object controller = root.getWindowInsetsController();
        Object state = Class.forName("android.view.InsetsController").getMethod("getState").invoke(controller);
        Insets bars;
        if (Build.VERSION.SDK_INT >= 31) {
            bars = (Insets) stateType.getMethod("calculateInsets", Rect.class, int.class, boolean.class)
                    .invoke(state, displayFrame, WindowInsets.Type.systemBars(), true);
        } else {
            Method source = stateType.getMethod("getSource", int.class);
            Method calculate = Class.forName("android.view.InsetsSource").getMethod("calculateInsets", Rect.class, boolean.class);
            Insets status = (Insets) calculate.invoke(source.invoke(state, stateType.getField("ITYPE_STATUS_BAR").getInt(null)),
                    displayFrame, true);
            Insets navigation = (Insets) calculate.invoke(source.invoke(state, stateType.getField("ITYPE_NAVIGATION_BAR").getInt(null)),
                    displayFrame, true);
            bars = Insets.max(status, navigation);
        }
        DisplayCutout cutout = getDisplay().getCutout();
        return cutout == null ? bars : Insets.max(bars, Insets.of(cutout.getSafeInsetLeft(), cutout.getSafeInsetTop(),
                cutout.getSafeInsetRight(), cutout.getSafeInsetBottom()));
    }

    /** Constructs one framework manager with explicit attribution context. */
    private static Object createManager(String className, Context context) {
        try {
            Constructor<?> constructor = Class.forName(className).getDeclaredConstructor(Context.class);
            constructor.setAccessible(true);
            return constructor.newInstance(context);
        } catch (ReflectiveOperationException error) {
            throw new IllegalStateException("Cannot initialize shell service: " + className, error);
        }
    }

    /** Assigns one known framework field and reports incompatible framework versions. */
    private static void set(Class<?> type, Object target, String name, Object value) throws Exception {
        Field field = type.getDeclaredField(name);
        field.setAccessible(true);
        field.set(target, value);
    }

    /** Attributes privileged Android calls to the actual shell package. */
    @Override public String getPackageName() { return "com.android.shell"; }

    /** Uses the same shell attribution for app-op checks. */
    @Override public String getOpPackageName() { return getPackageName(); }

    /** Keeps application-scoped services inside the standalone process. */
    @Override public Context getApplicationContext() { return this; }

    /** Exposes shell application metadata to native framework Views. */
    @Override public ApplicationInfo getApplicationInfo() { return applicationInfo; }

    /** Sends the Binder handoff with a null application-thread token, as a shell process. */
    void handoff(Intent intent) throws Exception {
        Object manager = Class.forName("android.app.ActivityManager").getMethod("getService").invoke(null);
        Method method = Class.forName("android.app.IActivityManager").getMethod("broadcastIntent",
                Class.forName("android.app.IApplicationThread"), Intent.class, String.class,
                Class.forName("android.content.IIntentReceiver"), int.class, String.class, Bundle.class,
                String[].class, int.class, Bundle.class, boolean.class, boolean.class, int.class);
        int result = (Integer) method.invoke(manager, null, intent, null, null, 0, null, null,
                null, -1, null, false, false, userId);
        if (result != 0) throw new IllegalStateException("Android rejected UI server handoff: " + result);
    }

    /** Launches tasks through the same shell ActivityManager entry point used by Operit1. */
    void launch(Intent intent, Bundle options) throws Exception {
        Object manager = Class.forName("android.app.ActivityManager").getMethod("getService").invoke(null);
        Method method = Class.forName("android.app.IActivityManager").getMethod("startActivityAsUser",
                Class.forName("android.app.IApplicationThread"), String.class, Intent.class, String.class,
                IBinder.class, String.class, int.class, int.class, Class.forName("android.app.ProfilerInfo"),
                Bundle.class, int.class);
        int result = (Integer) method.invoke(manager, null, getPackageName(), intent, null, null, null,
                0, 0, null, options, userId);
        if (result < 0) throw new IllegalStateException("Android rejected application launch: " + result);
    }

    /** Supplies explicit shell UID attribution on Android 12 and newer. */
    @Override public AttributionSource getAttributionSource() {
        return new AttributionSource.Builder(2000).setPackageName(getPackageName()).build();
    }

    /** Creates window and display managers with this context instead of system UID attribution. */
    @Override public Object getSystemService(String name) {
        if (!WINDOW_SERVICE.equals(name) && !DISPLAY_SERVICE.equals(name)) return super.getSystemService(name);
        synchronized (services) {
            if (services.containsKey(name)) return services.get(name);
            Object service = createManager(WINDOW_SERVICE.equals(name)
                    ? "android.view.WindowManagerImpl" : "android.hardware.display.DisplayManager", this);
            services.put(name, service);
            return service;
        }
    }
}
