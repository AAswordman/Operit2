"""Own exact X11 window captures and a viewing panel without changing source windows."""
import base64
import ctypes as C
import io
import json
import math
import os
import sys
import time


class XImage(C.Structure):
    """Expose the documented Xlib image layout through its pixel format fields."""
    _fields_ = [("width", C.c_int), ("height", C.c_int), ("xoffset", C.c_int),
                ("format", C.c_int), ("data", C.c_void_p), ("byte_order", C.c_int),
                ("bitmap_unit", C.c_int), ("bitmap_bit_order", C.c_int),
                ("bitmap_pad", C.c_int), ("depth", C.c_int),
                ("bytes_per_line", C.c_int), ("bits_per_pixel", C.c_int),
                ("red_mask", C.c_ulong), ("green_mask", C.c_ulong), ("blue_mask", C.c_ulong)]


class Visual(C.Structure):
    """Read the visual's color masks because named pixmaps do not carry a visual."""
    _fields_ = [("ext_data", C.c_void_p), ("visualid", C.c_ulong), ("visual_class", C.c_int),
                ("red_mask", C.c_ulong), ("green_mask", C.c_ulong), ("blue_mask", C.c_ulong)]


class WindowAttributes(C.Structure):
    """Expose the native visual and mapped state with the complete Xlib structure size."""
    _fields_ = [("x", C.c_int), ("y", C.c_int), ("width", C.c_int), ("height", C.c_int),
                ("border_width", C.c_int), ("depth", C.c_int), ("visual", C.POINTER(Visual)),
                ("root", C.c_ulong), ("window_class", C.c_int), ("bit_gravity", C.c_int),
                ("win_gravity", C.c_int), ("backing_store", C.c_int), ("backing_planes", C.c_ulong),
                ("backing_pixel", C.c_ulong), ("save_under", C.c_int), ("colormap", C.c_ulong),
                ("map_installed", C.c_int), ("map_state", C.c_int), ("all_event_masks", C.c_long),
                ("your_event_mask", C.c_long), ("do_not_propagate_mask", C.c_long),
                ("override_redirect", C.c_int), ("screen", C.c_void_p)]


class XError(C.Structure):
    """Read asynchronous X errors without letting Xlib terminate the host helper."""
    _fields_ = [("type", C.c_int), ("display", C.c_void_p), ("resource", C.c_ulong),
                ("serial", C.c_ulong), ("code", C.c_ubyte),
                ("request", C.c_ubyte), ("minor", C.c_ubyte)]


class InputEvent(C.Structure):
    """Share XKeyEvent and XButtonEvent's documented wire-compatible layout."""
    _fields_ = [("type", C.c_int), ("serial", C.c_ulong), ("send_event", C.c_int),
                ("display", C.c_void_p), ("window", C.c_ulong), ("root", C.c_ulong),
                ("subwindow", C.c_ulong), ("time", C.c_ulong), ("x", C.c_int), ("y", C.c_int),
                ("x_root", C.c_int), ("y_root", C.c_int), ("state", C.c_uint),
                ("detail", C.c_uint), ("same_screen", C.c_int)]


class XEvent(C.Union):
    """Reserve the complete native XEvent size when dispatching targeted input."""
    _fields_ = [("input", InputEvent), ("pad", C.c_long * 24)]


class WindowClosed(RuntimeError):
    """Distinguish expected source destruction from capture or compositor failure."""


class X11:
    """Use one private Xlib connection and report unsupported sessions explicitly."""

    def __init__(self):
        """Bind native APIs and open only a native X11 session."""
        if os.environ.get("XDG_SESSION_TYPE") == "wayland" or os.environ.get("WAYLAND_DISPLAY"):
            raise RuntimeError("Selected-window capture and previews require a native X11 session; Wayland is unsupported")
        self.lib = C.CDLL("libX11.so.6")
        self.composite = C.CDLL("libXcomposite.so.1")
        signatures = {
            "XOpenDisplay": (C.c_void_p, [C.c_char_p]),
            "XCloseDisplay": (C.c_int, [C.c_void_p]),
            "XDefaultRootWindow": (C.c_ulong, [C.c_void_p]),
            "XDefaultScreen": (C.c_int, [C.c_void_p]),
            "XInternAtom": (C.c_ulong, [C.c_void_p, C.c_char_p, C.c_int]),
            "XGetSelectionOwner": (C.c_ulong, [C.c_void_p, C.c_ulong]),
            "XGetWindowProperty": (C.c_int, [C.c_void_p, C.c_ulong, C.c_ulong, C.c_long, C.c_long,
                C.c_int, C.c_ulong, C.POINTER(C.c_ulong), C.POINTER(C.c_int), C.POINTER(C.c_ulong),
                C.POINTER(C.c_ulong), C.POINTER(C.c_void_p)]),
            "XGetGeometry": (C.c_int, [C.c_void_p, C.c_ulong, C.POINTER(C.c_ulong), C.POINTER(C.c_int),
                C.POINTER(C.c_int), C.POINTER(C.c_uint), C.POINTER(C.c_uint), C.POINTER(C.c_uint), C.POINTER(C.c_uint)]),
            "XTranslateCoordinates": (C.c_int, [C.c_void_p, C.c_ulong, C.c_ulong, C.c_int, C.c_int,
                C.POINTER(C.c_int), C.POINTER(C.c_int), C.POINTER(C.c_ulong)]),
            "XQueryTree": (C.c_int, [C.c_void_p, C.c_ulong, C.POINTER(C.c_ulong), C.POINTER(C.c_ulong),
                C.POINTER(C.c_void_p), C.POINTER(C.c_uint)]),
            "XGetImage": (C.POINTER(XImage), [C.c_void_p, C.c_ulong, C.c_int, C.c_int, C.c_uint,
                C.c_uint, C.c_ulong, C.c_int]),
            "XGetWindowAttributes": (C.c_int, [C.c_void_p, C.c_ulong, C.POINTER(WindowAttributes)]),
            "XDestroyImage": (C.c_int, [C.POINTER(XImage)]),
            "XFreePixmap": (C.c_int, [C.c_void_p, C.c_ulong]),
            "XFree": (C.c_int, [C.c_void_p]),
            "XSync": (C.c_int, [C.c_void_p, C.c_int]),
            "XSetErrorHandler": (C.c_void_p, [C.c_void_p]),
            "XSendEvent": (C.c_int, [C.c_void_p, C.c_ulong, C.c_int, C.c_long, C.POINTER(XEvent)]),
            "XKeysymToKeycode": (C.c_ubyte, [C.c_void_p, C.c_ulong]),
            "XChangeProperty": (C.c_int, [C.c_void_p, C.c_ulong, C.c_ulong, C.c_ulong,
                C.c_int, C.c_int, C.c_void_p, C.c_int]),
        }
        for name, (result, args) in signatures.items():
            function = getattr(self.lib, name)
            function.restype, function.argtypes = result, args
        for name, result, args in [
            ("XCompositeQueryExtension", C.c_int, [C.c_void_p, C.POINTER(C.c_int), C.POINTER(C.c_int)]),
            ("XCompositeQueryVersion", C.c_int, [C.c_void_p, C.POINTER(C.c_int), C.POINTER(C.c_int)]),
            ("XCompositeRedirectWindow", None, [C.c_void_p, C.c_ulong, C.c_int]),
            ("XCompositeUnredirectWindow", None, [C.c_void_p, C.c_ulong, C.c_int]),
            ("XCompositeNameWindowPixmap", C.c_ulong, [C.c_void_p, C.c_ulong]),
        ]:
            function = getattr(self.composite, name)
            function.restype, function.argtypes = result, args
        self.display = self.lib.XOpenDisplay(None)
        if not self.display:
            raise RuntimeError("The X11 display is unavailable")
        self.errors = []
        self.error_type = C.CFUNCTYPE(C.c_int, C.c_void_p, C.POINTER(XError))
        self.handler = self.error_type(self.on_error)
        self.previous_handler = self.lib.XSetErrorHandler(C.cast(self.handler, C.c_void_p))
        self.root = self.lib.XDefaultRootWindow(self.display)

    def on_error(self, display, event):
        """Trap errors on this connection and preserve GTK's handler on its own connection."""
        if display == self.display:
            self.errors.append((event.contents.code, event.contents.request, event.contents.resource))
            return 0
        if self.previous_handler:
            return self.error_type(self.previous_handler)(display, event)
        return 0

    def sync(self, removed_window=None):
        """Turn pending native errors into explicit capture failures."""
        self.lib.XSync(self.display, 0)
        errors, self.errors = self.errors, []
        errors = [error for error in errors if not (error[0] in (3, 9) and error[2] == removed_window)]
        if errors:
            if all(error[0] in (3, 9) for error in errors) and any(error[0] == 3 for error in errors):
                raise WindowClosed("The native window was closed")
            raise RuntimeError(f"X11 window operation failed (code, request, resource): {errors}")

    def atom(self, name):
        """Resolve an exact protocol atom without guessing window titles."""
        return self.lib.XInternAtom(self.display, name.encode("ascii"), 0)

    def property(self, window, name):
        """Read a complete 32-bit EWMH property in Xlib's native long representation."""
        kind, count, remaining = C.c_ulong(), C.c_ulong(), C.c_ulong()
        format_bits, data = C.c_int(), C.c_void_p()
        result = self.lib.XGetWindowProperty(self.display, window, self.atom(name), 0, 65536, 0, 0,
            C.byref(kind), C.byref(format_bits), C.byref(count), C.byref(remaining), C.byref(data))
        try:
            self.sync()
            if result or remaining.value:
                raise RuntimeError(f"Cannot read the complete X11 {name} property")
            if not data.value:
                return []
            if format_bits.value != 32:
                raise RuntimeError(f"Unexpected X11 format for {name}")
            return list(C.cast(data, C.POINTER(C.c_ulong * count.value)).contents)
        finally:
            if data.value:
                self.lib.XFree(data)

    def clients(self):
        """Read the window manager's exact native client inventory."""
        return self.property(self.root, "_NET_CLIENT_LIST")

    def mark_preview(self, window, padding):
        """Mark only the preview with its physical shadow margin for precise indicator bounds."""
        value = C.c_ulong(padding)
        self.lib.XChangeProperty(self.display, window, self.atom("_OPERIT_WINDOW_PREVIEW"),
            self.atom("CARDINAL"), 32, 0, C.byref(value), 1)
        self.sync()

    def geometry(self, window):
        """Return root-relative drawable bounds in physical pixels."""
        root, child = C.c_ulong(), C.c_ulong()
        x, y, width, height, border, depth = C.c_int(), C.c_int(), C.c_uint(), C.c_uint(), C.c_uint(), C.c_uint()
        status = self.lib.XGetGeometry(self.display, window, C.byref(root), C.byref(x), C.byref(y),
            C.byref(width), C.byref(height), C.byref(border), C.byref(depth))
        translated = self.lib.XTranslateCoordinates(self.display, window, self.root, 0, 0, C.byref(x), C.byref(y), C.byref(child))
        self.sync()
        if not status or not translated or not width.value or not height.value:
            raise RuntimeError("The selected native window has no drawable frame")
        return x.value, y.value, width.value, height.value

    def parent(self, window):
        """Find the window manager frame without touching its position or visibility."""
        root, parent, children, count = C.c_ulong(), C.c_ulong(), C.c_void_p(), C.c_uint()
        status = self.lib.XQueryTree(self.display, window, C.byref(root), C.byref(parent), C.byref(children), C.byref(count))
        if children.value:
            self.lib.XFree(children)
        self.sync()
        if not status:
            raise RuntimeError("The selected native window hierarchy is unavailable")
        return parent.value

    def resolve(self, pid, rect):
        """Match process and exact AT-SPI geometry, rejecting ambiguous native windows."""
        matches = set()
        for client in self.clients():
            if self.property(client, "_NET_WM_PID") != [pid]:
                continue
            candidate = client
            while candidate and candidate != self.root:
                if self.geometry(candidate) == rect:
                    matches.add((client, candidate))
                    break
                candidate = self.parent(candidate)
        if len(matches) != 1:
            raise RuntimeError("The exact AT-SPI window cannot be matched to one X11 drawable; refresh list_windows")
        return matches.pop()

    def image(self, drawable, width, height, visual_window):
        """Read native pixels directly without a desktop screenshot or per-frame PNG encoding."""
        import cairo
        attributes = WindowAttributes()
        status = self.lib.XGetWindowAttributes(self.display, visual_window, C.byref(attributes))
        self.sync()
        if not status or not attributes.visual or attributes.map_state != 2:
            raise RuntimeError("The selected native window is no longer viewable")
        visual = attributes.visual.contents
        pointer = self.lib.XGetImage(self.display, drawable, 0, 0, width, height, C.c_ulong(-1).value, 2)
        try:
            self.sync()
            if not pointer:
                raise RuntimeError("X11 could not read the selected window surface")
            image = pointer.contents
            if (image.bits_per_pixel != 32 or visual.red_mask != 0xFF0000 or visual.green_mask != 0xFF00
                    or visual.blue_mask != 0xFF or image.byte_order != (0 if sys.byteorder == "little" else 1)
                    or image.depth not in (24, 32)):
                raise RuntimeError("The selected X11 pixel format is unsupported")
            pixels = bytearray(C.string_at(image.data, image.bytes_per_line * height))
            format_type = cairo.FORMAT_ARGB32 if image.depth == 32 else cairo.FORMAT_RGB24
            return cairo.ImageSurface.create_for_data(pixels, format_type, width, height, image.bytes_per_line)
        finally:
            if pointer:
                self.lib.XDestroyImage(pointer)

    def input_target(self, window, root_x, root_y):
        """Resolve source descendants without hit-testing the user's overlying desktop windows."""
        target, x, y, child = window, C.c_int(), C.c_int(), C.c_ulong()
        while True:
            status = self.lib.XTranslateCoordinates(self.display, self.root, target, root_x, root_y,
                C.byref(x), C.byref(y), C.byref(child))
            self.sync()
            if not status:
                raise RuntimeError("The selected input coordinate space is unavailable")
            if not child.value:
                return target, x.value, y.value
            target = child.value

    def send_input(self, window, event_type, detail, root_x=0, root_y=0, state=0):
        """Dispatch native events to the selected source without moving the user's cursor or focus."""
        if event_type in (2, 3):
            target, x, y = window, 0, 0
        else:
            target, x, y = self.input_target(window, root_x, root_y)
        masks = {2: 1, 3: 2, 4: 4, 5: 8, 6: 64 | 256}
        event = XEvent()
        event.input = InputEvent(event_type, 0, 1, self.display, target, self.root, 0, 0,
            x, y, root_x, root_y, state, detail, 1)
        result = self.lib.XSendEvent(self.display, target, 1, masks[event_type], C.byref(event))
        self.sync()
        if not result:
            raise RuntimeError("X11 rejected input for the selected window")

    def close(self):
        """Release the private connection and its error handler."""
        if self.display:
            self.lib.XCloseDisplay(self.display)
            self.lib.XSetErrorHandler(self.previous_handler)
            self.display = None


class WindowSurface:
    """Keep an automatic Composite redirection while the original stays visible and usable."""

    def __init__(self, connection, pid, rect, client_only=False):
        """Resolve one exact source and require Composite's named-window-pixmap API."""
        self.x = connection
        self.pixmap, self.redirected = 0, False
        self.client, self.window = self.x.resolve(pid, rect)
        if client_only:
            self.window = self.client
        event, error, major, minor = C.c_int(), C.c_int(), C.c_int(), C.c_int()
        if (not self.x.composite.XCompositeQueryExtension(self.x.display, C.byref(event), C.byref(error))
                or not self.x.composite.XCompositeQueryVersion(self.x.display, C.byref(major), C.byref(minor))
                or (major.value, minor.value) < (0, 2)):
            raise RuntimeError("Selected-window capture requires XComposite 0.2 or later")
        self.x.composite.XCompositeRedirectWindow(self.x.display, self.window, 0)
        self.x.sync()
        self.redirected = True
        self.size = None

    def frame(self):
        """Rebind only after source resize; named pixmaps keep updating behind occluding windows."""
        _, _, width, height = self.x.geometry(self.window)
        if self.size != (width, height):
            if self.pixmap:
                self.x.lib.XFreePixmap(self.x.display, self.pixmap)
                self.pixmap = 0
            self.pixmap = self.x.composite.XCompositeNameWindowPixmap(self.x.display, self.window)
            self.x.sync()
            if not self.pixmap:
                raise RuntimeError("The selected window has no mapped XComposite surface")
            self.size = (width, height)
        return self.x.image(self.pixmap, width, height, self.window)

    def close(self):
        """Release only capture resources, leaving the original application untouched."""
        if self.pixmap:
            self.x.lib.XFreePixmap(self.x.display, self.pixmap)
            self.pixmap = 0
        if self.redirected:
            self.x.composite.XCompositeUnredirectWindow(self.x.display, self.window, 0)
            self.redirected = False
        self.x.sync(removed_window=self.window)


def screenshot(pid=None, rect=None):
    """Capture the selected drawable or the entire root using the same physical coordinate space."""
    connection, surface = X11(), None
    try:
        if pid is None:
            _, _, width, height = connection.geometry(connection.root)
            image = connection.image(connection.root, width, height, connection.root)
        else:
            surface = WindowSurface(connection, pid, rect)
            image = surface.frame()
        output = io.BytesIO()
        image.write_to_png(output)
        return {"pngBase64": base64.b64encode(output.getvalue()).decode("ascii")}
    finally:
        if surface:
            surface.close()
        connection.close()


def send_window_input(pid, rect, operation, params):
    """Dispatch selected-window pointer and key input through its exact native X11 hierarchy."""
    connection = X11()
    try:
        client, drawable = connection.resolve(pid, rect)
        if operation == "press_key":
            keys = {"ENTER": 0xFF0D, "TAB": 0xFF09, "ESCAPE": 0xFF1B, "BACKSPACE": 0xFF08,
                "DELETE": 0xFFFF, "HOME": 0xFF50, "END": 0xFF57, "LEFT": 0xFF51,
                "UP": 0xFF52, "RIGHT": 0xFF53, "DOWN": 0xFF54}
            code = connection.lib.XKeysymToKeycode(connection.display, keys[params["key_code"].upper()])
            if not code:
                raise RuntimeError("The requested key is unavailable in the X11 keymap")
            connection.send_input(client, 2, code)
            connection.send_input(client, 3, code)
        elif operation in ("tap", "long_press"):
            x, y = int(params["x"]), int(params["y"])
            connection.send_input(drawable, 4, 1, x, y)
            try:
                if operation == "long_press":
                    time.sleep(0.6)
            finally:
                connection.send_input(drawable, 5, 1, x, y, 256)
        elif operation == "swipe":
            sx, sy, ex, ey = [float(params[key]) for key in ("start_x", "start_y", "end_x", "end_y")]
            connection.send_input(drawable, 4, 1, int(sx), int(sy))
            try:
                for step in range(1, 31):
                    connection.send_input(drawable, 6, 0, int(sx + (ex - sx) * step / 30), int(sy + (ey - sy) * step / 30), 256)
                    time.sleep(params["duration"] / 30000)
            finally:
                connection.send_input(drawable, 5, 1, int(ex), int(ey), 256)
        else:
            raise ValueError(f"Unsupported selected-window input: {operation}")
    finally:
        connection.close()


def preview_bounds(pid):
    """Locate only the host helper's marked preview, excluding it from automation discovery."""
    connection = X11()
    try:
        matches = [window for window in connection.clients()
            if connection.property(window, "_NET_WM_PID") == [pid]
            and connection.property(window, "_OPERIT_WINDOW_PREVIEW")]
        if not matches:
            return None
        if len(matches) != 1:
            raise RuntimeError("The selected window preview identity is ambiguous")
        x, y, width, height = connection.geometry(matches[0])
        padding, = connection.property(matches[0], "_OPERIT_WINDOW_PREVIEW")
        # The outside margin contains the shadow, not the interactive viewing card.
        return f"[{x + padding},{y + padding}][{x + width - padding},{y + height - padding}]"
    finally:
        connection.close()


def show_preview(pid, rect):
    """Run a rounded, nonactivating GTK viewing panel under its owner's stdin lifetime."""
    import cairo
    import gi
    gi.require_version("Gtk", "3.0")
    gi.require_version("Gdk", "3.0")
    gi.require_version("GdkX11", "3.0")
    from gi.repository import Gtk, Gdk, GdkX11, GLib

    # Validate the session before GTK can select an implicit GDK backend.
    connection = X11()
    source = None
    try:
        screen = Gdk.Screen.get_default()
        if screen is None or not isinstance(Gdk.Display.get_default(), GdkX11.X11Display):
            raise RuntimeError("Window previews require the GTK X11 backend")
        if not screen.is_composited() or screen.get_rgba_visual() is None:
            raise RuntimeError("Rounded window previews require an X11 compositing window manager")
        source = WindowSurface(connection, pid, rect, client_only=True)
        frame = [source.frame()]
        window = Gtk.Window(type=Gtk.WindowType.TOPLEVEL)
        window.set_title("Operit Window Preview")
        window.set_wmclass("operit-window-preview", "OperitWindowPreview")
        window.set_decorated(False)
        window.set_resizable(False)
        window.set_accept_focus(False)
        window.set_focus_on_map(False)
        window.set_skip_taskbar_hint(True)
        window.set_skip_pager_hint(True)
        window.set_keep_above(True)
        window.set_app_paintable(True)
        window.set_visual(screen.get_rgba_visual())
        window.add_events(Gdk.EventMask.BUTTON_PRESS_MASK)
        state = {"closed": False, "failed": False, "ready": False, "width": 320, "docked": False}
        monitor = window.get_display().get_primary_monitor()
        if monitor is None:
            raise RuntimeError("The desktop has no primary preview monitor")
        area = monitor.get_workarea()

        def dimensions():
            """Keep preview geometry stable between content frames."""
            if state["docked"]:
                return 64, 64
            ratio = min(state["width"] / frame[0].get_width(), area.height * 0.7 / frame[0].get_height())
            return max(64, round(frame[0].get_width() * ratio)), max(64, round(frame[0].get_height() * ratio))

        def outline(context, x, y, width, height, radius):
            """Construct the same rounded contour for content and its subtle shadow."""
            for cx, cy, start in [(x + width - radius, y + radius, -math.pi / 2),
                    (x + width - radius, y + height - radius, 0),
                    (x + radius, y + height - radius, math.pi / 2), (x + radius, y + radius, math.pi)]:
                context.arc(cx, cy, radius, start, start + math.pi / 2)
            context.close_path()

        def resize():
            """Resize only the panel and exclude its shadow from pointer input."""
            width, height = dimensions()
            window.resize(width + 36, height + 36)
            radius = min(22, width / 2, height / 2)
            region = cairo.Region()
            for y in range(height):
                distance = max(0, radius - (y + 0.5), (y + 0.5) - (height - radius))
                inset = math.ceil(radius - math.sqrt(max(0, radius * radius - distance * distance)))
                region.union(cairo.RectangleInt(18 + inset, 18 + y, max(1, width - 2 * inset), 1))
            window.input_shape_combine_region(region)

        def draw(widget, context):
            """Paint native pixels inside a real alpha-clipped rounded card."""
            context.set_operator(cairo.OPERATOR_SOURCE)
            context.set_source_rgba(0, 0, 0, 0)
            context.paint()
            context.set_operator(cairo.OPERATOR_OVER)
            width, height = dimensions()
            radius = min(22, width / 2, height / 2)
            for spread in range(12, 0, -1):
                outline(context, 18 - spread, 20 - spread, width + spread * 2, height + spread * 2, radius + spread)
                context.set_source_rgba(0, 0, 0, 2 / 255)
                context.fill()
            context.save()
            outline(context, 18, 18, width, height, radius)
            context.clip()
            context.translate(18, 18)
            context.scale(width / frame[0].get_width(), height / frame[0].get_height())
            context.set_source_surface(frame[0], 0, 0)
            context.paint()
            context.restore()
            if not state["ready"]:
                state["ready"] = True
                print(json.dumps({"ready": True}), flush=True)
            return False

        def shutdown(*_args):
            """Close the preview when its menu, source, or parent pipe closes."""
            if not state["closed"]:
                state["closed"] = True
                window.destroy()
                Gtk.main_quit()
            return False

        def refresh():
            """Refresh at fifteen frames per second without moving or activating the source."""
            if state["closed"]:
                return False
            try:
                latest = source.frame()
                changed = (latest.get_width(), latest.get_height()) != (frame[0].get_width(), frame[0].get_height())
                frame[0] = latest
                if changed:
                    resize()
                window.queue_draw()
                return True
            except WindowClosed:
                return shutdown()
            except Exception as error:
                print(str(error), file=sys.stderr, flush=True)
                state["failed"] = True
                return shutdown()

        def size_action(_item, width):
            """Apply the user's requested viewing width without resizing the source."""
            state["width"], state["docked"] = width, False
            dock.set_label("Minimize preview")
            resize()

        def toggle_dock(_item):
            """Dock in the top-right corner and restore the prior viewing width."""
            state["docked"] = not state["docked"]
            dock.set_label("Restore preview" if state["docked"] else "Minimize preview")
            resize()
            width, _ = dimensions()
            window.move(area.x + area.width - width - 34, area.y - 2)

        def original(_item, fullscreen=False):
            """Let the window manager raise or fullscreen the original only on user request."""
            native = GdkX11.X11Window.foreign_new_for_display(window.get_display(), source.client)
            if native is None:
                raise RuntimeError("The original X11 window is unavailable")
            if fullscreen:
                native.fullscreen()
            native.focus(Gdk.CURRENT_TIME)
            shutdown()

        menu = Gtk.Menu()
        for label, action in [("Open original window", original), ("Fullscreen", lambda item: original(item, True))]:
            item = Gtk.MenuItem(label=label)
            item.connect("activate", action)
            menu.append(item)
        dock = Gtk.MenuItem(label="Minimize preview")
        dock.connect("activate", toggle_dock)
        menu.append(dock)
        sizes = Gtk.Menu()
        for width in (240, 320, 480):
            item = Gtk.MenuItem(label=f"{width} px")
            item.connect("activate", size_action, width)
            sizes.append(item)
        size_item = Gtk.MenuItem(label="Preview size")
        size_item.set_submenu(sizes)
        menu.append(size_item)
        close = Gtk.MenuItem(label="Close preview")
        close.connect("activate", shutdown)
        menu.append(Gtk.SeparatorMenuItem())
        menu.append(close)
        menu.show_all()

        def press(_widget, event):
            """Drag the viewing card or show its local actions without forwarding source clicks."""
            if event.button == 1:
                window.begin_move_drag(1, int(event.x_root), int(event.y_root), event.time)
            elif event.button == 3:
                menu.popup_at_pointer(event)
            return True

        def owner_closed(fd, _condition):
            """Observe EOF on GTK's event loop without leaving a blocked Python reader at shutdown."""
            if not os.read(fd, 4096):
                return shutdown()
            return True

        window.connect("draw", draw)
        window.connect("button-press-event", press)
        window.connect("delete-event", shutdown)
        window.realize()
        # Tag only this helper's window; no source property, opacity or geometry changes.
        connection.mark_preview(window.get_window().get_xid(), 18 * window.get_scale_factor())
        resize()
        width, _ = dimensions()
        window.move(area.x + area.width - width - 34, area.y - 2)
        window.show_all()
        GLib.io_add_watch(sys.stdin.fileno(), GLib.IO_IN | GLib.IO_HUP, owner_closed)
        GLib.timeout_add(67, refresh)
        Gtk.main()
        if state["failed"]:
            raise SystemExit(1)
    finally:
        if source:
            source.close()
        connection.close()
