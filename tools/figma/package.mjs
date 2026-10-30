/** Build a self-contained local development plugin for designers. */
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { zipSync, strToU8 } from "fflate";
import fixture from "../../apps/flutter/packages/operit_ui/assets/fixtures/theme-v1.json" with { type: "json" };
import { themeDocument } from "./src/theme_scene.ts";
import "./build.mjs";

const quickstart = `# Operit · Figma 静态图层插件

这个包只提供静态图层的导入和修改导出，尚未实现完整应用在 Figma 中运行和编辑。导入多个页面不会恢复导航、输入、工具执行和插件内部交互。详见随包的 INTERACTIVE_APP.md。

## 使用现有图层插件

1. 保留解压后的整个目录，打开 Figma Design 桌面端的设计文件。
2. 选择 Plugins → Development → Import plugin from manifest，打开本目录的 manifest.json。
3. 运行 Operit · UI Design Bridge，点击「导入静态图层包 / 场景」。
4. 选择已有的 .operit-design.json 或静态场景 ZIP；本包不包含完整软件界面。
5. 修改导入的原生图层，选中画板或其内部图层，点击「导出当前画板」或「导出整包修改」。

examples/theme-starter.operit-design.json 仅用于验证插件导入，不是整个应用，也不是本次完整应用设计需求的交付物。

## 修改的实际效果

文字、自由布局及插件 UI 的修改会导出来源与差异，仍需开发实施。图层导出不会自动改变运行中应用的结构或行为。

运行时静态稿可与锁定参考图对照检查位置、裁剪与字体。字体不足时插件会提示替代字体。

这是本地开发插件包，没有发布到 Figma 社区。源码位于项目 tools/figma。
`;

const files = new Map([
  [
    "manifest.json",
    new Uint8Array(await readFile(new URL("./manifest.json", import.meta.url))),
  ],
  [
    "dist/plugin.js",
    new Uint8Array(
      await readFile(new URL("./dist/plugin.js", import.meta.url)),
    ),
  ],
  [
    "dist/ui.html",
    new Uint8Array(await readFile(new URL("./dist/ui.html", import.meta.url))),
  ],
  [
    "examples/theme-starter.operit-design.json",
    strToU8(
      JSON.stringify(
        themeDocument(fixture, [], "Operit 主题组件示例"),
        null,
        2,
      ),
    ),
  ],
  ["README.md", strToU8(quickstart)],
  [
    "INTERACTIVE_APP.md",
    new Uint8Array(
      await readFile(new URL("./INTERACTIVE_APP.md", import.meta.url)),
    ),
  ],
  [
    "LICENSE",
    new Uint8Array(await readFile(new URL("../../LICENSE", import.meta.url))),
  ],
]);
const root = new URL("./dist/designer/", import.meta.url);
const archive = {};
for (const [path, bytes] of files) {
  const target = new URL(path, root);
  await mkdir(new URL(".", target), { recursive: true });
  await writeFile(target, bytes);
  archive[`operit-figma/${path}`] = bytes;
}
await writeFile(
  new URL("./dist/operit-figma-designer.zip", import.meta.url),
  zipSync(archive),
);
console.log("静态图层插件包：tools/figma/dist/operit-figma-designer.zip");
console.log(
  "解压后导入 operit-figma/manifest.json；此包尚未提供完整应用交互。",
);
