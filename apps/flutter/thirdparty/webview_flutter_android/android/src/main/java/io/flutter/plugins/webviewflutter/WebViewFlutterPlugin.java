// Copyright 2013 The Flutter Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

package io.flutter.plugins.webviewflutter;

import androidx.annotation.NonNull;
import androidx.annotation.Nullable;
import io.flutter.embedding.engine.plugins.FlutterPlugin;
import io.flutter.embedding.engine.plugins.activity.ActivityAware;
import io.flutter.embedding.engine.plugins.activity.ActivityPluginBinding;
import io.flutter.plugin.common.MethodChannel;

/**
 * Java platform implementation of the webview_flutter plugin.
 *
 * <p>Register this in an add to app scenario to gracefully handle activity and context changes.
 */
public class WebViewFlutterPlugin implements FlutterPlugin, ActivityAware {
  private FlutterPluginBinding pluginBinding;
  private ProxyApiRegistrar proxyApiRegistrar;
  private MethodChannel themeChannel;

  /**
   * Add an instance of this to {@link io.flutter.embedding.engine.plugins.PluginRegistry} to
   * register it.
   *
   * <p>Registration should eventually be handled automatically by v2 of the
   * GeneratedPluginRegistrant. https://github.com/flutter/flutter/issues/42694
   */
  public WebViewFlutterPlugin() {}

  @Override
  public void onAttachedToEngine(@NonNull FlutterPluginBinding binding) {
    pluginBinding = binding;
    themeChannel = new MethodChannel(binding.getBinaryMessenger(), "operit/webview_theme");
    // Applies appearance through native WebView contexts, independently of page JavaScript.
    themeChannel.setMethodCallHandler((call, result) -> {
      if (!call.method.equals("setPreferredColorScheme")) {
        result.notImplemented();
        return;
      }
      if (!"dark".equals(call.arguments) && !"light".equals(call.arguments)) {
        result.error("invalid_color_scheme", "Expected dark or light", null);
        return;
      }
      WebViewTheme.setDark("dark".equals(call.arguments));
      result.success(null);
    });

    proxyApiRegistrar =
        new ProxyApiRegistrar(
            binding.getBinaryMessenger(),
            binding.getApplicationContext(),
            new FlutterAssetManager.PluginBindingFlutterAssetManager(
                binding.getApplicationContext().getAssets(), binding.getFlutterAssets()));

    binding
        .getPlatformViewRegistry()
        .registerViewFactory(
            "plugins.flutter.io/webview",
            new FlutterViewFactory(proxyApiRegistrar.getInstanceManager()));

    proxyApiRegistrar.setUp();
  }

  @Override
  public void onDetachedFromEngine(@NonNull FlutterPluginBinding binding) {
    themeChannel.setMethodCallHandler(null);
    if (proxyApiRegistrar != null) {
      proxyApiRegistrar.externalNavigation.dispose();
      proxyApiRegistrar.userScripts.dispose();
      proxyApiRegistrar.localResources.dispose();
      proxyApiRegistrar.tearDown();
      proxyApiRegistrar.getInstanceManager().stopFinalizationListener();
      proxyApiRegistrar = null;
    }
  }

  @Override
  public void onAttachedToActivity(@NonNull ActivityPluginBinding activityPluginBinding) {
    if (proxyApiRegistrar != null) {
      proxyApiRegistrar.setContext(activityPluginBinding.getActivity());
    }
  }

  @Override
  public void onDetachedFromActivityForConfigChanges() {
    proxyApiRegistrar.setContext(pluginBinding.getApplicationContext());
  }

  @Override
  public void onReattachedToActivityForConfigChanges(
      @NonNull ActivityPluginBinding activityPluginBinding) {
    proxyApiRegistrar.setContext(activityPluginBinding.getActivity());
  }

  @Override
  public void onDetachedFromActivity() {
    proxyApiRegistrar.setContext(pluginBinding.getApplicationContext());
  }

  /** Maintains instances used to communicate with the corresponding objects in Dart. */
  @Nullable
  public AndroidWebkitLibraryPigeonInstanceManager getInstanceManager() {
    return proxyApiRegistrar.getInstanceManager();
  }
}
