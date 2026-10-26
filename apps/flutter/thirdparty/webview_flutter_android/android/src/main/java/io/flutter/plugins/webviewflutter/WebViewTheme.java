package io.flutter.plugins.webviewflutter;

import android.content.Context;
import android.content.res.Configuration;
import android.view.ContextThemeWrapper;
import android.webkit.WebView;
import java.util.Collections;
import java.util.Set;
import java.util.WeakHashMap;

/** Owns isolated WebView appearance contexts without changing the system theme. */
final class WebViewTheme {
  private static boolean dark;
  private static final Set<WebView> views =
      Collections.newSetFromMap(new WeakHashMap<WebView, Boolean>());

  /** Isolates browser resources while preserving the Activity context for native popups. */
  static Context createContext(Context parent) {
    Configuration configuration = configuration(parent.getResources().getConfiguration());
    ContextThemeWrapper context = new ContextThemeWrapper(parent, style());
    context.applyOverrideConfiguration(configuration);
    return context;
  }

  /** Tracks live browser views without extending their lifetime. */
  static void register(WebView view) {
    views.add(view);
  }

  /** Applies the app preference and notifies every live browser of changed traits. */
  @SuppressWarnings("deprecation")
  static void setDark(boolean value) {
    dark = value;
    for (WebView view : views) {
      Context context = view.getContext();
      Configuration configuration = configuration(context.getResources().getConfiguration());
      context.getResources().updateConfiguration(configuration, context.getResources().getDisplayMetrics());
      context.setTheme(style());
      view.dispatchConfigurationChanged(configuration);
    }
  }

  /** Copies host configuration while setting only the browser's night-mode bits. */
  private static Configuration configuration(Configuration original) {
    Configuration configuration = new Configuration(original);
    configuration.uiMode = (configuration.uiMode & ~Configuration.UI_MODE_NIGHT_MASK)
        | (dark ? Configuration.UI_MODE_NIGHT_YES : Configuration.UI_MODE_NIGHT_NO);
    return configuration;
  }

  /** Selects a native theme whose isLightTheme attribute drives browser media queries. */
  private static int style() {
    return dark ? android.R.style.Theme_Material : android.R.style.Theme_Material_Light;
  }
}
