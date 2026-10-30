/** Batch actual application states into one design handoff, independent of themes. */
import { readFile, writeFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createHash } from "node:crypto";
import { zipSync, strToU8, unzipSync, strFromU8 } from "fflate";
import { VmClient, vmWebSocketUrl } from "./capture.mjs";
import { sourceInventory, coverage } from "./inventory.mjs";

export function packSoftware(
  captures,
  { inventory = [], failures = [], scenarios = [] } = {},
) {
  const entries = {};
  const index = {
    format: "operit-software-design",
    version: 1,
    name: "Operit · 全软件 UI",
    captures: [],
    failures,
    scenarios,
    coverage: coverage(inventory, captures),
    warnings: [
      "observed 仅表示该组件出现在采集树中，不代表它的所有数据、滚动位置和交互状态都已覆盖。",
      "插件子路由、弹窗、工具展开状态需补充采集；没有采到的界面不会用主题示意图代替。",
    ],
  };
  for (const capture of captures) {
    if (capture.document?.format !== "operit-design")
      throw new Error(`Invalid capture: ${capture.id}`);
    const key = createHash("sha256")
      .update(capture.id)
      .digest("hex")
      .slice(0, 16);
    const file = `designs/${key}.operit-design.json`;
    const document = structuredClone(capture.document);
    document.name = `${capture.group} / ${capture.name ?? document.name}`;
    for (const scene of document.scenes)
      scene.source = {
        ...scene.source,
        scenario: capture.id,
        group: capture.group,
        scenarioSource: capture.source ?? {},
      };
    entries[file] = strToU8(JSON.stringify(document));
    index.captures.push({
      id: capture.id,
      name: capture.name ?? capture.document.name,
      group: capture.group,
      source: capture.source ?? {},
      file,
    });
  }
  entries["index.json"] = strToU8(JSON.stringify(index, null, 2));
  entries["README.txt"] = strToU8(
    "Operit 全软件 UI 设计包\n\n在 Operit Figma 设计桥接插件中直接导入这个 ZIP。designs/ 中的每个文件都是独立可编辑场景，含锁定的渲染参考图。index.json 记录页面、来源、失败项和覆盖清单。\n\n请先检查 coverage 中的 unrecorded 项：本包不会把未采集的界面宣称为已完成。设计修改可通过插件导出交接文件，供开发按来源实施。\n",
  );
  return zipSync(entries);
}
export function unpackSoftware(bytes) {
  const entries = unzipSync(bytes);
  const index = JSON.parse(
    strFromU8(entries["index.json"] ?? new Uint8Array()),
  );
  if (index.format !== "operit-software-design" || index.version !== 1)
    throw new Error("不是 Operit 全软件设计包");
  return index.captures.map((c) => {
    if (
      !/^designs\/[a-f0-9]+\.operit-design\.json$/.test(c.file) ||
      !entries[c.file]
    )
      throw new Error("设计包文件路径无效");
    return { ...c, document: JSON.parse(strFromU8(entries[c.file])) };
  });
}
export async function main(args = process.argv.slice(2)) {
  if (!args.length || args.includes("--help")) {
    console.log(
      "用法：node tools/figma/software.mjs --vm-service-file <文件> --out <software.zip> [--settle-ms 1200] [--only <id>]\n补录当前弹窗、工具结果或插件子页面：加 --record <状态ID> --group <分组> --name <名称> --resume <software.zip>\n--inventory-only 只导出源码界面清单，不表示已完成采集。",
    );
    return;
  }
  const values = {};
  for (let i = 0; i < args.length; i++) {
    if (args[i] === "--inventory-only") {
      values[args[i]] = true;
      continue;
    }
    if (
      ![
        "--vm-service-file",
        "--vm-url",
        "--out",
        "--settle-ms",
        "--only",
        "--record",
        "--resume",
        "--group",
        "--name",
        "--scope",
      ].includes(args[i]) ||
      !args[i + 1]
    )
      throw new Error(`未知或缺少参数：${args[i]}`);
    values[args[i]] = args[++i];
  }
  const inventory = await sourceInventory();
  if (values["--inventory-only"]) {
    await writeFile(
      resolve(values["--out"] ?? "operit-ui-inventory.json"),
      JSON.stringify(inventory, null, 2),
    );
    console.log(
      `已列出 ${inventory.length} 个界面 / 渲染器 / 插件资源候选；尚未采集。`,
    );
    return;
  }
  let endpoint = values["--vm-url"];
  if (values["--vm-service-file"]) {
    const text = (await readFile(values["--vm-service-file"], "utf8")).trim();
    if (/^(https?|wss?):\/\//.test(text)) endpoint = text;
    else {
      const info = JSON.parse(text);
      endpoint = info.uri ?? info.vmServiceUri;
    }
  }
  if (!endpoint) throw new Error("请指定运行中应用的 VM Service 地址");
  const captures = values["--resume"]
    ? unpackSoftware(await readFile(values["--resume"]))
    : [];
  const client = new VmClient(vmWebSocketUrl(endpoint));
  const failures = [];
  let scenarios = [];
  let isolate;
  const call = (params) =>
    client.request("ext.operit.design.capture", {
      isolateId: isolate,
      ...(values["--scope"] ? { scope: values["--scope"] } : {}),
      ...params,
    });
  try {
    await client.connect();
    const vm = await client.request("getVM");
    for (const candidate of vm.isolates ?? []) {
      const info = await client.request("getIsolate", {
        isolateId: candidate.id,
      });
      if (info.extensionRPCs?.includes("ext.operit.design.capture")) {
        isolate = candidate.id;
        break;
      }
    }
    if (!isolate)
      throw new Error("未找到设计采集扩展，请启用 OPERIT_DESIGN_BRIDGE");
    if (values["--record"])
      scenarios = [
        {
          id: values["--record"],
          name: values["--name"] ?? values["--record"],
          group: values["--group"] ?? "supplemental",
        },
      ];
    else {
      scenarios = (await call({ catalog: "true" })).scenarios;
      if (!Array.isArray(scenarios) || !scenarios.length)
        throw new Error("应用尚未挂载软件界面目录");
      if (values["--only"])
        scenarios = scenarios.filter((s) => s.id === values["--only"]);
      if (!scenarios.length) throw new Error("指定的界面不在目录中");
    }
    for (const scenario of scenarios) {
      let restoreFailed = false;
      try {
        console.log(`采集 ${scenario.id} / ${scenario.name}`);
        const result = await call({
          ...(!values["--record"] ? { scenario: scenario.id } : {}),
          name: scenario.name,
          settleMs: values["--settle-ms"] ?? "1200",
        });
        if (result.document?.format !== "operit-design")
          throw new Error("没有收到界面图层");
        const index = captures.findIndex((c) => c.id === scenario.id);
        const capture = { ...scenario, document: result.document };
        if (index < 0) captures.push(capture);
        else captures[index] = capture;
      } catch (error) {
        failures.push({ id: scenario.id, message: error.message });
        console.error(`${scenario.id}: ${error.message}`);
      } finally {
        if (!values["--record"]) {
          try {
            await call({ restore: "true" });
          } catch (error) {
            failures.push({
              id: scenario.id,
              message: `恢复导航失败：${error.message}`,
            });
            restoreFailed = true;
          }
        }
      }
      if (restoreFailed) break;
    }
  } finally {
    client.close();
  }
  if (!captures.length) throw new Error("没有成功采集的界面，未生成空设计包");
  const out = resolve(values["--out"] ?? "operit-software-ui.zip");
  await writeFile(
    out,
    packSoftware(captures, { inventory, failures, scenarios }),
  );
  console.log(
    `已打包 ${captures.length} 个实际界面，${failures.length} 项失败：${out}`,
  );
  console.log(
    "在 Figma 插件中直接导入 ZIP；index.json 包含覆盖清单和待补采集项。",
  );
  if (failures.length) process.exitCode = 1;
}
if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
)
  main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
