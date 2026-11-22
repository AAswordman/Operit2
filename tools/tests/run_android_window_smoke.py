"""Builds a device-only Binder owner; never installs the production window payload."""

import argparse
from pathlib import Path
import os
import json
import re
import shlex
import subprocess
import time
import zipfile


def run(arguments):
    """Runs one SDK command and preserves its diagnostics on failure."""
    subprocess.run([str(value) for value in arguments], check=True)


def shell(*arguments):
    """Quotes every Android shell argument independently of the Windows parent shell."""
    return subprocess.check_output(["adb", "shell", shlex.join(str(value) for value in arguments)], text=True)


def request(encoded, accessibility):
    """Waits for one correlated Binder response and fails on device-reported errors."""
    sequence = str(time.time_ns())
    json.loads(encoded)
    print(shell("am", "start", "-n", "app.operit.windowsmoke/.SmokeActivity",
                "--es", "request", encoded, "--es", "sequence", sequence,
                "--ez", "accessibility", str(accessibility).lower()).strip())
    deadline = time.monotonic() + 20
    while time.monotonic() < deadline:
        logs = subprocess.check_output(["adb", "logcat", "-d", "-s", "OperitWindowSmoke:I"], text=True)
        for line in logs.splitlines():
            marker = f"RESULT {sequence} "
            if marker in line:
                result = line.split(marker, 1)[1]
                if result != "STORED":
                    raise RuntimeError(result)
                result = subprocess.check_output(["adb", "exec-out", "run-as", "app.operit.windowsmoke", "cat",
                                                  f"files/result-{sequence}.json"], text=True, encoding="utf-8")
                subprocess.run(["adb", "shell", "run-as", "app.operit.windowsmoke", "rm", f"files/result-{sequence}.json"], check=True)
                print(result if len(result) < 2500 else result[:2400] + "…")
                response = json.loads(result)
                if not response.get("success"):
                    raise RuntimeError(response)
                return response["data"]
        time.sleep(0.25)
    raise TimeoutError("No response from the device test owner")


def verify(output):
    """Exercises native touch, floating-window gestures, exact display routing and cleanup."""
    output.mkdir(parents=True, exist_ok=True)
    evidence = {}

    def control(**parameters):
        """Sends one window-server control message through its production Binder."""
        return request(json.dumps(parameters), False)

    def automate(operation, **parameters):
        """Sends one production accessibility request to its exact native display."""
        return request(json.dumps({"operation": operation, "parameters": parameters}), True)

    def capture(name):
        """Captures the actual composed display without Windows text redirection."""
        with (output / name).open("wb") as image:
            subprocess.run(["adb", "exec-out", "screencap", "-p"], stdout=image, check=True)

    def descendants(node):
        """Visits actual accessibility nodes for semantic control discovery."""
        yield node
        for child in node.get("children", []):
            yield from descendants(child)

    density = int(re.search(r"(?:Override|Physical) density: (\d+)\s*$", shell("wm", "density")).group(1)) / 160
    screen_width, screen_height = map(int, re.search(r"(?:Override|Physical) size: (\d+)x(\d+)\s*$", shell("wm", "size")).groups())
    display_unit = min(screen_width // 2, screen_height // 3)
    display_width, display_height = 2 * display_unit, 3 * display_unit

    def bounds(window):
        """Parses the payload's physical content rectangle."""
        return tuple(map(int, re.findall(r"-?\d+", window["bounds"])))

    def current(window):
        """Reads the current geometry using the same opaque ID as window selection."""
        return next(entry for entry in control(operation="windows")["windows"] if entry["windowId"] == window["windowId"])

    def assert_aspect(window):
        """Checks the 2:3 card shape within integer display-pixel rounding."""
        left, top, right, bottom = bounds(window)
        assert abs(2 * (bottom - top) - 3 * (right - left)) <= 1, window

    def grip(window):
        """Finds the top-center handle relative to the card's live content bounds."""
        left, top, right, _ = bounds(window)
        return (left + right) // 2, top + round(10 * density)

    def minimize(window):
        """Taps the compact handle and its transient first menu action."""
        center, handle_y = grip(window)
        shell("input", "tap", center, handle_y)
        time.sleep(0.2)
        shell("input", "tap", center, bounds(window)[1] + round(40 * density))
        time.sleep(0.55)

    clock = control(operation="launch", component="com.google.android.deskclock/com.android.deskclock.DeskClock")
    time.sleep(2)
    assert_aspect(clock)
    evidence["clock"] = clock
    evidence["windows"] = automate("list_windows")
    native_clock = next(window for window in evidence["windows"]["windows"]
                        if window["displayId"] == clock["displayId"] and window["packageName"] == clock["packageName"])
    assert bounds(native_clock) == (0, 0, display_width, display_height), native_clock
    automate("tap", display_id=clock["displayId"], x=round(display_width * 0.3), y=round(display_height * 0.94))
    time.sleep(0.3)
    capture("01-clock.png")
    left, top, right, bottom = bounds(current(clock))
    shell("input", "tap", round(left + (right - left) * 0.7), round(top + (bottom - top) * 0.94))
    time.sleep(0.5)
    stopwatch = automate("get_page_info", display_id=clock["displayId"])
    assert stopwatch["activityName"] == "Stopwatch", stopwatch["activityName"]
    evidence["touch_stopwatch"] = stopwatch
    resets = [node for node in descendants(stopwatch["uiElements"]) if node.get("contentDesc") == "Reset"]
    if resets:
        left, top, right, bottom = map(int, re.findall(r"-?\d+", resets[0]["bounds"]))
        automate("tap", display_id=clock["displayId"], x=(left + right) // 2, y=(top + bottom) // 2)
        stopwatch = automate("get_page_info", display_id=clock["displayId"])
    start = next(node for node in descendants(stopwatch["uiElements"]) if node.get("contentDesc") == "Start")
    left, top, right, bottom = map(int, re.findall(r"-?\d+", start["bounds"]))
    automate("tap", display_id=clock["displayId"], x=(left + right) // 2, y=(top + bottom) // 2)
    time.sleep(1)
    evidence["stopwatch_running"] = automate("get_page_info", display_id=clock["displayId"])
    assert any(node.get("contentDesc") == "Pause" for node in descendants(evidence["stopwatch_running"]["uiElements"]))
    capture("02-stopwatch.png")
    documents = control(operation="launch", component="com.google.android.documentsui/com.android.documentsui.LauncherActivity")
    time.sleep(1)
    center, handle_y = grip(documents)
    drag_y = round(52 * density)
    shell("input", "swipe", center, handle_y, center, handle_y + drag_y, 650)
    time.sleep(0.35)
    dragged = current(documents)
    assert abs(bounds(dragged)[1] - bounds(documents)[1] - drag_y) <= 2, dragged
    center, handle_y = grip(dragged)
    flick_y = round(33 * density)
    shell("input", "swipe", center, handle_y, center, handle_y + flick_y, 95)
    time.sleep(0.7)
    flung = current(documents)
    assert bounds(flung)[1] > bounds(dragged)[1] + flick_y + round(8 * density), flung
    evidence["inertial_drag"] = [dragged, flung]

    left, top, right, bottom = bounds(flung)
    corner_x = right - round(17 * density)
    corner_y = bottom - round(15 * density)
    shell("input", "swipe", corner_x, corner_y, corner_x - round(30 * density), corner_y - round(65 * density), 520)
    resized_right = current(documents)
    assert bounds(resized_right)[:2] == (left, top) and bounds(resized_right)[2] < right, resized_right
    left, top, right, bottom = bounds(resized_right)
    corner_x = left + round(17 * density)
    corner_y = bottom - round(15 * density)
    shell("input", "swipe", corner_x, corner_y, corner_x + round(24 * density), corner_y - round(52 * density), 520)
    resized_left = current(documents)
    assert bounds(resized_left)[1:3] == (top, right) and bounds(resized_left)[0] > left, resized_left
    assert_aspect(resized_right)
    assert_aspect(resized_left)
    evidence["resize_both_corners"] = [resized_right, resized_left]

    docked_from_resize = []
    for side in (1, -1):
        before_resize = current(documents)
        left, top, right, bottom = bounds(before_resize)
        corner_x = right - round(17 * density) if side > 0 else left + round(17 * density)
        corner_y = bottom - round(15 * density)
        shrink = round(144 * density) - (right - left)
        shell("input", "swipe", corner_x, corner_y,
              corner_x + side * shrink, corner_y + round(1.5 * shrink), 850)
        time.sleep(0.55)
        docked = current(documents)
        assert docked["isMinimized"] and docked["displayId"] == documents["displayId"], docked
        control(operation="focus", windowId=documents["windowId"])
        time.sleep(0.55)
        expanded = current(documents)
        assert not expanded["isMinimized"], expanded
        assert bounds(expanded)[2] - bounds(expanded)[0] == round(184 * density), expanded
        assert bounds(expanded)[1] == top, expanded
        assert bounds(expanded)[0 if side > 0 else 2] == (left if side > 0 else right), expanded
        docked_from_resize.append({"side": side, "docked": docked, "restored": expanded})
    evidence["resize_to_dock"] = docked_from_resize
    resized_left = current(documents)

    evidence["documents"] = current(documents)
    evidence["two_windows"] = control(operation="windows")
    assert len(evidence["two_windows"]["windows"]) == 2
    assert clock["displayId"] != documents["displayId"]
    evidence["documents_tree"] = automate("get_page_info", display_id=documents["displayId"])
    assert evidence["documents_tree"]["packageName"] == documents["packageName"]
    capture("03-two-windows.png")
    minimize(resized_left)
    minimized = current(documents)
    assert_aspect(minimized)
    dock_left, dock_top, dock_right, dock_bottom = bounds(minimized)
    assert minimized["isMinimized"] and minimized["displayId"] == documents["displayId"], minimized
    assert dock_right == screen_width - round(8 * density) and dock_right - dock_left == round(80 * density), minimized
    assert dock_top < bounds(resized_left)[1], minimized
    evidence["minimized"] = minimized
    capture("04-minimized.png")
    shell("input", "tap", (dock_left + dock_right) // 2, (dock_top + dock_bottom) // 2)
    time.sleep(0.55)
    restored = current(documents)
    assert not restored["isMinimized"] and bounds(restored) == bounds(resized_left), restored
    assert restored["chromeWindowId"] == minimized["chromeWindowId"], restored
    capture("05-restored.png")

    minimize(restored)
    assert current(documents)["isMinimized"]
    control(operation="focus", windowId=documents["windowId"])
    time.sleep(0.55)
    restored = current(documents)
    assert not restored["isMinimized"] and bounds(restored) == bounds(resized_left), restored
    evidence["restored"] = restored
    control(operation="close_package", packageName=documents["packageName"])
    assert len(control(operation="windows")["windows"]) == 1
    try:
        control(operation="focus", windowId=documents["windowId"])
    except RuntimeError as error:
        evidence["stale_id_rejected"] = str(error)
    else:
        raise AssertionError("Closed window ID was accepted")

    task_before = next(block for block in re.split(r"(?m)(?=^RootTask id=)", shell("am", "stack", "list"))
                       if re.search(rf"^RootTask id=\d+ .* displayId={clock['displayId']}\b", block))
    task_id = re.match(r"RootTask id=(\d+)", task_before).group(1)
    clock = current(clock)
    center, handle_y = grip(clock)
    shell("input", "tap", center, handle_y)
    time.sleep(0.2)
    shell("input", "tap", center, bounds(clock)[1] + round(84 * density))
    time.sleep(0.7)
    task_after = next(block for block in re.split(r"(?m)(?=^RootTask id=)", shell("am", "stack", "list"))
                      if re.match(rf"RootTask id={task_id}\b", block))
    assert re.search(r"^RootTask .* displayId=0\b", task_after), task_after
    assert "mWindowingMode=fullscreen" in task_after, task_after
    assert "com.google.android.deskclock" in task_after, task_after
    capture("06-fullscreen-same-task.png")
    evidence["fullscreen_task"] = {"before": task_before, "after": task_after}
    assert not control(operation="windows")["windows"]
    process = next(line for line in shell("ps", "-A", "-o", "UID,PID,ARGS").splitlines()
                   if "app.operit.uiserver.UiServerMain" in line)
    assert process.split()[0] == "2000", process
    evidence["server_identity"] = process
    status = shell("cat", f"/proc/{process.split()[1]}/status")
    evidence["server_status"] = "\n".join(line for line in status.splitlines() if line.startswith(("Uid:", "Gid:", "Groups:")))
    assert re.search(r"^Gid:\s+2000\s+2000\s+2000\s+2000$", status, re.MULTILINE), evidence["server_status"]
    (output / "evidence.json").write_text(json.dumps(evidence, ensure_ascii=False, indent=2), encoding="utf-8")
    shell("am", "force-stop", "app.operit.windowsmoke")
    time.sleep(1)
    assert "app.operit.uiserver.UiServerMain" not in shell("ps", "-A", "-o", "ARGS")
    assert "operit-ui-device-smoke" not in shell("ls", "/data/local/tmp")
    print("PASS: direct Surface, physical touch, per-display trees/input, inertial drag, both resize corners, resize-to-dock, touch/Binder restore, same-task fullscreen, owner cleanup")


def main():
    """Builds and installs the tiny test owner beside the production companion."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--sdk", type=Path)
    parser.add_argument("--request")
    parser.add_argument("--accessibility", action="store_true")
    parser.add_argument("--start-server", action="store_true")
    parser.add_argument("--screenshot", type=Path)
    parser.add_argument("--verify", action="store_true")
    arguments = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    if arguments.verify:
        verify(root / "tools/release/work/android-window-smoke/floating-window")
        return
    if arguments.request:
        request(arguments.request, arguments.accessibility)
        return
    if arguments.screenshot:
        arguments.screenshot.parent.mkdir(parents=True, exist_ok=True)
        with arguments.screenshot.open("wb") as output_image:
            subprocess.run(["adb", "exec-out", "screencap", "-p"], stdout=output_image, check=True)
        return
    if arguments.start_server:
        run(["adb", "shell", "am", "start", "-n", "app.operit.windowsmoke/.SmokeActivity"])
        shell("run-as", "app.operit.windowsmoke", "rm", "-f", "files/ready")
        uid = re.search(r"uid:(\d+)", shell("pm", "list", "packages", "-U", "app.operit.windowsmoke")).group(1)
        directory = "/data/local/tmp/operit-ui-device-smoke"
        shell("mkdir", "-p", directory)
        run(["adb", "push", root / "apps/flutter/app/build/windowServer/outputs/apk/debug/windowServer-debug.apk", directory + "/server.jar"])
        shell("chmod", "444", directory + "/server.jar")
        subprocess.run(["adb", "shell", f"CLASSPATH={directory}/server.jar app_process64 / app.operit.uiserver.UiServerMain "
                        f"{uid} app.operit.windowsmoke device-smoke unused > {directory}/server.log 2>&1 < /dev/null &"], check=True)
        deadline = time.monotonic() + 12
        while time.monotonic() < deadline:
            ready = subprocess.run(["adb", "shell", "run-as", "app.operit.windowsmoke", "cat", "files/ready"],
                                   capture_output=True, text=True)
            if ready.returncode == 0 and ready.stdout.strip() == "ready":
                print("Production window server attached to the test owner")
                return
            time.sleep(0.25)
        print(shell("cat", directory + "/server.log"))
        raise TimeoutError("Production window server did not attach within 12 seconds")
    if arguments.sdk is None:
        parser.error("--sdk is required when building the test owner")
    fixture = root / "tools/tests/fixtures/android_window_smoke"
    output = root / "tools/release/work/android-window-smoke"
    output.mkdir(parents=True, exist_ok=True)
    classes = output / "classes"
    classes.mkdir(exist_ok=True)
    build_tools = arguments.sdk / "build-tools/35.0.0"
    platform = arguments.sdk / "platforms/android-35/android.jar"
    generated = root / "apps/flutter/app/build"
    sources = [fixture / "SmokeActivity.java",
               generated / "windowServer/generated/aidl_source_output_dir/debug/out/app/operit/windowserver/IUiServer.java",
               generated / "accessibilityCompanion/generated/aidl_source_output_dir/debug/out/app/operit/accessibility/IUiAutomationProvider.java"]
    run(["javac", "-source", "17", "-target", "17", "-classpath", platform, "-d", classes, *sources])
    class_archive = output / "classes.jar"
    with zipfile.ZipFile(class_archive, "w") as archive:
        for path in classes.rglob("*.class"):
            archive.write(path, path.relative_to(classes).as_posix())
    run([build_tools / "d8.bat", "--lib", platform, "--min-api", "30", "--output", output, class_archive])
    apk = output / "window-smoke.apk"
    run([build_tools / "aapt.exe", "package", "-f", "-M", fixture / "AndroidManifest.xml", "-I", platform, "-F", apk])
    with zipfile.ZipFile(apk, "a", compression=zipfile.ZIP_DEFLATED) as archive:
        archive.write(output / "classes.dex", "classes.dex")
    run([build_tools / "apksigner.bat", "sign", "--ks", Path(os.environ["USERPROFILE"]) / ".android/debug.keystore",
         "--ks-pass", "pass:android", "--key-pass", "pass:android", apk])
    run(["adb", "install", "-r", apk])
    print(f"Installed device test owner: {apk}")


if __name__ == "__main__":
    main()
