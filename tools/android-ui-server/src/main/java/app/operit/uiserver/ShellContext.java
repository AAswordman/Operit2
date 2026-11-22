package app.operit.uiserver;

import android.app.Application;
import android.app.Instrumentation;
import android.content.AttributionSource;
import android.content.Context;
import android.content.ContextWrapper;
import android.content.pm.ApplicationInfo;
import android.content.res.Configuration;
import android.graphics.Insets;
import android.graphics.Rect;
import android.hardware.display.DisplayManager;
import android.os.Build;
import android.os.Bundle;
import android.os.IBinder;
import android.os.UserHandle;
import android.content.Intent;
import android.util.DisplayMetrics;
import android.view.Display;
import android.view.InputChannel;
import android.view.WindowInsets;
import android.view.WindowManager;
import java.lang.reflect.Constructor;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.lang.reflect.InvocationTargetException;
import java.lang.reflect.Proxy;
import java.util.HashMap;
import java.util.Map;

/** Initializes the Android framework for an uninstalled app_process payload. */
final class ShellContext extends ContextWrapper {
    static final String PROCESS_NAME = "com.android.shell:operit-ui";
    private final Map<String, Object> services = new HashMap<>();
    private InputChannel pendingInputChannel;
    private final ApplicationInfo applicationInfo = new ApplicationInfo();
    private final int userId;
    private final Context windowContext;

    /** Wraps the framework's primary display with the shell attribution identity. */
    private ShellContext(Context base, int ownerUid, String ownerPackage) throws Exception {
        super(base);
        userId = (Integer) UserHandle.class.getMethod("getIdentifier").invoke(UserHandle.getUserHandleForUid(ownerUid));
        applicationInfo.packageName = "com.android.shell";
        applicationInfo.processName = PROCESS_NAME;
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
        Method libraries = Class.forName("com.android.internal.os.ZygoteInit").getDeclaredMethod("preloadSharedLibraries");
        libraries.setAccessible(true);
        libraries.invoke(null);
        initializeWindowManager();
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
        info.processName = PROCESS_NAME;
        info.uid = 2000;
        info.targetSdkVersion = Build.VERSION.SDK_INT;
        set(bindClass, bind, "appInfo", info);
        set(bindClass, bind, "processName", PROCESS_NAME);
        set(threadClass, thread, "mBoundApplication", bind);
        Context system = (Context) threadClass.getMethod("getSystemContext").invoke(thread);
        Display primary = system.getSystemService(DisplayManager.class).getDisplay(Display.DEFAULT_DISPLAY);
        if (primary == null) throw new IllegalStateException("Primary Android display is unavailable");
        Context shell = (Context) Context.class.getMethod("createPackageContextAsUser", String.class, int.class, UserHandle.class)
                .invoke(system, "com.android.shell", 0, UserHandle.getUserHandleForUid(ownerUid));
        Context displayContext = shell.createDisplayContext(primary);
        DisplayMetrics metrics = new DisplayMetrics();
        primary.getRealMetrics(metrics);
        Rect bounds = new Rect(0, 0, metrics.widthPixels, metrics.heightPixels);
        Configuration configuration = new Configuration(displayContext.getResources().getConfiguration());
        Object window = Configuration.class.getField("windowConfiguration").get(configuration);
        window.getClass().getMethod("setBounds", Rect.class).invoke(window, bounds);
        window.getClass().getMethod("setMaxBounds", Rect.class).invoke(window, bounds);
        ShellContext context = new ShellContext(displayContext.createConfigurationContext(configuration), ownerUid, ownerPackage);
        Application application = new Application();
        set(ContextWrapper.class, application, "mBase", context);
        set(threadClass, thread, "mInitialApplication", application);
        set(threadClass, thread, "mInstrumentation", new Instrumentation());
        return context;
    }

    /** Binds the actual window service without relying on zygote application shared memory. */
    private static void initializeWindowManager() throws Exception {
        IBinder binder = (IBinder) Class.forName("android.os.ServiceManager")
                .getMethod("getService", String.class).invoke(null, "window");
        if (binder == null) throw new IllegalStateException("Android window service is unavailable");
        Object manager = Class.forName("android.view.IWindowManager$Stub")
                .getMethod("asInterface", IBinder.class).invoke(null, binder);
        float scale = (Float) Class.forName("android.view.IWindowManager").getMethod("getCurrentAnimatorScale").invoke(manager);
        android.animation.ValueAnimator.class.getMethod("setDurationScale", float.class).invoke(null, scale);
        set(Class.forName("android.view.WindowManagerGlobal"), null, "sWindowManagerService", manager);
    }

    /** Reuses the registered owner's window session while this process owns all Views and Surfaces. */
    void attachWindowSession(IBinder binder) throws Exception {
        if (binder == null || !binder.isBinderAlive()) throw new IllegalStateException("Window owner session is unavailable");
        Class<?> sessionType = Class.forName("android.view.IWindowSession");
        Object session = Class.forName("android.view.IWindowSession$Stub")
                .getMethod("asInterface", IBinder.class).invoke(null, binder);
        Object standalone = Proxy.newProxyInstance(sessionType.getClassLoader(), new Class<?>[]{sessionType},
                (proxy, method, arguments) -> windowCall(session, method, arguments));
        set(Class.forName("android.view.WindowManagerGlobal"), null, "sWindowSession", standalone);
    }

    /** Owns native input channels while ViewRoot manages drawing and window layout only. */
    private Object windowCall(Object session, Method method, Object[] arguments) throws Throwable {
        InputChannel channel = null;
        boolean adding = method.getName().equals("addToDisplay") || method.getName().equals("addToDisplayAsUser");
        Class<?>[] parameters = method.getParameterTypes();
        for (int index = 0; index < parameters.length; index++) {
            if (arguments[index] instanceof WindowManager.LayoutParams original) {
                WindowManager.LayoutParams layout = new WindowManager.LayoutParams();
                layout.copyFrom(original);
                configureInput(layout, false);
                arguments[index] = layout;
            }
            if (adding && parameters[index] == InputChannel.class) {
                if (pendingInputChannel != null) throw new IllegalStateException("Previous window input channel was not attached");
                channel = new InputChannel();
                arguments[index] = channel;
            }
        }
        try {
            Object result = method.invoke(session, arguments);
            if (channel != null) {
                if ((Integer) result < 0) channel.dispose();
                else pendingInputChannel = channel;
            }
            return result;
        } catch (InvocationTargetException error) {
            if (channel != null) channel.dispose();
            throw error.getCause();
        }
    }

    /** Transfers the channel created for this exact native window to its standalone receiver. */
    InputChannel takeInputChannel() {
        InputChannel channel = pendingInputChannel;
        pendingInputChannel = null;
        if (channel == null) throw new IllegalStateException("Android window input channel is unavailable");
        return channel;
    }

    /** Separates ViewRoot rendering from the server-owned native input receiver. */
    private static void configureInput(WindowManager.LayoutParams layout, boolean standalone) throws ReflectiveOperationException {
        Class<?> type = WindowManager.LayoutParams.class;
        Field features = type.getField("inputFeatures");
        int flag = type.getField("INPUT_FEATURE_NO_INPUT_CHANNEL").getInt(null);
        features.setInt(layout, standalone ? features.getInt(layout) | flag : features.getInt(layout) & ~flag);
    }

    /** Supplies the registered window owner's context only for native Views. */
    Context windowContext() { return windowContext; }

    /** Lets the payload use physical coordinates without adding system movement animations. */
    void configureFloatingLayout(WindowManager.LayoutParams layout) throws ReflectiveOperationException {
        Class<?> type = WindowManager.LayoutParams.class;
        Field flags = type.getField("privateFlags");
        flags.setInt(layout, flags.getInt(layout) | type.getField("PRIVATE_FLAG_NO_MOVE_ANIMATION").getInt(null));
        configureInput(layout, true);
    }

    /** Reads full-display system bars and cutouts through the public metrics API. */
    Insets floatingInsets() {
        return windowContext.getSystemService(WindowManager.class).getMaximumWindowMetrics()
                .getWindowInsets().getInsetsIgnoringVisibility(WindowInsets.Type.systemBars() | WindowInsets.Type.displayCutout());
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
    private static void set(Class<?> type, Object target, String name, Object value) throws ReflectiveOperationException {
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
