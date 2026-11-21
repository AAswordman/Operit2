package app.operit.windowsmoke;

import android.app.Activity;
import android.content.BroadcastReceiver;
import android.content.ComponentName;
import android.content.Context;
import android.content.Intent;
import android.content.IntentFilter;
import android.content.ServiceConnection;
import android.os.Binder;
import android.os.Bundle;
import android.os.IBinder;
import android.os.ParcelFileDescriptor;
import android.util.Log;
import android.widget.TextView;
import app.operit.windowserver.IUiServer;
import app.operit.accessibility.IUiAutomationProvider;
import java.nio.charset.StandardCharsets;
import java.util.concurrent.Executors;
import java.util.concurrent.ExecutorService;

/** Device-only owner exercising the production payload and accessibility Binder protocols. */
public final class SmokeActivity extends Activity {
    private final ExecutorService worker = Executors.newSingleThreadExecutor();
    private final Binder owner = new Binder();
    private IUiServer server;
    private IUiAutomationProvider accessibility;
    private TextView status;
    private final BroadcastReceiver ready = new BroadcastReceiver() {
        /** Attaches this live test owner to the uninstalled production window server. */
        @Override public void onReceive(Context context, Intent intent) {
            if (!"device-smoke".equals(intent.getStringExtra("nonce"))) return;
            server = IUiServer.Stub.asInterface(intent.getBundleExtra("endpoint").getBinder("server"));
            worker.execute(() -> {
                try {
                    if (server.getProtocolVersion() != 1) throw new IllegalStateException("Wrong server protocol");
                    android.os.IInterface session = (android.os.IInterface) Class.forName("android.view.WindowManagerGlobal")
                            .getMethod("getWindowSession").invoke(null);
                    server.attach(owner, session.asBinder());
                    try (java.io.FileOutputStream output = openFileOutput("ready", MODE_PRIVATE)) {
                        output.write("ready".getBytes(StandardCharsets.UTF_8));
                    }
                    report("READY", "uid=" + android.os.Process.myUid());
                } catch (Exception error) { report("ERROR", error.toString()); }
            });
        }
    };
    private final ServiceConnection connection = new ServiceConnection() {
        /** Connects the installed, signature-protected production accessibility companion. */
        @Override public void onServiceConnected(ComponentName name, IBinder binder) {
            accessibility = IUiAutomationProvider.Stub.asInterface(binder);
            report("ACCESSIBILITY_READY", name.toString());
        }
        /** Makes companion loss visible to the device test. */
        @Override public void onServiceDisconnected(ComponentName name) { accessibility = null; }
    };

    /** Registers the same shell-permission-protected handoff as the Android Host. */
    @Override public void onCreate(Bundle state) {
        super.onCreate(state);
        status = new TextView(this);
        status.setText("Operit native window device test\nWaiting for standalone /data/local/tmp payload…");
        status.setTextSize(18);
        status.setPadding(32, 64, 32, 32);
        setContentView(status);
        registerReceiver(ready, new IntentFilter(getPackageName() + ".UI_SERVER_READY"),
                android.Manifest.permission.DUMP, null, Context.RECEIVER_EXPORTED);
        bindService(new Intent().setComponent(new ComponentName("app.operit.accessibility",
                "app.operit.accessibility.UiAutomationBinderService")), connection, BIND_AUTO_CREATE);
        execute(getIntent());
    }

    /** Receives explicit device-test requests without changing production tool interfaces. */
    @Override public void onNewIntent(Intent intent) { super.onNewIntent(intent); execute(intent); }

    /** Sends one actual Binder request and records its unmodified result in logcat. */
    private void execute(Intent intent) {
        String request = intent.getStringExtra("request");
        if (request == null) return;
        boolean tree = intent.getBooleanExtra("accessibility", false);
        String sequence = intent.getStringExtra("sequence");
        worker.execute(() -> {
            try {
                String result;
                if (tree) {
                    ParcelFileDescriptor pipe = accessibility.execute(request);
                    try (ParcelFileDescriptor.AutoCloseInputStream stream = new ParcelFileDescriptor.AutoCloseInputStream(pipe)) {
                        result = new String(stream.readAllBytes(), StandardCharsets.UTF_8);
                    }
                } else result = server.execute(request);
                try (java.io.FileOutputStream output = openFileOutput("result-" + sequence + ".json", MODE_PRIVATE)) {
                    output.write(result.getBytes(StandardCharsets.UTF_8));
                }
                Log.i("OperitWindowSmoke", "RESULT " + sequence + " STORED");
            } catch (Exception error) { Log.e("OperitWindowSmoke", "RESULT " + sequence + " ERROR " + error); }
        });
    }

    /** Displays connection state while preserving raw command results in logcat. */
    private void report(String event, String detail) {
        Log.i("OperitWindowSmoke", event + " " + detail);
        runOnUiThread(() -> status.setText("Operit native window device test\n" + event + "\n" + detail));
    }

    /** Releases the test owner's registrations and worker. */
    @Override public void onDestroy() {
        unregisterReceiver(ready);
        unbindService(connection);
        worker.shutdown();
        super.onDestroy();
    }
}
