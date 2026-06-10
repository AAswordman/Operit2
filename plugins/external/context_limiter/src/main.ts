import {
  DEFAULT_FLOOR_LIMIT,
  DEFAULT_LIMITER_ENABLED,
  ENV_KEYS,
  FLOOR_OPTIONS,
  HOOK_IDS,
  INPUT_MENU_TOGGLE_ACTION,
  PROMPT_TURN_KIND,
  TOGGLE_IDS,
} from "./constants";

function readEnv(key: (typeof ENV_KEYS)[keyof typeof ENV_KEYS]): string {
  if (typeof getEnv !== "function") {
    return "";
  }
  const value = getEnv(key);
  return value == null ? "" : String(value).trim();
}

function readFloorLimit(): number {
  const raw = readEnv(ENV_KEYS.floorLimit);
  const parsed = Number.parseInt(raw, 10);
  if (!Number.isFinite(parsed) || parsed < 1) {
    return DEFAULT_FLOOR_LIMIT;
  }
  return parsed;
}

function readLimiterEnabled(): boolean {
  const raw = readEnv(ENV_KEYS.enabled).toLowerCase();
  if (!raw) {
    return DEFAULT_LIMITER_ENABLED;
  }
  return raw === "1" || raw === "true" || raw === "yes" || raw === "on";
}

async function writeEnvValue(
  key: (typeof ENV_KEYS)[keyof typeof ENV_KEYS],
  value: string
): Promise<void> {
  await Tools.SoftwareSettings.writeEnvironmentVariable(key, value);
}

async function writeFloorLimit(nextLimit: number): Promise<void> {
  await writeEnvValue(ENV_KEYS.floorLimit, String(nextLimit));
}

async function writeLimiterEnabled(nextEnabled: boolean): Promise<void> {
  await writeEnvValue(ENV_KEYS.enabled, nextEnabled ? "true" : "false");
}

export function registerToolPkg() {
  ToolPkg.registerPromptFinalizeHook({
    id: HOOK_IDS.finalize,
    function: onFinalize,
  });

  ToolPkg.registerInputMenuTogglePlugin({
    id: HOOK_IDS.menu,
    function: onInputMenuToggle,
  });

  return true;
}

export async function onInputMenuToggle(
  event: ToolPkg.InputMenuToggleHookEvent
): Promise<
  ToolPkg.InputMenuToggleDefinitionResult[] |
  ToolPkg.InputMenuToggleObjectResult |
  null |
  void
> {
  const payload = event.eventPayload || {};
  const action = payload.action;
  const floorLimit = readFloorLimit();
  const limiterEnabled = readLimiterEnabled();

  if (action === INPUT_MENU_TOGGLE_ACTION.create) {
    return {
      toggles: [
        {
          id: TOGGLE_IDS.limiter,
          icon: "layers_outlined",
          title: "楼层限制器",
          description: limiterEnabled
            ? `已开启 · 保留最近 ${floorLimit} 层`
            : "已关闭",
          isChecked: limiterEnabled,
        },
        {
          id: TOGGLE_IDS.adjust,
          icon: "format_list_numbered",
          title: `调节楼层数 ▶ ${floorLimit}`,
          description: `点击切换: ${FLOOR_OPTIONS.join("/")}`,
          isChecked: true,
        },
      ],
    } as ToolPkg.InputMenuToggleObjectResult;
  }

  if (action === INPUT_MENU_TOGGLE_ACTION.toggle) {
    const toggleId = payload.toggleId;

    if (toggleId === TOGGLE_IDS.limiter) {
      await writeLimiterEnabled(!limiterEnabled);
      return { ok: true } as ToolPkg.InputMenuToggleObjectResult;
    }

    if (toggleId === TOGGLE_IDS.adjust) {
      const currentIndex = FLOOR_OPTIONS.indexOf(
        floorLimit as (typeof FLOOR_OPTIONS)[number]
      );
      const nextIndex = (currentIndex + 1) % FLOOR_OPTIONS.length;
      await writeFloorLimit(FLOOR_OPTIONS[nextIndex]!);
      return { ok: true } as ToolPkg.InputMenuToggleObjectResult;
    }
  }

  return { ok: false } as ToolPkg.InputMenuToggleObjectResult;
}

export function onFinalize(input: ToolPkg.PromptFinalizeHookEvent) {
  const payload = input.eventPayload || {};
  const history = payload.preparedHistory || payload.chatHistory || [];

  if (!history.length) {
    return null;
  }

  const floorLimit = readFloorLimit();
  const limiterEnabled = readLimiterEnabled();

  if (!limiterEnabled) {
    console.log(`[limiter] disabled, pass through ${history.length} msgs`);
    return null;
  }

  const systemMsgs: ToolPkg.PromptTurn[] = [];
  const nonSystemMsgs: ToolPkg.PromptTurn[] = [];

  for (const message of history) {
    if (message.kind === PROMPT_TURN_KIND.SYSTEM) {
      systemMsgs.push(message);
      continue;
    }
    nonSystemMsgs.push(message);
  }

  const userCount = nonSystemMsgs.filter(
    (message) => message.kind === PROMPT_TURN_KIND.USER
  ).length;
  if (userCount <= floorLimit) {
    const result = systemMsgs.concat(nonSystemMsgs);
    console.log(
      `[limiter] ${userCount} floors <= limit ${floorLimit}, no trim, msgs: ${history.length} -> ${result.length}`
    );
    return { preparedHistory: result };
  }

  let keepFromIndex = 0;
  let countedUsers = 0;
  for (let index = nonSystemMsgs.length - 1; index >= 0; index -= 1) {
    if (nonSystemMsgs[index].kind !== PROMPT_TURN_KIND.USER) {
      continue;
    }
    countedUsers += 1;
    if (countedUsers === floorLimit) {
      keepFromIndex = index;
      break;
    }
  }

  const keptMsgs = nonSystemMsgs.slice(keepFromIndex);
  const finalMsgs = systemMsgs.concat(keptMsgs);

  console.log(
    `[limiter] floors: ${userCount}, limit: ${floorLimit}, msgs: ${history.length} -> ${finalMsgs.length} (${systemMsgs.length} sys + ${keptMsgs.length} non-system)`
  );

  return { preparedHistory: finalMsgs };
}
