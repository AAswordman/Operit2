import type { CharacterRepository } from "./canonical";
import { parseChatSelection } from "./chat-bindings";
import { assertCard, assertGroup, requireId, assertObject, assertString } from "./validation";
import type { DomainOperation, DomainInput } from "./api";
import { parseDomainPayload } from "./domain";
import type { Request } from "./model";

/** Captures only plugin-owned references; independent configuration values remain with their actual owners. */
export interface SelectionReferences {
  readonly selection: string;
  readonly themeConfigId: string | null;
  readonly ttsConfigId: string | null;
}

/** Defines one private picker row, not an independent Theme SDK record or a persisted role-owned theme. */
export interface ThemeChoice { readonly id: string; readonly label: string }

/** Describes local application dependencies, not a new SDK DTO, host registration or storage framework. */
export interface IndependentConfigurationAccess {
  /** Projects actual independent theme records into ephemeral picker labels without storing their values in roles. */
  readThemeChoices(): Promise<readonly ThemeChoice[]>;
  /** Rejects an unknown or deleted independent theme before any selection mutation. */
  validateTheme(id: string): Promise<void>;
  /** Rejects an unknown or deleted independent speech config before any selection mutation. */
  validateTts(id: string): Promise<void>;
  /** Resolves only after the real theme owner has confirmed applying this exact config. */
  applyTheme(id: string): Promise<void>;
  /** Resolves only after the real speech owner has confirmed applying this exact config. */
  applyTts(id: string): Promise<void>;
}

/** Records a confirmed step without claiming that writes to independent stores were atomic. */
export type ApplicationStep =
  | { readonly type: "selection"; readonly selection: string }
  | { readonly type: "theme"; readonly id: string }
  | { readonly type: "tts"; readonly id: string };

/** Identifies the independent write that rejected; its persistence status requires explicit verification. */
export type FailedConfigurationStep = { readonly type: "theme" | "tts"; readonly id: string };

/** Lets a real repository operation prevalidate its references inside its own immutable record snapshot. */
export type ValidateReferences = (references: SelectionReferences) => Promise<SelectionReferences>;

/** Returns only after the selection owner has actually confirmed its write or file publication. */
export interface SelectionCommit<T> { readonly references: SelectionReferences; readonly value: T }

/** Coordinates validation, one real selection commit, and awaited independent applications without compensation. */
export interface SelectionApplication {
  /** Serializes user selection operations through their full application sequence, preserving every rejection. */
  commit<T>(prepareAndCommit: (validate: ValidateReferences) => Promise<SelectionCommit<T>>): Promise<T>;
}

/** Copies an explicit reference plan without filling missing fields, inferring participants or changing the active actor. */
export function copyReferences(references: SelectionReferences): SelectionReferences {
  parseChatSelection(references.selection);
  if (references.themeConfigId !== null) requireId(references.themeConfigId, "主题配置标识");
  if (references.ttsConfigId !== null) requireId(references.ttsConfigId, "TTS 配置标识");
  return Object.freeze({ selection: references.selection, themeConfigId: references.themeConfigId, ttsConfigId: references.ttsConfigId });
}

/** Reads the chosen record's own references; a role group never applies any participant's theme or speech config. */
export async function readSelectionReferences(repository: Pick<CharacterRepository, "getCharacter" | "getGroup">, selection: string): Promise<SelectionReferences> {
  const parsed = parseChatSelection(selection);
  switch (parsed.kind) {
    case "card": {
      const card = await repository.getCharacter(parsed.id); assertCard(card);
      if (card.id !== parsed.id) throw new Error("角色卡读取结果与选择标识不一致：" + selection);
      return copyReferences({ selection, themeConfigId: card.themeConfigId, ttsConfigId: card.ttsConfigId });
    }
    case "group": {
      const group = await repository.getGroup(parsed.id); assertGroup(group);
      if (group.id !== parsed.id) throw new Error("群组读取结果与选择标识不一致：" + selection);
      return copyReferences({ selection, themeConfigId: group.themeConfigId, ttsConfigId: null });
    }
  }
}

/** Formats only confirmed writes, leaving the rejecting independent write explicitly unverified. */
function describeConfirmed(steps: readonly ApplicationStep[]): string {
  return steps.map(
    /** Distinguishes the selection owner and each independent configuration owner's actual acknowledgement. */
    step => {
      switch (step.type) {
        case "selection": return "选择已提交（" + step.selection + "）";
        case "theme": return "主题已确认应用（" + step.id + "）";
        case "tts": return "TTS 已确认应用（" + step.id + "）";
      }
    },
  ).join("；");
}

/** Preserves an original application error and genuine partial progress without rollback, retry or atomicity claims. */
export class SelectionApplicationFailure extends Error {
  readonly cause: unknown;
  readonly completed: readonly ApplicationStep[];
  readonly failedStep: FailedConfigurationStep;

  /** Exposes confirmed progress and the unverified failed write on the same visible selector error. */
  constructor(completed: readonly ApplicationStep[], failedStep: FailedConfigurationStep, cause: unknown) {
    const label = failedStep.type === "theme" ? "主题" : "TTS";
    super(describeConfirmed(completed) + "；" + label + "应用失败（" + failedStep.id + "），该配置的实际存储状态待核对。未自动撤销或重试。原始错误：" + String(cause));
    this.name = "SelectionApplicationFailure";
    this.cause = cause;
    this.completed = Object.freeze(completed.map(
      /** Copies acknowledgement metadata so a caught error cannot rewrite the recorded application history. */
      step => Object.freeze({ ...step }),
    ));
    this.failedStep = Object.freeze({ ...failedStep });
  }
}

/** Applies each explicitly referenced configuration once, propagating an owner rejection with confirmed partial progress. */
async function applyConfigurations(references: SelectionReferences, access: IndependentConfigurationAccess): Promise<void> {
  const completed: ApplicationStep[] = [{ type: "selection", selection: references.selection }];
  if (references.themeConfigId !== null) {
    const id = references.themeConfigId;
    try { await access.applyTheme(id); }
    catch (failure) { throw new SelectionApplicationFailure(completed, { type: "theme", id }, failure); }
    completed.push({ type: "theme", id });
  }
  if (references.ttsConfigId !== null) {
    const id = references.ttsConfigId;
    try { await access.applyTts(id); }
    catch (failure) { throw new SelectionApplicationFailure(completed, { type: "tts", id }, failure); }
    completed.push({ type: "tts", id });
  }
}

/** Coordinates real owner acknowledgements, not a transaction between selection files and independent preferences. */
export function createSelectionApplication(access: IndependentConfigurationAccess): SelectionApplication {
  let busy = false;
  const waiters: (() => void)[] = [];
  /** Reserves the next user operation without recovering, retrying or discarding any previous operation's error. */
  async function acquire(): Promise<void> {
    if (!busy) { busy = true; return; }
    await new Promise<void>(
      /** Retains only the continuation for a genuinely distinct waiting operation. */
      resolve => waiters.push(resolve),
    );
  }
  /** Releases ownership after success or rejection while preserving each operation's own result. */
  function release(): void {
    const next = waiters.shift();
    if (next === undefined) busy = false;
    else next();
  }
  return {
    /** Performs all explicit-reference validation before committing the selection, then awaits every requested apply. */
    async commit<T>(prepareAndCommit: (validate: ValidateReferences) => Promise<SelectionCommit<T>>): Promise<T> {
      await acquire();
      const validated = new WeakSet<SelectionReferences>();
      let validations = 0;
      try {
        const committed = await prepareAndCommit(
          /** Issues one immutable validation receipt inside the selection owner's real record snapshot. */
          async references => {
            validations += 1;
            if (validations !== 1) throw new Error("A selection operation must validate exactly one reference plan");
            const checked = copyReferences(references);
            if (checked.themeConfigId !== null) await access.validateTheme(checked.themeConfigId);
            if (checked.ttsConfigId !== null) await access.validateTts(checked.ttsConfigId);
            validated.add(checked);
            return checked;
          },
        );
        if (!validated.has(committed.references)) throw new Error("Selection commit did not retain its exact validated reference plan");
        await applyConfigurations(committed.references, access);
        return committed.value;
      } finally { release(); }
    },
  };
}

/** Enumerates the real service entrypoints that change an explicitly selected actor. */
export type SelectionOperation = Extract<DomainOperation, "character.setActive" | "group.setActive" | "activePrompt.setCard" | "activePrompt.setGroup" | "activePrompt.activateForChat" | "chat.configuration.binding.write">;

/** Narrows only actual selection mutations; initialization and configuration reads never apply global preferences. */
export function isSelectionOperation(operation: DomainOperation): operation is SelectionOperation {
  switch (operation) {
    case "character.setActive": case "group.setActive": case "activePrompt.setCard": case "activePrompt.setGroup": case "activePrompt.activateForChat": case "chat.configuration.binding.write": return true;
    default: return false;
  }
}

/** Keeps the finite editor request path on the same central selection logic as public APIs and sidebar IPC. */
export function isSelectionRequest(request: Request): request is Extract<Request, { action: "activate" | "writeChatBinding" }> {
  return request.action === "activate" || request.action === "writeChatBinding";
}

/** Resolves only the caller's explicit editor selection without reading global active state or applying a member's refs. */
export function editorSelection(request: Extract<Request, { action: "activate" | "writeChatBinding" }>): string {
  if (request.action === "writeChatBinding") return request.selection;
  switch (request.type) {
    case "card": return "card:" + request.id;
    case "group": return "group:" + request.id;
    default: throw new Error("Editor activation requires an explicit card or group type");
  }
}

/** Parses each existing request through its actual domain contract before resolving the one requested persisted record. */
export function domainSelection<K extends DomainOperation>(operation: K & SelectionOperation, input: DomainInput<K>, repository: Pick<CharacterRepository, "listCharacters">): Promise<string>;

/** Validates each explicit selection operation against its declared input fields before resolving its record. */
export async function domainSelection(operation: SelectionOperation, input: DomainInput<DomainOperation>, repository: Pick<CharacterRepository, "listCharacters">): Promise<string> {
  switch (operation) {
    case "character.setActive": case "activePrompt.setCard": return "card:" + parseDomainPayload("activePrompt.setCard", input).id;
    case "group.setActive": case "activePrompt.setGroup": return "group:" + parseDomainPayload("activePrompt.setGroup", input).id;
    case "chat.configuration.binding.write": return parseDomainPayload("chat.configuration.binding.write", input).selection;
    case "activePrompt.activateForChat": {
      const parsed = parseDomainPayload("activePrompt.activateForChat", input);
      if (parsed.characterGroupId !== null && parsed.characterCardName !== null) throw new Error("Chat activation cannot select both a card and a group");
      if (parsed.characterGroupId !== null) return "group:" + parsed.characterGroupId.trim();
      if (parsed.characterCardName === null) throw new Error("Chat activation requires an explicit character or group selection");
      const name = parsed.characterCardName.trim(), matches = (await repository.listCharacters()).filter(
        /** Matches the established exact character-name operation without inventing a replacement selection. */
        card => card.name === name,
      );
      if (matches.length !== 1) throw new Error("Chat character name must identify exactly one persisted record: " + name);
      return "card:" + matches[0].id;
    }
  }
}

/** Validates the plugin's one private UI projection without inventing independent SDK fields or substituting rows. */
export function assertThemeChoices(value: unknown): asserts value is readonly ThemeChoice[] {
  if (!Array.isArray(value)) throw new Error("主题选择目录必须是数组");
  const ids = new Set<string>();
  for (const choice of value) {
    assertObject(choice, "主题选择项");
    const id = requireId(choice.id, "主题配置 ID"); assertString(choice.label, "主题配置名称");
    if (choice.label.trim() === "") throw new Error("主题配置名称无效");
    if (ids.has(id)) throw new Error("主题配置 ID 重复：" + id);
    ids.add(id);
  }
}

/** Reads the actual connected directory on demand and validates every returned identity before opening a picker. */
export async function readThemeChoices(access: IndependentConfigurationAccess): Promise<ThemeChoice[]> {
  const choices = await access.readThemeChoices(); assertThemeChoices(choices);
  return choices.map(
    /** Keeps only private presentation fields, never an independent configuration's stored theme values. */
    choice => ({ id: choice.id, label: choice.label }),
  );
}
