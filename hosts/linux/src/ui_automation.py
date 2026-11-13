"""AT-SPI window discovery, complete window trees and desktop input."""
import json
import sys
import time
import pyatspi

def windows():
    """Enumerate native accessibility windows with process-qualified identities."""
    desktop = pyatspi.Registry.getDesktop(0)
    for app in desktop:
        for window in app:
            yield f"{app.get_process_id()}:{window.path}", app, window


def window_bounds(window):
    """Read screen bounds only when the accessible object exposes Component."""
    if "Component" not in window.get_interfaces():
        return None
    rect = window.queryComponent().getExtents(pyatspi.DESKTOP_COORDS)
    return f"[{rect.x},{rect.y}][{rect.x + rect.width},{rect.y + rect.height}]"


def describe_window(identity, app, window):
    """Describe one top-level accessible window without traversing its children."""
    state = window.getState()
    return {
        "windowId": identity, "title": window.name, "packageName": app.name,
        "bounds": window_bounds(window), "isActive": state.contains(pyatspi.STATE_ACTIVE),
        "isFocused": state.contains(pyatspi.STATE_ACTIVE),
        "isMinimized": state.contains(pyatspi.STATE_ICONIFIED),
    }


def selected_surface(identity):
    """Resolve an explicit native identity or the session's active window."""
    for window_id, app, window in windows():
        if (identity is not None and window_id == identity) or (identity is None and window.getState().contains(pyatspi.STATE_ACTIVE)):
            return window_id, app, window
    raise RuntimeError("The selected AT-SPI window is unavailable; enable accessibility and refresh list_windows")


def read_node(node):
    """Preserve every exposed descendant and its native role and states."""
    interfaces = node.get_interfaces()
    text = None
    if node.getRole() != pyatspi.ROLE_PASSWORD_TEXT:
        parts = [node.name] if node.name else []
        if "Text" in interfaces:
            content = node.queryText().getText(0, -1)
            if content and content != node.name:
                parts.append(content)
        text = "\n".join(parts) or None
    bounds = None
    if "Component" in interfaces:
        rect = node.queryComponent().getExtents(pyatspi.DESKTOP_COORDS)
        if rect.width > 0 and rect.height > 0:
            bounds = f"[{rect.x},{rect.y}][{rect.x + rect.width},{rect.y + rect.height}]"
    children = [read_node(child) for child in node if child is not None]
    state = node.getState()
    return {
        "className": node.getRoleName(),
        "text": text,
        "contentDesc": node.description,
        "resourceId": str(node.path),
        "bounds": bounds,
        "isClickable": "Action" in interfaces and node.queryAction().nActions > 0,
        "properties": {
            "role": node.getRoleName(), "isEnabled": state.contains(pyatspi.STATE_ENABLED),
            "isFocused": state.contains(pyatspi.STATE_FOCUSED),
            "isFocusable": state.contains(pyatspi.STATE_FOCUSABLE),
            "isEditable": state.contains(pyatspi.STATE_EDITABLE),
            "isShowing": state.contains(pyatspi.STATE_SHOWING), "interfaces": list(interfaces),
        },
        "children": children,
    }


def find_inputs(node, resource_id, matches):
    """Collect matching editable nodes throughout the selected window."""
    selected = str(node.path) == resource_id if resource_id is not None else node.getState().contains(pyatspi.STATE_FOCUSED)
    if selected and "EditableText" in node.get_interfaces():
        matches.append(node)
    for child in node:
        if child is not None:
            find_inputs(child, resource_id, matches)


def execute(request):
    """Execute a single structured request without interpreting shell source."""
    operation, params = request["operation"], request["parameters"]
    if operation == "list_windows":
        return [describe_window(*entry) for entry in windows()]
    identity, app, window = selected_surface(params.get("window_id"))
    if operation == "focus_window":
        if not window.queryComponent().grabFocus():
            raise RuntimeError("AT-SPI rejected focus for the selected window")
        deadline = time.monotonic() + 2
        while not window.getState().contains(pyatspi.STATE_ACTIVE):
            if time.monotonic() >= deadline:
                raise RuntimeError("The selected AT-SPI window did not become active")
            time.sleep(0.02)
        return {}
    if operation == "get_page_info":
        tree = read_node(window)
        return {"windowId": identity, "packageName": app.name, "activityName": window.name, "uiElements": tree, "scope": "desktop", "truncated": False}
    if operation in ("tap", "long_press"):
        x, y = int(params["x"]), int(params["y"])
        if operation == "tap":
            pyatspi.Registry.generateMouseEvent(x, y, "b1c")
        else:
            pyatspi.Registry.generateMouseEvent(x, y, "b1p")
            try:
                time.sleep(0.6)
            finally:
                pyatspi.Registry.generateMouseEvent(x, y, "b1r")
    elif operation == "swipe":
        sx, sy, ex, ey = [float(params[key]) for key in ("start_x", "start_y", "end_x", "end_y")]
        pyatspi.Registry.generateMouseEvent(int(sx), int(sy), "b1p")
        try:
            for step in range(1, 31):
                pyatspi.Registry.generateMouseEvent(int(sx + (ex - sx) * step / 30), int(sy + (ey - sy) * step / 30), "abs")
                time.sleep(params["duration"] / 30000)
        finally:
            pyatspi.Registry.generateMouseEvent(int(ex), int(ey), "b1r")
    elif operation == "set_input_text":
        matches = []
        find_inputs(window, params.get("resource_id"), matches)
        if len(matches) != 1:
            raise RuntimeError("Select one unique editable accessible element")
        if not matches[0].queryEditableText().setTextContents(params["text"]):
            raise RuntimeError("AT-SPI rejected text input")
    elif operation == "press_key":
        keys = {"ENTER": 0xFF0D, "TAB": 0xFF09, "ESCAPE": 0xFF1B, "BACKSPACE": 0xFF08, "DELETE": 0xFFFF, "HOME": 0xFF50, "END": 0xFF57, "LEFT": 0xFF51, "UP": 0xFF52, "RIGHT": 0xFF53, "DOWN": 0xFF54}
        pyatspi.Registry.generateKeyboardEvent(keys[params["key_code"].upper()], None, pyatspi.KEY_SYM)
    else:
        raise ValueError(f"Unknown UI operation: {operation}")
    return {}


if __name__ == "__main__":
    json.dump(execute(json.load(sys.stdin)), sys.stdout, ensure_ascii=False)
