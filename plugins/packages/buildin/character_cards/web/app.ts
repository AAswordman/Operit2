import { createEditor } from "./bridge/editor";
import { host, waitForHost } from "./bridge/transport";
import { initializeHostScreen } from "./bridge/screens";
import { installTheme } from "./shared/ui/theme";

const editor = createEditor();
installTheme(editor.state);
editor.mount();

/** Initializes the exact management or popup mode supplied by the existing package UiRoute. */
async function initialize(): Promise<void> {
  try {
    await waitForHost();
    window.applyCharacterMemoryTheme(await host().currentTheme());
    await initializeHostScreen(editor);
  } catch (error) { editor.showStartupError(error); }
}

void initialize();
