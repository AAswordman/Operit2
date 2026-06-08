"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.registerToolPkg = registerToolPkg;
exports.onInputMenuToggle = onInputMenuToggle;
exports.onFinalize = onFinalize;
const constants_1 = require("./constants");
function readEnv(key) {
    if (typeof getEnv !== "function") {
        return "";
    }
    const value = getEnv(key);
    return value == null ? "" : String(value).trim();
}
function readFloorLimit() {
    const raw = readEnv(constants_1.ENV_KEYS.floorLimit);
    const parsed = Number.parseInt(raw, 10);
    if (!Number.isFinite(parsed) || parsed < 1) {
        return constants_1.DEFAULT_FLOOR_LIMIT;
    }
    return parsed;
}
function readLimiterEnabled() {
    const raw = readEnv(constants_1.ENV_KEYS.enabled).toLowerCase();
    if (!raw) {
        return constants_1.DEFAULT_LIMITER_ENABLED;
    }
    return raw === "1" || raw === "true" || raw === "yes" || raw === "on";
}
async function writeEnvValue(key, value) {
    await Tools.SoftwareSettings.writeEnvironmentVariable(key, value);
}
async function writeFloorLimit(nextLimit) {
    await writeEnvValue(constants_1.ENV_KEYS.floorLimit, String(nextLimit));
}
async function writeLimiterEnabled(nextEnabled) {
    await writeEnvValue(constants_1.ENV_KEYS.enabled, nextEnabled ? "true" : "false");
}
function registerToolPkg() {
    ToolPkg.registerPromptFinalizeHook({
        id: constants_1.HOOK_IDS.finalize,
        function: onFinalize,
    });
    ToolPkg.registerInputMenuTogglePlugin({
        id: constants_1.HOOK_IDS.menu,
        function: onInputMenuToggle,
    });
    return true;
}
async function onInputMenuToggle(event) {
    const payload = event.eventPayload || {};
    const action = payload.action;
    const floorLimit = readFloorLimit();
    const limiterEnabled = readLimiterEnabled();
    if (action === constants_1.INPUT_MENU_TOGGLE_ACTION.create) {
        return {
            toggles: [
                {
                    id: constants_1.TOGGLE_IDS.limiter,
                    icon: "layers_outlined",
                    title: "楼层限制器",
                    description: limiterEnabled
                        ? `已开启 · 保留最近 ${floorLimit} 层`
                        : "已关闭",
                    isChecked: limiterEnabled,
                },
                {
                    id: constants_1.TOGGLE_IDS.adjust,
                    icon: "format_list_numbered",
                    title: `调节楼层数 ▶ ${floorLimit}`,
                    description: `点击切换: ${constants_1.FLOOR_OPTIONS.join("/")}`,
                    isChecked: true,
                },
            ],
        };
    }
    if (action === constants_1.INPUT_MENU_TOGGLE_ACTION.toggle) {
        const toggleId = payload.toggleId;
        if (toggleId === constants_1.TOGGLE_IDS.limiter) {
            await writeLimiterEnabled(!limiterEnabled);
            return { ok: true };
        }
        if (toggleId === constants_1.TOGGLE_IDS.adjust) {
            const currentIndex = constants_1.FLOOR_OPTIONS.indexOf(floorLimit);
            const nextIndex = (currentIndex + 1) % constants_1.FLOOR_OPTIONS.length;
            await writeFloorLimit(constants_1.FLOOR_OPTIONS[nextIndex]);
            return { ok: true };
        }
    }
    return { ok: false };
}
function onFinalize(input) {
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
    const systemMsgs = [];
    const nonSystemMsgs = [];
    for (const message of history) {
        if (message.kind === constants_1.PROMPT_TURN_KIND.SYSTEM) {
            systemMsgs.push(message);
            continue;
        }
        nonSystemMsgs.push(message);
    }
    const userCount = nonSystemMsgs.filter((message) => message.kind === constants_1.PROMPT_TURN_KIND.USER).length;
    if (userCount <= floorLimit) {
        const result = systemMsgs.concat(nonSystemMsgs);
        console.log(`[limiter] ${userCount} floors <= limit ${floorLimit}, no trim, msgs: ${history.length} -> ${result.length}`);
        return { preparedHistory: result };
    }
    let keepFromIndex = 0;
    let countedUsers = 0;
    for (let index = nonSystemMsgs.length - 1; index >= 0; index -= 1) {
        if (nonSystemMsgs[index].kind !== constants_1.PROMPT_TURN_KIND.USER) {
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
    console.log(`[limiter] floors: ${userCount}, limit: ${floorLimit}, msgs: ${history.length} -> ${finalMsgs.length} (${systemMsgs.length} sys + ${keptMsgs.length} non-system)`);
    return { preparedHistory: finalMsgs };
}
