import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";

/** Presents one editable prompt and explicit controls for the append-only background timeline. */
export default function Settings(ctx: ComposeDslContext): ComposeNode {
    const { UI, Modifier: M } = ctx;
    const [prompt, setPrompt] = ctx.useState("prompt", "");
    const [memory, setMemory] = ctx.useState("memory", true);
    const [ready, setReady] = ctx.useState("ready", false);
    const [status, setStatus] = ctx.useState("status", "正在加载…");
    /** Dispatches settings to the plugin's single main-runtime owner. */
    async function request(action: "read" | "save" | "retry"): Promise<void> {
        try {
            const result = await ToolPkg.ipc.call<{ action: string; prompt: string; memoryEnabled: boolean }, { prompt: string; memoryEnabled: boolean; pending: number; errors: string[] }>("coding.settings", { action, prompt, memoryEnabled: memory }, { targetRuntime: "main" });
            setPrompt(result.prompt); setMemory(result.memoryEnabled); setReady(true);
            setStatus((action === "save" ? "已保存。" : "") + "待总结：" + result.pending + (result.errors.length ? "\n" + result.errors.join("\n") : ""));
        } catch (error) { setStatus(error instanceof Error ? error.message : String(error)); }
    }
    ctx.useEffect("load", () => { void request("read"); return () => {}; }, []);
    return UI.Column({ modifier: M.fillMaxSize().padding(20), spacing: 12 }, [
        UI.Text({ text: "编程套装", style: "titleLarge" }),
        UI.Text({ text: "启用本插件期间，角色卡、角色记忆和依赖角色卡的插件暂停运行。停用后按原有启用设置恢复。", style: "bodyMedium" }),
        UI.Text({ text: "编程提示词（工作区 AGENTS.md 会另行自动加入）", style: "titleSmall" }),
        UI.TextField({ value: prompt, onValueChange: setPrompt, enabled: ready, minLines: 10, maxLines: 18 }),
        UI.Row({ verticalAlignment: "center" }, [UI.Switch({ checked: memory, onCheckedChange: setMemory, enabled: ready }), UI.Text({ text: "后台追加项目记忆" })]),
        UI.Text({ text: "记忆文件：项目或对话产出目录下的 .operit/coding-memory.md。按时间追加，不自动召回，AI 按需搜索。每分钟处理一项，使用 MEMORY 模型并记录用量。", style: "bodySmall" }),
        UI.Row({}, [UI.Button({ onClick: () => request("save"), enabled: ready }, UI.Text({ text: "保存" })), UI.Button({ onClick: () => request("retry"), enabled: ready }, UI.Text({ text: "重试失败总结" }))]),
        UI.Text({ text: status, style: "bodySmall" }),
    ]);
}
