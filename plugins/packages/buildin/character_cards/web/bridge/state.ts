import type { UIState } from "./contracts";
/** Creates only ephemeral editor state; persisted records are supplied by the package. */
export function createState(): UIState {
  return { managementView: "characters", avatarSources: new Map(), snapshot: null, dialogs: [], busy: false, error: "", toastTimer: null, pointers: new Map(), gestureMoved: false };
}
