import test from "node:test";
import assert from "node:assert/strict";
import { readFile, mkdir } from "node:fs/promises";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { chromium } from "playwright";

test(
  "Material editor creates, saves, runs and reopens graphs at desktop and phone widths",
  { timeout: 60000 },
  async () => {
    globalThis.PluginConfig = {
      async use(_key, initial) {
        return initial;
      },
      async flush() {},
    };
    const require = createRequire(import.meta.url);
    const { dispatch } = require("../dist/service.js");
    const html = await readFile(
      new URL("../resources/workflow.html", import.meta.url),
      "utf8",
    );
    const server = createServer(async (request, response) => {
      if (request.url === "/service") {
        try {
          let body = "";
          for await (const chunk of request) body += chunk;
          const snapshot = await dispatch(JSON.parse(body));
          response.setHeader("Content-Type", "application/json");
          response.end(JSON.stringify(snapshot));
        } catch (failure) {
          response.statusCode = 400;
          response.end(String(failure));
        }
      } else {
        response.setHeader("Content-Type", "text/html");
        response.end(html);
      }
    });
    await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
    const browser = await chromium.launch({
      channel: "msedge",
      headless: true,
    });
    try {
      const page = await browser.newPage({
        viewport: { width: 1280, height: 800 },
      });
      const errors = [];
      page.on("pageerror", (error) => errors.push(String(error)));
      await page.addInitScript(() => {
        window.WorkflowHost = {
          async request(message) {
            const response = await fetch("/service", {
              method: "POST",
              body: JSON.stringify(message),
            });
            if (!response.ok) throw new Error(await response.text());
            return response.json();
          },
        };
      });
      await page.goto(`http://127.0.0.1:${server.address().port}`);
      await page
        .getByRole("button", { name: "新建工作流", exact: true })
        .click();
      await page.getByLabel("名称", { exact: true }).fill("浏览器验证");
      await page.getByRole("button", { name: "确定", exact: true }).click();
      await page.getByRole("button", { name: "触发", exact: true }).click();
      await page.getByRole("button", { name: "应用", exact: true }).click();
      await page.getByRole("button", { name: "保存", exact: true }).click();
      await page.getByText("已保存", { exact: true }).waitFor();
      const node = page.locator(".graph-node");
      const bounds = await node.boundingBox();
      assert.ok(bounds.width <= 230, "A single node must not be auto-enlarged");
      const callsBeforeDrag = (await dispatch({ action: "list" })).workflows[0]
        .revision;
      const positionBeforeDrag = (await dispatch({ action: "list" }))
        .workflows[0].nodes[0].position;
      await page.mouse.move(bounds.x + 90, bounds.y + 30);
      await page.mouse.down();
      await page.mouse.move(bounds.x + 180, bounds.y + 90, { steps: 8 });
      await page.mouse.up();
      assert.equal(
        (await dispatch({ action: "list" })).workflows[0].revision,
        callsBeforeDrag,
        "Dragging stays within the browser",
      );
      await page.getByRole("button", { name: "返回列表", exact: true }).click();
      await page
        .getByRole("button", { name: "保存并返回", exact: true })
        .click();
      assert.notDeepEqual(
        (await dispatch({ action: "list" })).workflows[0].nodes[0].position,
        positionBeforeDrag,
        "Canvas-local pointer state must be committed to the saved workflow",
      );
      await page
        .getByRole("button", { name: "打开工作流", exact: true })
        .click();
      await page.getByRole("button", { name: "运行", exact: true }).click();
      await page
        .getByRole("dialog")
        .getByText("成功", { exact: true })
        .waitFor();
      await page.getByRole("button", { name: "关闭", exact: true }).click();
      await page.getByRole("button", { name: "条件", exact: true }).click();
      await page.getByRole("button", { name: "应用", exact: true }).click();
      await page.locator(".react-flow__controls-fitview").click();
      await page
        .locator(".react-flow__handle.source")
        .first()
        .dragTo(page.locator(".react-flow__handle.target"));
      await page.locator(".react-flow__edge").waitFor();
      await page.getByRole("button", { name: "保存", exact: true }).click();
      await page.getByText("已保存", { exact: true }).waitFor();
      assert.equal(
        (await dispatch({ action: "list" })).workflows[0].connections.length,
        1,
      );
      await mkdir("web/test-results", { recursive: true });
      await page.screenshot({ path: "web/test-results/desktop.png" });
      await page.setViewportSize({ width: 390, height: 844 });
      await page.getByRole("button", { name: "返回列表", exact: true }).click();
      await page
        .getByRole("button", { name: "打开工作流", exact: true })
        .click();
      assert.equal(
        await page.evaluate(
          () => document.documentElement.scrollWidth > innerWidth,
        ),
        false,
      );
      assert.ok((await page.locator(".toolbar").boundingBox()).height <= 66);
      assert.equal(await page.locator(".palette").count(), 0);
      assert.equal(await page.locator(".react-flow__attribution").count(), 0);
      await page.getByRole("button", { name: "添加节点", exact: true }).click();
      await page
        .getByText("选择要放入画布的节点类型", { exact: true })
        .waitFor();
      await page.screenshot({ path: "web/test-results/phone-node-picker.png" });
      await page.getByRole("button", { name: "关闭", exact: true }).click();
      await page.locator(".MuiDrawer-paper").waitFor({ state: "hidden" });
      await page.screenshot({ path: "web/test-results/phone.png" });
      const controls = await page
        .locator(".react-flow__controls")
        .boundingBox();
      const addNode = await page
        .getByRole("button", { name: "添加节点", exact: true })
        .boundingBox();
      assert.ok(controls.x + controls.width < addNode.x);
      await page.setViewportSize({ width: 554, height: 1196 });
      assert.equal(
        await page.evaluate(
          () => document.documentElement.scrollWidth > innerWidth,
        ),
        false,
      );
      assert.ok((await page.locator(".toolbar").boundingBox()).height <= 66);
      await page.screenshot({ path: "web/test-results/phone-wide.png" });
      await page.setViewportSize({ width: 390, height: 844 });
      await page.locator(".graph-node").first().click();
      await page.getByRole("button", { name: "编辑节点", exact: true }).click();
      await page.getByLabel("节点名称", { exact: true }).fill("触屏编辑");
      await page.getByRole("button", { name: "应用", exact: true }).click();
      await page.getByRole("button", { name: "返回列表", exact: true }).click();
      await page
        .getByRole("button", { name: "保存并返回", exact: true })
        .click();
      await page.getByRole("button", { name: "新建", exact: true }).click();
      await page.getByLabel("名称", { exact: true }).fill("空工作流");
      await page.getByRole("button", { name: "确定", exact: true }).click();
      await page.getByRole("button", { name: "运行", exact: true }).click();
      await page
        .getByRole("dialog")
        .getByText("失败", { exact: true })
        .waitFor();
      await page.getByRole("button", { name: "关闭", exact: true }).click();
      await page.getByRole("button", { name: "设置", exact: true }).click();
      await page
        .getByRole("button", { name: "删除工作流", exact: true })
        .click();
      await page.getByRole("button", { name: "确定", exact: true }).click();
      await page.getByText("我的工作流", { exact: true }).waitFor();
      assert.equal((await dispatch({ action: "list" })).workflows.length, 1);
      assert.deepEqual(errors, []);
    } finally {
      await browser.close();
      server.close();
    }
  },
);
