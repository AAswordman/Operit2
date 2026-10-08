import type { Snapshot } from "../../src/model";
import type { Dialog, DialogInput, DialogOf, DialogType, GraphDialog, NativeRecord, UIState, WebAction, WebRequest, WebResponses } from "./contracts";
import type { EntityKind } from "./screens";
/** Describes the explicit dependencies shared by editor feature factories. */
export interface EditorContext {
  state: UIState; app: HTMLElement; dialogsRoot: HTMLElement;
  /** Requires the loaded aggregate without inventing empty collections. */
  snapshot(): Snapshot;
  /** Returns the active nested editor. */
  topDialog(): Dialog;
  /** Requires the active editor's exact discriminator. */
  topDialog<T extends DialogType>(type: T): DialogOf<T>;
  /** Opens a typed draft and initializes only its intentional editor defaults. */
  pushDialog<T extends DialogInput>(draft: T): DialogOf<T["type"]>;
  /** Closes only the active editor. */
  popDialog(): void;
  /** Dismisses a dialog and delivers cancellation only for an explicitly presented root. */
  dismissDialog(): Promise<void>;
  /** Reports a confirmed domain mutation to the corresponding presented entity editor. */
  entityChanged(entity: EntityKind, id: string, operation: "saved" | "deleted"): Promise<void>;
  /** Routes explicit presentation fields without editing a domain draft. */
  onScreenInput(element: HTMLInputElement): boolean;
  /** Uses the package's sole typed operation transport. */
  request<A extends WebAction>(operation: WebRequest<A>): Promise<WebResponses[A]>;
  /** Renders the sole top-level character page. */
  renderMain(): void;
  /** Renders the current nested dialog stack. */
  renderDialogs(): void;
  /** Displays an accessible status announcement. */
  toast(message: string): void;
  /** Opens the explicit owner's USER.md document. */
  openUser(ownerKey: string, name: string): Promise<void>;
  /** Opens an explicit owner's authoritative graph. */
  openGraph(ownerKey: string, name: string): Promise<void>;
  /** Requires a graph already open for this exact owner. */
  owningGraph(ownerKey: string): GraphDialog;
  /** Opens an export for a saved record. */
  openExport(type: "character" | "group", value: NativeRecord): void;
  /** Applies a camera or selection update without replacing the captured surface. */
  updateGraphSurface(dialog: GraphDialog): void;
  /** Dispatches a single accessible click through the finite action registry. */
  onClick(event: MouseEvent | KeyboardEvent): Promise<void>;
}
/** Declares one feature's exact action ownership for the UI dispatcher. */
export interface ActionFeature {
  names: readonly string[];
  /** Executes only registered actions using the injected editor context. */
  handleAction(action: string, element: HTMLElement | SVGElement): Promise<void>;
}
