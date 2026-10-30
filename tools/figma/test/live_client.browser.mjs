import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { createLiveServer } from "../live.mjs";
const require = createRequire(import.meta.url);
const { chromium } = require(
  process.env.OPERIT_PLAYWRIGHT_MODULE ?? "playwright",
);
let revision = 0,
  count = 0,
  input = "",
  label = "Business label",
  designLabel;
let width = 220;
const requests = [];
const node = (id, type, text, x, y, w = 220, h = 36, extra = {}) => ({
  id,
  name: id,
  type,
  x,
  y,
  width: w,
  height: h,
  ...(text !== undefined ? { text } : {}),
  source: {
    runtimeTarget: id,
    runtimeSelector: id,
    runtimeProperties:
      type === "text" ? ["text", "fontSize", "fill"] : ["width", "height"],
    runtimePersistence: "source",
  },
  ...extra,
});
function frame() {
  return {
    protocol: "operit-design-runtime",
    version: 1,
    revision: ++revision,
    document: {
      format: "operit-design",
      version: 1,
      name: "Browser fixture",
      origin: "flutter-runtime",
      assets: [],
      warnings: [],
      scenes: [
        {
          id: "root",
          name: "root",
          type: "frame",
          x: 0,
          y: 0,
          width: 480,
          height: 420,
          fill: "#ffffffff",
          children: [
            node("count", "text", `Count ${count}`, 20, 20),
            node("button", "text", "Increment", 20, 70),
            node("input", "text", input, 20, 120, 220, 36, {
              source: {
                runtimeTarget: "input",
                runtimeProperties: [],
                runtimeInput: true,
                runtimePersistence: "source",
              },
            }),
            node("label", "text", designLabel ?? label, 20, 180, width),
            node("box", "frame", undefined, 20, 240, 220, 40, {
              fill: "#eeeeeeff",
              source: {
                runtimeTarget: "box",
                runtimeProperties: [
                  "width",
                  "height",
                  "paddingLeft",
                  "paddingRight",
                  "paddingTop",
                  "paddingBottom",
                ],
                runtimePersistence: "source",
              },
              paddingLeft: 12,
              paddingRight: 12,
              paddingTop: 8,
              paddingBottom: 8,
            }),
          ],
        },
      ],
    },
    edits: designLabel
      ? [{ selector: "label", properties: { text: designLabel } }]
      : [],
  };
}
const live = await createLiveServer({
  runtime: {
    async request(request) {
      requests.push(request);
      if (request.operation === "event") {
        if (request.event.type === "tap" && request.event.target === "button") {
          count++;
          label = "Business " + count;
        }
        if (request.event.type === "input") {
          await new Promise((resolve) => setTimeout(resolve, 30));
          input = request.event.text.slice(0, 5);
        }
      }
      if (request.operation === "edit") {
        if (request.properties.text !== undefined)
          designLabel = request.properties.text;
        if (request.properties.width !== undefined)
          width = request.properties.width;
      }
      if (request.operation === "clear") designLabel = undefined;
      return frame();
    },
    close() {},
  },
});
const browser = await chromium.launch({
  headless: true,
  ...(process.env.OPERIT_BROWSER_EXECUTABLE
    ? { executablePath: process.env.OPERIT_BROWSER_EXECUTABLE }
    : {}),
});
try {
  const page = await browser.newPage({
    viewport: { width: 1100, height: 800 },
  });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(live.url);
  await page.getByText("Count 0", { exact: true }).waitFor();
  assert.equal(
    new URL(page.url()).hash,
    "",
    "token removed from browser address",
  );
  await page.getByText("Increment", { exact: true }).click();
  await page.getByText("Count 1", { exact: true }).waitFor();
  const field = page.locator('[data-runtime-key="input"]');
  await field.focus();
  await field.fill("abcdef");
  await page.waitForFunction(
    () =>
      document.querySelector('[data-runtime-key="input"]').value === "abcde",
  );
  assert.equal(
    await field.evaluate((el) => el === document.activeElement),
    true,
    "focused input survives update",
  );
  await page.getByRole("button", { name: "选中编辑", exact: true }).click();
  await page.getByText("Business 1", { exact: true }).click();
  await page.getByLabel("文字", { exact: true }).fill("Designer label");
  await page.getByLabel("文字", { exact: true }).press("Tab");
  await page.getByText("Designer label", { exact: true }).waitFor();
  await page.getByRole("button", { name: "继续操作", exact: true }).click();
  await page.getByText("Increment", { exact: true }).click();
  await page.getByText("Count 2", { exact: true }).waitFor();
  assert.equal(
    await page.getByText("Designer label", { exact: true }).count(),
    1,
  );
  await page.getByRole("button", { name: "选中编辑", exact: true }).click();
  await page.getByText("Designer label", { exact: true }).click();
  await page
    .getByRole("button", { name: "撤销该控件修改", exact: true })
    .click();
  await page.getByText("Business 2", { exact: true }).waitFor();
  const before = requests.filter((r) => r.operation === "edit").length;
  await page
    .locator('[data-runtime-key="box"]')
    .evaluate((el) => (el.style.width = "245px"));
  await page.waitForFunction(
    () =>
      document.querySelector('[data-runtime-key="label"]').style.width ===
      "245px",
  );
  const edits = requests.filter((r) => r.operation === "edit").slice(before);
  assert.deepEqual(
    edits[0].properties,
    { width: 245 },
    "direct DOM width edit does not overwrite unrelated padding",
  );
  assert.deepEqual(errors, []);
  console.log(
    "PASS browser fixture: HTTP event forwarding, focus/formatter acknowledgement, editing across state updates, undo and direct DOM edits",
  );
} finally {
  await browser.close();
  await live.close();
}
