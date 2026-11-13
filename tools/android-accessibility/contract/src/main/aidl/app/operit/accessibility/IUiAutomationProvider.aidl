package app.operit.accessibility;

import android.os.ParcelFileDescriptor;

// Versioned device UI transport. Responses are UTF-8 JSON streamed through a pipe.
interface IUiAutomationProvider {
    int getProtocolVersion();
    ParcelFileDescriptor execute(String requestJson);
}
