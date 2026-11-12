package com.operit.edgephone;

import android.app.Activity;
import android.app.AlertDialog;
import android.content.ComponentName;
import android.content.Intent;
import android.content.ServiceConnection;
import android.os.Build;
import android.os.Bundle;
import android.os.IBinder;
import android.text.InputType;
import android.view.View;
import android.view.WindowManager;
import android.widget.Button;
import android.widget.CheckBox;
import android.widget.EditText;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;
import java.net.InetAddress;
import java.net.NetworkInterface;
import java.util.Collections;

/** Old-phone-first UI, using Android framework widgets without Flutter or WebView. */
public final class MainActivity extends Activity implements EdgeService.Listener {
    private EdgeService service;
    private JSONObject snapshot;
    private FaceView face;
    private TextView info;
    private TextView addresses;
    private TextView operation;
    private LinearLayout dynamic;
    private String dynamicKey = "";
    private boolean settings;
    private boolean busy;
    private boolean bound;
    private String pendingPairing;
    private AlertDialog pairingDialog;
    private final ServiceConnection connection = new ServiceConnection() {
        /** Attaches to the existing foreground node after Activity creation or rotation. */
        @Override public void onServiceConnected(ComponentName name, IBinder binder) {
            service = ((EdgeService.LocalBinder) binder).service();
            service.attach(MainActivity.this);
        }
        /** Shows service termination explicitly without resetting persisted credentials. */
        @Override public void onServiceDisconnected(ComponentName name) {
            service = null;
            changed(null, "节点服务已断开");
        }
    };

    /** Opens the face immediately while the node starts on its own worker. */
    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        View content = findViewById(android.R.id.content);
        content.setOnApplyWindowInsetsListener((view, insets) -> {
            view.setPadding(insets.getSystemWindowInsetLeft(), insets.getSystemWindowInsetTop(),
                insets.getSystemWindowInsetRight(), insets.getSystemWindowInsetBottom());
            return insets.consumeSystemWindowInsets();
        });
        content.requestApplyInsets();
        if (state != null) {
            settings = state.getBoolean("settings");
            pendingPairing = state.getString("pairing");
        }
        showScreen();
        Intent intent = new Intent(this, EdgeService.class);
        if (Build.VERSION.SDK_INT >= 26) startForegroundService(intent); else startService(intent);
        if (Build.VERSION.SDK_INT >= 33 && checkSelfPermission("android.permission.POST_NOTIFICATIONS") != android.content.pm.PackageManager.PERMISSION_GRANTED) {
            requestPermissions(new String[]{"android.permission.POST_NOTIFICATIONS"}, 1);
        }
    }

    /** Observes local state only while this Activity is visible. */
    @Override protected void onStart() {
        super.onStart();
        bound = bindService(new Intent(this, EdgeService.class), connection, BIND_AUTO_CREATE);
    }

    /** Releases only the UI binding, preserving the service-owned Edge identity. */
    @Override protected void onStop() {
        if (service != null) service.detach(this);
        service = null;
        if (bound) { unbindService(connection); bound = false; }
        super.onStop();
    }

    /** Preserves the current management screen and pending pairing transaction. */
    @Override protected void onSaveInstanceState(Bundle state) {
        state.putBoolean("settings", settings);
        state.putString("pairing", pendingPairing);
        super.onSaveInstanceState(state);
    }

    /** Returns from settings without ending the foreground device node. */
    @Override public void onBackPressed() {
        if (settings) { settings = false; showScreen(); } else super.onBackPressed();
    }

    /** Updates expression and management snapshots without rebuilding editable fields. */
    @Override public void changed(JSONObject status, String error) {
        snapshot = status;
        if (error != null) { if (info != null) info.setText(error); return; }
        if (status == null) return;
        JSONObject pending = status.optJSONObject("pendingPairing");
        pendingPairing = pending == null ? null : pending.optString("pairingId");
        if (face != null) face.expression(status.optString("expression"));
        if (info != null) info.setText(getString(R.string.node_info, status.optString("nodeId"), status.optString("spaceName")));
        if (settings) updateSettings(status);
        if (pendingPairing != null && pairingDialog == null) pairingCode(pendingPairing);
    }

    /** Creates either the full-screen face or a compact native management page. */
    private void showScreen() {
        face = null; info = null; addresses = null; dynamic = null; operation = null; dynamicKey = "";
        LinearLayout root = new LinearLayout(this);
        root.setOrientation(LinearLayout.VERTICAL);
        int padding = dp(16);
        if (!settings) {
            face = new FaceView(this);
            root.addView(face, new LinearLayout.LayoutParams(-1, 0, 1));
            info = text(root, getString(R.string.starting));
            button(root, R.string.settings, () -> { settings = true; showScreen(); });
            setContentView(root);
        } else {
            root.setPadding(padding, padding, padding, padding);
            button(root, R.string.back_to_face, () -> { settings = false; showScreen(); });
            info = text(root, getString(R.string.starting));
            text(root, getString(R.string.local_addresses));
            addresses = text(root, ""); addresses.setTextIsSelectable(true);
            text(root, getString(R.string.pairing_help));
            EditText address = new EditText(this);
            address.setSingleLine(true); address.setHint(R.string.address_hint);
            address.setInputType(InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS);
            root.addView(address);
            button(root, R.string.pair, () -> run(with("startPairing", "address", address.getText().toString()), data -> {
                JSONObject pairing = data.optJSONObject("pairing");
                if (pairing != null) {
                    pendingPairing = pairing.optString("pairingId");
                    if (pairingDialog == null) pairingCode(pendingPairing);
                }
            }));
            CheckBox keepScreen = new CheckBox(this);
            keepScreen.setText(R.string.keep_screen);
            keepScreen.setChecked(getPreferences(0).getBoolean("keepScreen", false));
            keepScreen.setOnCheckedChangeListener((button, checked) -> { getPreferences(0).edit().putBoolean("keepScreen", checked).apply(); screenPolicy(); });
            root.addView(keepScreen);
            button(root, R.string.refresh, () -> run(EdgeService.command("status"), data -> { dynamicKey = ""; changed(data, null); }));
            operation = text(root, "");
            dynamic = new LinearLayout(this); dynamic.setOrientation(LinearLayout.VERTICAL); root.addView(dynamic);
            button(root, R.string.leave, () -> confirm(R.string.confirm_leave, () -> run(EdgeService.command("leave"), null)));
            button(root, R.string.stop, () -> {
                if (service == null) { stopService(new Intent(this, EdgeService.class)); finish(); return; }
                run(EdgeService.command("stop"), data -> { stopService(new Intent(this, EdgeService.class)); finish(); });
            });
            ScrollView scroll = new ScrollView(this); scroll.setFillViewport(true); scroll.addView(root); setContentView(scroll);
        }
        screenPolicy();
        if (snapshot != null) changed(snapshot, null);
    }

    /** Displays only active interfaces rather than advertising the wildcard bind address. */
    private String localAddresses(String listener) throws Exception {
        int separator = listener.lastIndexOf(':');
        if (separator < 0) throw new IllegalStateException("节点监听地址无效");
        String port = listener.substring(separator + 1);
        StringBuilder result = new StringBuilder();
        for (NetworkInterface network : Collections.list(NetworkInterface.getNetworkInterfaces())) {
            if (!network.isUp() || network.isLoopback()) continue;
            for (InetAddress address : Collections.list(network.getInetAddresses())) {
                if (address.isLoopbackAddress() || address.isLinkLocalAddress()) continue;
                String host = address.getHostAddress();
                if (address instanceof java.net.Inet6Address) host = "[" + host + "]";
                result.append(host).append(':').append(port).append('\n');
            }
        }
        return result.length() == 0 ? getString(R.string.no_address) : result.toString().trim();
    }

    /** Rebuilds management rows only when pairing or consent state actually changes. */
    private void updateSettings(JSONObject state) {
        try { addresses.setText(localAddresses(state.optString("address"))); }
        catch (Exception error) { addresses.setText(error.toString()); }
        JSONArray peers = state.optJSONArray("peers"), prompts = state.optJSONArray("prompts"), joins = state.optJSONArray("joins");
        String key = String.valueOf(peers) + prompts + joins;
        if (key.equals(dynamicKey)) return;
        dynamicKey = key;
        dynamic.removeAllViews();
        if (prompts != null) for (int i = 0; i < prompts.length(); i++) {
            JSONObject prompt = prompts.optJSONObject(i);
            if (prompt != null) text(dynamic, getString(R.string.incoming_code, prompt.optString("displayName"), prompt.optString("confirmationCode")));
        }
        if (peers != null) for (int i = 0; i < peers.length(); i++) {
            JSONObject peer = peers.optJSONObject(i);
            if (peer == null) continue;
            String node = peer.optString("nodeId");
            text(dynamic, peer.optString("displayName") + "\n" + node);
            if (peer.optBoolean("outbound")) button(dynamic, R.string.join, () -> run(with("join", "node", node), null));
            button(dynamic, R.string.unpair, () -> confirm(R.string.confirm_unpair, () -> run(with("unpair", "node", node), null)));
        }
        if (joins != null) for (int i = 0; i < joins.length(); i++) {
            JSONObject join = joins.optJSONObject(i);
            if (join == null) continue;
            String id = join.optString("requestId"), status = join.optString("status");
            text(dynamic, join.optString("spaceName") + " · " + joinLabel(status));
            if ("pending".equals(status) || "approving".equals(status) || "approved".equals(status)) {
                button(dynamic, R.string.refresh_join, () -> run(with("refreshJoin", "id", id), null));
                button(dynamic, R.string.cancel_join, () -> run(with("cancelJoin", "id", id), null));
            }
        }
    }

    /** Maps the shared SDK consent states to human-readable text. */
    private String joinLabel(String status) {
        switch (status) {
            case "pending": return "等待审批";
            case "approving": case "approved": return "审批通过，点击刷新完成加入";
            case "joined": return "已加入";
            case "rejected": return "已拒绝";
            case "cancelled": return "已取消";
            case "expired": return "已过期";
            default: throw new IllegalArgumentException("未知的空间申请状态：" + status);
        }
    }

    /** Keeps a failed confirmation editable without creating another pairing transaction. */
    private void pairingCode(String id) {
        EditText code = new EditText(this); code.setSingleLine(true);
        code.setInputType(InputType.TYPE_CLASS_NUMBER); code.setHint(R.string.pair_code);
        pairingDialog = new AlertDialog.Builder(this).setTitle(R.string.pair_code).setView(code)
            .setPositiveButton(R.string.confirm, null).setNegativeButton(R.string.cancel, (dialog, which) -> {
                pendingPairing = null; run(with("cancelPairing", "id", id), null);
            }).create();
        pairingDialog.setCanceledOnTouchOutside(false);
        pairingDialog.setOnCancelListener(dialog -> { pendingPairing = null; run(with("cancelPairing", "id", id), null); });
        pairingDialog.setOnDismissListener(dialog -> pairingDialog = null);
        pairingDialog.setOnShowListener(dialog -> pairingDialog.getButton(AlertDialog.BUTTON_POSITIVE).setOnClickListener(view -> {
            String value = code.getText().toString();
            if (!value.matches("[0-9]{6}")) { code.setError(getString(R.string.pair_code)); return; }
            JSONObject command = with("finishPairing", "id", id);
            try { command.put("code", value); } catch (JSONException error) { throw new IllegalArgumentException(error); }
            run(command, data -> { pendingPairing = null; if (pairingDialog != null) pairingDialog.dismiss(); });
        }));
        pairingDialog.show();
    }

    interface Success { void complete(JSONObject data); }
    /** Prevents repeated taps from replaying pairing, admission or departure actions. */
    private void run(JSONObject command, Success success) {
        if (busy) return;
        if (service == null) { showError("节点服务未连接"); return; }
        busy = true;
        if (operation != null) operation.setText("正在处理…");
        service.request(command, (data, error) -> {
            busy = false;
            if (isFinishing() || isDestroyed()) return;
            if (operation != null) operation.setText(error == null ? "操作完成" : error);
            if (error != null) showError(error);
            else if (success != null) success.complete(data);
        });
    }

    /** Applies the explicit screen-on preference only to the visible face. */
    private void screenPolicy() {
        if (!settings && getPreferences(0).getBoolean("keepScreen", false)) getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        else getWindow().clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
    }
    /** Builds a command field without constructing executable text. */
    private JSONObject with(String action, String key, String value) {
        JSONObject command = EdgeService.command(action);
        try { command.put(key, value); } catch (JSONException error) { throw new IllegalArgumentException(error); }
        return command;
    }
    /** Confirms local destructive membership and pairing changes. */
    private void confirm(int title, Runnable action) {
        new AlertDialog.Builder(this).setTitle(title).setPositiveButton(R.string.confirm, (dialog, which) -> action.run()).setNegativeButton(R.string.cancel, null).show();
    }
    /** Reports actual native errors without presenting an operation as successful. */
    private void showError(String error) { new AlertDialog.Builder(this).setTitle(R.string.error_title).setMessage(error).setPositiveButton(android.R.string.ok, null).show(); }
    /** Adds a wrapping text row suitable for small and low-density phone displays. */
    private TextView text(LinearLayout parent, String value) {
        TextView view = new TextView(this); view.setText(value); view.setTextSize(14); view.setPadding(dp(8), dp(8), dp(8), dp(8)); parent.addView(view); return view;
    }
    /** Adds a native minimum-touch-size action row. */
    private void button(LinearLayout parent, int title, Runnable action) {
        Button button = new Button(this); button.setText(title); button.setMinHeight(dp(48)); button.setOnClickListener(view -> action.run()); parent.addView(button, new LinearLayout.LayoutParams(-1, -2));
    }
    /** Converts logical dimensions for old phones with different screen densities. */
    private int dp(int value) { return Math.round(value * getResources().getDisplayMetrics().density); }
}
