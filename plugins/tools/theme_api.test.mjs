import { composeContextScript } from "../../tools/runtime/compose_scripts.mjs";
import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { runInNewContext } from "node:vm";

/** Runs the production SDK JavaScript without compiling the Rust wrapper. */
async function createRuntime(theme) {
  const runtime = runInNewContext(composeContextScript() + "; OperitComposeDslRuntime;", { console });
  return runtime.createContext({ theme });
}

test("Theme reads immutable UI colors and awaits change subscribers", async () => {
  const initial = { brightness: "light", colors: { primary: "#6750a4ff" } };
  const runtime = await createRuntime(initial);
  const theme = runtime.ctx.Theme;
  initial.colors.primary = "#000000ff";
  assert.equal(theme.getCurrent().colors.primary, "#6750a4ff");
  assert.ok(Object.isFrozen(theme.getCurrent().colors));
  const changes = [];
  const unsubscribe = theme.subscribe(async (snapshot) => {
    await Promise.resolve();
    changes.push(snapshot.colors.primary);
  });
  const changed = { brightness: "light", colors: { primary: "#008800ff" } };
  await runtime.invokeAction("__operit_theme_changed", changed);
  assert.deepEqual(changes, ["#008800ff"]);
  await runtime.invokeAction("__operit_theme_changed", changed);
  assert.equal(changes.length, 1);
  unsubscribe();
  await runtime.invokeAction("__operit_theme_changed", { ...changed, brightness: "dark" });
  assert.equal(changes.length, 1);
  assert.equal(theme.getCurrent().brightness, "dark");
});

test("Theme rejects missing or invalid host snapshots", async () => {
  const runtime = await createRuntime(undefined);
  assert.throws(() => runtime.ctx.Theme.getCurrent(), /Theme is unavailable/);
  await assert.rejects(
    runtime.invokeAction("__operit_theme_changed", { brightness: "light", colors: { primary: "blue" } }),
    /Invalid Theme color/,
  );
});
