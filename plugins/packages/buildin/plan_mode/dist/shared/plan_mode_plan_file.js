"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.openPlanDatabase = openPlanDatabase;
exports.normalizePlanText = normalizePlanText;
exports.normalizePlanContent = normalizePlanContent;
exports.hasPlanFile = hasPlanFile;
exports.readPlanFile = readPlanFile;
exports.planFileMatchesContent = planFileMatchesContent;
exports.writePlanFile = writePlanFile;
exports.deletePlanFile = deletePlanFile;
exports.watchPlanFile = watchPlanFile;
let database = null;
/** Opens the plan package's synchronized object database. */
function openPlanDatabase() {
    if (database === null)
        database = Tools.Storage.objects.open({ path: ToolPkg.getSpaceDataDir() + "/plans.sqlite" });
    return database;
}
/** Normalizes plan text for stable comparison and persistence. */
function normalizePlanText(content) { return content.replace(/\r\n/g, "\n").trim(); }
/** Requires nonempty plan text and terminates it consistently. */
function normalizePlanContent(content) {
    const normalized = normalizePlanText(content);
    if (!normalized)
        throw new Error("plan content is empty");
    return normalized + "\n";
}
/** Reports whether the chat owns a persisted plan. */
async function hasPlanFile(chatId) {
    return (await (await openPlanDatabase()).collection("plans").get(chatId)) !== null;
}
/** Reads a structured plan from plugin storage, independently of active views. */
async function readPlanFile(chatId) {
    const entry = await (await openPlanDatabase()).collection("plans").get(chatId);
    if (entry === null)
        return null;
    return parsePlanFile(chatId, entry.value);
}
/** Validates one structured plan record for both reads and live observation. */
function parsePlanFile(chatId, value) {
    if (value === null || typeof value !== "object" || Array.isArray(value) || typeof value.content !== "string") {
        throw new Error("Persisted plan has invalid content");
    }
    return { chatId, content: value.content };
}
/** Compares a candidate plan with this chat's durable plan. */
async function planFileMatchesContent(chatId, content) {
    const normalized = normalizePlanText(content);
    if (!normalized)
        return false;
    const plan = await readPlanFile(chatId);
    return plan !== null && normalizePlanText(plan.content) === normalized;
}
/** Commits normalized plan content to the owning package's collection. */
async function writePlanFile(chatId, content) {
    const normalized = normalizePlanContent(content);
    await (await openPlanDatabase()).commit([
        { op: "put", collection: "plans", key: chatId, value: { content: normalized } },
        { op: "delete", collection: "modes", key: chatId },
    ]);
    return { chatId, content: normalized };
}
/** Removes the chat's plan with an exact read-version precondition. */
async function deletePlanFile(chatId) {
    const plans = (await openPlanDatabase()).collection("plans");
    const entry = await plans.get(chatId);
    if (entry !== null)
        await plans.remove(chatId, { expectedVersion: entry.version });
    return { chatId, deleted: entry !== null };
}
/** Streams current plan records when a local or remote transaction changes the chat. */
async function* watchPlanFile(chatId, signal) {
    const database = await openPlanDatabase();
    for await (const entry of database.collection("plans").watch(chatId, { signal })) {
        yield entry === null ? null : parsePlanFile(chatId, entry.value);
    }
}
