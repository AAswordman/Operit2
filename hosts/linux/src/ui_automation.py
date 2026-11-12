"""AT-SPI collection and input for the active Linux desktop session."""
import json
import sys
import time
import pyatspi

MAX_NODES = 4000
MAX_DEPTH = 40
count = 0
truncated = False


def active_surface():
    """Find the active accessible window in the session's AT-SPI desktop."""
    desktop = pyatspi.Registry.getDesktop(0)
    for app in desktop:
        for window in app:
            if window.getState().contains(pyatspi.STATE_ACTIVE):
                return app, window
    raise RuntimeError("No active AT-SPI window; enable desktop accessibility and grant session access")


def read_node(node, depth=0):
    """Convert one accessible subtree while respecting the collection budget."""
    global count, truncated
    count += 1
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
    children = []
    if depth >= MAX_DEPTH or count >= MAX_NODES:
        truncated |= node.childCount > 0
    else:
        for child in node:
            if count >= MAX_NODES:
                truncated = True
                break
            if child is not None:
                children.append(read_node(child, depth + 1))
    return {
        "className": node.getRoleName(),
        "text": text,
        "contentDesc": node.description,
        "resourceId": str(node.path),
        "bounds": bounds,
        "isClickable": "Action" in interfaces and node.queryAction().nActions > 0,
        "children": children,
    }


def find_inputs(node, resource_id, budget, matches, depth=0):
    """Collect matching editable nodes and reject an incomplete search."""
    if budget[0] >= MAX_NODES or depth > MAX_DEPTH:
        raise RuntimeError("Editable element search exceeded its node budget")
    budget[0] += 1
    selected = str(node.path) == resource_id if resource_id is not None else node.getState().contains(pyatspi.STATE_FOCUSED)
    if selected and "EditableText" in node.get_interfaces():
        matches.append(node)
    for child in node:
        if child is not None:
            find_inputs(child, resource_id, budget, matches, depth + 1)


def execute(request):
    """Execute a single structured request without interpreting shell source."""
    operation, params = request["operation"], request["parameters"]
    app, window = active_surface()
    if operation == "get_page_info":
        tree = read_node(window)
        return {"packageName": app.name, "activityName": window.name, "uiElements": tree, "scope": "desktop", "truncated": truncated}
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
        find_inputs(window, params.get("resource_id"), [0], matches)
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
