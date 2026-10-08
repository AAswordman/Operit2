import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fixture } from "./fixtures.mjs";
import { loadModule, plain } from "./runtime.mjs";

const inputs = loadModule("tests/selection-settings-input.ts");

/** Controls only the formally declared independent Settings boundary; all role selection and persistence execute in real plugin code. */
export function createSelectionSettingsFixture() {
  const snapshot = JSON.parse(readFileSync(new URL("../../../../../core/crates/runtime/application/src/data/preferences/theme_snapshot_fixture.json", import.meta.url), "utf8"));
  const theme = plain(inputs.themeSettingsInput(snapshot)), priorTheme = { ...plain(theme), id: "independent-theme-one", name: "Controlled prior independent theme" };
  const originalTts = fixture().snapshot.ttsConfigs[0], secondTts = plain(inputs.ttsSettingsInput(originalTts));
  const themes = new Map([[priorTheme.id, priorTheme], [theme.id, theme]]), speech = new Map([[originalTts.id, originalTts], [secondTts.id, secondTts]]);
  const calls = [], current = { theme: priorTheme.id, tts: originalTts.id }, failures = new Map(), gates = new Map(), receipts = new Map();

  /** Records an exact formal SDK call without implementing any plugin CRUD, active state or chat binding. */
  function observe(method, args) { calls.push({ method, args: plain(args) }); }
  /** Retains explicitly planned SDK-boundary gates and post-write failures without retrying or manufacturing successful acknowledgements. */
  async function acknowledged(method, receipt) {
    if (gates.has(method)) { const gate = gates.get(method); gate.entered(); await gate.pending; }
    if (failures.has(method)) { const failure = failures.get(method); failures.delete(method); throw failure; }
    if (receipts.has(method)) return plain(receipts.get(method));
    return plain(receipt);
  }
  const settings = {
    /** Supplies complete controlled records in the actual SDK Theme shape, never an empty production source. */
    async listThemeConfigs(...args) { assert.equal(args.length, 0); observe("listThemeConfigs", args); return acknowledged("listThemeConfigs", [...themes.values()]); },
    /** Supplies complete controlled SDK speech records with both genuinely declared test IDs. */
    async listTtsConfigs(...args) { assert.equal(args.length, 0); observe("listTtsConfigs", args); return acknowledged("listTtsConfigs", [...speech.values()]); },
    /** Simulates only the independent owner write and exact full-record acknowledgement for a declared ID. */
    async applyThemeConfig(...args) {
      assert.equal(args.length, 1); observe("applyThemeConfig", args); const [id] = args;
      assert.equal(typeof id, "string"); if (!themes.has(id)) throw new Error("Controlled Theme owner rejected unknown ID: " + id);
      current.theme = id; return acknowledged("applyThemeConfig", themes.get(id));
    },
    /** Simulates only the independent speech preference write, including explicitly planned acknowledgement failures. */
    async setCurrentTtsConfigId(...args) {
      assert.equal(args.length, 1); observe("setCurrentTtsConfigId", args); const [id] = args;
      assert.equal(typeof id, "string"); if (!speech.has(id)) throw new Error("Controlled TTS owner rejected unknown ID: " + id);
      current.tts = id; return acknowledged("setCurrentTtsConfigId", id);
    },
    /** Returns the controlled independent owner's real current test state without querying plugin usage or roles. */
    async getCurrentTtsConfigId(...args) { assert.equal(args.length, 0); observe("getCurrentTtsConfigId", args); return acknowledged("getCurrentTtsConfigId", current.tts); },
  };
  return { settings, calls, current, themes, speech,
    /** Clears only the SDK-call audit; canonical test input identities and independent state are retained. */
    clearCalls() { calls.length = 0; },
    /** Injects an exact owner rejection after its controlled mutation to expose genuine partial-progress semantics. */
    failAfterWrite(method, failure) { assert.equal(failures.has(method), false); failures.set(method, failure); },
    /** Submits a deliberately contradictory acknowledgement to verify the production adapter validates exact IDs. */
    returnReceipt(method, receipt) { assert.equal(receipts.has(method), false); receipts.set(method, receipt); },
    /** Blocks one independent acknowledgement so tests can audit busy controls and the absence of early completion. */
    hold(method) {
      assert.equal(Object.hasOwn(settings, method), true); assert.equal(gates.has(method), false); let release, entered;
      const pending = new Promise(
        /** Records only a real independently controlled acknowledgement barrier. */
        resolve => { release = resolve; },
      );
      const reached = new Promise(
        /** Signals the actual production adapter's invocation at this controlled SDK boundary. */
        resolve => { entered = resolve; },
      );
      gates.set(method, { pending, entered });
      return { reached,
        /** Releases the original pending owner acknowledgement without resubmitting any application operation. */
        release() { assert.equal(gates.has(method), true); gates.delete(method); release(); },
      };
    },
  };
}
