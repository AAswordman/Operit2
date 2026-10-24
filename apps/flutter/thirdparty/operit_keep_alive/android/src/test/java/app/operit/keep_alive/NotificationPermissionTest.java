package app.operit.keep_alive;

import android.Manifest;
import android.app.Activity;
import android.app.Notification;
import android.app.NotificationManager;
import android.app.Service;
import android.content.pm.PackageManager;
import android.os.Build;
import org.junit.After;
import org.junit.Before;
import org.junit.Test;
import org.junit.runner.RunWith;
import org.robolectric.RobolectricTestRunner;
import org.robolectric.RuntimeEnvironment;
import org.robolectric.annotation.Config;
import org.robolectric.util.ReflectionHelpers;
import java.util.HashMap;
import java.util.Map;
import io.flutter.embedding.engine.plugins.FlutterPlugin.FlutterPluginBinding;
import io.flutter.embedding.engine.plugins.activity.ActivityPluginBinding;
import io.flutter.plugin.common.BinaryMessenger;
import io.flutter.plugin.common.MethodCall;
import io.flutter.plugin.common.MethodChannel;
import static org.junit.Assert.*;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

/** Exercises actual plugin initialization and Android activity permission handoffs. */
@RunWith(RobolectricTestRunner.class)
@Config(sdk = 28, manifest = Config.NONE)
public class NotificationPermissionTest {
    private OperitKeepAlivePlugin plugin;
    private FlutterPluginBinding engine;
    private ActivityPluginBinding binding;
    private Activity activity;
    private MethodChannel.Result result;

    /** Attaches the real plugin to a messenger and a controllable Android activity. */
    @Before
    public void setUp() {
        plugin = new OperitKeepAlivePlugin();
        engine = mock(FlutterPluginBinding.class);
        when(engine.getApplicationContext()).thenReturn(RuntimeEnvironment.getApplication());
        when(engine.getBinaryMessenger()).thenReturn(mock(BinaryMessenger.class));
        plugin.onAttachedToEngine(engine);
        binding = mock(ActivityPluginBinding.class);
        activity = mock(Activity.class);
        when(binding.getActivity()).thenReturn(activity);
        when(activity.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS))
            .thenReturn(PackageManager.PERMISSION_DENIED);
        result = mock(MethodChannel.Result.class);
    }

    /** Releases process observers and restores the Robolectric runtime SDK after each test. */
    @After
    public void tearDown() {
        ReflectionHelpers.setStaticField(Build.VERSION.class, "SDK_INT", 28);
        ReflectionHelpers.setStaticField(KeepAliveServiceHost.class, "workPresenter", null);
        plugin.onDetachedFromEngine(engine);
    }

    /** Initializes residency through the same method channel request used by Flutter. */
    private void initialize() {
        Map<String, Object> options = new HashMap<>();
        options.put("androidServiceClassName", Service.class.getName());
        plugin.onMethodCall(new MethodCall("initialize", options), result);
        verify(result, never()).error(anyString(), anyString(), any());
    }

    /** Reproduces the denied Android 14 permission and requires a system authorization request. */
    @Test
    public void deniedPermissionIsRequestedAtInitialization() {
        ReflectionHelpers.setStaticField(Build.VERSION.class, "SDK_INT", 34);
        plugin.onAttachedToActivity(binding);
        verify(activity, never()).requestPermissions(any(), anyInt());
        initialize();
        verify(activity).requestPermissions(eq(new String[] {Manifest.permission.POST_NOTIFICATIONS}), anyInt());
    }

    /** Defers authorization until a real activity attaches without blocking Core initialization. */
    @Test
    public void initializationBeforeActivityRequestsWhenAttached() {
        ReflectionHelpers.setStaticField(Build.VERSION.class, "SDK_INT", 33);
        initialize();
        plugin.onAttachedToActivity(binding);
        verify(activity).requestPermissions(eq(new String[] {Manifest.permission.POST_NOTIFICATIONS}), anyInt());
    }

    /** Preserves existing grants without asking the user again. */
    @Test
    public void grantedPermissionDoesNotPrompt() {
        ReflectionHelpers.setStaticField(Build.VERSION.class, "SDK_INT", 34);
        when(activity.checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS))
            .thenReturn(PackageManager.PERMISSION_GRANTED);
        plugin.onAttachedToActivity(binding);
        initialize();
        verify(activity, never()).requestPermissions(any(), anyInt());
    }

    /** Keeps pre-Android-13 devices on their existing install-time authorization model. */
    @Test
    public void olderAndroidDoesNotRequestRuntimeNotificationPermission() {
        plugin.onAttachedToActivity(binding);
        initialize();
        verify(activity, never()).requestPermissions(any(), anyInt());
    }

    /** Retains one request across configuration changes and releases old activity listeners. */
    @Test
    public void rotationDoesNotRepeatThePermissionPrompt() {
        ReflectionHelpers.setStaticField(Build.VERSION.class, "SDK_INT", 34);
        plugin.onAttachedToActivity(binding);
        initialize();
        plugin.onDetachedFromActivityForConfigChanges();
        verify(binding).removeRequestPermissionsResultListener(plugin);
        plugin.onReattachedToActivityForConfigChanges(binding);
        verify(activity, times(1)).requestPermissions(any(), anyInt());
        verify(binding, times(2)).addRequestPermissionsResultListener(plugin);
    }

    /** Republishes the established foreground notification immediately after authorization. */
    @Test
    public void grantedPermissionRepostsTheExistingForegroundNotification() {
        ReflectionHelpers.setStaticField(Build.VERSION.class, "SDK_INT", 34);
        plugin.onAttachedToActivity(binding);
        initialize();
        Service service = mock(Service.class);
        NotificationManager manager = mock(NotificationManager.class);
        when(service.getSystemService(NotificationManager.class)).thenReturn(manager);
        Notification idle = new Notification();
        WorkNotificationPresenter presenter = new WorkNotificationPresenter(service, "core", 2401, 1, idle);
        ReflectionHelpers.setStaticField(KeepAliveServiceHost.class, "workPresenter", presenter);
        org.mockito.ArgumentCaptor<Integer> code = org.mockito.ArgumentCaptor.forClass(Integer.class);
        verify(activity).requestPermissions(any(), code.capture());
        plugin.onRequestPermissionsResult(code.getValue(),
            new String[] {Manifest.permission.POST_NOTIFICATIONS}, new int[] {PackageManager.PERMISSION_GRANTED});
        verify(manager).notify(2401, idle);
    }

    /** Reposts the same Core revision when returning from notification settings. */
    @Test
    public void capabilityRefreshRepostsWithoutRequiringANewWorkRevision() throws Exception {
        plugin.onAttachedToActivity(binding);
        initialize();
        Service service = mock(Service.class);
        NotificationManager manager = mock(NotificationManager.class);
        when(service.getSystemService(NotificationManager.class)).thenReturn(manager);
        Notification idle = new Notification();
        WorkNotificationPresenter presenter = new WorkNotificationPresenter(service, "core", 2401, 1, idle);
        presenter.update("{\"schemaVersion\":1,\"revision\":7,\"works\":[]}");
        clearInvocations(manager);
        ReflectionHelpers.setStaticField(KeepAliveServiceHost.class, "workPresenter", presenter);
        MethodChannel.Result capabilities = mock(MethodChannel.Result.class);
        plugin.onMethodCall(new MethodCall("workCapabilities", new HashMap<>()), capabilities);
        verify(manager).notify(2401, idle);
        verify(capabilities).success(anyMap());
        verify(capabilities, never()).error(anyString(), anyString(), any());
    }

    /** Handles the matching grant callback and leaves unrelated permission requests to their owners. */
    @Test
    public void permissionCallbackRecognizesOnlyItsOwnRequest() {
        ReflectionHelpers.setStaticField(Build.VERSION.class, "SDK_INT", 34);
        plugin.onAttachedToActivity(binding);
        initialize();
        org.mockito.ArgumentCaptor<Integer> code = org.mockito.ArgumentCaptor.forClass(Integer.class);
        verify(activity).requestPermissions(any(), code.capture());
        assertFalse(plugin.onRequestPermissionsResult(code.getValue() + 1, new String[0], new int[0]));
        assertTrue(plugin.onRequestPermissionsResult(code.getValue(),
            new String[] {Manifest.permission.POST_NOTIFICATIONS}, new int[] {PackageManager.PERMISSION_GRANTED}));
        assertTrue(plugin.onRequestPermissionsResult(code.getValue(),
            new String[] {Manifest.permission.POST_NOTIFICATIONS}, new int[] {PackageManager.PERMISSION_DENIED}));
        verify(activity, times(1)).requestPermissions(any(), anyInt());
    }
}
