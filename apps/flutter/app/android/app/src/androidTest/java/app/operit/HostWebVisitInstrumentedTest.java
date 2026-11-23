package app.operit;

import static org.junit.Assert.*;

import android.app.Presentation;
import android.content.Context;
import android.graphics.PixelFormat;
import android.hardware.display.DisplayManager;
import android.hardware.display.VirtualDisplay;
import android.media.Image;
import android.media.ImageReader;
import android.os.Handler;
import android.os.Looper;
import android.webkit.WebView;
import android.widget.FrameLayout;
import androidx.test.ext.junit.runners.AndroidJUnit4;
import androidx.test.platform.app.InstrumentationRegistry;
import io.flutter.plugin.common.EventChannel;
import io.flutter.plugin.common.MethodChannel;
import io.flutter.plugin.platform.PlatformView;
import io.flutter.plugins.webviewflutter.HostWebVisitManager;
import java.net.ServerSocket;
import java.net.Socket;
import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.charset.StandardCharsets;
import java.util.HashMap;
import java.util.Collections;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.TimeUnit;
import org.json.JSONObject;
import org.junit.Test;
import org.junit.runner.RunWith;

/** Exercises the real system WebView and private displays, without starting a Flutter engine. */
@RunWith(AndroidJUnit4.class)
public class HostWebVisitInstrumentedTest {
  private final Context context = InstrumentationRegistry.getInstrumentation().getTargetContext();
  private final HostWebVisitManager host = HostWebVisitManager.getInstance();

  /** Requires JavaScript animation frames to run with no Activity or workspace mounted. */
  @Test public void executesDynamicPageWithoutFlutterOrActivity() throws Exception {
    try (HttpPage page = new HttpPage(200, "<title>Offscreen</title><body>Loading<script>"
        + "setTimeout(()=>requestAnimationFrame(()=>document.body.textContent='Rendered offscreen'),300)"
        + "</script></body>")) {
      JSONObject response = new JSONObject(host.execute(context, new JSONObject(request("unused", page.url())).toString()));
      assertTrue(response.toString(), response.getBoolean("success"));
      assertEquals("Rendered offscreen", response.getJSONObject("result").getString("content"));
    }
  }

  /** Keeps failed HTTP responses from becoming successful extracted text. */
  @Test public void returnsHttpFailure() throws Exception {
    try (HttpPage page = new HttpPage(403, "<body>Forbidden</body>")) {
      JSONObject response = new JSONObject(host.execute(context, new JSONObject(request("unused", page.url())).toString()));
      assertFalse(response.getBoolean("success"));
      assertTrue(response.getString("error").startsWith("HTTP 403"));
    }
  }

  /** Transfers one native browser between presentations and verifies that its DOM survives. */
  @Test public void presentationBorrowsTheExecutingWebViewAndPreservesDom() throws Exception {
    String id = "transfer-" + UUID.randomUUID();
    Object owner = new Object();
    CompletableFuture<Map<String, Object>> result = new CompletableFuture<>();
    CompletableFuture<Void> loaded = new CompletableFuture<>();
    PlatformView[] surfaces = new PlatformView[2];
    TestDisplay[] target = new TestDisplay[1];
    WebView[] browser = new WebView[1];
    try (HttpPage page = new HttpPage(200, "<title>Transfer</title><body>Original</body>")) {
      main(() -> {
        host.observe(owner, new EventChannel.EventSink() {
          /** Records actual native readiness without waiting for page timers in the test. */
          @Override public void success(Object value) {
            Map<?, ?> state = (Map<?, ?>) value;
            Map<?, ?> request = (Map<?, ?>) state.get("request");
            if (id.equals(request.get("requestId")) && Boolean.TRUE.equals(state.get("pageLoaded"))) loaded.complete(null);
          }
          /** Fails the test if native observation fails. */
          @Override public void error(String code, String message, Object details) {
            loaded.completeExceptionally(new IllegalStateException(message));
          }
          /** Ends no task when a presentation subscription closes. */
          @Override public void endOfStream() {}
        });
        host.visit(context, request(id, page.url()), new MethodChannel.Result() {
          /** Captures the result returned from the same executing browser. */
          @Override public void success(Object value) { result.complete((Map<String, Object>) value); }
          /** Fails native channel errors explicitly. */
          @Override public void error(String code, String message, Object details) {
            result.completeExceptionally(new IllegalStateException(message));
          }
          /** Rejects a missing native executor. */
          @Override public void notImplemented() { result.completeExceptionally(new AssertionError("Missing executor")); }
        });
        host.control(id, "manual");
      });
      loaded.get(15, TimeUnit.SECONDS);
      main(() -> {
        target[0] = new TestDisplay(context);
        surfaces[0] = host.surfaceFactory(owner).create(target[0].presentation.getContext(), 1, id);
        target[0].presentation.setContentView(surfaces[0].getView());
        target[0].presentation.show();
      });
      InstrumentationRegistry.getInstrumentation().waitForIdleSync();
      main(() -> {
        FrameLayout frame = (FrameLayout) surfaces[0].getView();
        assertEquals(1, frame.getChildCount());
        browser[0] = (WebView) frame.getChildAt(0);
        surfaces[0].dispose();
        assertEquals(0, frame.getChildCount());
        surfaces[1] = host.surfaceFactory(owner).create(target[0].presentation.getContext(), 2, id);
        target[0].presentation.setContentView(surfaces[1].getView());
      });
      InstrumentationRegistry.getInstrumentation().waitForIdleSync();
      CompletableFuture<Void> changed = new CompletableFuture<>();
      main(() -> {
        assertSame(browser[0], ((FrameLayout) surfaces[1].getView()).getChildAt(0));
        browser[0].evaluateJavascript("document.body.textContent='Same live DOM'", raw -> changed.complete(null));
      });
      changed.get(5, TimeUnit.SECONDS);
      main(() -> host.control(id, "extract"));
      Map<String, Object> response = result.get(10, TimeUnit.SECONDS);
      assertEquals(Boolean.TRUE, response.get("success"));
      assertEquals("Same live DOM", ((Map<?, ?>) response.get("result")).get("content"));
    } finally {
      main(() -> {
        host.control(id, "cancel");
        for (PlatformView surface : surfaces) if (surface != null) surface.dispose();
        if (target[0] != null) target[0].close();
        host.unobserve(owner);
        host.releaseSurfaces(owner);
      });
    }
  }

  /** Builds the public request schema passed through JNI and platform channels. */
  private static Map<String, Object> request(String id, String url) {
    Map<String, Object> request = new HashMap<>();
    request.put("requestId", id);
    request.put("url", url);
    request.put("headers", Collections.emptyList());
    request.put("userAgent", "OperitWebVisitTest");
    request.put("includeImageLinks", false);
    return request;
  }

  /** Runs native view operations on their required Android looper. */
  private static void main(Runnable operation) {
    InstrumentationRegistry.getInstrumentation().runOnMainSync(operation);
  }

  /** Supplies a real presentation window without opening an Activity or a Flutter engine. */
  private static final class TestDisplay implements AutoCloseable {
    final ImageReader images;
    final VirtualDisplay display;
    final Presentation presentation;

    /** Creates a visible destination for the borrowed WebView. */
    TestDisplay(Context context) {
      images = ImageReader.newInstance(800, 600, PixelFormat.RGBA_8888, 2);
      images.setOnImageAvailableListener(reader -> {
        try (Image image = reader.acquireLatestImage()) {}
      }, new Handler(Looper.getMainLooper()));
      display = ((DisplayManager) context.getSystemService(Context.DISPLAY_SERVICE))
          .createVirtualDisplay("Visit transfer test", 800, 600, 160, images.getSurface(), DisplayManager.VIRTUAL_DISPLAY_FLAG_PRESENTATION);
      presentation = new Presentation(context, display.getDisplay());
    }

    /** Releases every test display resource on the same main looper. */
    @Override public void close() {
      presentation.dismiss();
      display.release();
      images.setOnImageAvailableListener(null, null);
      images.close();
    }
  }

  /** Serves deterministic pages over real HTTP to the system browser renderer. */
  private static final class HttpPage implements AutoCloseable {
    final ServerSocket server;
    volatile boolean closed;

    /** Starts a local server that closes each response after writing its exact byte length. */
    HttpPage(int status, String html) throws Exception {
      server = new ServerSocket(0);
      byte[] body = html.getBytes(StandardCharsets.UTF_8);
      Thread worker = new Thread(() -> {
        while (!closed) {
          try (Socket client = server.accept()) {
            BufferedReader input = new BufferedReader(new InputStreamReader(client.getInputStream(), StandardCharsets.US_ASCII));
            String line;
            while ((line = input.readLine()) != null && !line.isEmpty()) {}
            client.getOutputStream().write(("HTTP/1.1 " + status + " Test\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: "
                + body.length + "\r\nConnection: close\r\n\r\n").getBytes(StandardCharsets.US_ASCII));
            client.getOutputStream().write(body);
          } catch (Exception error) { if (!closed) throw new RuntimeException(error); }
        }
      }, "web-visit-test-http");
      worker.setDaemon(true);
      worker.start();
    }

    /** Returns the exact local endpoint used by this fixture. */
    String url() { return "http://127.0.0.1:" + server.getLocalPort() + "/page"; }

    /** Stops accepting connections without leaking the fixture server. */
    @Override public void close() throws Exception { closed = true; server.close(); }
  }
}
