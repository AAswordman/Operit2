"""AT-SPI window discovery, exact X11 surfaces and desktop input."""
import json
import sys
import time
import pyatspi


def windows(excluded_pid=None):
    """Enumerate native accessibility windows with process-qualified identities."""
    desktop = pyatspi.Registry.getDesktop(0)
    for app in desktop:
        if app.get_process_id() == excluded_pid:
            continue
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


def read_node(node, origin=(0, 0)):
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
            bounds = f"[{rect.x - origin[0]},{rect.y - origin[1]}][{rect.x + rect.width - origin[0]},{rect.y + rect.height - origin[1]}]"
    children = [read_node(child, origin) for child in node if child is not None]
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


def pointer_action(node, x, y):
    """Resolve the deepest native click action at the selected source coordinate."""
    interfaces = node.get_interfaces()
    if "Component" not in interfaces:
        return None
    rect = node.queryComponent().getExtents(pyatspi.DESKTOP_COORDS)
    if not (rect.x <= x < rect.x + rect.width and rect.y <= y < rect.y + rect.height):
        return None
    for child in node:
        if child is not None:
            action = pointer_action(child, x, y)
            if action is not None:
                return action
    if "Action" in interfaces:
        actions = node.queryAction()
        for index in range(actions.nActions):
            if actions.getName(index).casefold() in ("click", "press", "activate"):
                if not node.getState().contains(pyatspi.STATE_ENABLED):
                    raise RuntimeError("The selected accessible control is disabled")
                return actions, index
    return None


def capture_surface(window):
    """Read the selected XComposite surface, including when another window covers it."""
    from window_surface import screenshot
    if window is None:
        return screenshot()
    rect = window.queryComponent().getExtents(pyatspi.DESKTOP_COORDS)
    return screenshot(window.get_process_id(), (rect.x, rect.y, rect.width, rect.height))


def execute(request):
    """Execute a single structured request without interpreting shell source."""
    operation, params = request["operation"], dict(request["parameters"])
    if operation == "list_windows":
        result = [describe_window(*entry) for entry in windows(params.get("preview_process_id"))]
        if params.get("preview_process_id"):
            from window_surface import preview_bounds
            bounds = preview_bounds(params["preview_process_id"])
            for entry in result:
                if bounds is not None and entry["windowId"] == params["preview_window_id"]:
                    entry["bounds"] = bounds
        return result
    if operation == "capture_screenshot" and "window_id" not in params:
        return capture_surface(None)
    identity, app, window = selected_surface(params.get("window_id"))
    if operation == "preview_window":
        from window_surface import show_preview
        rect = window.queryComponent().getExtents(pyatspi.DESKTOP_COORDS)
        show_preview(app.get_process_id(), (rect.x, rect.y, rect.width, rect.height))
        return None
    if operation == "focus_window":
        # Selection is owned by the Host; only the preview's explicit menu raises the original.
        return {}
    selected = "window_id" in params
    if selected and window.getState().contains(pyatspi.STATE_ICONIFIED):
        raise RuntimeError("Focus the minimized window before inspecting or operating it")
    rect = window.queryComponent().getExtents(pyatspi.DESKTOP_COORDS) if selected else None
    origin = (rect.x, rect.y) if selected else (0, 0)
    if operation == "get_page_info":
        tree = read_node(window, origin)
        return {"windowId": identity, "packageName": app.name, "activityName": window.name, "uiElements": tree, "scope": "window" if selected else "desktop", "truncated": False}
    if operation == "capture_screenshot":
        return capture_surface(window)
    for xkey, ykey in (("x", "y"), ("start_x", "start_y"), ("end_x", "end_y")):
        if xkey in params:
            if selected and not (0 <= params[xkey] < rect.width and 0 <= params[ykey] < rect.height):
                raise ValueError("Input coordinate is outside the selected window")
            params[xkey], params[ykey] = params[xkey] + origin[0], params[ykey] + origin[1]
    if selected and operation in ("tap", "long_press", "swipe", "press_key"):
        action = pointer_action(window, params["x"], params["y"]) if operation == "tap" else None
        if action is not None:
            actions, index = action
            if not actions.doAction(index):
                raise RuntimeError("AT-SPI rejected the selected control's click action")
            return {}
        from window_surface import send_window_input
        send_window_input(app.get_process_id(), (rect.x, rect.y, rect.width, rect.height), operation, params)
        return {}
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
    request = json.loads(sys.stdin.readline())
    try:
        result = execute(request)
        if request["operation"] != "preview_window":
            json.dump(result, sys.stdout, ensure_ascii=False)
    except Exception as error:
        if request["operation"] == "preview_window":
            print(json.dumps({"ready": False, "error": str(error)}), flush=True)
        else:
            raise
