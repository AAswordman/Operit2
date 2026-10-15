import test from "node:test";
import assert from "node:assert/strict";
import { readFile, mkdir } from "node:fs/promises";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { chromium } from "playwright";
import { hostTheme } from "./host-theme.mjs";
import { fileURLToPath } from "node:url";

const screenshotDirectory = new URL("../test-results/", import.meta.url);

/** Checks every exit-animation frame for premature dialog content removal. */
async function verifyDialogExit(page, buttonText) {
  const result = await page.getByRole("dialog").evaluate(async (dialog, label) => {
    const title = dialog.querySelector(".MuiDialogTitle-root").textContent;
    const content = dialog.querySelector(".MuiDialogContent-root");
    const text = content.textContent;
    const values = [...content.querySelectorAll("input, textarea")].map((field) => field.value);
    const button = [...dialog.querySelectorAll("button")].find(
      (item) => item.textContent === label,
    );
    button.click();
    let frames = 0;
    while (dialog.isConnected && frames < 120) {
      await new Promise(requestAnimationFrame);
      if (!dialog.isConnected) break;
      frames++;
      if (
        dialog.querySelector(".MuiDialogTitle-root").textContent !== title ||
        content.textContent !== text ||
        JSON.stringify([...content.querySelectorAll("input, textarea")].map((field) => field.value)) !== JSON.stringify(values)
      ) return { preserved: false, frames };
    }
    return { preserved: !dialog.isConnected, frames };
  }, buttonText);
  assert.equal(result.preserved, true, "Dialog content must survive its exit animation");
  assert.ok(result.frames > 0, "The test must observe the exit animation");
}

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
  globalThis.getToolCatalog = () => ({
    tools: [{
      name: "demo:send",
      description: "Sends a structured value.",
      parameters: [
        { name: "title", type: "string", description: "Title", required: true, default: null },
        { name: "enabled", type: "boolean", description: "Enabled", required: false, default: "true" },
      ],
      category: "Demo",
      source: "package",
      packageName: "demo",
    }],
  });
    const require = createRequire(import.meta.url);
    const { dispatch } = require("../../dist/service.js");
    const html = await readFile(
      new URL("../../resources/workflow.html", import.meta.url),
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
    let browser;
    try {
      browser = await chromium.launch({
        ...(process.env.OPERIT_TEST_BROWSER_CHANNEL
          ? { channel: process.env.OPERIT_TEST_BROWSER_CHANNEL } : {}),
        headless: true,
      });
      const page = await browser.newPage({
        viewport: { width: 1280, height: 800 },
      });
      const errors = [];
      page.on("pageerror", (error) => errors.push(String(error)));
      await page.addInitScript(() => {
        window.WorkflowHost = {
          async currentTheme() {
            return window.workflowTheme;
          },
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
      await page.addInitScript((theme) => { window.workflowTheme = theme; }, hostTheme);
      await page.goto(`http://127.0.0.1:${server.address().port}`);
      await page.getByRole("button", { name: "模板", exact: true }).click();
      assert.equal(await page.locator(".template-option").count(), 6);
      await page
        .getByText("AI 主动发消息（定时）", { exact: true })
        .waitFor();
      await page.getByRole("button", { name: "关闭", exact: true }).click();
      await page
        .getByRole("button", { name: "新建工作流", exact: true })
        .click();
      await page.getByLabel("名称", { exact: true }).fill("浏览器验证");
      const documentId = await page.evaluate(() => {
        window.themeTestDocument = "same-document";
        const previous = window.workflowTheme;
        window.applyWorkflowTheme({
          ...previous,
          colors: { ...previous.colors, primary: "#008800", surface: "#fff8e1" },
        });
        return window.themeTestDocument;
      });
      await page.waitForFunction(() => getComputedStyle(document.documentElement)
        .getPropertyValue("--operit-primary").trim() === "#008800");
      assert.equal(await page.getByLabel("名称", { exact: true }).inputValue(), "浏览器验证");
      assert.equal(await page.evaluate(() => window.themeTestDocument), documentId);
      await verifyDialogExit(page, "关闭");
      await page.getByRole("button", { name: "新建工作流", exact: true }).click();
      await page.getByLabel("名称", { exact: true }).fill("浏览器验证");
      await page.getByRole("button", { name: "确定", exact: true }).click();
      await page.getByRole("button", { name: "添加节点", exact: true }).click();
      await page.getByRole("button", { name: "触发", exact: true }).click();
      await page.getByRole("button", { name: "应用", exact: true }).click();
      await page.getByRole("button", { name: "保存", exact: true }).click();
      await page.getByText("已保存", { exact: true }).waitFor();
      const node = page.locator(".graph-node");
      const bounds = await node.boundingBox();
      assert.ok(bounds.width <= 185, "Canvas nodes must remain compact");
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
      const desktopWorkflowCard = await page
        .locator(".workflow-card")
        .first()
        .boundingBox();
      assert.ok(
        desktopWorkflowCard.height < 170,
        "Desktop workflow cards must remain compact",
      );
      await page
        .getByRole("button", { name: "打开工作流", exact: true })
        .click();
      await page.getByRole("button", { name: "运行", exact: true }).click();
      await page.getByRole("button", { name: "返回列表", exact: true }).waitFor();
      assert.equal(
        await page.getByRole("button", { name: "返回列表", exact: true }).isDisabled(),
        false,
      );
      const runningNode = page.locator(".react-flow__node").first();
      const runningNodeId = await runningNode.getAttribute("data-id");
      const workflowId = (await dispatch({ action: "list" })).workflows[0].id;
      await page.waitForFunction(
        () => typeof window.receiveWorkflowProgress === "function",
      );
      await page.evaluate(({ id, nodeId }) => {
        window.receiveWorkflowProgress({
          id: "browser-progress",
          workflowId: id,
          workflowName: "浏览器验证",
          triggerId: null,
          status: "RUNNING",
          startedAt: Date.now(),
          nodes: {
            [nodeId]: {
              status: "running",
              output: "",
              startedAt: Date.now(),
            },
          },
          logs: [],
        });
      }, { id: workflowId, nodeId: runningNodeId });
      await page.locator(".graph-node.is-running").waitFor();
      await page.getByRole("button", { name: "返回列表", exact: true }).click();
      await page.getByText("最近 运行中", { exact: true }).waitFor();
      await page
        .getByRole("button", { name: "打开工作流", exact: true })
        .click();
      await page.locator(".graph-node.is-running").waitFor();
      await page.evaluate(({ id, nodeId }) => {
        window.receiveWorkflowProgress({
          id: "browser-progress",
          workflowId: id,
          workflowName: "浏览器验证",
          triggerId: null,
          status: "SUCCESS",
          startedAt: Date.now(),
          finishedAt: Date.now(),
          nodes: {
            [nodeId]: {
              status: "success",
              output: "",
              startedAt: Date.now(),
              finishedAt: Date.now(),
            },
          },
          logs: [],
        });
      }, { id: workflowId, nodeId: runningNodeId });
      await page.locator(".graph-node.is-success").waitFor();
      await page.getByRole("button", { name: "添加节点", exact: true }).click();
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
      await page.locator(".graph-node").first().click();
      await page.getByRole("button", { name: "编辑选中节点", exact: true }).click();
      await page.getByRole("button", { name: "取消", exact: true }).click();
      await page.getByRole("button", { name: "添加节点", exact: true }).click();
      await page.getByRole("button", { name: "执行", exact: true }).click();
      await page.getByRole("button", { name: "应用", exact: true }).click();
      await page.locator(".graph-node").last().click();
      await page.getByRole("button", { name: "编辑选中节点", exact: true }).click();
      await page.getByLabel("执行工具", { exact: true }).click();
      await page.getByRole("option", { name: "demo:send", exact: true }).click();
      await page.getByLabel("title", { exact: true }).fill("来自工作流");
      await page.getByRole("combobox", { name: "值来源" }).first().click();
      await page.getByRole("option", { name: "节点输出", exact: true }).click();
      assert.equal(await page.getByLabel("来源节点", { exact: true }).count(), 1);
      assert.equal(await page.getByText("enabled · 可选", { exact: true }).count(), 1);
      await page.getByRole("button", { name: "应用", exact: true }).click();
      await page.getByRole("button", { name: "保存", exact: true }).click();
      await page.getByText("已保存", { exact: true }).waitFor();
      await mkdir(screenshotDirectory, { recursive: true });
      await page.screenshot({ path: fileURLToPath(new URL("desktop.png", screenshotDirectory)) });
      await page.setViewportSize({ width: 390, height: 844 });
      await page.getByRole("button", { name: "返回列表", exact: true }).click();
      const workflowCard = await page
        .locator(".workflow-card")
        .first()
        .boundingBox();
      assert.ok(
        workflowCard.height < 260,
        "Phone workflow cards must remain compact",
      );
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
      await page.screenshot({ path: fileURLToPath(new URL("phone-node-picker.png", screenshotDirectory)) });
      await page.getByRole("button", { name: "关闭", exact: true }).click();
      await page.locator(".MuiDrawer-paper").waitFor({ state: "hidden" });
      await page.screenshot({ path: fileURLToPath(new URL("phone.png", screenshotDirectory)) });
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
      await page.screenshot({ path: fileURLToPath(new URL("phone-wide.png", screenshotDirectory)) });
      await page.setViewportSize({ width: 390, height: 844 });
      await page.locator(".graph-node").first().click();
      await page.getByRole("button", { name: "编辑节点", exact: true }).click();
      await page.getByLabel("节点名称", { exact: true }).fill("触屏编辑");
      await verifyDialogExit(page, "取消");
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
      const emptyWorkflowId = (await dispatch({ action: "list" })).workflows
        .find((item) => item.name === "空工作流").id;
      await page.evaluate((id) => {
        window.receiveWorkflowProgress({
          id: "browser-failed-progress",
          workflowId: id,
          workflowName: "空工作流",
          triggerId: null,
          status: "FAILED",
          startedAt: Date.now(),
          finishedAt: Date.now(),
          nodes: {},
          logs: [{
            time: Date.now(),
            nodeId: "",
            level: "error",
            message: "工作流至少需要一个触发节点",
          }],
        });
      }, emptyWorkflowId);
      await page.getByRole("button", { name: "设置", exact: true }).click();
      await page
        .getByRole("button", { name: "删除工作流", exact: true })
        .click();
      await page.getByRole("button", { name: "确定", exact: true }).click();
      await page.getByText("我的工作流", { exact: true }).waitFor();
      assert.equal((await dispatch({ action: "list" })).workflows.length, 1);
      assert.deepEqual(errors, []);
    } finally {
      await browser?.close();
      await new Promise((resolve, reject) => server.close((error) =>
        error ? reject(error) : resolve()));
    }
  },
);
