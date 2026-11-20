package com.operit.edgephone;

import android.Manifest;
import android.app.Activity;
import android.app.AlertDialog;
import android.content.ComponentName;
import android.content.Intent;
import android.content.ServiceConnection;
import android.content.pm.PackageManager;
import android.graphics.drawable.GradientDrawable;
import android.os.Build;
import android.os.Bundle;
import android.os.IBinder;
import android.os.SystemClock;
import android.text.InputFilter;
import android.text.InputType;
import android.text.TextUtils;
import android.view.Gravity;
import android.view.HapticFeedbackConstants;
import android.view.InputDevice;
import android.view.MotionEvent;
import android.view.View;
import android.view.WindowManager;
import android.widget.Button;
import android.widget.EditText;
import android.widget.FrameLayout;
import android.widget.LinearLayout;
import android.widget.ScrollView;
import android.widget.TextView;
import org.json.JSONArray;
import org.json.JSONException;
import org.json.JSONObject;

/** Owns the watch-first presentation while the foreground service owns the node. */
public final class MainActivity extends Activity implements EdgeService.Listener {
    private enum Page { HOME, SCAN, INVITATION, DEVICES, MANUAL, CODE }
    private static final int SCAN_PERMISSION = 1;

    private EdgeService service;
    private JSONObject snapshot;
    private String startupError;
    private Page page = Page.HOME;
    private PetView pet;
    private QrScannerView scanner;
    private FrameLayout scannerFrame;
    private String invitationCode;
    private String invitationName;
    private String invitationNode;
    private TextView statusText;
    private TextView operation;
    private TextView addressList;
    private Button primary;
    private LinearLayout deviceRows;
    private String deviceKey;
    private long nextJoinCheck;
    private boolean joinChecking;
    private boolean joinCheckFailed;
    private boolean busy;
    private boolean bound;
    private boolean round;
    private boolean hasCamera;
    private boolean resumed;
    private int generation;

    private final ServiceConnection connection = new ServiceConnection() {
        /** Observes the existing foreground node on this display. */
        @Override public void onServiceConnected(ComponentName name, IBinder binder) {
            service = ((EdgeService.LocalBinder) binder).service();
            service.attach(MainActivity.this);
        }

        /** Keeps the local pet usable and describes the lost service explicitly. */
        @Override public void onServiceDisconnected(ComponentName name) {
            service = null;
            changed(null, "节点服务已断开");
        }
    };

    /** Opens the pet first and applies physical screen insets before drawing controls. */
    @Override protected void onCreate(Bundle state) {
        super.onCreate(state);
        round = Build.VERSION.SDK_INT >= 23 && getResources().getConfiguration().isScreenRound();
        hasCamera = getPackageManager().hasSystemFeature(PackageManager.FEATURE_CAMERA_ANY);
        if (state != null) page = Page.valueOf(state.getString("page", Page.HOME.name()));
        View content = findViewById(android.R.id.content);
        content.setOnApplyWindowInsetsListener((view, insets) -> {
            view.setPadding(insets.getSystemWindowInsetLeft(), insets.getSystemWindowInsetTop(),
                insets.getSystemWindowInsetRight(), insets.getSystemWindowInsetBottom());
            return insets.consumeSystemWindowInsets();
        });
        content.requestApplyInsets();
        show(page);
        Intent intent = new Intent(this, EdgeService.class);
        if (Build.VERSION.SDK_INT >= 26) startForegroundService(intent);
        else startService(intent);
    }

    /** Subscribes to display state only while the app is visible. */
    @Override protected void onStart() {
        super.onStart();
        bound = bindService(new Intent(this, EdgeService.class), connection, BIND_AUTO_CREATE);
    }

    /** Opens the camera only while a scan page has foreground focus. */
    @Override protected void onResume() {
        super.onResume();
        resumed = true;
        if (scanner != null) scanner.setScanning(true);
    }

    /** Releases the scan preview immediately when leaving this Activity. */
    @Override protected void onPause() {
        resumed = false;
        if (scanner != null) scanner.setScanning(false);
        super.onPause();
    }

    /** Releases the UI observer while preserving the active edge node. */
    @Override protected void onStop() {
        if (service != null) service.detach(this);
        service = null;
        if (bound) {
            unbindService(connection);
            bound = false;
        }
        super.onStop();
    }

    /** Restores the current screen after configuration changes without persisting a QR secret. */
    @Override protected void onSaveInstanceState(Bundle state) {
        state.putString("page", (page == Page.INVITATION ? Page.SCAN : page).name());
        super.onSaveInstanceState(state);
    }

    /** Uses the system back gesture between focused watch screens. */
    @Override public void onBackPressed() {
        switch (page) {
            case HOME: super.onBackPressed(); break;
            case SCAN: show(Page.DEVICES); break;
            case INVITATION: if (!busy) show(Page.SCAN); break;
            case MANUAL: show(Page.DEVICES); break;
            case CODE: cancelCode(); break;
            case DEVICES: show(Page.HOME); break;
        }
    }

    /** Applies node state without rebuilding the pet, scanner or a focused input field. */
    @Override public void changed(JSONObject state, String error) {
        snapshot = state;
        startupError = error;
        if (page == Page.MANUAL && state != null && state.optJSONObject("pendingPairing") != null) {
            show(Page.CODE);
            return;
        }
        if (page == Page.CODE && state != null && state.optJSONObject("pendingPairing") == null) {
            show(Page.DEVICES);
            return;
        }
        if (page == Page.MANUAL && addressList != null && state != null && addressList.getText().length() == 0) {
            readAddresses();
        }
        if (page == Page.HOME) updateHome();
        if (page == Page.DEVICES) updateDevices();
        screenPolicy();
    }

    /** Mounts one small-screen page and clears controls from the previous page. */
    private void show(Page next) {
        generation++;
        page = next;
        pet = null;
        scanner = null;
        scannerFrame = null;
        if (next != Page.INVITATION) {
            invitationCode = null;
            invitationName = null;
            invitationNode = null;
        }
        statusText = null;
        primary = null;
        operation = null;
        addressList = null;
        deviceRows = null;
        deviceKey = null;
        getWindow().getDecorView().setSystemUiVisibility(next == Page.HOME && round
            ? View.SYSTEM_UI_FLAG_FULLSCREEN | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION | View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY
            : 0);
        if (next == Page.HOME) setContentView(buildHome());
        else showDevicePage(next);
        changed(snapshot, startupError);
    }

    /** Keeps each device management page inside the system and round-screen insets. */
    private void showDevicePage(Page next) {
        LinearLayout content = column();
        content.setGravity(Gravity.CENTER_HORIZONTAL);
        content.setBackground(new GradientDrawable(GradientDrawable.Orientation.TOP_BOTTOM,
            new int[] {0xff192334, PetStyle.NIGHT, PetStyle.NIGHT}));
        int width = getResources().getDisplayMetrics().widthPixels;
        if (next == Page.SCAN) {
            int gutter = round ? Math.round(width * .15f) : Math.max(dp(18), (width - dp(340)) / 2);
            content.setPadding(gutter, dp(round ? 13 : 32), gutter, dp(round ? 17 : 24));
            buildScanner(content);
            setContentView(content);
        } else {
            int gutter = round ? Math.round(width * .15f) : Math.max(dp(18), (width - dp(440)) / 2);
            content.setPadding(gutter, dp(round ? 28 : 20), gutter, dp(round ? 32 : 20));
            WatchScrollView scroll = new WatchScrollView();
            scroll.setFillViewport(true);
            scroll.setVerticalScrollBarEnabled(false);
            scroll.addView(content);
            setContentView(scroll);
            if (next == Page.DEVICES) buildDevices(content);
            else if (next == Page.MANUAL) buildManual(content);
            else if (next == Page.INVITATION) buildInvitation(content);
            else buildCode(content);
        }
    }

    /** Places the pet on a full-screen stage with one compact entry at the bottom. */
    private View buildHome() {
        FrameLayout content = new FrameLayout(this);
        content.setBackgroundColor(PetStyle.NIGHT);
        pet = new PetView(this);
        pet.setPadding(0, dp(39), 0, dp(56));
        pet.setOnClickListener(view -> {
            JSONObject settings = snapshot == null ? null : snapshot.optJSONObject("pet");
            if (settings == null || settings.optBoolean("haptics")) {
                view.performHapticFeedback(HapticFeedbackConstants.VIRTUAL_KEY);
            }
            pet.greet();
        });
        pet.setOnLongClickListener(view -> { show(Page.DEVICES); return true; });
        content.addView(pet, new FrameLayout.LayoutParams(-1, -1));

        statusText = PetStyle.label(this, "", 11, PetStyle.MUTED, false);
        FrameLayout.LayoutParams statusLayout = new FrameLayout.LayoutParams(-2, -2, Gravity.TOP | Gravity.CENTER_HORIZONTAL);
        statusLayout.topMargin = dp(20);
        content.addView(statusText, statusLayout);

        primary = PetStyle.homeAction(this);
        primary.setOnClickListener(view -> {
            if (canScanFirst()) openScanner();
            else show(Page.DEVICES);
        });
        FrameLayout.LayoutParams actionLayout = new FrameLayout.LayoutParams(-2, dp(48), Gravity.BOTTOM | Gravity.CENTER_HORIZONTAL);
        actionLayout.bottomMargin = dp(8);
        content.addView(primary, actionLayout);
        return content;
    }

    /** Keeps connection status explicit and independent of the facial expression. */
    private void updateHome() {
        boolean connected = false;
        String status;
        if (startupError != null) status = "节点连接异常";
        else if (snapshot == null) status = "正在启动";
        else if (!hasPeers()) status = "等待配对";
        else if (!inSpace()) status = "等待加入空间";
        else {
            connected = array("activePeers").length() > 0;
            status = connected ? "已连接" : "暂未连接";
        }
        if (!TextUtils.equals(statusText.getText(), status)) {
            statusText.setText(status);
            statusText.setTextColor(connected ? PetStyle.MINT : PetStyle.MUTED);
        }
        boolean scan = canScanFirst();
        String actionLabel = scan ? "扫码加入" : "设备空间";
        if (!TextUtils.equals(primary.getText(), actionLabel)) {
            primary.setText(actionLabel);
            primary.setCompoundDrawablesRelativeWithIntrinsicBounds(scan ? R.drawable.ic_edge_scan : R.drawable.ic_edge_space, 0, 0, 0);
        }
        primary.setEnabled(snapshot != null || startupError != null);
        if (snapshot != null) {
            pet.appearance(snapshot.optString("expression"), PetStyle.accent(snapshot.optJSONObject("pet").optString("accent")));
        }
    }

    /** Offers QR as the first action on camera-equipped, unpaired devices. */
    private boolean canScanFirst() { return hasCamera && snapshot != null && !hasPeers(); }

    /** Opens the standard Space scanner from a deliberate user action. */
    private void openScanner() {
        show(Page.SCAN);
        if (!cameraPermitted()) requestCamera();
    }

    /** Detects the extra runtime camera permission on Android 6 and later. */
    private boolean cameraPermitted() {
        return Build.VERSION.SDK_INT < 23 || checkSelfPermission(Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED;
    }

    /** Requests camera access only from a user-visible scanner. */
    private void requestCamera() {
        if (Build.VERSION.SDK_INT >= 23) requestPermissions(new String[] {Manifest.permission.CAMERA}, SCAN_PERMISSION);
    }

    /** Updates the scan page when Android completes the permission dialog. */
    @Override public void onRequestPermissionsResult(int request, String[] permissions, int[] results) {
        super.onRequestPermissionsResult(request, permissions, results);
        if (request != SCAN_PERMISSION || page != Page.SCAN) return;
        if (cameraPermitted()) show(Page.SCAN);
        else notice("请在系统设置中允许使用相机");
    }

    /** Gives the camera preview space and names the existing host screen that shows the invitation. */
    private void buildScanner(LinearLayout content) {
        label(content, "扫码加入空间", round ? 15 : 20, PetStyle.INK, true);
        label(content, "扫描 Operit 设备空间的二维码", 12, PetStyle.MUTED, false);
        operation = label(content, "", 12, PetStyle.PEACH, false);
        operation.setVisibility(View.GONE);
        if (!cameraPermitted()) {
            action(content, "允许相机", true, this::requestCamera);
            action(content, "手动配对", false, () -> show(Page.MANUAL));
            return;
        }
        scanner = new QrScannerView(this, new QrScannerView.Listener() {
            /** Inspects the decoded invitation before showing the confirmation screen. */
            @Override public void scanned(String code) {
                notice("正在读取邀请…");
                run(command("inspectInvitation", "code", code), data -> {
                    invitationCode = code;
                    invitationName = data.optString("displayName");
                    invitationNode = data.optString("nodeId");
                    show(Page.INVITATION);
                }, MainActivity.this::scanFailure);
            }

            /** Leaves the choice of opening another scan or manual pairing with the user. */
            @Override public void unavailable() { scanFailure("相机不可用"); }
        });
        LinearLayout.LayoutParams preview = new LinearLayout.LayoutParams(-1, 0, 1);
        preview.topMargin = dp(8);
        scannerFrame = new FrameLayout(this);
        scannerFrame.setBackground(PetStyle.surface(this, PetStyle.PANEL, 18, PetStyle.LINE));
        scannerFrame.setClipToOutline(true);
        scannerFrame.addView(scanner, new FrameLayout.LayoutParams(-1, -1));
        content.addView(scannerFrame, preview);
        primary = action(content, "重新扫码", true, () -> show(Page.SCAN));
        primary.setVisibility(View.GONE);
        scanner.setScanning(resumed);
    }

    /** Keeps scanner errors readable on the watch and allows a new scan. */
    private void scanFailure(String error) {
        scanner.setScanning(false);
        scannerFrame.setVisibility(View.GONE);
        notice(error);
        primary.setVisibility(View.VISIBLE);
    }

    /** Requests admission only after the user confirms the inviting device. */
    private void buildInvitation(LinearLayout content) {
        label(content, "加入设备空间", 16, PetStyle.INK, true);
        if (!invitationName.isEmpty()) label(content, invitationName, 15, PetStyle.MINT, true);
        TextView identity = label(content, invitationNode, 11, PetStyle.MUTED, false);
        identity.setMaxLines(1);
        identity.setEllipsize(TextUtils.TruncateAt.MIDDLE);
        operation = label(content, "", 12, PetStyle.PEACH, false);
        operation.setVisibility(View.GONE);
        primary = action(content, "确认加入", true, () -> {
            primary.setEnabled(false);
            notice("正在配对…");
            run(command("pairInvitation", "code", invitationCode), data -> {
                String node = data.optJSONObject("peer").optString("nodeId");
                show(Page.DEVICES);
                if (!hasValue(array("members"), node)) {
                    joinCheckFailed = false;
                    run(command("join", "node", node), null);
                }
            }, error -> {
                primary.setEnabled(true);
                notice(error);
            });
        });
        action(content, "返回扫码", false, () -> { if (!busy) show(Page.SCAN); });
    }

    /** Groups device membership and secondary management below the main action. */
    private void buildDevices(LinearLayout content) {
        label(content, "设备空间", 18, PetStyle.INK, true);
        operation = label(content, "", 12, PetStyle.PEACH, false);
        operation.setVisibility(View.GONE);
        deviceRows = column();
        content.addView(deviceRows, new LinearLayout.LayoutParams(-1, -2));
        action(content, "手动配对", !hasCamera, () -> show(Page.MANUAL));
        action(content, "回到桌宠", false, () -> show(Page.HOME));
        action(content, "停止节点", false, () -> confirm("停止节点并退出？", this::stopNode));
    }

    /** Shows only actions that are valid in the current pairing or admission state. */
    private void updateDevices() {
        if (snapshot == null) {
            notice(startupError == null ? "正在启动" : startupError);
            return;
        }
        JSONArray peers = array("peers");
        JSONArray prompts = array("prompts");
        JSONArray activePeers = array("activePeers");
        JSONObject join = latestJoin();
        String key = peers.toString() + prompts + activePeers + join + array("members") + snapshot.optString("spaceName");
        if (!key.equals(deviceKey)) {
            deviceKey = key;
            deviceRows.removeAllViews();
            if (inSpace()) {
                label(deviceRows, snapshot.optString("spaceName"), 15, PetStyle.MINT, true);
                action(deviceRows, "退出空间", false, () -> confirm("退出当前空间？", () -> run(EdgeService.command("leave"), null)));
            } else if (join != null) {
                showJoin(deviceRows, join);
            } else if (peers.length() == 0 && !hasCamera) {
                label(deviceRows, "使用设备地址与六位码加入空间", 12, PetStyle.MUTED, false);
            }
            if (hasCamera && !inSpace() && !joinActive(join)) {
                action(deviceRows, "扫码加入", true, this::openScanner);
            }
            for (int index = 0; index < prompts.length(); index++) {
                JSONObject prompt = prompts.optJSONObject(index);
                label(deviceRows, "对端配对码 · " + prompt.optString("displayName"), 12, PetStyle.MUTED, false);
                label(deviceRows, prompt.optString("confirmationCode"), 21, PetStyle.MINT, true);
            }
            for (int index = 0; index < peers.length(); index++) {
                JSONObject peer = peers.optJSONObject(index);
                String node = peer.optString("nodeId");
                boolean connected = hasValue(activePeers, node);
                label(deviceRows, peer.optString("displayName"), 14, PetStyle.INK, true);
                if (!inSpace() && !joinActive(join)) {
                    Button joinButton = action(deviceRows, "申请加入空间", true, () -> {
                        joinCheckFailed = false;
                        run(command("join", "node", node), null);
                    });
                    joinButton.setEnabled(connected || peer.optBoolean("outbound"));
                    if (!joinButton.isEnabled()) label(deviceRows, "请保持对方设备在线", 12, PetStyle.MUTED, false);
                }
                label(deviceRows, connected ? "在线" : "离线", 11, PetStyle.MUTED, false);
                action(deviceRows, "解除配对", false, () -> confirm("解除与这台设备的配对？",
                    () -> run(command("unpair", "node", node), null)));
            }
        }
        if (joinActive(join)) trackJoin(join);
    }

    /** Shows admission progress and the existing explicit cancel operation. */
    private void showJoin(LinearLayout content, JSONObject join) {
        String status = join.optString("status");
        String request = join.optString("requestId");
        if (joinActive(join)) {
            label(content, "等待空间确认", 15, PetStyle.MINT, true);
            label(content, join.optString("spaceName"), 12, PetStyle.MUTED, false);
            action(content, "刷新申请", false, () -> {
                joinCheckFailed = false;
                nextJoinCheck = SystemClock.elapsedRealtime() + 3000L;
                run(command("refreshJoin", "id", request), null);
            });
            action(content, "取消申请", false, () -> run(command("cancelJoin", "id", request), null));
        } else if ("rejected".equals(status)) {
            label(content, "加入申请未获批准", 13, PetStyle.PEACH, false);
        } else if ("expired".equals(status)) {
            label(content, "加入申请已过期", 13, PetStyle.PEACH, false);
        }
    }

    /** Observes the submitted join at a bounded cadence while its page is visible. */
    private void trackJoin(JSONObject join) {
        long now = SystemClock.elapsedRealtime();
        if (busy || joinChecking || joinCheckFailed || service == null || now < nextJoinCheck) return;
        nextJoinCheck = now + 3000L;
        joinChecking = true;
        int requestGeneration = generation;
        service.request(command("refreshJoin", "id", join.optString("requestId")), (data, error) -> {
            joinChecking = false;
            if (error != null && requestGeneration == generation) {
                joinCheckFailed = true;
                notice(error);
            }
        });
    }

    /** Keeps the explicit address and six-digit flow in a secondary page. */
    private void buildManual(LinearLayout content) {
        label(content, "手动配对", 18, PetStyle.INK, true);
        operation = label(content, "", 12, PetStyle.PEACH, false);
        operation.setVisibility(View.GONE);
        EditText address = input(content, "TCP 地址与端口", InputType.TYPE_CLASS_TEXT | InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS, 100);
        address.setId(R.id.peer_address);
        action(content, "开始配对", true, () -> run(command("startPairing", "address", address.getText().toString()), null));
        label(content, "本表 TCP 地址", 13, PetStyle.INK, true);
        addressList = label(content, "", 12, PetStyle.MUTED, false);
        action(content, "更新地址", false, this::readAddresses);
        if (snapshot != null) {
            label(content, "本机节点 ID", 13, PetStyle.INK, true);
            TextView node = label(content, snapshot.optString("nodeId"), 11, PetStyle.MUTED, false);
            node.setTextIsSelectable(true);
        }
        action(content, "返回设备", false, () -> show(Page.DEVICES));
    }

    /** Reads the currently reachable addresses when the user opens manual pairing. */
    private void readAddresses() {
        addressList.setText("正在读取地址");
        run(EdgeService.command("addresses"), data -> {
            JSONArray addresses = data.optJSONArray("addresses");
            StringBuilder lines = new StringBuilder();
            for (int index = 0; index < addresses.length(); index++) {
                if (index > 0) lines.append('\n');
                lines.append(addresses.optString(index).substring("tcp://".length()));
            }
            addressList.setText(lines);
        }, error -> addressList.setText("请检查 Wi-Fi 后更新地址"));
    }

    /** Displays a separate compact PIN entry instead of covering the watch face with a dialog. */
    private void buildCode(LinearLayout content) {
        label(content, "输入对端配对码", 17, PetStyle.INK, true);
        operation = label(content, "", 12, PetStyle.PEACH, false);
        operation.setVisibility(View.GONE);
        EditText pin = input(content, "六位数字", InputType.TYPE_CLASS_NUMBER, 6);
        pin.setId(R.id.pairing_pin);
        action(content, "确认配对", true, () -> {
            JSONObject pending = snapshot == null ? null : snapshot.optJSONObject("pendingPairing");
            if (pending == null) return;
            String digits = pin.getText().toString();
            if (!digits.matches("[0-9]{6}")) {
                notice("请输入六位数字配对码");
                return;
            }
            JSONObject command = command("finishPairing", "id", pending.optString("pairingId"));
            put(command, "code", digits);
            run(command, null);
        });
        action(content, "取消配对", false, this::cancelCode);
    }

    /** Cancels exactly the pending manual transaction before returning to management. */
    private void cancelCode() {
        JSONObject pending = snapshot == null ? null : snapshot.optJSONObject("pendingPairing");
        if (pending == null) show(Page.DEVICES);
        else run(command("cancelPairing", "id", pending.optString("pairingId")), null);
    }

    /** Ensures only one visible management operation is submitted at a time. */
    private void run(JSONObject command, EdgeService.Callback ignored) {
        run(command, null, this::notice);
    }

    /** Sends one management command and ignores a result from an obsolete page. */
    private void run(JSONObject command, Success success, Failure failure) {
        if (busy) return;
        if (service == null) {
            failure.show("节点服务未连接");
            return;
        }
        busy = true;
        int requestGeneration = generation;
        service.request(command, (data, error) -> {
            busy = false;
            if (isFinishing() || isDestroyed() || requestGeneration != generation) return;
            if (error != null) failure.show(error);
            else {
                notice("");
                if (success != null) success.complete(data);
            }
        });
    }

    /** Stops the foreground owner without changing node credentials or preferences. */
    private void stopNode() {
        if (service == null || snapshot == null) finishNode();
        else run(EdgeService.command("stop"), data -> finishNode(), this::notice);
    }

    /** Closes the service and its activity after explicit user confirmation. */
    private void finishNode() {
        stopService(new Intent(this, EdgeService.class));
        finish();
    }

    /** Keeps the display awake while scanning and applies the pet's saved screen preference. */
    private void screenPolicy() {
        JSONObject settings = snapshot == null ? null : snapshot.optJSONObject("pet");
        if (page == Page.SCAN || (page == Page.HOME && settings != null && settings.optBoolean("keepAwake"))) {
            getWindow().addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        } else {
            getWindow().clearFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
        }
    }

    /** Returns the most recent admission, disregarding a completed join from an earlier Space. */
    private JSONObject latestJoin() {
        JSONArray joins = array("joins");
        JSONObject newest = null;
        for (int index = 0; index < joins.length(); index++) {
            JSONObject join = joins.optJSONObject(index);
            if ("joined".equals(join.optString("status"))) continue;
            if (newest == null || join.optLong("createdAt") > newest.optLong("createdAt")) newest = join;
        }
        return newest;
    }

    /** Recognizes in-progress SDK admission states. */
    private boolean joinActive(JSONObject join) {
        if (join == null) return false;
        String status = join.optString("status");
        return "pending".equals(status) || "approving".equals(status) || "approved".equals(status);
    }

    /** Checks the persisted Space projection instead of treating pairing as admission. */
    private boolean inSpace() { return array("members").length() > 1; }

    /** Returns whether there is any saved pairing in either direction. */
    private boolean hasPeers() { return array("peers").length() > 0; }

    /** Checks an exact identity in a native list. */
    private boolean hasValue(JSONArray values, String value) {
        for (int index = 0; index < values.length(); index++) if (value.equals(values.optString(index))) return true;
        return false;
    }

    /** Returns an empty presentation list while the first native snapshot is pending. */
    private JSONArray array(String key) { return snapshot == null ? new JSONArray() : snapshot.optJSONArray(key); }

    /** Builds typed local commands for the JNI owner. */
    private JSONObject command(String action, String key, String value) {
        JSONObject command = EdgeService.command(action);
        put(command, key, value);
        return command;
    }

    /** Writes a scalar command field independently of transport encoding. */
    private void put(JSONObject object, String key, String value) {
        try { object.put(key, value); }
        catch (JSONException error) { throw new IllegalArgumentException(error); }
    }

    /** Confirms only the local departure, unlink and stop actions. */
    private void confirm(String title, Runnable action) {
        new AlertDialog.Builder(this).setMessage(title)
            .setPositiveButton("确认", (dialog, selected) -> action.run())
            .setNegativeButton("取消", null).show();
    }

    /** Reports an operation failure in the page that owns the operation. */
    private void notice(String message) {
        if (operation == null) return;
        operation.setText(message);
        operation.setVisibility(message.isEmpty() ? View.GONE : View.VISIBLE);
    }

    /** Creates a column suitable for both fixed and vertically scrollable pages. */
    private LinearLayout column() {
        LinearLayout layout = new LinearLayout(this);
        layout.setOrientation(LinearLayout.VERTICAL);
        return layout;
    }

    /** Adds a wrapping watch label with a small vertical gap. */
    private TextView label(LinearLayout parent, String text, int size, int color, boolean bold) {
        TextView label = PetStyle.label(this, text, size, color, bold);
        LinearLayout.LayoutParams layout = new LinearLayout.LayoutParams(-1, -2);
        layout.bottomMargin = dp(3);
        parent.addView(label, layout);
        return label;
    }

    /** Adds a full-width touch target that fits inside round display corners. */
    private Button action(LinearLayout parent, String text, boolean primary, Runnable onClick) {
        Button button = PetStyle.action(this, text, primary);
        button.setOnClickListener(view -> onClick.run());
        LinearLayout.LayoutParams layout = new LinearLayout.LayoutParams(-1, -2);
        layout.topMargin = dp(6);
        parent.addView(button, layout);
        return button;
    }

    /** Uses the system keyboard only on the explicitly requested manual pages. */
    private EditText input(LinearLayout parent, String hint, int type, int maxLength) {
        EditText field = new EditText(this);
        field.setTextColor(PetStyle.INK);
        field.setHintTextColor(PetStyle.MUTED);
        field.setTextSize(14);
        field.setSingleLine(true);
        field.setInputType(type);
        field.setFilters(new InputFilter[] {new InputFilter.LengthFilter(maxLength)});
        field.setHint(hint);
        field.setEllipsize(TextUtils.TruncateAt.END);
        field.setPadding(dp(12), dp(12), dp(12), dp(12));
        field.setBackground(PetStyle.surface(this, PetStyle.PANEL, 16, PetStyle.LINE));
        parent.addView(field, new LinearLayout.LayoutParams(-1, -2));
        return field;
    }

    /** Scales a logical touch dimension for the watch display. */
    private int dp(int value) { return PetStyle.dp(this, value); }

    private interface Success { void complete(JSONObject data); }
    private interface Failure { void show(String message); }

    /** Handles the watch crown as scrolling input on the management pages. */
    private final class WatchScrollView extends ScrollView {
        /** Allows rotary focus without taking focus from a selected text field. */
        WatchScrollView() {
            super(MainActivity.this);
            setFocusableInTouchMode(true);
            requestFocus();
        }

        /** Converts rotary ticks to a predictable vertical movement. */
        @Override public boolean onGenericMotionEvent(MotionEvent event) {
            if (Build.VERSION.SDK_INT >= 26 && event.getAction() == MotionEvent.ACTION_SCROLL
                && event.isFromSource(InputDevice.SOURCE_ROTARY_ENCODER)) {
                scrollBy(0, Math.round(-event.getAxisValue(MotionEvent.AXIS_SCROLL) * dp(30)));
                return true;
            }
            return super.onGenericMotionEvent(event);
        }
    }
}
