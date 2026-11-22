"""Exercise the Linux provider against an AT-SPI session with real nested fake objects."""
import importlib.util
from pathlib import Path
import sys
import types
import unittest
from unittest.mock import patch


class State:
    """Expose the AT-SPI state membership contract."""

    def __init__(self, *states):
        """Store the states supplied by the fake accessibility provider."""
        self.states = set(states)

    def contains(self, state):
        """Read one exact accessibility state."""
        return state in self.states


class Node:
    """Model native identities and descendants independently of window order."""

    def __init__(self, name, path, children=(), states=()):
        """Create an accessible object with the requested native data."""
        self.name, self.path, self.children = name, path, list(children)
        self.description, self.state = "", State(*states)

    def __iter__(self):
        """Yield descendants in provider order."""
        return iter(self.children)

    def get_process_id(self):
        """Return a native process identity shared by this fake application's objects."""
        return 42

    def getState(self):
        """Return live window state."""
        return self.state

    def get_interfaces(self):
        """Expose real desktop bounds for local-coordinate conversion."""
        return ["Component"]

    def queryComponent(self):
        """Expose a stable frame at a nonzero desktop offset."""
        return types.SimpleNamespace(getExtents=lambda coords: types.SimpleNamespace(x=120, y=80, width=640, height=480))

    def getRole(self):
        """Return a non-password role."""
        return 1

    def getRoleName(self):
        """Return a native accessibility role name."""
        return "panel"


class LinuxWindowAutomationTests(unittest.TestCase):
    """Verify discovery and complete trees at the actual platform-provider boundary."""

    def setUp(self):
        """Import the production provider against an isolated AT-SPI registry."""
        self.active = Node("active", "/active", states=("active",))
        self.other = Node("background", "/background")
        self.app = Node("editor", "/app", (self.active, self.other))
        fake = types.ModuleType("pyatspi")
        for state in ("ACTIVE", "FOCUSED", "ENABLED", "FOCUSABLE", "EDITABLE", "SHOWING", "ICONIFIED"):
            setattr(fake, "STATE_" + state, state.lower())
        fake.ROLE_PASSWORD_TEXT = 99
        fake.DESKTOP_COORDS = 0
        self.mouse = []
        fake.Registry = types.SimpleNamespace(getDesktop=lambda index: [self.app], generateMouseEvent=lambda *args: self.mouse.append(args))
        self.original = sys.modules.get("pyatspi")
        sys.modules["pyatspi"] = fake
        path = Path(__file__).resolve().parents[2] / "hosts/linux/src/ui_automation.py"
        spec = importlib.util.spec_from_file_location("operit_linux_ui_provider", path)
        self.provider = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.provider)

    def tearDown(self):
        """Restore the interpreter's original module binding."""
        if self.original is None:
            sys.modules.pop("pyatspi", None)
        else:
            sys.modules["pyatspi"] = self.original

    def test_discovery_and_explicit_selection_survive_window_reordering(self):
        """Keep selection independent of foreground state, titles and list positions."""
        windows = self.provider.execute({"operation": "list_windows", "parameters": {}})
        self.assertEqual([item["windowId"] for item in windows], ["42:/active", "42:/background"])
        self.app.children.reverse()
        page = self.provider.execute({"operation": "get_page_info", "parameters": {"window_id": "42:/background"}})
        self.assertEqual(page["activityName"], "background")
        self.assertEqual(page["windowId"], "42:/background")
        self.assertFalse(page["truncated"])
        self.assertEqual(page["scope"], "window")
        self.assertEqual(page["uiElements"]["bounds"], "[0,0][640,480]")
        with self.assertRaisesRegex(RuntimeError, "unavailable"):
            self.provider.execute({"operation": "get_page_info", "parameters": {"window_id": "closed"}})

    def test_local_input_and_main_screen_coordinates(self):
        """Translate local input once and leave unselected screen input unchanged."""
        sent = []
        surface = types.ModuleType("window_surface")
        surface.send_window_input = lambda *args: sent.append(args)
        with patch.dict(sys.modules, {"window_surface": surface}):
            self.provider.execute({"operation": "tap", "parameters": {"window_id": "42:/active", "x": 10, "y": 20}})
            self.provider.execute({"operation": "tap", "parameters": {"window_id": "42:/background", "x": 10, "y": 20}})
        self.assertEqual([(item[3]["x"], item[3]["y"]) for item in sent], [(130, 100), (130, 100)])
        self.assertEqual(self.mouse, [])
        self.provider.execute({"operation": "tap", "parameters": {"x": 10, "y": 20}})
        self.assertEqual(self.mouse[-1], (10, 20, "b1c"))
        page = self.provider.execute({"operation": "get_page_info", "parameters": {}})
        self.assertEqual(page["scope"], "desktop")
        self.assertEqual(page["uiElements"]["bounds"], "[120,80][760,560]")
        with self.assertRaisesRegex(ValueError, "outside"):
            self.provider.execute({"operation": "tap", "parameters": {"window_id": "42:/active", "x": 640, "y": 20}})

    def test_screenshot_keeps_the_explicit_surface(self):
        """Route screenshots to the selected object and reject stale identities."""
        self.provider.capture_surface = lambda window: {"target": None if window is None else window.path}
        self.assertEqual(self.provider.execute({"operation": "capture_screenshot", "parameters": {}}), {"target": None})
        self.assertEqual(self.provider.execute({"operation": "capture_screenshot", "parameters": {"window_id": "42:/active"}}), {"target": "/active"})
        with self.assertRaisesRegex(RuntimeError, "unavailable"):
            self.provider.execute({"operation": "capture_screenshot", "parameters": {"window_id": "closed"}})

    def test_hosted_input_targets_the_selected_background_source(self):
        """Avoid global desktop input while converting selected local coordinates exactly once."""
        sent = []
        surface = types.ModuleType("window_surface")
        surface.send_window_input = lambda *args: sent.append(args)
        with patch.dict(sys.modules, {"window_surface": surface}):
            self.provider.execute({"operation": "focus_window", "parameters": {
                "window_id": "42:/background", "hosted_window": True}})
            self.provider.execute({"operation": "tap", "parameters": {
                "window_id": "42:/background", "hosted_window": True, "x": 10, "y": 20}})
        self.assertEqual(sent[0][:3], (42, (120, 80, 640, 480), "tap"))
        self.assertEqual((sent[0][3]["x"], sent[0][3]["y"]), (130, 100))
        self.assertEqual(self.mouse, [])

    def test_preview_is_excluded_and_only_indicator_bounds_change(self):
        """Keep source identity and tree coordinates independent from the visible viewing card."""
        preview = Node("preview", "/preview")
        preview.get_process_id = lambda: 99
        preview.children = [Node("viewing card", "/card")]
        registry = sys.modules["pyatspi"].Registry
        registry.getDesktop = lambda index: [self.app, preview]
        surface = types.ModuleType("window_surface")
        surface.preview_bounds = lambda pid: "[900,20][1220,260]"
        with patch.dict(sys.modules, {"window_surface": surface}):
            entries = self.provider.execute({"operation": "list_windows", "parameters": {
                "preview_process_id": 99, "preview_window_id": "42:/background"}})
        self.assertEqual(len(entries), 2)
        self.assertEqual(entries[1]["windowId"], "42:/background")
        self.assertEqual(entries[1]["bounds"], "[900,20][1220,260]")
        page = self.provider.execute({"operation": "get_page_info", "parameters": {"window_id": "42:/background"}})
        self.assertEqual(page["uiElements"]["bounds"], "[0,0][640,480]")

    def test_full_tree_exceeds_the_old_width_and_depth_limits(self):
        """Retain the final descendant after more than 4000 nodes and 40 levels."""
        self.other.children = [Node(str(index), f"/node/{index}") for index in range(4501)]
        deep = Node("last-deep-node", "/deep/last")
        for index in range(55):
            deep = Node("container", f"/deep/{index}", [deep])
        self.other.children.append(deep)
        page = self.provider.execute({"operation": "get_page_info", "parameters": {"window_id": "42:/background"}})
        tree = page["uiElements"]
        self.assertEqual(tree["children"][4500]["text"], "4500")
        node = tree["children"][-1]
        for _ in range(55):
            node = node["children"][0]
        self.assertEqual(node["text"], "last-deep-node")
        self.assertEqual(node["properties"]["role"], "panel")


if __name__ == "__main__":
    unittest.main()
