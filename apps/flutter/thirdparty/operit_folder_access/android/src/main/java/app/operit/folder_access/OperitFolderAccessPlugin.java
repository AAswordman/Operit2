package app.operit.folder_access;

import android.app.Activity;
import android.content.Intent;
import android.net.Uri;
import android.provider.DocumentsContract;
import android.database.Cursor;
import java.util.HashMap;
import java.util.Map;
import androidx.annotation.NonNull;
import io.flutter.embedding.engine.plugins.FlutterPlugin;
import io.flutter.embedding.engine.plugins.activity.ActivityAware;
import io.flutter.embedding.engine.plugins.activity.ActivityPluginBinding;
import io.flutter.plugin.common.MethodCall;
import io.flutter.plugin.common.MethodChannel;
import io.flutter.plugin.common.PluginRegistry;

/** Keeps SAF grants instead of attempting to turn document capabilities into paths. */
public final class OperitFolderAccessPlugin implements FlutterPlugin, ActivityAware,
        MethodChannel.MethodCallHandler, PluginRegistry.ActivityResultListener {
    private static final int REQUEST = 45120;
    private MethodChannel channel;
    private ActivityPluginBinding binding;
    private MethodChannel.Result pending;

    @Override public void onAttachedToEngine(@NonNull FlutterPluginBinding engine) {
        channel = new MethodChannel(engine.getBinaryMessenger(), "operit/folder_access");
        channel.setMethodCallHandler(this);
    }
    @Override public void onDetachedFromEngine(@NonNull FlutterPluginBinding engine) {
        cancelPending();
        channel.setMethodCallHandler(null);
        channel = null;
    }
    @Override public void onAttachedToActivity(@NonNull ActivityPluginBinding activity) {
        binding = activity;
        activity.addActivityResultListener(this);
    }
    @Override public void onDetachedFromActivityForConfigChanges() { detach(); }
    @Override public void onReattachedToActivityForConfigChanges(@NonNull ActivityPluginBinding activity) { onAttachedToActivity(activity); }
    @Override public void onDetachedFromActivity() { detach(); cancelPending(); }
    private void detach() {
        if (binding != null) binding.removeActivityResultListener(this);
        binding = null;
    }
    private void cancelPending() {
        if (pending != null) pending.error("PICKER_DETACHED", "Workspace picker was detached", null);
        pending = null;
    }
    @Override public void onMethodCall(@NonNull MethodCall call, @NonNull MethodChannel.Result result) {
        if (!call.method.equals("pickWorkspaceDirectory")) { result.notImplemented(); return; }
        if (binding == null) { result.error("NO_ACTIVITY", "No activity for workspace selection", null); return; }
        if (pending != null) { result.error("PICK_IN_PROGRESS", "Workspace picker is already open", null); return; }
        pending = result;
        try {
            Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT_TREE);
            intent.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION
                    | Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION | Intent.FLAG_GRANT_PREFIX_URI_PERMISSION);
            String initial = call.argument("initialDirectory");
            if (android.os.Build.VERSION.SDK_INT >= 26 && initial != null && initial.startsWith("content://"))
                intent.putExtra(DocumentsContract.EXTRA_INITIAL_URI, Uri.parse(initial));
            binding.getActivity().startActivityForResult(intent, REQUEST);
        } catch (Exception error) {
            pending = null;
            result.error("PICK_FAILED", error.getMessage(), null);
        }
    }
    @Override public boolean onActivityResult(int requestCode, int resultCode, Intent data) {
        if (requestCode != REQUEST) return false;
        MethodChannel.Result result = pending;
        pending = null;
        if (result == null) return true;
        if (resultCode != Activity.RESULT_OK) { result.success(null); return true; }
        try {
            Uri uri = data == null ? null : data.getData();
            if (uri == null || !DocumentsContract.isTreeUri(uri)) throw new IllegalArgumentException("Missing document tree URI");
            int flags = data.getFlags() & (Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
            if ((flags & Intent.FLAG_GRANT_READ_URI_PERMISSION) == 0) throw new SecurityException("Document provider did not grant read access");
            binding.getActivity().getContentResolver().takePersistableUriPermission(uri, flags);
            String name = "Documents";
            Uri document = DocumentsContract.buildDocumentUriUsingTree(uri, DocumentsContract.getTreeDocumentId(uri));
            try (Cursor cursor = binding.getActivity().getContentResolver().query(document,
                    new String[]{DocumentsContract.Document.COLUMN_DISPLAY_NAME}, null, null, null)) {
                if (cursor != null && cursor.moveToFirst() && cursor.getString(0) != null) name = cursor.getString(0);
            }
            Map<String, String> source = new HashMap<>();
            source.put("backend", "android_documents");
            source.put("root", uri.toString());
            source.put("namespace", "/virtual/android/documents");
            source.put("name", name);
            result.success(source);
        } catch (Exception error) { result.error("TREE_GRANT_FAILED", error.getMessage(), null); }
        return true;
    }
}
