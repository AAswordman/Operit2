import type { Theme } from "../../../src/model";
import type { UIState } from "../../bridge/contracts";
import { applyMaterialTheme } from "./material";

/** Installs the live host Material palette and invalidates only existing graph measurements. */
export function installTheme(state: UIState): void {
  /** Applies the shared actual host palette without selecting another theme implementation. */
  window.applyCharacterMemoryTheme = function applyCharacterMemoryTheme(theme: Theme): void {
    applyMaterialTheme(theme);
    for (const dialog of state.dialogs) if (dialog.type === "graph") dialog.layoutSignature = null;
  };
}
