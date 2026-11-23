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
import io.flutter.plugin.common.EventChannel;

/**
 * Java platform implementation of the webview_flutter plugin.
 *
 * <p>Register this in an add to app scenario to gracefully handle activity and context changes.
 */
public class WebViewFlutterPlugin implements FlutterPlugin, ActivityAware {
  private FlutterPluginBinding pluginBinding;
  private ProxyApiRegistrar proxyApiRegistrar;
  private MethodChannel themeChannel;
  private MethodChannel visitChannel;
  private EventChannel visitEvents;

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
    HostWebVisitManager visits = HostWebVisitManager.getInstance();
    visitChannel = new MethodChannel(binding.getBinaryMessenger(), "operit/webview_visits");
    visitChannel.setMethodCallHandler((call, result) -> {
      if (call.method.equals("visit")) {
        visits.visit(binding.getApplicationContext(), call.arguments, result);
      } else if (call.method.equals("control")) {
        try {
          visits.control(call.argument("requestId"), call.argument("action"));
          result.success(null);
        } catch (Exception error) { result.error("web_visit", error.toString(), null); }
      } else { result.notImplemented(); }
    });
    visitEvents = new EventChannel(binding.getBinaryMessenger(), "operit/webview_visit_events");
    visitEvents.setStreamHandler(new EventChannel.StreamHandler() {
      /** Replays active process-owned visits after a Flutter engine connects. */
      @Override public void onListen(Object arguments, EventChannel.EventSink sink) {
        visits.observe(WebViewFlutterPlugin.this, sink);
      }
      /** Unsubscribes this presentation engine without cancelling tasks. */
      @Override public void onCancel(Object arguments) { visits.unobserve(WebViewFlutterPlugin.this); }
    });
    binding.getPlatformViewRegistry().registerViewFactory("operit/host_web_visit", visits.surfaceFactory(this));
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
    HostWebVisitManager.getInstance().unobserve(this);
    HostWebVisitManager.getInstance().releaseSurfaces(this);
    visitChannel.setMethodCallHandler(null);
    visitEvents.setStreamHandler(null);
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
