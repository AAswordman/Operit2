package io.flutter.plugins.webviewflutter;

import android.webkit.WebView;
import androidx.webkit.ScriptHandler;
import androidx.webkit.WebViewCompat;
import androidx.webkit.WebViewFeature;
import io.flutter.plugin.common.BinaryMessenger;
import io.flutter.plugin.common.MethodCall;
import io.flutter.plugin.common.MethodChannel;
import java.util.Collections;
import java.util.LinkedHashMap;
import java.util.Map;
import java.util.WeakHashMap;

/** Owns document-start scripts and lifecycle injection for older WebView providers. */
final class WebViewUserScripts implements MethodChannel.MethodCallHandler {
  private final MethodChannel channel;
  private final ProxyApiRegistrar registrar;
  private final Map<WebView, Map<String, UserScript>> scripts = new WeakHashMap<>();
  private long nextIdentifier = 0;

  /** Retains either a native registration or a guarded lifecycle script. */
  private static final class UserScript {
    final ScriptHandler handler;
    final String source;

    /** Creates one removable registration without retaining the owning WebView. */
    UserScript(ScriptHandler handler, String source) {
      this.handler = handler;
      this.source = source;
    }

    /** Releases the native registration when the provider supports it. */
    void remove() {
      if (handler != null) handler.remove();
    }
  }

  /** Registers the per-engine user-script API on Flutter's platform thread. */
  WebViewUserScripts(BinaryMessenger messenger, ProxyApiRegistrar registrar) {
    this.registrar = registrar;
    channel = new MethodChannel(messenger, "operit/webview_user_scripts");
    channel.setMethodCallHandler(this);
  }

  /** Registers scripts according to provider capability and owns their removal. */
  @Override
  public void onMethodCall(MethodCall call, MethodChannel.Result result) {
    if (!call.method.equals("add") && !call.method.equals("remove") && !call.method.equals("removeAll")) {
      result.notImplemented();
      return;
    }
    try {
      Number viewIdentifier = call.argument("viewIdentifier");
      if (viewIdentifier == null) throw new IllegalArgumentException("Missing viewIdentifier");
      WebView view = registrar.getInstanceManager().getInstance(viewIdentifier.longValue());
      if (view == null) throw new IllegalArgumentException("Unknown WebView");
      switch (call.method) {
        case "add": {
          String source = call.argument("source");
          if (source == null) throw new IllegalArgumentException("Missing source");
          ScriptHandler handler = null;
          if (WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)) {
            handler = WebViewCompat.addDocumentStartJavaScript(view, source, Collections.singleton("*"));
          }
          String identifier = "operit-user-script-" + (++nextIdentifier);
          scripts.computeIfAbsent(view, ignored -> new LinkedHashMap<>())
              .put(identifier, new UserScript(handler, guardScript(identifier, source)));
          result.success(identifier);
          return;
        }
        case "remove": {
          String identifier = call.argument("identifier");
          Map<String, UserScript> owned = scripts.get(view);
          if (owned == null || identifier == null) throw new IllegalArgumentException("Unknown script handle");
          UserScript script = owned.remove(identifier);
          if (script == null) throw new IllegalArgumentException("Unknown script handle");
          script.remove();
          if (owned.isEmpty()) scripts.remove(view);
          result.success(null);
          return;
        }
        case "removeAll": {
          removeAll(view);
          result.success(null);
          return;
        }
      }
    } catch (RuntimeException error) {
      result.error("user_script_operation_failed", error.getMessage(), null);
    }
  }

  /** Guards each lifecycle script once per document, including reloads of the same URL. */
  private static String guardScript(String identifier, String source) {
    return "(function(){var key='" + identifier + "';"
        + "var scripts=document.__operitLifecycleUserScripts;"
        + "if(!scripts){scripts=Object.create(null);"
        + "Object.defineProperty(document,'__operitLifecycleUserScripts',{value:scripts});}"
        + "if(scripts[key])return;scripts[key]=true;\n"
        + source + "\n})();";
  }

  /** Injects legacy scripts in registration order without repeating native registrations. */
  void injectLegacyScripts(WebView view) {
    Map<String, UserScript> owned = scripts.get(view);
    if (owned == null) return;
    for (UserScript script : owned.values()) {
      if (script.handler == null) view.evaluateJavascript(script.source, null);
    }
  }

  /** Releases all scripts when their view is destroyed or explicitly cleared. */
  void removeAll(WebView view) {
    Map<String, UserScript> owned = scripts.remove(view);
    if (owned != null) owned.values().forEach(UserScript::remove);
  }

  /** Removes all native handlers before detaching the Flutter engine. */
  void dispose() {
    channel.setMethodCallHandler(null);
    scripts.values().forEach(owned -> owned.values().forEach(UserScript::remove));
    scripts.clear();
  }
}
