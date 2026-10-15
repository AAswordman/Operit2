import { createEditor } from "./bridge/editor";
import { host, waitForHost } from "./bridge/transport";
import { initializeHostScreen } from "./bridge/screens";
import { installTheme } from "./shared/ui/theme";

performance.mark("character-memory.app-start");
const editor = createEditor();
installTheme(editor.state);
editor.mount();

/** Initializes the exact management or popup mode supplied by the existing package UiRoute. */
async function initialize(): Promise<void> {
  try {
    await waitForHost();
    performance.mark("character-memory.host-ready");
    performance.measure("character-memory.host-wait", "character-memory.app-start", "character-memory.host-ready");
    const started = performance.now();
    const initial = await host().loadScreen();
    performance.measure("character-memory.load-screen", { start: started, end: performance.now() });
    window.applyCharacterMemoryTheme(initial.theme);
    await initializeHostScreen(editor, initial);
    performance.mark("character-memory.content-ready");
  } catch (error) { editor.showStartupError(error); }
}

void initialize();
