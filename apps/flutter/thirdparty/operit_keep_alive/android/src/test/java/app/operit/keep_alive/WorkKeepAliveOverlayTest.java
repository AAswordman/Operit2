package app.operit.keep_alive;

import android.app.Service;
import android.content.Intent;
import android.graphics.PixelFormat;
import android.os.IBinder;
import android.view.View;
import android.view.WindowManager;
import org.json.JSONObject;
import org.junit.After;
import org.junit.Before;
import org.junit.Test;
import org.junit.runner.RunWith;
import org.mockito.ArgumentCaptor;
import org.robolectric.Robolectric;
import org.robolectric.RobolectricTestRunner;
import org.robolectric.annotation.Config;
import org.robolectric.shadows.ShadowSettings;
import org.robolectric.util.ReflectionHelpers;
import static org.junit.Assert.*;
import static org.mockito.ArgumentMatchers.*;
import static org.mockito.Mockito.*;

/** Exercises task lifetime, permission transitions, and independent service window ownership. */
@RunWith(RobolectricTestRunner.class)
@Config(sdk = 28, manifest = Config.NONE)
public class WorkKeepAliveOverlayTest {
    /** Provides a concrete service context without starting an application runtime. */
    public static class TestService extends Service {
        /** Exposes no binder because the keep-alive window belongs to the service itself. */
        @Override
        public IBinder onBind(Intent intent) { return null; }
    }

    private WindowManager manager;
    private Service service;
    private WorkKeepAliveOverlay overlay;

    /** Grants overlay access and connects a real service context to a controllable window manager. */
    @Before
    public void setUp() {
        service = spy(Robolectric.buildService(TestService.class).create().get());
        manager = mock(WindowManager.class);
        when(service.getSystemService(WindowManager.class)).thenReturn(manager);
        ShadowSettings.setCanDrawOverlays(true);
        overlay = new WorkKeepAliveOverlay(service);
        ReflectionHelpers.setStaticField(KeepAliveServiceHost.class, "workOverlay", null);
        ReflectionHelpers.setStaticField(KeepAliveServiceHost.class, "workSnapshot", null);
    }

    /** Releases windows and static work state between independent lifecycle tests. */
    @After
    public void tearDown() {
        overlay.close();
        ReflectionHelpers.setStaticField(KeepAliveServiceHost.class, "workOverlay", null);
        ReflectionHelpers.setStaticField(KeepAliveServiceHost.class, "workSnapshot", null);
    }

    /** Constructs real protocol snapshots with explicit execution end times. */
    private JSONObject snapshot(long revision, String works) throws Exception {
        return new JSONObject("{\"schemaVersion\":1,\"revision\":" + revision + ",\"works\":" + works + "}");
    }

    /** Creates exactly one system overlay without accepting input, focus, or screen-on requests. */
    @Test
    public void unfinishedWorkAddsOneNoninteractivePixel() throws Exception {
        overlay.update(snapshot(1, "[{\"endedAtMillis\":null}]"));
        overlay.refresh();
        ArgumentCaptor<View> view = ArgumentCaptor.forClass(View.class);
        ArgumentCaptor<WindowManager.LayoutParams> layout = ArgumentCaptor.forClass(WindowManager.LayoutParams.class);
        verify(manager, times(1)).addView(view.capture(), layout.capture());
        WindowManager.LayoutParams params = layout.getValue();
        assertSame(service, view.getValue().getContext());
        assertEquals(1, params.width);
        assertEquals(1, params.height);
        assertEquals(WindowManager.LayoutParams.TYPE_APPLICATION_OVERLAY, params.type);
        assertEquals(PixelFormat.TRANSLUCENT, params.format);
        assertTrue((params.flags & WindowManager.LayoutParams.FLAG_NOT_TOUCHABLE) != 0);
        assertTrue((params.flags & WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE) != 0);
        assertEquals(0, params.flags & WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        assertTrue(params.alpha > 0 && params.alpha < 0.1f);
    }

    /** Keeps protection for parallel work and permission confirmation until the final execution ends. */
    @Test
    public void lastUnfinishedWorkControlsRemoval() throws Exception {
        overlay.update(snapshot(1, "[{\"endedAtMillis\":null},{\"endedAtMillis\":null}]"));
        overlay.update(snapshot(2, "[{\"endedAtMillis\":100},{\"status\":\"waiting\",\"endedAtMillis\":null}]"));
        verify(manager, never()).removeViewImmediate(any());
        overlay.update(snapshot(3, "[{\"endedAtMillis\":100},{\"endedAtMillis\":200}]"));
        verify(manager).removeViewImmediate(any(View.class));
        overlay.close();
        verify(manager, times(1)).removeViewImmediate(any());
    }

    /** Terminal records retained for notification display never retain a keep-alive window. */
    @Test
    public void idleAndRetainedResultsDoNotAddWindows() throws Exception {
        overlay.update(snapshot(1, "[]"));
        overlay.update(snapshot(2, "[{\"status\":\"completed\",\"endedAtMillis\":100},"
            + "{\"status\":\"failed\",\"endedAtMillis\":200},{\"status\":\"cancelled\",\"endedAtMillis\":300}]"));
        verifyNoInteractions(manager);
    }

    /** Attaches ongoing work after returning from settings without requiring a new Core revision. */
    @Test
    public void permissionGrantRefreshesRetainedWork() throws Exception {
        ShadowSettings.setCanDrawOverlays(false);
        overlay.update(snapshot(1, "[{\"endedAtMillis\":null}]"));
        verifyNoInteractions(manager);
        ShadowSettings.setCanDrawOverlays(true);
        overlay.refresh();
        verify(manager).addView(any(View.class), any(WindowManager.LayoutParams.class));
    }

    /** Removes the window when permission is revoked and reinstates it only for unfinished work. */
    @Test
    public void permissionRevocationRemovesWindow() throws Exception {
        overlay.update(snapshot(1, "[{\"endedAtMillis\":null}]"));
        ShadowSettings.setCanDrawOverlays(false);
        overlay.refresh();
        verify(manager).removeViewImmediate(any(View.class));
        overlay.update(snapshot(2, "[]"));
        ShadowSettings.setCanDrawOverlays(true);
        overlay.refresh();
        verify(manager, times(1)).addView(any(View.class), any(WindowManager.LayoutParams.class));
    }

    /** Service destruction removes active work protection even before Core publishes its terminal state. */
    @Test
    public void serviceCloseReleasesActiveWindow() throws Exception {
        overlay.update(snapshot(1, "[{\"endedAtMillis\":null}]"));
        ReflectionHelpers.setStaticField(KeepAliveServiceHost.class, "service", service);
        ReflectionHelpers.setStaticField(KeepAliveServiceHost.class, "workOverlay", overlay);
        KeepAliveServiceHost.detach(service);
        verify(manager).removeViewImmediate(any(View.class));
        overlay.refresh();
        verify(manager, times(1)).addView(any(View.class), any(WindowManager.LayoutParams.class));
    }

    /** A rejected system window cannot record a nonexistent attachment or interrupt Core work updates. */
    @Test
    public void failedAttachmentLeavesNoWindowToRemove() throws Exception {
        doThrow(new WindowManager.BadTokenException("permission changed"))
            .when(manager).addView(any(), any());
        overlay.update(snapshot(1, "[{\"endedAtMillis\":null}]"));
        overlay.close();
        verify(manager, never()).removeViewImmediate(any());
    }

    /** Stale Core snapshots cannot resurrect an overlay after its work has finished. */
    @Test
    public void serviceRejectsStaleWorkBeforeUpdatingOverlay() throws Exception {
        ReflectionHelpers.setStaticField(KeepAliveServiceHost.class, "workOverlay", overlay);
        KeepAliveServiceHost.publishWorkSnapshot(snapshot(10, "[{\"endedAtMillis\":null}]").toString());
        KeepAliveServiceHost.publishWorkSnapshot(snapshot(11, "[]").toString());
        KeepAliveServiceHost.publishWorkSnapshot(snapshot(10, "[{\"endedAtMillis\":null}]").toString());
        verify(manager, times(1)).addView(any(View.class), any(WindowManager.LayoutParams.class));
        verify(manager, times(1)).removeViewImmediate(any(View.class));
    }
}
