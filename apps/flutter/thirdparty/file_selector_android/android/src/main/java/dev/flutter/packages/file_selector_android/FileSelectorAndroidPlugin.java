// Copyright 2013 The Flutter Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

package dev.flutter.packages.file_selector_android;

import androidx.annotation.NonNull;
import androidx.annotation.Nullable;
import io.flutter.embedding.engine.plugins.FlutterPlugin;
import io.flutter.embedding.engine.plugins.activity.ActivityAware;
import io.flutter.embedding.engine.plugins.activity.ActivityPluginBinding;

/** Native portion of the Android platform implementation of the file_selector plugin. */
public class FileSelectorAndroidPlugin implements FlutterPlugin, ActivityAware {
  @Nullable private FileSelectorApiImpl fileSelectorApi;
  @Nullable private FileSelectorSaveChannel fileSelectorSaveChannel;
  private FlutterPluginBinding pluginBinding;

  @Override
  public void onAttachedToEngine(@NonNull FlutterPluginBinding binding) {
    pluginBinding = binding;
    fileSelectorSaveChannel = new FileSelectorSaveChannel(binding.getBinaryMessenger());
  }

  @Override
  public void onDetachedFromEngine(@NonNull FlutterPluginBinding binding) {
    if (fileSelectorSaveChannel != null) {
      fileSelectorSaveChannel.close();
      fileSelectorSaveChannel = null;
    }
    pluginBinding = null;
  }

  @Override
  public void onAttachedToActivity(@NonNull ActivityPluginBinding binding) {
    fileSelectorApi = new FileSelectorApiImpl(binding);
    FileSelectorApi.Companion.setUp(pluginBinding.getBinaryMessenger(), fileSelectorApi);
    if (fileSelectorSaveChannel != null) {
      fileSelectorSaveChannel.setActivityPluginBinding(binding);
    }
  }

  @Override
  public void onDetachedFromActivityForConfigChanges() {
    if (fileSelectorApi != null) {
      fileSelectorApi.setActivityPluginBinding(null);
    }
    if (fileSelectorSaveChannel != null) {
      fileSelectorSaveChannel.setActivityPluginBinding(null);
    }
  }

  @Override
  public void onReattachedToActivityForConfigChanges(@NonNull ActivityPluginBinding binding) {
    if (fileSelectorApi != null) {
      fileSelectorApi.setActivityPluginBinding(binding);
    } else {
      fileSelectorApi = new FileSelectorApiImpl(binding);
      FileSelectorApi.Companion.setUp(pluginBinding.getBinaryMessenger(), fileSelectorApi);
    }
    if (fileSelectorSaveChannel != null) {
      fileSelectorSaveChannel.setActivityPluginBinding(binding);
    }
  }

  @Override
  public void onDetachedFromActivity() {
    if (fileSelectorApi != null) {
      fileSelectorApi.setActivityPluginBinding(null);
    }
    if (fileSelectorSaveChannel != null) {
      fileSelectorSaveChannel.setActivityPluginBinding(null);
    }
  }
}
