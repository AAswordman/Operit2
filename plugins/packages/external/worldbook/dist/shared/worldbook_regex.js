"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.newRegexRule = newRegexRule;
exports.compileRegex = compileRegex;
exports.validateRegexRule = validateRegexRule;
exports.applyRegexRules = applyRegexRules;
exports.prepareRegexDisplay = prepareRegexDisplay;
exports.readRegexRules = readRegexRules;
exports.writeRegexRules = writeRegexRules;
exports.importRegexRules = importRegexRules;
exports.onRegexMessageTransform = onRegexMessageTransform;
exports.onRegexPromptFinalize = onRegexPromptFinalize;
exports.regexWorldInfo = regexWorldInfo;
const index_js_1 = require("../i18n/index.js");
const worldbook_variables_js_1 = require("./worldbook_variables.js");
/** Creates an editable rule with explicit defaults matching the upstream editor. */
function newRegexRule() {
    const t = (0, index_js_1.currentWorldBookI18n)().regex;
    return { id: `regex-${Date.now().toString(36)}-${Math.random().toString(36).slice(2)}`,
        scriptName: t.newRule, findRegex: "", replaceString: "", trimStrings: [], placement: [2],
        disabled: false, markdownOnly: true, promptOnly: false, runOnEdit: false,
        substituteRegex: 0, minDepth: null, maxDepth: null, characterCardId: "" };
}
/** Parses a plain pattern or a slash-delimited JavaScript pattern without changing its flags. */
function compileRegex(pattern) {
    const t = (0, index_js_1.currentWorldBookI18n)().regex;
    if (!pattern)
        throw new Error(t.patternRequired);
    const end = pattern.startsWith("/") ? pattern.lastIndexOf("/") : -1;
    if (end === 0)
        throw new Error(t.patternSlashMissing);
    try {
        return end > 0 ? new RegExp(pattern.slice(1, end), pattern.slice(end + 1)) : new RegExp(pattern);
    }
    catch (error) {
        throw new Error(t.patternInvalid(String(error)));
    }
}
/** Reports malformed rule JSON in the current host language while preserving parser details. */
function parseRegexJson(content) {
    try {
        return JSON.parse(content);
    }
    catch (error) {
        throw new Error((0, index_js_1.currentWorldBookI18n)().regex.jsonInvalid(String(error)));
    }
}
/** Validates imported and persisted data before any rule can run or be saved. */
function validateRegexRule(value) {
    const t = (0, index_js_1.currentWorldBookI18n)().regex;
    if (!value || typeof value !== "object" || Array.isArray(value))
        throw new Error(t.ruleObjectRequired);
    const raw = value;
    const defaults = { ...newRegexRule(), markdownOnly: false };
    const rule = { ...defaults, ...raw };
    for (const key of ["id", "scriptName", "findRegex", "replaceString", "characterCardId"]) {
        if (typeof rule[key] !== "string")
            throw new Error(t.fieldTextRequired(key));
    }
    if (!rule.id.trim() || !rule.scriptName.trim())
        throw new Error(t.identityRequired);
    for (const key of ["disabled", "markdownOnly", "promptOnly", "runOnEdit"]) {
        if (typeof rule[key] !== "boolean")
            throw new Error(t.fieldBooleanRequired(key));
    }
    if (!Array.isArray(rule.placement) || !rule.placement.length || rule.placement.some(item => ![1, 2, 5, 6].includes(item))) {
        throw new Error(t.placementInvalid);
    }
    if (!Array.isArray(rule.trimStrings) || rule.trimStrings.some(item => typeof item !== "string"))
        throw new Error(t.trimInvalid);
    if (![0, 1, 2].includes(rule.substituteRegex))
        throw new Error(t.macroModeInvalid);
    for (const key of ["minDepth", "maxDepth"]) {
        if (rule[key] !== null && (!Number.isInteger(rule[key]) || rule[key] < -1))
            throw new Error(t.depthInvalid(key));
    }
    if (rule.minDepth !== null && rule.maxDepth !== null && rule.maxDepth >= 0 && rule.minDepth > rule.maxDepth)
        throw new Error(t.depthRangeInvalid);
    // Dynamic patterns are compiled after macro substitution; the syntax of their literal template may be incomplete.
    if (rule.substituteRegex === 0)
        compileRegex(rule.findRegex);
    else if (!rule.findRegex)
        throw new Error(t.patternRequired);
    return rule;
}
/** Substitutes worldbook variable macros, optionally escaping the substituted values for a pattern. */
function substitute(text, context, escapeValues = false) {
    if (!escapeValues)
        return (0, worldbook_variables_js_1.renderWorldBookContent)(text, context.variables);
    return text.replace(/\{\{[^{}]+\}\}|<%=[\s\S]*?%>/g, token => {
        const rendered = (0, worldbook_variables_js_1.renderWorldBookContent)(token, context.variables);
        if (rendered === token)
            return token;
        return rendered.replace(/[.*+?^${}()|[\]\\/]/g, "\\$&").replace(/\n/g, "\\n").replace(/\r/g, "\\r").replace(/\t/g, "\\t");
    });
}
/** Applies rules in stored order with upstream stage, edit, depth and capture semantics. */
function applyRegexRules(text, rules, context) {
    const t = (0, index_js_1.currentWorldBookI18n)().regex;
    let result = text;
    for (const rule of rules) {
        if (!selectsRule(rule, context))
            continue;
        const pattern = rule.substituteRegex === 0 ? rule.findRegex : substitute(rule.findRegex, context, rule.substituteRegex === 2);
        try {
            result = result.replace(compileRegex(pattern), (...args) => {
                const hasNames = typeof args[args.length - 1] === "object";
                const captures = args.slice(0, hasNames ? -3 : -2);
                const named = hasNames ? args[args.length - 1] : {};
                const template = rule.replaceString.replace(/\{\{match\}\}/gi, "$0");
                const replaced = template.replace(/\$(\d+)|\$<([^>]+)>/g, (_token, number, name) => {
                    let capture = number !== undefined ? captures[Number(number)] : named[name];
                    let value = typeof capture === "string" ? capture : "";
                    for (const trim of rule.trimStrings)
                        value = value.split(substitute(trim, context)).join("");
                    return value;
                });
                return substitute(replaced, context);
            });
        }
        catch (error) {
            throw new Error(t.ruleFailed(rule.scriptName, String(error)));
        }
    }
    return result;
}
/** Shares stage, participant, edit and depth selection between native and boundary execution. */
function selectsRule(rule, context) {
    if (rule.disabled || !rule.placement.includes(context.placement))
        return false;
    if (rule.characterCardId && rule.characterCardId !== context.participantId)
        return false;
    const selected = context.stage === "display" ? rule.markdownOnly
        : context.stage === "prompt" ? rule.promptOnly : !rule.markdownOnly && !rule.promptOnly;
    return selected && (!context.isEdit || rule.runOnEdit)
        && !(rule.minDepth !== null && rule.minDepth >= -1 && context.depth < rule.minDepth)
        && !(rule.maxDepth !== null && rule.maxDepth >= 0 && context.depth > rule.maxDepth);
}
/** Resolves literal macros once while preserving capture tokens for the native matcher. */
function nativeReplacement(template, context) {
    const references = /\$(\d+)|\$<([^>]+)>/g;
    template = template.replace(/\{\{match\}\}/gi, "$0");
    let result = "", offset = 0;
    for (const match of template.matchAll(references)) {
        result += substitute(template.slice(offset, match.index), context).replace(/\$/g, () => "$$") + match[0];
        offset = match.index + match[0].length;
    }
    return result + substitute(template.slice(offset), context).replace(/\$/g, () => "$$");
}
/** Reads one configuration snapshot for the ordered window; native streams own all text matching. */
async function prepareRegexDisplay(event) {
    if (event.eventName !== "prepare_display")
        throw new Error((0, index_js_1.currentWorldBookI18n)().regex.unsupportedStage(event.eventName));
    const { chatId, messages } = event.eventPayload;
    if (!messages.length)
        return { programs: [], messagePrograms: [] };
    const rules = await readRegexRules();
    const selections = messages.map(message => {
        const context = { stage: "display", placement: message.sender === "user" ? 1 : 2,
            depth: message.depth, isEdit: message.isEdit, participantId: message.participantId ?? "", variables: null };
        const selected = message.sender === "user" || message.sender === "ai" ? rules.map(rule => ({ rule, kinds: [
                ...(selectsRule(rule, context) ? ["markdown"] : []),
                ...(message.sender === "ai" && selectsRule(rule, { ...context, placement: 6 }) ? ["thinking"] : []),
            ] })).filter(item => item.kinds.length) : [];
        return { context, selected };
    });
    const participants = [...new Set(selections.filter(item => item.selected.length).map(item => item.context.participantId))];
    const variables = participants.length ? await (0, worldbook_variables_js_1.readWorldBookVariableContexts)(chatId, participants) : new Map();
    const prepared = new Map();
    const ruleIds = new Map();
    const programIds = new Map();
    const programs = [];
    const messagePrograms = selections.map(({ context, selected }) => {
        context.variables = variables.get(context.participantId) ?? null;
        const rules = selected.map(({ rule, kinds }) => {
            let variants = prepared.get(rule);
            if (!variants) {
                variants = new Map();
                prepared.set(rule, variants);
            }
            const key = JSON.stringify([context.participantId, kinds]);
            let native = variants.get(key);
            if (!native) {
                const pattern = rule.substituteRegex === 0 ? rule.findRegex : substitute(rule.findRegex, context, rule.substituteRegex === 2);
                const regex = compileRegex(pattern);
                native = { pattern: regex.source, flags: regex.flags, kinds,
                    replacement: nativeReplacement(rule.replaceString, context),
                    trimStrings: rule.trimStrings.map(trim => substitute(trim, context)) };
                variants.set(key, native);
                ruleIds.set(native, ruleIds.size);
            }
            return native;
        });
        const key = rules.map(rule => ruleIds.get(rule)).join(",");
        let index = programIds.get(key);
        if (index === undefined) {
            index = programs.length;
            programIds.set(key, index);
            programs.push({ rules });
        }
        return index;
    });
    return { programs, messagePrograms };
}
/** Reads plugin-owned rules through VFS; an unconfigured book has no rules. */
async function readRegexRules() {
    const t = (0, index_js_1.currentWorldBookI18n)().regex;
    const path = `${ToolPkg.getConfigDir()}/regex_scripts.json`;
    const exists = await Tools.Files.exists(path);
    if (!exists.exists)
        return [];
    const file = await Tools.Files.read(path);
    if (typeof file.content !== "string")
        throw new Error(t.configReadFailed);
    const data = parseRegexJson(file.content);
    if (!Array.isArray(data))
        throw new Error(t.configArrayRequired);
    const rules = data.map(validateRegexRule);
    if (new Set(rules.map(rule => rule.id)).size !== rules.length)
        throw new Error(t.duplicateId);
    return rules;
}
/** Validates the complete ordered configuration before its VFS commit refreshes visible messages. */
async function writeRegexRules(rules) {
    const t = (0, index_js_1.currentWorldBookI18n)().regex;
    const checked = rules.map(validateRegexRule);
    if (new Set(checked.map(rule => rule.id)).size !== checked.length)
        throw new Error(t.duplicateId);
    const root = ToolPkg.getConfigDir();
    await Tools.Files.mkdir(root, true);
    await Tools.Files.write(`${root}/regex_scripts.json`, JSON.stringify(checked, null, 2), false);
}
/** Accepts a standalone rule, ordered export, or character-card regex extension without partial imports. */
function importRegexRules(content) {
    const t = (0, index_js_1.currentWorldBookI18n)().regex;
    const parsed = parseRegexJson(content);
    let raw;
    if (Array.isArray(parsed))
        raw = parsed;
    else if (parsed && Array.isArray(parsed.regex_scripts))
        raw = parsed.regex_scripts;
    else if (parsed?.extensions && Array.isArray(parsed.extensions.regex_scripts))
        raw = parsed.extensions.regex_scripts;
    else if (parsed?.data?.extensions && Array.isArray(parsed.data.extensions.regex_scripts))
        raw = parsed.data.extensions.regex_scripts;
    else if (parsed && typeof parsed.findRegex === "string")
        raw = [parsed];
    else
        throw new Error(t.importMissing);
    return raw.map(value => validateRegexRule({ ...value, id: newRegexRule().id }));
}
/** Changes only the selected semantic text part at the host-selected boundary. */
async function onRegexMessageTransform(event) {
    const t = (0, index_js_1.currentWorldBookI18n)().regex;
    if (event.eventName !== "before_persist")
        throw new Error(t.unsupportedStage(event.eventName));
    const payload = event.eventPayload;
    if (payload.sender !== "user" && payload.sender !== "ai")
        return null;
    const rules = await readRegexRules();
    if (!rules.length)
        return null;
    const participant = payload.participantId ?? "";
    const variables = await (0, worldbook_variables_js_1.readWorldBookVariableContext)(payload.chatId, participant);
    return applyRegexRules(payload.text, rules, {
        stage: "source",
        placement: payload.kind === "thinking" ? 6 : payload.sender === "user" ? 1 : 2,
        depth: payload.depth, isEdit: payload.isEdit, participantId: participant, variables,
    });
}
/** Runs prompt-only replacements after injection, identically for sends and token estimates. */
async function onRegexPromptFinalize(event) {
    if (event.eventName !== "before_send_to_model")
        return null;
    const rules = await readRegexRules();
    if (!rules.some(rule => rule.promptOnly && !rule.disabled))
        return null;
    const payload = event.eventPayload;
    const participant = payload.metadata?.executionContext?.participantId ?? "";
    const chatId = payload.chatId ?? "";
    const variables = await (0, worldbook_variables_js_1.readWorldBookVariableContext)(chatId, participant);
    const context = { stage: "prompt", placement: 1, depth: 0, isEdit: false, participantId: participant, variables };
    const input = payload.processedInput ?? "";
    const processedInput = applyRegexRules(input, rules, context);
    const history = payload.preparedHistory ?? [];
    const last = history.length - 1;
    const hasCurrent = last >= 0 && history[last].kind === "USER" &&
        (history[last].content === input || history[last].content === payload.rawInput);
    let depth = hasCurrent ? -1 : 0;
    const preparedHistory = [...history];
    for (let index = last; index >= 0; index--) {
        const turn = history[index];
        if (turn.kind !== "USER" && turn.kind !== "ASSISTANT")
            continue;
        depth++;
        if (index === last && hasCurrent) {
            preparedHistory[index] = { ...turn, content: processedInput };
            continue;
        }
        const placement = turn.kind === "USER" ? 1 : 2;
        // Reasoning is delimited in assistant prompt turns; tool turns remain separate and untouched.
        const content = turn.kind === "USER" ? applyRegexRules(turn.content, rules, { ...context, placement, depth })
            : turn.content.replace(/<think>([\s\S]*?)<\/think>|([\s\S]+?)(?=<think>|$)/g, (whole, thinking, body) => thinking !== undefined
                ? `<think>${applyRegexRules(thinking, rules, { ...context, placement: 6, depth })}</think>`
                : applyRegexRules(body ?? whole, rules, { ...context, placement, depth }));
        preparedHistory[index] = { ...turn, content };
    }
    return { processedInput, preparedHistory };
}
/** Applies source and prompt world-info rules before the entry is wrapped for injection. */
function regexWorldInfo(text, rules, participantId, variables) {
    const context = { stage: "source", placement: 5, depth: 0, isEdit: false, participantId, variables };
    return applyRegexRules(applyRegexRules(text, rules, context), rules, { ...context, stage: "prompt" });
}
