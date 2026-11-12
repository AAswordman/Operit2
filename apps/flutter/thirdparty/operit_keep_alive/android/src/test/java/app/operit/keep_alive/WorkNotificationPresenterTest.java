package app.operit.keep_alive;

import android.app.Activity;
import android.app.Notification;
import android.app.NotificationManager;
import android.app.Service;
import android.content.Intent;
import android.content.pm.PackageManager;
import android.os.IBinder;
import android.os.Looper;
import java.time.Duration;
import org.json.JSONArray;
import org.json.JSONObject;
import org.junit.After;
import org.junit.Before;
import org.junit.Test;
import org.junit.runner.RunWith;
import org.robolectric.Robolectric;
import org.robolectric.RobolectricTestRunner;
import org.robolectric.RuntimeEnvironment;
import org.robolectric.annotation.Config;
import org.robolectric.util.ReflectionHelpers;
import io.flutter.embedding.engine.plugins.FlutterPlugin.FlutterPluginBinding;
import io.flutter.embedding.engine.plugins.activity.ActivityPluginBinding;
import io.flutter.plugin.common.BinaryMessenger;
import static org.junit.Assert.*;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;
import static org.robolectric.Shadows.shadowOf;

/** Exercises notification consumption through the same activation path used by Android clicks. */
@RunWith(RobolectricTestRunner.class)
@Config(sdk = 28, manifest = Config.NONE)
public class WorkNotificationPresenterTest {
    private static final int NOTIFICATION_ID = 2401;
    private Service service;
    private NotificationManager manager;
    private Notification idle;
    private WorkNotificationPresenter presenter;
    private OperitKeepAlivePlugin plugin;
    private FlutterPluginBinding engine;

    /** Provides a concrete context without starting the application's Core runtime. */
    public static class TestService extends Service {
        /** Keeps the test service independent of binder clients. */
        @Override
        public IBinder onBind(Intent intent) { return null; }
    }

    /** Connects the real presenter and plugin to controllable Android notification delivery. */
    @Before
    public void setUp() {
        service = spy(Robolectric.buildService(TestService.class).create().get());
        manager = mock(NotificationManager.class);
        when(service.getSystemService(NotificationManager.class)).thenReturn(manager);
        PackageManager packages = mock(PackageManager.class);
        when(packages.getLaunchIntentForPackage(service.getPackageName()))
            .thenAnswer(invocation -> new Intent(service, Activity.class));
        when(service.getPackageManager()).thenReturn(packages);
        idle = new Notification();
        presenter = new WorkNotificationPresenter(service, "core", NOTIFICATION_ID, 1, idle);
        ReflectionHelpers.setStaticField(KeepAliveServiceHost.class, "workPresenter", presenter);
        plugin = new OperitKeepAlivePlugin();
        engine = mock(FlutterPluginBinding.class);
        when(engine.getApplicationContext()).thenReturn(RuntimeEnvironment.getApplication());
        when(engine.getBinaryMessenger()).thenReturn(mock(BinaryMessenger.class));
        plugin.onAttachedToEngine(engine);
    }

    /** Cancels expiry callbacks and releases plugin observers between tests. */
    @After
    public void tearDown() {
        presenter.close();
        ReflectionHelpers.setStaticField(KeepAliveServiceHost.class, "workPresenter", null);
        plugin.onDetachedFromEngine(engine);
    }

    /** Constructs protocol work with a real activation and explicit terminal status. */
    private JSONObject work(String id, String status) throws Exception {
        boolean terminal = !status.equals("running") && !status.equals("waiting");
        long now = System.currentTimeMillis();
        return new JSONObject().put("workId", id).put("title", id).put("status", status)
            .put("phase", "receiving").put("detail", "").put("progress", JSONObject.NULL)
            .put("startedAtMillis", now - 1000)
            .put("endedAtMillis", terminal ? now : JSONObject.NULL).put("canStop", !terminal)
            .put("activation", new JSONObject().put("type", "open_chat").put("chatId", id));
    }

    /** Publishes a complete revision through the real presenter. */
    private void update(long revision, JSONObject... works) throws Exception {
        JSONArray items = new JSONArray();
        for (JSONObject work : works) items.put(work);
        presenter.update(new JSONObject().put("schemaVersion", 1).put("revision", revision)
            .put("works", items).toString());
    }

    /** Creates the shared content and action-button activation payload. */
    private Intent activation(String id) {
        return new Intent().putExtra(WorkNotificationPresenter.WORK_ID, id)
            .putExtra(WorkNotificationPresenter.ACTIVATION,
                "{\"type\":\"open_chat\",\"chatId\":\"" + id + "\"}");
    }

    /** Opening a result consumes it immediately, even before Dart acknowledges action readiness. */
    @Test
    public void openingCompletedWorkRestoresIdleAndDoesNotRepost() throws Exception {
        JSONObject completed = work("completed", "completed");
        update(1, completed);
        org.mockito.ArgumentCaptor<Notification> posted = org.mockito.ArgumentCaptor.forClass(Notification.class);
        verify(manager).notify(eq(NOTIFICATION_ID), posted.capture());
        Notification notification = posted.getValue();
        assertEquals(notification.contentIntent, notification.actions[0].actionIntent);
        Intent intent = shadowOf(notification.contentIntent).getSavedIntent();
        clearInvocations(manager);
        assertTrue(plugin.onNewIntent(intent));
        assertFalse(intent.hasExtra(WorkNotificationPresenter.WORK_ID));
        verify(manager).notify(NOTIFICATION_ID, idle);
        presenter.refresh();
        update(2, completed);
        verify(manager, times(3)).notify(NOTIFICATION_ID, idle);
        verify(manager, times(3)).notify(eq(NOTIFICATION_ID), any(Notification.class));
    }

    /** An opened result's expiry cannot overwrite newer work with the idle notification. */
    @Test
    public void consumedResultExpiryDoesNotHideNewWork() throws Exception {
        update(1, work("completed", "completed"));
        plugin.onNewIntent(activation("completed"));
        update(2, work("running", "running"));
        clearInvocations(manager);
        shadowOf(Looper.getMainLooper()).idleFor(Duration.ofSeconds(61));
        verifyNoInteractions(manager);
    }

    /** Initial Activity attachment consumes the same result as an existing-window click. */
    @Test
    public void startupActivityConsumesCompletedWork() throws Exception {
        update(1, work("completed", "completed"));
        clearInvocations(manager);
        ActivityPluginBinding binding = mock(ActivityPluginBinding.class);
        Activity activity = mock(Activity.class);
        when(binding.getActivity()).thenReturn(activity);
        when(activity.getIntent()).thenReturn(activation("completed"));
        plugin.onAttachedToActivity(binding);
        verify(manager).notify(NOTIFICATION_ID, idle);
    }

    /** Failed and cancelled results use the same read lifecycle as successful completions. */
    @Test
    public void openingFailedAndCancelledWorkConsumesOnlyThatResult() throws Exception {
        update(1, work("failed", "failed"), work("cancelled", "cancelled"));
        clearInvocations(manager);
        plugin.onNewIntent(activation("failed"));
        org.mockito.ArgumentCaptor<Notification> posted = org.mockito.ArgumentCaptor.forClass(Notification.class);
        verify(manager).notify(eq(NOTIFICATION_ID), posted.capture());
        assertEquals("cancelled", posted.getValue().extras.getString(Notification.EXTRA_TITLE));
        plugin.onNewIntent(activation("cancelled"));
        verify(manager).notify(NOTIFICATION_ID, idle);
    }

    /** Opening running or waiting work keeps its live status and future completion visible. */
    @Test
    public void openingActiveWorkDoesNotConsumeItsFutureResult() throws Exception {
        update(1, work("running", "running"), work("waiting", "waiting"));
        clearInvocations(manager);
        plugin.onNewIntent(activation("running"));
        plugin.onNewIntent(activation("waiting"));
        verifyNoInteractions(manager);
        update(2, work("running", "completed"));
        org.mockito.ArgumentCaptor<Notification> posted = org.mockito.ArgumentCaptor.forClass(Notification.class);
        verify(manager).notify(eq(NOTIFICATION_ID), posted.capture());
        assertEquals("running", posted.getValue().extras.getString(Notification.EXTRA_TITLE));
        assertNotSame(idle, posted.getValue());
    }

    /** Reading an older result cannot replace another execution's active notification with idle. */
    @Test
    public void openingResultPreservesConcurrentWork() throws Exception {
        update(1, work("completed", "completed"), work("running", "running"));
        clearInvocations(manager);
        plugin.onNewIntent(activation("completed"));
        org.mockito.ArgumentCaptor<Notification> posted = org.mockito.ArgumentCaptor.forClass(Notification.class);
        verify(manager).notify(eq(NOTIFICATION_ID), posted.capture());
        assertEquals("running", posted.getValue().extras.getString(Notification.EXTRA_TITLE));
        verify(manager, never()).notify(NOTIFICATION_ID, idle);
    }

    /** An obsolete click cannot hide an unrelated, newer result. */
    @Test
    public void unknownExecutionDoesNotClearCurrentResult() throws Exception {
        update(1, work("new", "completed"));
        clearInvocations(manager);
        assertTrue(plugin.onNewIntent(activation("old")));
        verifyNoInteractions(manager);
    }

    /** A launch remains valid after the foreground service has already been destroyed. */
    @Test
    public void detachedNotificationOwnerDoesNotRejectNavigation() {
        ReflectionHelpers.setStaticField(KeepAliveServiceHost.class, "workPresenter", null);
        assertTrue(plugin.onNewIntent(activation("completed")));
    }
}
