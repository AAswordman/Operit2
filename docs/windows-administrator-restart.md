# Windows administrator authorization

Windows assigns elevation to a process at creation. The `windows.admin` action
therefore restarts Operit through UAC; it cannot elevate the existing process.
The requirement description tells the user that authorization restarts the app.

The Windows runner owns this transition in `administrator_restart.cpp`:

1. The existing process creates unique readiness and cancellation events and
   launches its own executable through `ShellExecuteExW` with `runas`. Original
   command-line arguments are preserved with Windows argument quoting.
2. Before COM registration, the single-instance guard, Flutter window creation,
   or Core initialization, the replacement consumes `--operit-admin-restart`.
   It checks its elevated token, opens the original process, and checks that the
   executable and Windows account match. Switching accounts would change the
   app's private storage and is rejected.
3. The replacement opens both events, acknowledges readiness, and waits for the
   original process handle to signal exit. It does not create a window or Core
   while waiting.
4. After readiness, the existing process posts `WM_QUIT` through
   `RequestOperitApplicationExit`. This bypasses close-to-tray and runs the normal
   Flutter and runtime teardown. The process retains its single-instance mutex
   until exit.
5. Only after that process has exited does the replacement remove the internal
   arguments and proceed through the normal single-instance startup guard. It
   never bypasses that guard, so a concurrent ordinary launch still cannot create
   two Cores with the same peer identity. Duplicate launchers close their own
   mutex handle before forwarding activation, so they do not keep the old
   process identity alive while the replacement starts.

UAC cancellation leaves the existing process running. Launch failure, replacement
exit before readiness, or a 15-second readiness timeout reports an authorization
error without requesting application exit. Failure to post `WM_QUIT` cancels the
replacement. The replacement rejects cancellation or a 60-second shutdown timeout
without starting Flutter or Core. Rejected replacements do not open a crash UI;
the existing process reports readiness failure and native details go to the
Windows debug output.

Validation should cover accepting and cancelling UAC, requesting authorization
when already elevated, starting with arguments containing spaces and trailing
backslashes, and an ordinary duplicate launch during handoff. With close-to-tray
enabled and detached windows open, accepting UAC must still terminate the old
process before the elevated main window appears. Verify that the new process
reports `windows.admin` as `Satisfied` and that only one Core owns the local peer.
