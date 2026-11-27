export const DEFAULT_PROMPT = `你是一个运行在 Operit 的 AI agent，保持有帮助的语气。帮助用户理解、修改和验证代码，并完成他们交代的编程任务。

- 开始修改前，阅读相关代码和项目指令，了解现有实现、依赖与验证方式，沿用项目已有的结构和习惯。
- 围绕用户目标解决问题。修改范围应足以解决根因；避免顺带增加功能、整理无关代码，或为假想需求引入抽象。
- 保留用户已有的修改。遇到不熟悉的代码或文件，先查明用途再改动。
- 优先使用项目已有的运行和测试方式。根据具体缺失或错误补充必要的环境信息，避免无目的地尝试不同环境或安装额外工具。
- 根据修改的风险选择相关验证，优先覆盖需求和容易出错的关键行为。验证失败时，先区分实现问题、测试问题和环境问题，再针对原因处理。
- 完成所需检查后，只有新修改、新失败或尚未解决的疑点才需要扩大或重复验证。满足用户要求后结束任务。
- 简洁说明完成了什么、如何验证，以及仍未解决的问题；准确区分已验证的结果和推测。`;

export interface Binding { base: string; workspace: string | null; }
export interface MemoryJob { id: string; chatId: string; timestamp: number; variant: number; path: string; turns: { kind: string; content: string }[]; summary: string | null; error: string | null; }

/** Opens the single main-runtime configuration and durable background queue. */
export async function config() {
    return PluginConfig.use("settings", { prompt: DEFAULT_PROMPT, memoryEnabled: true,
        bindings: {} as Record<string, Binding>, jobs: [] as MemoryJob[] });
}

/** Joins a host-provided VFS base without interpreting it as a physical path. */
export function join(base: string, leaf: string): string {
    return base.replace(/\/$/, "") + "/" + leaf;
}

/** Returns the sole human-readable timeline owned by this project or output directory. */
export function memoryPath(binding: Binding): string {
    return join(binding.base, ".operit/coding-memory.md");
}
