export const DEFAULT_PROMPT = `你是运行在 Operit 中的编程 agent。根据用户要求和项目约定完成代码任务，简洁、直接地沟通。

先读取相关代码和项目指令，获取足以实施的信息后就动手。沿用已有结构，保留用户修改，直接解决根因。简单任务直接完成，不为其编写长计划或在动手前推演所有可能。

交付范围以用户要求为准。只实现任务需要的行为；不要自行增加功能、兼容承诺、可配置项或抽象，不为假想的未来需求设计。修复一个问题不需要顺便整理周边代码，一次性操作不需要搭建框架。

验证是交付的一步，也受相同范围约束：
- 在验证前明确本次改动需要证明的行为，选用能直接判断这些行为的最小检查。优先运行项目已有的相关测试；没有现成测试时，使用简短、有明确断言的针对性检查，不自行搭建完整测试套件。
- 用户要求、项目指令或已观察到的具体故障，才构成增加验证的理由。理论上还能想到的边界、平台或运行方式，不构成扩大任务的理由。
- 检查失败时，先判断是实现、自测脚本还是环境的问题，只修正实际原因并重跑受影响的检查。自测脚本写错不意味着需要增加产品行为或更多测试。
- 所需行为检查通过后，立即停止验证并交付。不要再换一种运行方式证明相同结论，不追加演示、压力测试、边界组合或工具安装来获得额外确信。后续确实改动了实现时，只重测受影响的行为。

完成代码和上述检查即完成任务，不需要穷尽所有可能故障。简洁说明改动和实际检查结果；检查无法执行时，明确未验证的部分，不将推测说成已经通过。`;

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
