"""Exercise the real Linux provider, Composite capture and GTK preview in an isolated X11 session.

Run under dbus-run-session with Xvfb, openbox, xcompmgr, GTK 3, Pycairo and pyatspi installed.
OPERIT_LINUX_TEST_DEPS may point at a locally extracted dependency tree.
"""
import base64
import ctypes as C
import io
import json
import os
from pathlib import Path
import select
import subprocess
import sys
import tempfile
import time
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
PROVIDER = ROOT / "hosts/linux/src/ui_automation.py"


def response(child):
    """Read a bounded structured fixture response while preserving its lifetime pipe."""
    if not select.select([child.stdout], [], [], 12)[0]:
        raise AssertionError(f"Native helper response timed out: {child.args}")
    line = child.stdout.readline()
    if not line:
        raise AssertionError(f"Native helper exited: {child.stderr.read()}")
    return json.loads(line)


def fixture():
    """Expose a live GTK window whose color and button text prove actual input dispatch."""
    import gi
    gi.require_version("Gtk", "3.0")
    gi.require_version("GdkX11", "3.0")
    from gi.repository import Gtk, GdkX11, GLib
    window = Gtk.Window(title="Operit native source fixture")
    window.set_decorated(False)
    cover = "--cover" in sys.argv
    window.move(0, 0) if cover else window.move(120, 100)
    window.set_default_size(1024, 768) if cover else window.set_default_size(320, 240)
    window.set_keep_above(cover)
    fixed = Gtk.Fixed()
    background = Gtk.DrawingArea()
    background.set_size_request(1024, 768) if cover else background.set_size_request(320, 240)
    fixed.put(background, 0, 0)
    button = Gtk.Button(label="Count 0")
    button.set_size_request(140, 40)
    if not cover:
        fixed.put(button, 60, 80)
    window.add(fixed)
    state = [0]

    def draw(_widget, context):
        """Paint a saturated background that exposes square corner leakage."""
        if cover:
            context.set_source_rgb(0.6, 0.6, 0.6)
        else:
            context.set_source_rgb(1, 0, 1) if state[0] == 0 else context.set_source_rgb(0, 1, 0)
        context.paint()
        return False

    def clicked(_button):
        """Report a real GTK control activation and redraw the source surface."""
        state[0] += 1
        button.set_label(f"Count {state[0]}")
        window.queue_draw()
        print(json.dumps({"count": state[0]}), flush=True)

    def ready():
        """Publish native source identity after map and layout complete."""
        origin = window.get_window().get_origin()
        print(json.dumps({"pid": os.getpid(), "xid": window.get_window().get_xid(),
            "rect": [origin.x, origin.y, 320, 240]}), flush=True)
        return False

    def lifetime(_fd, _condition):
        """Terminate the fixture when the owning regression closes its pipe."""
        os.read(sys.stdin.fileno(), 1024)
        Gtk.main_quit()
        return False

    background.connect("draw", draw)
    button.connect("clicked", clicked)
    window.show_all()
    GLib.timeout_add(250, ready)
    GLib.io_add_watch(sys.stdin.fileno(), GLib.IO_IN | GLib.IO_HUP, lifetime)
    Gtk.main()


class NativeLinuxPreviewTests(unittest.TestCase):
    """Verify background capture, source preservation, targeted input and preview cleanup."""

    @classmethod
    def setUpClass(cls):
        """Start a private display, window manager and compositor rather than using the user's desktop."""
        cls.children = []
        cls.temp = tempfile.TemporaryDirectory(prefix="operit-x11-")
        cls.addClassCleanup(cls.cleanup)
        cls.env = dict(os.environ, DISPLAY=":193", XDG_SESSION_TYPE="x11", GDK_BACKEND="x11",
            GTK_MODULES="gail:atk-bridge", NO_AT_BRIDGE="0", XDG_RUNTIME_DIR=cls.temp.name)
        cls.env.pop("WAYLAND_DISPLAY", None)
        deps = cls.env.get("OPERIT_LINUX_TEST_DEPS")
        if deps:
            cls.env["PATH"] = f"{deps}/usr/bin:" + cls.env["PATH"]
            cls.env["LD_LIBRARY_PATH"] = f"{deps}/usr/lib64"
            cls.env["PYTHONPATH"] = f"{deps}/usr/lib/python3.14/site-packages"
            cls.env["XDG_DATA_DIRS"] = f"{deps}/usr/share:/usr/local/share:/usr/share"
        server = cls.start(["Xvfb", "-displayfd", "1", "-screen", "0", "1024x768x24", "+extension", "Composite", "-nolisten", "unix", "-listen", "tcp", "-ac"])
        if not select.select([server.stdout], [], [], 5)[0]:
            raise AssertionError("Private X server did not start")
        display = server.stdout.readline().strip()
        if not display.isdecimal():
            raise AssertionError(server.stderr.read())
        cls.env["DISPLAY"] = "127.0.0.1:" + display
        subprocess.run(["dbus-update-activation-environment", "DISPLAY=" + cls.env["DISPLAY"],
            "GDK_BACKEND=x11", "WAYLAND_DISPLAY="], env=cls.env, check=True)
        config = Path(cls.temp.name) / "openbox.xml"
        config.write_text('<openbox_config xmlns="http://openbox.org/3.4/rc"><theme><name>Clearlooks</name></theme></openbox_config>')
        manager = cls.start(["openbox", "--config-file", str(config)])
        compositor = cls.start(["xcompmgr", "-n"])
        time.sleep(0.8)
        for child in (manager, compositor):
            if child.poll() is not None:
                raise AssertionError(child.stderr.read())
        sys.path.insert(0, str(PROVIDER.parent))
        os.environ.update(cls.env)
        os.environ.pop("WAYLAND_DISPLAY", None)

    @classmethod
    def start(cls, args):
        """Track every helper for deterministic teardown even after an assertion failure."""
        child = subprocess.Popen(args, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            text=True, env=cls.env)
        cls.children.append(child)
        return child

    @classmethod
    def cleanup(cls):
        """Release all fixtures and native session resources."""
        for child in reversed(cls.children):
            if child.poll() is None:
                child.terminate()
                try:
                    child.wait(timeout=3)
                except subprocess.TimeoutExpired:
                    child.kill()
                    child.wait()
            for stream in (child.stdin, child.stdout, child.stderr):
                stream.close()
        cls.temp.cleanup()

    def invoke(self, operation, **parameters):
        """Use the actual production transport and return only its structured response."""
        result = subprocess.run([sys.executable, str(PROVIDER)], input=json.dumps({"operation": operation,
            "parameters": parameters}) + "\n", text=True, capture_output=True, env=self.env, timeout=20)
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)

    def test_live_preview_preserves_original_and_closes_on_eof(self):
        """Keep an occluded source live, rounded and operable, then close only its viewing panel."""
        import cairo
        from window_surface import X11, WindowSurface
        source = self.start([sys.executable, str(Path(__file__)), "--fixture"])
        native = response(source)
        deadline = time.monotonic() + 8
        identity = None
        while time.monotonic() < deadline:
            entries = self.invoke("list_windows")
            identity = next((entry["windowId"] for entry in entries if entry["windowId"].startswith(str(native["pid"]) + ":")), None)
            if identity:
                break
            time.sleep(0.1)
        self.assertIsNotNone(identity, "GTK fixture was not exposed through AT-SPI")
        cover = self.start([sys.executable, str(Path(__file__)), "--fixture", "--cover"])
        response(cover)
        preview = self.start([sys.executable, str(PROVIDER)])
        preview.stdin.write(json.dumps({"operation": "preview_window", "parameters": {"window_id": identity}}) + "\n")
        preview.stdin.flush()
        ready = response(preview)
        self.assertTrue(ready["ready"], ready)
        x = X11()
        try:
            self.assertEqual(x.geometry(native["xid"]), tuple(native["rect"]))
            self.assertEqual(x.property(native["xid"], "_NET_WM_WINDOW_OPACITY"), [])
            desktop = self.invoke("capture_screenshot")
            desktop = cairo.ImageSurface.create_from_png(io.BytesIO(base64.b64decode(desktop["pngBase64"])))
            self.assertEqual((desktop.get_width(), desktop.get_height()), (1024, 768))
            self.assertEqual(bytes(desktop.get_data()[120 * desktop.get_stride() + 140 * 4:][:3]), b"\x99\x99\x99")
            listing = self.invoke("list_windows", preview_process_id=preview.pid, preview_window_id=identity)
            self.assertFalse(any(item["windowId"].startswith(str(preview.pid) + ":") for item in listing))
            selected = next(item for item in listing if item["windowId"] == identity)
            self.assertNotEqual(selected["bounds"], f"[120,100][440,340]")
            screenshot = self.invoke("capture_screenshot", window_id=identity, hosted_window=True)
            image = cairo.ImageSurface.create_from_png(io.BytesIO(base64.b64decode(screenshot["pngBase64"])))
            self.assertEqual((image.get_width(), image.get_height()), (320, 240))
            self.invoke("tap", window_id=identity, hosted_window=True, x=130, y=100)
            self.assertEqual(response(source)["count"], 1)
            time.sleep(0.2)
            screenshot = self.invoke("capture_screenshot", window_id=identity, hosted_window=True)
            image = cairo.ImageSurface.create_from_png(io.BytesIO(base64.b64decode(screenshot["pngBase64"])))
            self.assertEqual(bytes(image.get_data()[20 * image.get_stride() + 20 * 4:][:3]), b"\x00\xff\x00")
            previews = [client for client in x.clients() if x.property(client, "_NET_WM_PID") == [preview.pid]
                and x.property(client, "_OPERIT_WINDOW_PREVIEW")]
            self.assertEqual(len(previews), 1)
            # Read the composited panel itself: its image must be rounded in pixel alpha.
            panel_rect = x.geometry(previews[0])
            panel = WindowSurface(x, preview.pid, panel_rect)
            try:
                image = panel.frame()
                corner = bytes(image.get_data()[19 * image.get_stride() + 19 * 4:][:4])
                self.assertLess(corner[3], 30, "The rounded corner must have only a light translucent shadow")
                center = bytes(image.get_data()[48 * image.get_stride() + 48 * 4:][:4])
                self.assertEqual(center, b"\x00\xff\x00\xff")
            finally:
                panel.close()
            # After the covering window closes, ordinary desktop input must still reach the original.
            cover.stdin.close()
            self.assertEqual(cover.wait(timeout=5), 0)
            import gi
            gi.require_version("Gdk", "3.0")
            gi.require_version("GdkX11", "3.0")
            from gi.repository import Gdk, GdkX11
            display = Gdk.Display.get_default()
            original = GdkX11.X11Window.foreign_new_for_display(display, native["xid"])
            original.raise_()
            original.focus(Gdk.CURRENT_TIME)
            display.sync()
            xtest = C.CDLL("libXtst.so.6")
            xtest.XTestFakeMotionEvent.argtypes = [C.c_void_p, C.c_int, C.c_int, C.c_int, C.c_ulong]
            xtest.XTestFakeButtonEvent.argtypes = [C.c_void_p, C.c_uint, C.c_int, C.c_ulong]
            xtest.XTestFakeMotionEvent(x.display, -1, native["rect"][0] + 130, native["rect"][1] + 100, 0)
            xtest.XTestFakeButtonEvent(x.display, 1, 1, 0)
            xtest.XTestFakeButtonEvent(x.display, 1, 0, 0)
            x.sync()
            self.assertEqual(response(source)["count"], 2)
            preview.stdin.close()
            self.assertEqual(preview.wait(timeout=5), 0, preview.stderr.read())
            self.assertEqual(x.geometry(native["xid"]), tuple(native["rect"]))
            self.assertIsNone(source.poll(), "Closing a preview must not close its original")
            self.assertNotIn(previews[0], x.clients())
            listing = self.invoke("list_windows", preview_process_id=preview.pid, preview_window_id=identity)
            self.assertTrue(any(item["windowId"] == identity for item in listing))
            # Source destruction also releases the viewing surface without requiring another tool call.
            second = self.start([sys.executable, str(PROVIDER)])
            second.stdin.write(json.dumps({"operation": "preview_window", "parameters": {"window_id": identity}}) + "\n")
            second.stdin.flush()
            self.assertTrue(response(second)["ready"])
            source.stdin.close()
            self.assertEqual(source.wait(timeout=5), 0)
            self.assertEqual(second.wait(timeout=5), 0, second.stderr.read())
        finally:
            x.close()
            source.stdin.close()
            source.wait(timeout=5)
            cover.stdin.close()
            cover.wait(timeout=5)

    def test_wayland_is_rejected_before_opening_a_capture_connection(self):
        """Report the actual unsupported capability without substituting another capture target."""
        from window_surface import X11
        with patch.dict(os.environ, {"XDG_SESSION_TYPE": "wayland"}):
            with self.assertRaisesRegex(RuntimeError, "Wayland is unsupported"):
                X11()


if __name__ == "__main__":
    if "--fixture" in sys.argv:
        fixture()
    else:
        unittest.main()
