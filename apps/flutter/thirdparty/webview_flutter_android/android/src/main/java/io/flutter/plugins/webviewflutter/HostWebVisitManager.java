package io.flutter.plugins.webviewflutter;

import android.annotation.SuppressLint;
import android.app.Presentation;
import android.content.Context;
import android.content.MutableContextWrapper;
import android.graphics.PixelFormat;
import android.hardware.display.DisplayManager;
import android.hardware.display.VirtualDisplay;
import android.media.Image;
import android.media.ImageReader;
import android.net.Uri;
import android.os.Handler;
import android.os.Looper;
import android.os.SystemClock;
import android.view.View;
import android.view.ViewGroup;
import android.webkit.RenderProcessGoneDetail;
import android.webkit.WebResourceError;
import android.webkit.WebResourceRequest;
import android.webkit.WebResourceResponse;
import android.webkit.WebView;
import android.webkit.WebViewClient;
import android.widget.FrameLayout;
import androidx.annotation.NonNull;
import androidx.annotation.Nullable;
import io.flutter.plugin.common.EventChannel;
import io.flutter.plugin.common.MethodChannel;
import io.flutter.plugin.common.StandardMessageCodec;
import io.flutter.plugin.platform.PlatformView;
import io.flutter.plugin.platform.PlatformViewFactory;
import java.nio.charset.StandardCharsets;
import java.io.ByteArrayOutputStream;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.UUID;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.TimeUnit;
import java.util.function.Consumer;
import org.json.JSONArray;
import org.json.JSONObject;

/** Owns browser execution in the application process, independently from Flutter and Activities. */
public final class HostWebVisitManager {
  private static final HostWebVisitManager INSTANCE = new HostWebVisitManager();
  private final Handler main = new Handler(Looper.getMainLooper());
  private final Map<String, Visit> visits = new LinkedHashMap<>();
  private final Map<Object, EventChannel.EventSink> observers = new LinkedHashMap<>();

  /** Returns the process owner shared by JNI execution and Flutter presentation. */
  public static HostWebVisitManager getInstance() { return INSTANCE; }

  /** Executes on a Core worker while all WebView operations remain on Android's main looper. */
  public String execute(Context context, String encoded) throws Exception {
    if (Looper.myLooper() == Looper.getMainLooper()) {
      throw new IllegalStateException("A web visit must not block the Android main thread");
    }
    JSONObject request = new JSONObject(encoded);
    request.put("requestId", "host-visit-" + UUID.randomUUID());
    CompletableFuture<Map<String, Object>> completion = new CompletableFuture<>();
    main.post(() -> start(context, request, completion::complete));
    try {
      return new JSONObject(completion.get(50, TimeUnit.SECONDS)).toString();
    } catch (Exception error) {
      main.post(() -> {
        Visit visit = visits.get(request.optString("requestId"));
        if (visit != null) visit.fail("visit_web worker interrupted or timed out");
      });
      throw error;
    }
  }

  /** Registers a presentation client and immediately replays tasks still executing. */
  public void observe(Object owner, EventChannel.EventSink sink) {
    observers.put(owner, sink);
    for (Visit visit : new ArrayList<>(visits.values())) {
      if (!visit.finished) sink.success(visit.snapshot());
    }
  }

  /** Removes only the engine observer, leaving process-owned execution alive. */
  public void unobserve(Object owner) { observers.remove(owner); }

  /** Returns views borrowed by a destroyed Flutter engine to their native task owner. */
  public void releaseSurfaces(Object owner) {
    for (Visit visit : new ArrayList<>(visits.values())) {
      if (visit.presentationOwner == owner && visit.presented != null) visit.detach(visit.presented);
    }
  }

  /** Starts a request received from an external runtime through the same native executor. */
  public void visit(Context context, Object arguments, MethodChannel.Result result) {
    try {
      start(context, new JSONObject((Map<?, ?>) arguments), result::success);
    } catch (Exception error) {
      result.error("web_visit", error.toString(), null);
    }
  }

  /** Applies explicit user controls to one task without affecting other visits. */
  public void control(String id, String action) {
    Visit visit = visits.get(id);
    if (visit == null || visit.finished) return;
    switch (action) {
      case "cancel": visit.fail("visit_web cancelled"); break;
      case "manual": visit.manual = true; visit.status = "Manual mode"; visit.publish(); break;
      case "extract": visit.extract = true; visit.sample(); break;
      default: throw new IllegalArgumentException("Unknown web visit action: " + action);
    }
  }

  /** Builds presentation containers that borrow the exact WebView used by the host task. */
  public PlatformViewFactory surfaceFactory(Object owner) {
    return new PlatformViewFactory(StandardMessageCodec.INSTANCE) {
      /** Attaches a live browser without issuing another navigation. */
      @NonNull @Override
      public PlatformView create(Context context, int viewId, @Nullable Object args) {
        Visit visit = visits.get((String) args);
        FrameLayout frame = new FrameLayout(context) {
          /** Transfers hidden workspace surfaces back to the attached private display. */
          @Override protected void onWindowVisibilityChanged(int visibility) {
            super.onWindowVisibilityChanged(visibility);
            main.post(() -> { if (visit != null) visit.syncPresentation(); });
          }
          /** Restores native presentation only after its real window has attached. */
          @Override protected void onAttachedToWindow() {
            super.onAttachedToWindow();
            main.post(() -> { if (visit != null) visit.syncPresentation(); });
          }
          /** Keeps execution attached to the private display during surface removal. */
          @Override protected void onDetachedFromWindow() {
            super.onDetachedFromWindow();
            main.post(() -> { if (visit != null) visit.syncPresentation(); });
          }
          /** Observes retained workspace tabs hidden without disposing their widgets. */
          @Override protected void onVisibilityChanged(@NonNull View changedView, int visibility) {
            super.onVisibilityChanged(changedView, visibility);
            main.post(() -> { if (visit != null) visit.syncPresentation(); });
          }
        };
        if (visit != null && !visit.finished) visit.attach(frame, owner);
        return new PlatformView() {
          /** Returns the interactive container placed inside the Flutter workspace. */
          @Override public View getView() { return frame; }
          /** Returns the browser to its private display while its task is still running. */
          @Override public void dispose() {
            if (visit != null) visit.detach(frame);
          }
        };
      }
    };
  }

  /** Creates one task and reports setup failures through the same terminal response. */
  private void start(Context context, JSONObject request, Consumer<Map<String, Object>> completion) {
    String id = request.optString("requestId");
    if (id.isEmpty() || visits.containsKey(id)) {
      Map<String, Object> response = new LinkedHashMap<>();
      response.put("requestId", id);
      response.put("success", false);
      response.put("error", "Missing or duplicate web visit requestId");
      completion.accept(response);
      return;
    }
    Visit visit = new Visit(context.getApplicationContext(), request, completion);
    visits.put(id, visit);
    try { visit.start(); }
    catch (Exception error) { visit.fail(error.toString()); }
  }

  /** Owns a single engine, private display, deadlines, navigation generation and result. */
  private final class Visit {
    final Context context;
    final JSONObject request;
    final String id;
    final Consumer<Map<String, Object>> completion;
    final Runnable timeout = () -> fail("visit_web timed out");
    final Runnable poll = this::sample;
    final Runnable scriptTimeout = () -> fail("WebView script evaluation timed out");
    WebView webView;
    MutableContextWrapper viewContext;
    FrameLayout offscreen;
    FrameLayout presented;
    Object presentationOwner;
    ImageReader images;
    VirtualDisplay display;
    Presentation presentation;
    boolean finished;
    boolean manual;
    boolean extract;
    boolean evaluating;
    boolean rendererGone;
    boolean pageLoaded;
    int generation;
    long stableSince;
    String fingerprint;
    String script;
    String url;
    String title = "";
    String status = "Loading page";
    Map<String, Object> response;

    /** Retains only application context and one task's immutable request. */
    Visit(Context context, JSONObject request, Consumer<Map<String, Object>> completion) {
      this.context = context;
      this.request = request;
      this.id = request.optString("requestId");
      this.url = request.optString("url").trim();
      this.completion = completion;
    }

    /** Attaches a real WebView to a private virtual display before navigation starts. */
    @SuppressLint("SetJavaScriptEnabled")
    void start() throws Exception {
      Uri uri = Uri.parse(url);
      if (uri.getScheme() == null) { url = "https://" + url; uri = Uri.parse(url); }
      requireWebUrl(uri);
      try (java.io.InputStream input = context.getAssets().open("operit_web_visit.js")) {
        ByteArrayOutputStream bytes = new ByteArrayOutputStream();
        byte[] buffer = new byte[8192];
        int count;
        while ((count = input.read(buffer)) != -1) bytes.write(buffer, 0, count);
        script = new String(bytes.toByteArray(), StandardCharsets.UTF_8);
      }
      main.postDelayed(timeout, 45000);
      // Drain every image: a full consumer queue would stall the browser compositor.
      images = ImageReader.newInstance(1280, 720, PixelFormat.RGBA_8888, 2);
      images.setOnImageAvailableListener(reader -> {
        try (Image image = reader.acquireLatestImage()) { /* No image crosses into Flutter. */ }
      }, main);
      DisplayManager manager = (DisplayManager) context.getSystemService(Context.DISPLAY_SERVICE);
      display = manager.createVirtualDisplay("Operit web visit", 1280, 720, 160, images.getSurface(),
          DisplayManager.VIRTUAL_DISPLAY_FLAG_PRESENTATION | DisplayManager.VIRTUAL_DISPLAY_FLAG_OWN_CONTENT_ONLY);
      if (display == null) throw new IllegalStateException("Unable to create browser private display");
      presentation = new Presentation(context, display.getDisplay());
      presentation.getWindow().addFlags(android.view.WindowManager.LayoutParams.FLAG_HARDWARE_ACCELERATED);
      offscreen = new FrameLayout(presentation.getContext());
      viewContext = new MutableContextWrapper(presentation.getContext());
      webView = new WebView(viewContext);
      webView.getSettings().setJavaScriptEnabled(true);
      webView.getSettings().setDomStorageEnabled(true);
      webView.getSettings().setMediaPlaybackRequiresUserGesture(true);
      webView.getSettings().setAllowFileAccess(false);
      webView.getSettings().setAllowContentAccess(false);
      if (android.os.Build.VERSION.SDK_INT >= 26) {
        webView.setRendererPriorityPolicy(WebView.RENDERER_PRIORITY_IMPORTANT, false);
      }
      String userAgent = request.optString("userAgent");
      if (!userAgent.isEmpty()) webView.getSettings().setUserAgentString(userAgent);
      webView.setWebViewClient(new WebViewClient() {
        /** Leaves application links outside this read-only task and records main-frame navigations. */
        @Override public boolean shouldOverrideUrlLoading(WebView view, WebResourceRequest navigation) {
          String scheme = navigation.getUrl().getScheme();
          return !"https".equals(scheme) && !"http".equals(scheme);
        }
        /** Invalidates samples from the previous document, including same-URL reloads. */
        @Override public void onPageStarted(WebView view, String nextUrl, android.graphics.Bitmap icon) {
          generation++;
          pageLoaded = false;
          url = nextUrl;
          fingerprint = null;
          status = "Loading page";
          publish();
        }
        /** Reports top-level network errors without treating subresource errors as whole-page failure. */
        @Override public void onReceivedError(WebView view, WebResourceRequest resource, WebResourceError error) {
          if (resource.isForMainFrame()) fail(error.getDescription().toString());
        }
        /** Returns unsuccessful top-level HTTP responses as errors rather than extracted success pages. */
        @Override public void onReceivedHttpError(WebView view, WebResourceRequest resource, WebResourceResponse error) {
          if (resource.isForMainFrame()) fail("HTTP " + error.getStatusCode() + " " + error.getReasonPhrase());
        }
        /** Acknowledges renderer loss and closes only the affected browser task. */
        @Override public boolean onRenderProcessGone(WebView view, RenderProcessGoneDetail detail) {
          rendererGone = true;
          fail("WebView renderer exited (crashed=" + detail.didCrash() + ")");
          return true;
        }
      });
      offscreen.addView(webView, new FrameLayout.LayoutParams(-1, -1));
      presentation.setContentView(offscreen);
      presentation.show();
      Map<String, String> headers = new LinkedHashMap<>();
      JSONArray values = request.getJSONArray("headers");
      for (int index = 0; index < values.length(); index++) {
        JSONObject header = values.getJSONObject(index);
        headers.put(header.getString("name"), header.getString("value"));
      }
      webView.loadUrl(url, headers);
      if (!finished) {
        publish();
        main.postDelayed(poll, 250);
      }
    }

    /** Samples through the Android looper, never through page timers or Flutter frames. */
    void sample() {
      if (finished || evaluating) return;
      main.removeCallbacks(poll);
      evaluating = true;
      int document = generation;
      main.postDelayed(scriptTimeout, 8000);
      try {
        webView.evaluateJavascript(script, raw -> {
          main.removeCallbacks(scriptTimeout);
          evaluating = false;
          if (finished) return;
          try {
            if (document == generation && raw != null && !raw.equals("null")) {
              Object decoded = new org.json.JSONTokener(raw).nextValue();
              JSONObject sample = new JSONObject((String) decoded);
              JSONObject result = sample.getJSONObject("result");
              url = result.getString("url");
              title = result.getString("title");
              if (!request.getBoolean("includeImageLinks")) result.put("imageLinks", new JSONArray());
              if (sample.getBoolean("ready") || (extract && sample.getBoolean("hasDocument"))) {
                pageLoaded = true;
                boolean captcha = sample.getBoolean("captcha");
                status = captcha ? "Captcha verification required" : manual ? "Manual mode" : "Reading page";
                String next = result.toString();
                long now = SystemClock.uptimeMillis();
                if (!next.equals(fingerprint)) { fingerprint = next; stableSince = now; }
                if (extract || (!manual && !captcha && !result.getString("content").trim().isEmpty() && now - stableSince >= 1000)) {
                  succeed(result);
                  return;
                }
                publish();
              }
            }
            main.postDelayed(poll, 250);
          } catch (Exception error) { fail("WebView extraction failed: " + error); }
        });
      } catch (Exception error) {
        evaluating = false;
        main.removeCallbacks(scriptTimeout);
        fail("WebView evaluation failed: " + error);
      }
    }

    /** Presents the existing browser and its unchanged DOM inside a Flutter platform view. */
    void attach(FrameLayout target, Object owner) {
      if (presented != null) throw new IllegalStateException("This visit already has a presentation");
      presented = target;
      presentationOwner = owner;
      syncPresentation();
    }

    /** Keeps the browser on an attached display while its workspace window is hidden. */
    void syncPresentation() {
      if (finished || presented == null || webView == null) return;
      boolean visible = presented.isAttachedToWindow() && presented.isShown()
          && presented.getWindowVisibility() == View.VISIBLE;
      FrameLayout target = visible ? presented : offscreen;
      if (webView.getParent() == target) return;
      removeParent();
      viewContext.setBaseContext(visible ? presented.getContext() : presentation.getContext());
      target.addView(webView, new FrameLayout.LayoutParams(-1, -1));
    }

    /** Returns a running browser to its private display when the workspace stops showing it. */
    void detach(FrameLayout target) {
      if (presented != target) return;
      removeParent();
      presented = null;
      presentationOwner = null;
      if (finished) { destroy(); return; }
      viewContext.setBaseContext(presentation.getContext());
      offscreen.addView(webView, new FrameLayout.LayoutParams(-1, -1));
    }

    /** Removes the view from its current container before transferring presentation ownership. */
    void removeParent() {
      if (webView != null && webView.getParent() instanceof ViewGroup) {
        ((ViewGroup) webView.getParent()).removeView(webView);
      }
    }

    /** Completes a visit using the DOM sampled from its own browser instance. */
    void succeed(JSONObject result) throws Exception {
      Map<String, Object> terminal = new LinkedHashMap<>();
      terminal.put("requestId", id);
      terminal.put("success", true);
      terminal.put("result", toValue(result));
      finish(terminal);
    }

    /** Completes failed and cancelled tasks through one exactly-once cleanup path. */
    void fail(String message) {
      Map<String, Object> terminal = new LinkedHashMap<>();
      terminal.put("requestId", id);
      terminal.put("success", false);
      terminal.put("error", message);
      finish(terminal);
    }

    /** Stops execution before delivering the terminal event and worker response. */
    void finish(Map<String, Object> terminal) {
      if (finished) return;
      finished = true;
      response = terminal;
      main.removeCallbacks(timeout);
      main.removeCallbacks(poll);
      main.removeCallbacks(scriptTimeout);
      if (webView != null && !rendererGone) webView.stopLoading();
      try {
        publish();
        completion.accept(terminal);
      } finally {
        presented = null;
        presentationOwner = null;
        destroy();
      }
    }

    /** Releases offscreen display resources and the completed browser deterministically. */
    void destroy() {
      visits.remove(id);
      removeParent();
      if (webView != null) { webView.destroy(); webView = null; }
      releaseDisplay();
    }

    /** Closes compositor resources without waiting for Flutter to draw another frame. */
    void releaseDisplay() {
      if (presentation != null) { presentation.dismiss(); presentation = null; }
      if (display != null) { display.release(); display = null; }
      if (images != null) {
        images.setOnImageAvailableListener(null, null);
        images.close(); images = null;
      }
    }

    /** Serializes request identity and observed state without transferring execution ownership. */
    Map<String, Object> snapshot() {
      Map<String, Object> state = new LinkedHashMap<>();
      state.put("request", toValue(request));
      state.put("currentUrl", url);
      state.put("pageTitle", title);
      state.put("statusText", status);
      state.put("pageLoaded", pageLoaded);
      state.put("manual", manual);
      state.put("completed", finished);
      state.put("response", response);
      return state;
    }

    /** Broadcasts task changes only to currently connected presentation engines. */
    void publish() {
      Map<String, Object> state = snapshot();
      for (Map.Entry<Object, EventChannel.EventSink> observer : new ArrayList<>(observers.entrySet())) {
        try { observer.getValue().success(state); }
        catch (RuntimeException error) {
          observers.remove(observer.getKey());
          android.util.Log.w("HostWebVisit", "Browser observer disconnected", error);
        }
      }
    }
  }

  /** Rejects local or executable schemes at the browser task boundary. */
  private static void requireWebUrl(Uri uri) {
    if ((!"http".equals(uri.getScheme()) && !"https".equals(uri.getScheme())) || uri.getHost() == null) {
      throw new IllegalArgumentException("visit_web requires an HTTP(S) URL");
    }
  }

  /** Converts JSON into StandardMessageCodec values recursively without serializing null sentinels. */
  private static Object toValue(Object value) {
    if (value == JSONObject.NULL) return null;
    if (value instanceof JSONObject) {
      Map<String, Object> result = new LinkedHashMap<>();
      JSONObject object = (JSONObject) value;
      java.util.Iterator<String> keys = object.keys();
      while (keys.hasNext()) { String key = keys.next(); result.put(key, toValue(object.opt(key))); }
      return result;
    }
    if (value instanceof JSONArray) {
      List<Object> result = new ArrayList<>();
      JSONArray array = (JSONArray) value;
      for (int index = 0; index < array.length(); index++) result.add(toValue(array.opt(index)));
      return result;
    }
    return value;
  }
}
