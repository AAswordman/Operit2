package dev.floating_windows;

import android.view.WindowManager;
import android.os.Looper;
import io.flutter.embedding.android.FlutterView;
import io.flutter.embedding.engine.FlutterEngine;
import io.flutter.embedding.engine.dart.DartExecutor;
import io.flutter.embedding.engine.systemchannels.LifecycleChannel;
import io.flutter.embedding.engine.plugins.FlutterPlugin.FlutterPluginBinding;
import io.flutter.embedding.engine.plugins.PluginRegistry;
import io.flutter.plugin.common.BinaryMessenger;
import io.flutter.plugin.common.MethodCall;
import io.flutter.plugin.common.MethodChannel;
import java.util.HashMap;
import java.util.Map;
import org.junit.After;
import org.junit.Before;
import org.junit.Test;
import org.junit.runner.RunWith;
import org.robolectric.RobolectricTestRunner;
import org.robolectric.RuntimeEnvironment;
import org.robolectric.annotation.Config;
import org.robolectric.shadows.ShadowSettings;
import org.robolectric.util.ReflectionHelpers;
import org.mockito.InOrder;

import static org.junit.Assert.*;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;
import static org.robolectric.Shadows.shadowOf;

/** Exercises native surface ownership with distinct executor and messenger identities. */
@RunWith(RobolectricTestRunner.class)
@Config(sdk = 28, manifest = Config.NONE)
public class FloatingWindowsPluginTest {
    private FloatingWindowsPlugin owner;
    private FloatingWindowsPlugin child;
    private FlutterEngine engine;
    private FlutterView view;
    private WindowManager manager;
    private LifecycleChannel lifecycle;
    private Object surface;
    private Map<String, Object> surfaces;

    /** Registers a child engine whose plugin binding exposes its executor as the messenger. */
    @Before
    public void setUp() throws Exception {
        surfaces = ReflectionHelpers.getStaticField(FloatingWindowsPlugin.class, "surfaces");
        surfaces.clear();
        owner = attach(mock(BinaryMessenger.class));
        DartExecutor executor = mock(DartExecutor.class);
        BinaryMessenger delegate = mock(BinaryMessenger.class);
        when(executor.getBinaryMessenger()).thenReturn(delegate);
        assertNotSame(executor, delegate);
        child = attach(executor);
        engine = mock(FlutterEngine.class);
        view = mock(FlutterView.class);
        manager = mock(WindowManager.class);
        lifecycle = mock(LifecycleChannel.class);
        when(engine.getLifecycleChannel()).thenReturn(lifecycle);
        PluginRegistry registry = mock(PluginRegistry.class);
        when(engine.getDartExecutor()).thenReturn(executor);
        when(engine.getPlugins()).thenReturn(registry);
        when(registry.get(FloatingWindowsPlugin.class)).thenReturn(child);
        Class<?> surfaceType = Class.forName("dev.floating_windows.FloatingWindowsPlugin$Surface");
        surface = ReflectionHelpers.callConstructor(surfaceType);
        ReflectionHelpers.setField(surface, "id", "pet");
        ReflectionHelpers.setField(surface, "engine", engine);
        ReflectionHelpers.setField(surface, "view", view);
        ReflectionHelpers.setField(surface, "manager", manager);
        ReflectionHelpers.setField(surface, "owner", ReflectionHelpers.getField(owner, "messenger"));
        ReflectionHelpers.setField(surface, "params", new WindowManager.LayoutParams(200, 280));
        surfaces.put("pet", surface);
    }

    /** Clears the process-level surface registry without constructing native Flutter engines. */
    @After
    public void tearDown() {
        surfaces.clear();
    }

    /** Attaches the real plugin handler to an Android context and a supplied messenger. */
    private FloatingWindowsPlugin attach(BinaryMessenger messenger) {
        FloatingWindowsPlugin plugin = new FloatingWindowsPlugin();
        FlutterPluginBinding binding = mock(FlutterPluginBinding.class);
        when(binding.getApplicationContext()).thenReturn(RuntimeEnvironment.getApplication());
        when(binding.getBinaryMessenger()).thenReturn(messenger);
        plugin.onAttachedToEngine(binding);
        return plugin;
    }

    /** Configures and reads the child's own surface despite different messenger wrappers. */
    @Test
    public void childConfiguresAndReadsItsSurface() {
        Map<String, Object> options = new HashMap<>();
        options.put("role", "floating_windows.window");
        options.put("width", 160);
        options.put("height", 240);
        MethodChannel.Result configured = mock(MethodChannel.Result.class);
        child.onMethodCall(new MethodCall("configure", options), configured);
        verify(configured).success(null);
        verify(configured, never()).error(anyString(), any(), any());

        MethodChannel.Result bounds = mock(MethodChannel.Result.class);
        child.onMethodCall(new MethodCall("getBounds", null), bounds);
        Map<String, Object> expected = new HashMap<>();
        expected.put("x", 0.0f);
        expected.put("y", 0.0f);
        expected.put("width", 160.0f);
        expected.put("height", 240.0f);
        verify(bounds).success(expected);
        verify(bounds, never()).error(anyString(), any(), any());
    }

    /** Prevents the owning main engine and unrelated engines from configuring a child's surface. */
    @Test
    public void otherEnginesCannotControlCurrentSurface() {
        for (FloatingWindowsPlugin plugin : new FloatingWindowsPlugin[] {owner, attach(mock(BinaryMessenger.class))}) {
            MethodChannel.Result result = mock(MethodChannel.Result.class);
            plugin.onMethodCall(new MethodCall("configure", Map.of("role", "floating_windows.window")), result);
            verify(result).error("FLOATING_WINDOW_ERROR", "Only an overlay engine can control its current surface", null);
            verify(result, never()).success(any());
        }
    }

    /** Keeps owner-only invocation and destruction inaccessible to the child or unrelated engines. */
    @Test
    public void foreignEnginesCannotInvokeOrDestroySurface() {
        for (FloatingWindowsPlugin plugin : new FloatingWindowsPlugin[] {child, attach(mock(BinaryMessenger.class))}) {
            for (String method : new String[] {"invoke", "destroy"}) {
                MethodChannel.Result result = mock(MethodChannel.Result.class);
                plugin.onMethodCall(new MethodCall(method, Map.of("id", "pet", "method", "ready")), result);
                verify(result).error(eq("FLOATING_WINDOW_ERROR"), anyString(), isNull());
                verify(result, never()).success(any());
                assertTrue(surfaces.containsKey("pet"));
            }
        }
    }

    /** Prevents semantics updates from targeting a parentless overlay after hiding it. */
    @Test
    public void hideDisconnectsEngineBeforeRemovingWindow() {
        ReflectionHelpers.setField(surface, "configured", true);
        ReflectionHelpers.setField(surface, "attached", true);
        MethodChannel.Result result = mock(MethodChannel.Result.class);

        child.onMethodCall(new MethodCall("hide", null), result);

        InOrder order = inOrder(view, manager);
        order.verify(view).detachFromFlutterEngine();
        order.verify(manager).removeViewImmediate(view);
        verify(result).success(null);
        verify(result, never()).error(anyString(), any(), any());
        assertFalse(ReflectionHelpers.<Boolean>getField(surface, "attached"));
        assertTrue(surfaces.containsKey("pet"));
        verify(engine, never()).destroy();
    }

    /** Restores rendering and accessibility only after a hidden view regains its window parent. */
    @Test
    public void showReattachesEngineAfterAddingWindowAndKeepsRepeatedShowAttached() {
        ShadowSettings.setCanDrawOverlays(true);
        ReflectionHelpers.setField(surface, "configured", true);
        MethodChannel.Result shown = mock(MethodChannel.Result.class);

        child.onMethodCall(new MethodCall("show", null), shown);

        InOrder order = inOrder(manager, view, lifecycle);
        order.verify(manager).addView(eq(view), any(WindowManager.LayoutParams.class));
        order.verify(view).attachToFlutterEngine(engine);
        order.verify(lifecycle).appIsResumed();
        verify(shown).success(null);
        verify(shown, never()).error(anyString(), any(), any());
        assertTrue(ReflectionHelpers.<Boolean>getField(surface, "attached"));

        child.onMethodCall(new MethodCall("show", null), mock(MethodChannel.Result.class));
        verify(view, times(1)).attachToFlutterEngine(engine);
        verify(manager, times(1)).addView(eq(view), any(WindowManager.LayoutParams.class));
        verify(manager).updateViewLayout(eq(view), any(WindowManager.LayoutParams.class));
    }

    /** Keeps the accessibility delegate disconnected throughout window and engine destruction. */
    @Test
    public void destroyDisconnectsEngineBeforeRemovingWindowAndDestroyingEngine() {
        ReflectionHelpers.setField(surface, "attached", true);
        MethodChannel.Result result = mock(MethodChannel.Result.class);

        owner.onMethodCall(new MethodCall("destroy", Map.of("id", "pet")), result);
        shadowOf(Looper.getMainLooper()).idle();

        InOrder order = inOrder(view, manager, engine);
        order.verify(view).detachFromFlutterEngine();
        order.verify(manager).removeViewImmediate(view);
        order.verify(engine).destroy();
        verify(result).success(null);
        assertFalse(surfaces.containsKey("pet"));
    }
}
