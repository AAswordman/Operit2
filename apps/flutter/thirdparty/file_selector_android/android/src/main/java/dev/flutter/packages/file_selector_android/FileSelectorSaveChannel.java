// Copyright 2026 The Operit Authors.
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

package dev.flutter.packages.file_selector_android;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import android.os.Build;
import android.provider.DocumentsContract;
import androidx.annotation.NonNull;
import androidx.annotation.Nullable;
import io.flutter.embedding.engine.plugins.activity.ActivityPluginBinding;
import io.flutter.plugin.common.BinaryMessenger;
import io.flutter.plugin.common.MethodCall;
import io.flutter.plugin.common.MethodChannel;
import io.flutter.plugin.common.PluginRegistry;
import java.io.OutputStream;

/** Saves byte payloads to Android Storage Access Framework documents. */
final class FileSelectorSaveChannel
    implements MethodChannel.MethodCallHandler, PluginRegistry.ActivityResultListener {
  private static final String CHANNEL_NAME = "dev.flutter.packages.file_selector_android/save";
  private static final int SAVE_FILE_REQUEST_CODE = 46092;

  private final MethodChannel channel;
  @Nullable private ActivityPluginBinding activityPluginBinding;
  @Nullable private PendingSave pendingSave;
  private boolean resultListenerAttached;

  /** Creates the channel that receives unified file-selector save operations. */
  FileSelectorSaveChannel(@NonNull BinaryMessenger binaryMessenger) {
    channel = new MethodChannel(binaryMessenger, CHANNEL_NAME);
    channel.setMethodCallHandler(this);
  }

  /** Updates the activity that owns save-document result delivery. */
  void setActivityPluginBinding(@Nullable ActivityPluginBinding binding) {
    detachResultListener();
    activityPluginBinding = binding;
    attachResultListener();
  }

  /** Releases MethodChannel and activity result resources. */
  void close() {
    detachResultListener();
    activityPluginBinding = null;
    pendingSave = null;
    channel.setMethodCallHandler(null);
  }

  /** Routes one file-selector save invocation from Dart. */
  @Override
  public void onMethodCall(@NonNull MethodCall call, @NonNull MethodChannel.Result result) {
    if (call.method.equals("saveFile")) {
      saveFile(call, result);
      return;
    }
    result.notImplemented();
  }

  /** Opens the Android document creator for the requested byte payload. */
  private void saveFile(@NonNull MethodCall call, @NonNull MethodChannel.Result result) {
    if (pendingSave != null) {
      result.error("SAVE_IN_PROGRESS", "A document save is already active", null);
      return;
    }
    final byte[] bytes = call.argument("bytes");
    final String name = call.argument("name");
    final String mimeType = call.argument("mimeType");
    final String initialDirectory = call.argument("initialDirectory");
    final ActivityPluginBinding binding = activityPluginBinding;
    if (bytes == null || name == null || name.isEmpty() || mimeType == null || mimeType.isEmpty()) {
      result.error("INVALID_SAVE_ARGS", "bytes, name, and mimeType are required", null);
      return;
    }
    if (binding == null) {
      result.error("NO_ACTIVITY", "No activity is available for document saving", null);
      return;
    }
    pendingSave = new PendingSave(bytes, result);
    attachResultListener();
    try {
      final Intent intent = new Intent(Intent.ACTION_CREATE_DOCUMENT);
      intent.addCategory(Intent.CATEGORY_OPENABLE);
      intent.setType(mimeType);
      intent.putExtra(Intent.EXTRA_TITLE, name);
      if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O && initialDirectory != null) {
        intent.putExtra(DocumentsContract.EXTRA_INITIAL_URI, Uri.parse(initialDirectory));
      }
      binding.getActivity().startActivityForResult(intent, SAVE_FILE_REQUEST_CODE);
    } catch (Exception exception) {
      clearPendingSave();
      result.error("SAVE_START_FAILED", exception.getMessage(), null);
    }
  }

  /** Writes the pending payload after Android returns a selected document URI. */
  @Override
  public boolean onActivityResult(int requestCode, int resultCode, @Nullable Intent data) {
    if (requestCode != SAVE_FILE_REQUEST_CODE) {
      return false;
    }
    final PendingSave save = takePendingSave();
    if (save == null) {
      return true;
    }
    if (resultCode != Activity.RESULT_OK) {
      save.result.success(null);
      return true;
    }
    final Uri uri = data == null ? null : data.getData();
    if (uri == null) {
      save.result.error("MISSING_DOCUMENT", "Document picker returned no URI", null);
      return true;
    }
    final ActivityPluginBinding binding = activityPluginBinding;
    if (binding == null) {
      save.result.error("NO_ACTIVITY", "No activity is available for document saving", null);
      return true;
    }
    try (OutputStream output = binding.getActivity().getContentResolver().openOutputStream(uri, "w")) {
      if (output == null) {
        throw new IllegalStateException("Unable to open selected document for writing");
      }
      output.write(save.bytes);
      output.flush();
      save.result.success(uri.toString());
    } catch (Exception exception) {
      save.result.error("SAVE_WRITE_FAILED", exception.getMessage(), null);
    }
    return true;
  }

  /** Attaches result delivery while a save request remains active. */
  private void attachResultListener() {
    if (resultListenerAttached || pendingSave == null || activityPluginBinding == null) {
      return;
    }
    activityPluginBinding.addActivityResultListener(this);
    resultListenerAttached = true;
  }

  /** Detaches this channel from the currently bound activity result stream. */
  private void detachResultListener() {
    if (resultListenerAttached && activityPluginBinding != null) {
      activityPluginBinding.removeActivityResultListener(this);
    }
    resultListenerAttached = false;
  }

  /** Removes the active save request without completing its MethodChannel result. */
  private void clearPendingSave() {
    pendingSave = null;
    detachResultListener();
  }

  /** Returns the active save request and releases its result listener. */
  @Nullable
  private PendingSave takePendingSave() {
    final PendingSave save = pendingSave;
    clearPendingSave();
    return save;
  }

  /** Stores one MethodChannel result with the bytes selected for saving. */
  private static final class PendingSave {
    final byte[] bytes;
    final MethodChannel.Result result;

    /** Creates the state held until Android returns from document creation. */
    PendingSave(@NonNull byte[] bytes, @NonNull MethodChannel.Result result) {
      this.bytes = bytes;
      this.result = result;
    }
  }
}
