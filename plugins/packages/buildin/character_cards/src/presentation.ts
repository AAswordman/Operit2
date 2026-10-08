import type { ActivePrompt, JsonValue } from "./model";

/** Identifies the two plugin-owned entity categories without exposing them to Core. */
export type EntityKind = "card" | "group";
/** Identifies the selector's exact allowed categories. */
export type SelectionKind = EntityKind | "all";
/** Identifies the management route, which has no presentation result channel. */
export interface ManageScreen { readonly mode: "manage" }
/** Supplies readonly selector parameters rather than mutable editor state. */
export interface SelectScreen { readonly mode: "select"; readonly kind: SelectionKind; readonly selected: ActivePrompt | null }
/** Supplies the exact entity to preview without permitting a different target. */
export interface PreviewScreen { readonly mode: "preview"; readonly entity: EntityKind; readonly id: string }
/** Supplies the exact entity to edit without permitting a different target. */
export interface EditScreen { readonly mode: "edit"; readonly entity: EntityKind; readonly id: string }
/** Supplies the exact attachment scope or an explicitly unselected scope. */
export interface AttachmentScreen { readonly mode: "memory-attachment"; readonly ownerKey: string | null; readonly folderPath: string | null }
/** Defines every screen understood by the plugin-owned WebView. */
export type ScreenInput = ManageScreen | SelectScreen | PreviewScreen | EditScreen | AttachmentScreen;
/** Defines screens that require a real host presentation request. */
export type PresentedScreen = Exclude<ScreenInput, ManageScreen>;
/** Carries a known plugin screen input inside the generic host presentation envelope. */
export interface ScreenPresentation<I extends PresentedScreen = PresentedScreen> { readonly requestId: string; readonly input: I }
/** Defines results interpreted by the plugin's consumer, not by the generic host. */
export type ScreenResult =
  | { mode: "select"; selection: ActivePrompt }
  | { mode: "preview"; entity: EntityKind; id: string }
  | { mode: "edit"; entity: EntityKind; id: string; operation: "saved" | "deleted" }
  | { mode: "memory-attachment"; ownerKey: string; folderPath: string; content: string };
/** Connects package-owned views to an explicitly supplied presentation receiver. */
export interface ScreenReceiver {
  /** Delivers the package result through the active presentation action. */
  complete(result: ScreenResult): Promise<void>;
  /** Cancels only the current presentation without mutating a domain selection. */
  cancel(): Promise<void>;
}
/** Carries the readonly input and the exact host request currently owned by this screen. */
export interface CurrentScreen { readonly requestId: string | null; readonly input: ScreenInput }
/** Matches the generic PresentationV1 completion event exactly. */
export interface PresentationComplete { type: "toolpkg.presentation.complete"; requestId: string; value: JsonValue }
/** Matches the generic PresentationV1 cancellation event exactly. */
export type PresentationCancel = { type: "toolpkg.presentation.cancel"; requestId: string };
/** Owns result validation and at-most-once completion for one screen instance. */
export interface ScreenSession {
  /** Returns copied readonly parameters rather than the session's mutable internals. */
  currentScreen(): CurrentScreen;
  /** Validates the result for this exact input and closes this presentation once. */
  completeScreen(result: unknown): PresentationComplete;
  /** Closes this exact presentation with an explicit cancellation event. */
  cancelScreen(): PresentationCancel;
}

/** Requires an object with exactly the declared fields and rejects unknown protocol spellings. */
function fields(value: unknown, keys: readonly string[], path: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error(`${path} must be an object`);
  const object = value as Record<string, unknown>, actual = Object.keys(object);
  if (actual.length !== keys.length || actual.some(
    /** Compares exact field identities rather than interpreting string fragments. */
    key => keys.indexOf(key) < 0,
  )) throw new Error(`${path} must have exactly ${keys.join(", ")}`);
  return object;
}

/** Requires text without normalizing user-authored content or identity spelling. */
function text(value: unknown, path: string): string {
  if (typeof value !== "string") throw new Error(`${path} must be a string`);
  return value;
}

/** Requires a nonblank identity without changing its persisted spelling. */
function identity(value: unknown, path: string): string {
  const result = text(value, path);
  if (result.trim() === "") throw new Error(`${path} must not be blank`);
  return result;
}

/** Requires an exact plugin entity category. */
function entityKind(value: unknown): EntityKind {
  if (value !== "card" && value !== "group") throw new Error("screen.entity must be card or group");
  return value;
}

/** Decodes the complete externally tagged prompt identity and copies its readonly fields. */
function selection(value: unknown): ActivePrompt {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error("screen.selection must be an active prompt object");
  const keys = Object.keys(value);
  if (keys.length !== 1) throw new Error("screen.selection must have one active prompt discriminator");
  const object = value as Record<string, unknown>;
  switch (keys[0]) {
    case "CharacterCard": return { CharacterCard: { id: identity(fields(object.CharacterCard, ["id"], "screen.selection.CharacterCard").id, "screen.selection.id") } };
    case "CharacterGroup": return { CharacterGroup: { id: identity(fields(object.CharacterGroup, ["id"], "screen.selection.CharacterGroup").id, "screen.selection.id") } };
    default: throw new Error("screen.selection has an unknown active prompt discriminator");
  }
}

/** Parses only the plugin-owned input nested inside a generic presentation request. */
export function parseScreenInput(value: unknown): ScreenInput {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error("screen.input must be an object");
  const object = value as Record<string, unknown>;
  switch (object.mode) {
    case "manage": fields(object, ["mode"], "screen.input"); return { mode: "manage" };
    case "select": {
      fields(object, ["mode", "kind", "selected"], "screen.input");
      if (object.kind !== "card" && object.kind !== "group" && object.kind !== "all") throw new Error("screen.kind must be card, group, or all");
      const selected = object.selected === null ? null : selection(object.selected);
      if (selected !== null && ((object.kind === "card" && !("CharacterCard" in selected)) || (object.kind === "group" && !("CharacterGroup" in selected)))) throw new Error("screen.selected does not match screen.kind");
      return { mode: "select", kind: object.kind, selected };
    }
    case "preview":
    case "edit": fields(object, ["mode", "entity", "id"], "screen.input"); return { mode: object.mode, entity: entityKind(object.entity), id: identity(object.id, "screen.id") };
    case "memory-attachment": {
      fields(object, ["mode", "ownerKey", "folderPath"], "screen.input");
      const ownerKey = object.ownerKey === null ? null : identity(object.ownerKey, "screen.ownerKey");
      const folderPath = object.folderPath === null ? null : text(object.folderPath, "screen.folderPath");
      if (ownerKey === null && folderPath !== null) throw new Error("screen.folderPath requires an explicitly selected ownerKey");
      return { mode: "memory-attachment", ownerKey, folderPath };
    }
    default: throw new Error("screen.input has an unknown mode");
  }
}

/** Validates a result against the immutable mode, category, and target of its own request. */
export function parseScreenResult(value: unknown, input: PresentedScreen): ScreenResult {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error("screen.result must be an object");
  const object = value as Record<string, unknown>;
  if (object.mode !== input.mode) throw new Error("screen.result.mode does not match the active presentation");
  switch (input.mode) {
    case "select": {
      fields(object, ["mode", "selection"], "screen.result");
      const result = selection(object.selection);
      if ((input.kind === "card" && !("CharacterCard" in result)) || (input.kind === "group" && !("CharacterGroup" in result))) throw new Error("screen.result.selection does not match screen.kind");
      return { mode: "select", selection: result };
    }
    case "preview":
    case "edit": {
      fields(object, input.mode === "edit" ? ["mode", "entity", "id", "operation"] : ["mode", "entity", "id"], "screen.result");
      if (object.entity !== input.entity || object.id !== input.id) throw new Error("screen.result does not match the presented entity");
      if (input.mode === "preview") return { mode: "preview", entity: input.entity, id: input.id };
      if (object.operation !== "saved" && object.operation !== "deleted") throw new Error("screen.result.operation must be saved or deleted");
      return { mode: "edit", entity: input.entity, id: input.id, operation: object.operation };
    }
    case "memory-attachment": {
      fields(object, ["mode", "ownerKey", "folderPath", "content"], "screen.result");
      const ownerKey = identity(object.ownerKey, "screen.result.ownerKey"), folderPath = text(object.folderPath, "screen.result.folderPath");
      if ((input.ownerKey !== null && ownerKey !== input.ownerKey) || (input.folderPath !== null && folderPath !== input.folderPath)) throw new Error("screen.result does not match the presented attachment scope");
      return { mode: "memory-attachment", ownerKey, folderPath, content: text(object.content, "screen.result.content") };
    }
  }
}

/** Creates an explicit management session or validates a real generic PresentationV1 request. */
export function createScreenSession(presentation: ScreenPresentation | null): ScreenSession {
  let input: ScreenInput, requestId: string | null;
  if (presentation === null) { input = { mode: "manage" }; requestId = null; }
  else {
    const envelope = fields(presentation, ["requestId", "input"], "presentation");
    requestId = identity(envelope.requestId, "presentation.requestId");
    input = parseScreenInput(envelope.input);
    if (input.mode === "manage") throw new Error("The management route must not have a presentation request");
  }
  let finished = false;
  /** Requires the real active request before acknowledging a completion or cancellation. */
  function activeRequest(): { requestId: string; input: PresentedScreen } {
    if (requestId === null || input.mode === "manage") throw new Error("The management route has no presentation result channel");
    if (finished) throw new Error(`Presentation ${requestId} has already finished`);
    return { requestId, input };
  }
  return {
    /** Copies screen parameters so WebView consumers cannot mutate this session's contract. */
    currentScreen(): CurrentScreen { return { requestId, input: parseScreenInput(input) }; },
    /** Returns the explicit generic completion discriminator only after validating this result. */
    completeScreen(value: ScreenResult): PresentationComplete {
      const active = activeRequest(), result = parseScreenResult(value, active.input);
      finished = true;
      return { type: "toolpkg.presentation.complete", requestId: active.requestId, value: result };
    },
    /** Returns the explicit cancellation discriminator without a synthetic domain result. */
    cancelScreen(): PresentationCancel {
      const active = activeRequest();
      finished = true;
      return { type: "toolpkg.presentation.cancel", requestId: active.requestId };
    },
  };
}
