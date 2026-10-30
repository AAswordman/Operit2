import { createElement } from "react";
import { createRoot } from "react-dom/client";
import { Renderer } from "@openuidev/react-lang";
import { ThemeProvider } from "@openuidev/react-ui";
import { openuiLibrary } from "@openuidev/react-ui/genui-lib";
import type { ActionEvent, McpClientLike, OpenUIError } from "@openuidev/react-lang";
import type { ComposeThemeSnapshot } from "../../../../types/compose-dsl";
import type { ProgramInput } from "../src/protocol";
import "@openuidev/react-ui/styles/index.css";
import "./surface.css";

interface Bootstrap {
  input: ProgramInput;
  formState: Record<string, unknown>;
  theme: ComposeThemeSnapshot;
}

interface Host {
  bootstrap(): Promise<Bootstrap>;
  state(value: Record<string, unknown>): Promise<void>;
  resize(height: number): Promise<void>;
  action(event: ActionEvent): Promise<unknown>;
  callTool(request: { name: string; arguments?: Record<string, unknown> }): Promise<unknown>;
}

declare global {
  interface Window {
    OpenUiHost: Host;
    OpenUiPlugin: { update(input: ProgramInput): void; theme(theme: ComposeThemeSnapshot): void };
  }
}

/** Requires the package document contract before creating the upstream renderer. */
function requiredElement(id: string): HTMLElement {
  const element = document.getElementById(id);
  if (element === null) throw new Error("Incomplete OpenUI document: " + id);
  return element;
}
const container = requiredElement("root");
const errorPanel = requiredElement("error");
const surface = requiredElement("surface");
const root = createRoot(container);
let current: ProgramInput = { program: "", isStreaming: true, revision: -1 };
let initialFormState: Record<string, unknown> = {};
let mode: "light" | "dark" = "light";
let initialized = false;
let measuredHeight = -1;

const diagnostics: { host: string | null; renderer: OpenUIError[] } = { host: null, renderer: [] };

/** Displays the current host and renderer diagnostics without treating an empty list as an error. */
function showDiagnostics(): void {
  errorPanel.hidden = diagnostics.host === null && diagnostics.renderer.length === 0;
  errorPanel.textContent = JSON.stringify(diagnostics, null, 2);
}

/** Reports an explicit capability or transport failure beside the original interface. */
function reportError(error: unknown): void {
  diagnostics.host = String(error);
  showDiagnostics();
}

/** Replaces the upstream diagnostic list, including its normal empty-list notification. */
function onRendererErrors(errors: OpenUIError[]): void {
  diagnostics.renderer = errors;
  showDiagnostics();
}

/** Delivers upstream actions unchanged to the package's host bridge. */
function onAction(event: ActionEvent): void {
  void window.OpenUiHost.action(event).catch(reportError);
}

/** Persists upstream form changes without recreating the React root. */
function onStateUpdate(state: Record<string, unknown>): void {
  void window.OpenUiHost.state(state).catch(reportError);
}

const toolProvider: McpClientLike = {
  /** Sends Query and Mutation through the public tool host and preserves structured results. */
  async callTool(request) {
    const value = await window.OpenUiHost.callTool(request);
    return { content: [], structuredContent: value };
  },
};

/** Renders the unmodified full upstream library inside its own official theme provider. */
function render(): void {
  root.render(createElement(ThemeProvider, { mode },
    createElement(Renderer, {
      response: current.program, library: openuiLibrary, isStreaming: current.isStreaming,
      initialState: initialFormState, onAction, onStateUpdate, toolProvider, onError: onRendererErrors,
      publishObservability: false,
    }),
  ));
}

/** Accepts only monotonic input revisions, including updates received during bootstrap. */
function update(input: ProgramInput): void {
  if (input.revision < current.revision) return;
  current = input;
  if (initialized) render();
}

/** Applies only a valid host brightness through the upstream theme API. */
function theme(snapshot: ComposeThemeSnapshot): void {
  if (snapshot.brightness !== "light" && snapshot.brightness !== "dark") throw new Error("Invalid OpenUI host brightness");
  mode = snapshot.brightness;
  if (initialized) render();
}
window.OpenUiPlugin = { update, theme };

/** Reports the document's natural content height without reading the viewport height. */
function measure(): void {
  const height = Math.max(1, Math.ceil(surface.getBoundingClientRect().height));
  if (height === measuredHeight) return;
  measuredHeight = height;
  void window.OpenUiHost.resize(height).catch(reportError);
}
const observer = new ResizeObserver(measure);
observer.observe(surface);

/** Hydrates state once, then keeps the same official renderer for all stream revisions. */
async function initialize(): Promise<void> {
  const bootstrap = await window.OpenUiHost.bootstrap();
  initialFormState = bootstrap.formState;
  theme(bootstrap.theme);
  update(bootstrap.input);
  initialized = true;
  render();
}
void initialize().catch(reportError);
