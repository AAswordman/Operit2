"""Exercise the Linux provider against an AT-SPI session with real nested fake objects."""
import importlib.util
from pathlib import Path
import sys
import types
import unittest


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
        """Expose no optional patterns for simple container nodes."""
        return []

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
        fake.Registry = types.SimpleNamespace(getDesktop=lambda index: [self.app])
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
        with self.assertRaisesRegex(RuntimeError, "unavailable"):
            self.provider.execute({"operation": "get_page_info", "parameters": {"window_id": "closed"}})

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
