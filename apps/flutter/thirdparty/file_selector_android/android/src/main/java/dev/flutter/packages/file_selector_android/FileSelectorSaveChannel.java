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
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.File;
import java.util.List;
import java.io.IOException;
import java.util.HashMap;
import java.util.Map;
import android.os.Handler;
import android.os.Looper;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

/** Saves byte payloads to Android Storage Access Framework documents. */
final class FileSelectorSaveChannel
    implements MethodChannel.MethodCallHandler, PluginRegistry.ActivityResultListener {
  private static final String CHANNEL_NAME = "dev.flutter.packages.file_selector_android/save";
  private static final int SAVE_FILE_REQUEST_CODE = 46092;
  private static final int READ_FILE_REQUEST_CODE = 46093;
  @Nullable private MethodChannel.Result pendingRead;
  private final Handler main = new Handler(Looper.getMainLooper());

  private final MethodChannel channel;
  private final Map<String, File> selectedSources = new HashMap<>();
  private final ExecutorService saveExecutor = Executors.newSingleThreadExecutor();
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
    pendingRead = null;
    channel.setMethodCallHandler(null);
    saveExecutor.execute(() -> {
      for (File source : selectedSources.values()) {
        if (!source.delete()) throw new IllegalStateException("Cannot release selected document: " + source);
      }
      selectedSources.clear();
    });
    saveExecutor.shutdown();
  }

  /** Routes one file-selector save invocation from Dart. */
  @Override
  public void onMethodCall(@NonNull MethodCall call, @NonNull MethodChannel.Result result) {
    switch (call.method) {
      case "releaseSelectedFile":
        releaseSelectedFile(call, result);
        return;
      case "openFileWithProgress":
        openFileWithProgress(call, result);
        return;
      case "saveFile":
        saveFile(call, result);
        return;
      case "saveFileFromPath":
        saveFileFromPath(call, result);
        return;
      default:
        result.notImplemented();
    }
  }

  /** Deletes only a source copy registered by this picker, on the same queue as its creator. */
  private void releaseSelectedFile(MethodCall call, MethodChannel.Result result) {
    final String path = call.argument("path");
    saveExecutor.execute(() -> {
      final File source = selectedSources.get(path);
      if (source == null) {
        main.post(() -> result.error("UNKNOWN_SOURCE", "The source is not owned by this picker", null));
        return;
      }
      if (!source.delete()) {
        main.post(() -> result.error("SOURCE_RELEASE_FAILED", "Cannot remove selected document staging", null));
        return;
      }
      selectedSources.remove(path);
      main.post(() -> result.success(null));
    });
  }

  /** Selects a document whose cache copy reports bounded reads from a worker thread. */
  private void openFileWithProgress(MethodCall call, MethodChannel.Result result) {
    if (pendingRead != null || pendingSave != null) {
      result.error("PICK_IN_PROGRESS", "A document operation is already active", null);
      return;
    }
    final ActivityPluginBinding binding = activityPluginBinding;
    final List<String> mimeTypes = call.argument("mimeTypes");
    if (binding == null || mimeTypes == null || mimeTypes.isEmpty()) {
      result.error("INVALID_OPEN_ARGS", "An activity and MIME types are required", null);
      return;
    }
    pendingRead = result;
    attachResultListener();
    try {
      final Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
      intent.addCategory(Intent.CATEGORY_OPENABLE);
      intent.setType(mimeTypes.size() == 1 ? mimeTypes.get(0) : "*/*");
      intent.putExtra(Intent.EXTRA_MIME_TYPES, mimeTypes.toArray(new String[0]));
      binding.getActivity().startActivityForResult(intent, READ_FILE_REQUEST_CODE);
    } catch (Exception exception) {
      pendingRead = null;
      detachResultListener();
      result.error("PICK_START_FAILED", exception.getMessage(), null);
    }
  }

  /** Copies the selected source without blocking activity callbacks or Flutter rendering. */
  private boolean readActivityResult(int resultCode, @Nullable Intent data) {
    final MethodChannel.Result result = pendingRead;
    pendingRead = null;
    detachResultListener();
    if (result == null) return true;
    if (resultCode != Activity.RESULT_OK) { result.success(null); return true; }
    final Uri uri = data == null ? null : data.getData();
    final ActivityPluginBinding binding = activityPluginBinding;
    if (uri == null || binding == null) {
      result.error("MISSING_SOURCE", "Document selection has no source or activity", null);
      return true;
    }
    final android.content.Context context = binding.getActivity().getApplicationContext();
    saveExecutor.execute(() -> {
      try {
        final File target = File.createTempFile("operit-selected-", ".backup", context.getCacheDir());
        long completed = 0;
        long lastReport = 0;
        reportRead(0, null);
        try (InputStream input = context.getContentResolver().openInputStream(uri);
             FileOutputStream output = new FileOutputStream(target)) {
          if (input == null) throw new IOException("Unable to open selected document");
          final byte[] buffer = new byte[64 * 1024];
          int count;
          while ((count = input.read(buffer)) != -1) {
            output.write(buffer, 0, count);
            completed += count;
            final long now = android.os.SystemClock.elapsedRealtime();
            if (now - lastReport >= 50) { reportRead(completed, null); lastReport = now; }
          }
          output.getFD().sync();
        } catch (Exception exception) {
          if (!target.delete()) throw new IOException("Cannot remove failed document staging", exception);
          throw exception;
        }
        reportRead(completed, completed);
        final Map<String, Object> response = new HashMap<>();
        selectedSources.put(target.getAbsolutePath(), target);
        response.put("path", target.getAbsolutePath());
        response.put("size", completed);
        main.post(() -> result.success(response));
      } catch (Exception exception) {
        main.post(() -> result.error("SOURCE_READ_FAILED", exception.getMessage(), null));
      }
    });
    return true;
  }

  /** Emits actual read counts without trusting provider-declared file lengths. */
  private void reportRead(long completed, @Nullable Long total) {
    final Map<String, Object> values = new HashMap<>();
    values.put("completed", completed);
    values.put("total", total);
    main.post(() -> channel.invokeMethod("readProgress", values));
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
    startDocumentSave(new BytePendingSave(bytes, result), name, mimeType, initialDirectory, binding);
  }

  /** Opens the document creator for a host-owned file reference. */
  private void saveFileFromPath(@NonNull MethodCall call, @NonNull MethodChannel.Result result) {
    if (pendingSave != null) {
      result.error("SAVE_IN_PROGRESS", "A document save is already active", null);
      return;
    }
    final String sourcePath = call.argument("sourcePath");
    final String name = call.argument("name");
    final String mimeType = call.argument("mimeType");
    final String initialDirectory = call.argument("initialDirectory");
    final ActivityPluginBinding binding = activityPluginBinding;
    if (sourcePath == null || sourcePath.isEmpty() || name == null || name.isEmpty()
        || mimeType == null || mimeType.isEmpty()) {
      result.error("INVALID_SAVE_ARGS", "sourcePath, name, and mimeType are required", null);
      return;
    }
    if (binding == null) {
      result.error("NO_ACTIVITY", "No activity is available for document saving", null);
      return;
    }
    startDocumentSave(new FilePendingSave(sourcePath, result, Boolean.TRUE.equals(call.argument("reportProgress")), channel), name, mimeType, initialDirectory, binding);
  }

  /** Starts one explicitly typed save request through the Android document creator. */
  private void startDocumentSave(PendingSave save, String name, String mimeType,
      @Nullable String initialDirectory, ActivityPluginBinding binding) {
    pendingSave = save;
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
      save.result.error("SAVE_START_FAILED", exception.getMessage(), null);
    }
  }

  /** Writes the pending payload after Android returns a selected document URI. */
  @Override
  public boolean onActivityResult(int requestCode, int resultCode, @Nullable Intent data) {
    if (requestCode == READ_FILE_REQUEST_CODE) return readActivityResult(resultCode, data);
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
    final android.content.ContentResolver resolver = binding.getActivity().getContentResolver();
    saveExecutor.execute(() -> {
      try (OutputStream output = resolver.openOutputStream(uri, "w")) {
        if (output == null) {
          throw new IllegalStateException("Unable to open selected document for writing");
        }
        save.writeTo(output);
        output.flush();
      } catch (Exception exception) {
        main.post(() -> save.result.error("SAVE_WRITE_FAILED", exception.getMessage(), null));
        return;
      }
      main.post(() -> save.result.success(uri.toString()));
    });
    return true;
  }

  /** Attaches result delivery while a save request remains active. */
  private void attachResultListener() {
    if (resultListenerAttached || (pendingSave == null && pendingRead == null) || activityPluginBinding == null) {
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

  /** Owns one explicitly typed save source and its MethodChannel completion. */
  private abstract static class PendingSave {
    final MethodChannel.Result result;

    /** Retains the completion until the document has been written. */
    PendingSave(MethodChannel.Result result) {
      this.result = result;
    }

    /** Writes this source into the selected document. */
    abstract void writeTo(OutputStream output) throws IOException;
  }

  /** Preserves the existing byte-payload save operation for its callers. */
  private static final class BytePendingSave extends PendingSave {
    private final byte[] bytes;

    /** Stores the byte payload for an explicitly requested byte save. */
    BytePendingSave(byte[] bytes, MethodChannel.Result result) {
      super(result);
      this.bytes = bytes;
    }

    /** Writes the caller-owned byte payload. */
    @Override
    void writeTo(OutputStream output) throws IOException {
      output.write(bytes);
    }
  }

  /** Streams a host-owned snapshot file without allocating an archive-sized array. */
  private static final class FilePendingSave extends PendingSave {
    private final String sourcePath;
    private final boolean reportProgress;
    private final MethodChannel channel;
    private final Handler main = new Handler(Looper.getMainLooper());

    /** Stores only the source file path until the output document is selected. */
    FilePendingSave(String sourcePath, MethodChannel.Result result, boolean reportProgress, MethodChannel channel) {
      super(result);
      this.sourcePath = sourcePath;
      this.reportProgress = reportProgress;
      this.channel = channel;
    }

    /** Copies the source file using a fixed-size buffer. */
    @Override
    void writeTo(OutputStream output) throws IOException {
      try (FileInputStream input = new FileInputStream(sourcePath)) {
        final long total = input.getChannel().size();
        long completed = 0;
        long lastReport = 0;
        progress(0, total);
        final byte[] buffer = new byte[64 * 1024];
        int count;
        while ((count = input.read(buffer)) != -1) {
          output.write(buffer, 0, count);
          completed += count;
          final long now = android.os.SystemClock.elapsedRealtime();
          if (now - lastReport >= 50) { progress(completed, total); lastReport = now; }
        }
        if (completed != total) throw new IOException("Source changed during save");
        output.flush();
        progress(completed, total);
      }
    }

    /** Posts only measured copy counters on the Flutter platform thread. */
    private void progress(long completed, long total) {
      if (!reportProgress) return;
      final Map<String, Object> values = new HashMap<>();
      values.put("completed", completed);
      values.put("total", total);
      main.post(() -> channel.invokeMethod("saveProgress", values));
    }
  }
}
