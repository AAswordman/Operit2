package io.flutter.plugins.webviewflutter;

import android.app.Activity;
import android.content.ActivityNotFoundException;
import android.content.Context;
import android.content.Intent;
import android.net.Uri;
import io.flutter.plugin.common.BinaryMessenger;
import io.flutter.plugin.common.MethodChannel;
import java.net.URISyntaxException;
import java.util.Locale;

/** Opens approved browser links through Android's browsable Intent contract. */
final class WebViewExternalNavigation {
  private final MethodChannel channel;

  /** Registers the engine channel while resolving the current Activity on every request. */
  WebViewExternalNavigation(BinaryMessenger messenger, ProxyApiRegistrar registrar) {
    channel = new MethodChannel(messenger, "operit/webview_external_navigation");
    channel.setMethodCallHandler((call, result) -> {
      if (!call.method.equals("open")) {
        result.notImplemented();
        return;
      }
      if (!(call.arguments instanceof String)) {
        result.error("invalid_external_url", "Expected an external URL", null);
        return;
      }
      try {
        Intent intent = createIntent((String) call.arguments);
        Context context = registrar.getContext();
        if (!(context instanceof Activity)) intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        context.startActivity(intent);
        result.success(true);
      } catch (ActivityNotFoundException error) {
        result.success(false);
      } catch (URISyntaxException | IllegalArgumentException | SecurityException error) {
        result.error("external_navigation_failed", error.getMessage(), null);
      }
    });
  }

  /** Preserves package and payload while removing explicit components and URI permissions. */
  static Intent createIntent(String url) throws URISyntaxException {
    Uri original = Uri.parse(url);
    Intent intent = "intent".equalsIgnoreCase(original.getScheme())
        ? Intent.parseUri(url, Intent.URI_INTENT_SCHEME)
        : new Intent(Intent.ACTION_VIEW, original);
    Uri data = intent.getData();
    String scheme = data == null ? null : data.getScheme();
    if (scheme == null || scheme.isEmpty() || "intent".equalsIgnoreCase(scheme)
        || "file".equalsIgnoreCase(scheme) || "content".equalsIgnoreCase(scheme)
        || "javascript".equalsIgnoreCase(scheme) || "data".equalsIgnoreCase(scheme)
        || "about".equalsIgnoreCase(scheme) || "blob".equalsIgnoreCase(scheme)) {
      throw new IllegalArgumentException("The link does not identify an external application URL");
    }
    intent.setAction(Intent.ACTION_VIEW);
    intent.addCategory(Intent.CATEGORY_BROWSABLE);
    intent.setComponent(null);
    intent.setSelector(null);
    intent.setFlags(0);
    intent.removeExtra("browser_fallback_url");
    return intent;
  }

  /** Classifies application protocols without cancelling ordinary subframe documents. */
  static boolean isApplicationUri(Uri uri) {
    if (uri == null) return false;
    String scheme = uri.getScheme();
    if (scheme == null || scheme.isEmpty()) return false;
    switch (scheme.toLowerCase(Locale.ROOT)) {
      case "http": case "https": case "file": case "content": case "about":
      case "data": case "blob": case "javascript":
        return false;
      default:
        return true;
    }
  }

  /** Removes engine callbacks before the plugin registrar is released. */
  void dispose() {
    channel.setMethodCallHandler(null);
  }
}
