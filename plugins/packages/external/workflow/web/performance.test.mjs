import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { build } from "esbuild";
import { chromium } from "playwright";
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const { newWorkflow, newNode } = require("../dist/model.js");

test("dragging measures page and card renders without host traffic", async () => {
  const graph = newWorkflow("Performance");
  graph.nodes = Array.from({ length: 30 }, (_, index) =>
    newNode("trigger", (index % 6) * 270, Math.floor(index / 6) * 150),
  );
  const result = await build({
    entryPoints: ["web/app.tsx"],
    bundle: true,
    minify: true,
    write: false,
    outfile: "app.js",
    jsx: "automatic",
    plugins: [
      {
        name: "render-counts",
        /** Instruments only this test bundle, leaving shipped code untouched. */
        setup(builder) {
          builder.onLoad({ filter: /app\.tsx$/ }, async ({ path }) => {
            const source = await readFile(path, "utf8");
            return {
              contents: source
                .replace(
                  "function App() {",
                  "function App() { window.renderCounts.app++;",
                )
                .replace(
                  "function WorkflowCard({ data, selected }: NodeProps<GraphNode>) {",
                  "function WorkflowCard({ data, selected }: NodeProps<GraphNode>) { window.renderCounts.card++;",
                ),
              loader: "tsx",
            };
          });
        },
      },
    ],
  });
  const browser = await chromium.launch({ channel: "msedge", headless: true });
  try {
    const page = await browser.newPage({
      viewport: { width: 1280, height: 800 },
    });
    await page.evaluate((workflow) => {
      window.renderCounts = { app: 0, card: 0, calls: 0 };
      window.WorkflowHost = {
        async request() {
          window.renderCounts.calls++;
          return { workflows: [workflow], runs: [] };
        },
      };
    }, graph);
    await page.setContent(
      '<div id="root"></div><style>html,body,#root{height:100%;margin:0}</style>',
    );
    await page.addStyleTag({
      content: result.outputFiles.find((file) => file.path.endsWith(".css"))
        .text,
    });
    await page.addScriptTag({
      content: result.outputFiles.find((file) => file.path.endsWith(".js"))
        .text,
    });
    await page.getByRole("button", { name: "打开工作流", exact: true }).click();
    const target = page.locator(".graph-node").first();
    await target.waitFor();
    await page.evaluate(
      () =>
        new Promise((resolve) =>
          requestAnimationFrame(() => requestAnimationFrame(resolve)),
        ),
    );
    const bounds = await target.boundingBox();
    await page.mouse.move(
      bounds.x + bounds.width / 2,
      bounds.y + bounds.height / 2,
    );
    await page.mouse.down();
    await page.evaluate(() => {
      window.renderCounts = { app: 0, card: 0, calls: 0 };
    });
    const session = await page.context().newCDPSession(page);
    await session.send("Performance.enable");
    const before = await session.send("Performance.getMetrics");
    await page.mouse.move(
      bounds.x + bounds.width / 2 + 150,
      bounds.y + bounds.height / 2 + 70,
      { steps: 45 },
    );
    const counts = await page.evaluate(() => window.renderCounts);
    const after = await session.send("Performance.getMetrics");
    const cpuMs =
      (after.metrics.find((metric) => metric.name === "TaskDuration").value -
        before.metrics.find((metric) => metric.name === "TaskDuration").value) *
      1000;
    console.log(JSON.stringify({ ...counts, dragTaskMs: Math.round(cpuMs) }));
    assert.equal(counts.calls, 0);
    assert.equal(
      counts.app,
      0,
      "Pointer movement must not render the application shell",
    );
    assert.ok(
      counts.card <= graph.nodes.length,
      "Unchanged cards must not rerender for every pointer step",
    );
    await page.mouse.up();
    await page.getByText("有未保存的修改", { exact: true }).waitFor();
  } finally {
    await browser.close();
  }
});
