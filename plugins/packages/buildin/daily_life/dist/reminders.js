"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.parseDueDate = parseDueDate;
exports.receiveReminder = receiveReminder;
exports.deliverReminders = deliverReminders;
const reminderKeyPrefix = "reminder_";
const interruptedDeliveryError = "Notification delivery was interrupted; its outcome is unknown. It has not been resent.";
/** Holds the native preference owner shared by reminder operations. */
let store = null;
let persistenceFailure = null;
let ownerQueue = Promise.resolve();
/** Requires non-empty reminder input without rewriting its content. */
function requireText(value, field) {
    if (typeof value !== "string" || value.trim() === "") {
        throw new Error(`${field} must be a non-empty string`);
    }
    return value;
}
/** Parses an explicit ISO 8601 instant and rejects normalized invalid calendar dates. */
function parseDueDate(value) {
    requireText(value, "due_date");
    const match = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})(?::(\d{2})(?:\.(\d{1,3}))?)?(Z|[+-]\d{2}:\d{2})$/.exec(value);
    if (match === null)
        throw new Error("due_date must be an ISO 8601 date-time with Z or an explicit UTC offset");
    const year = Number(match[1]);
    const month = Number(match[2]);
    const day = Number(match[3]);
    const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
    const days = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if (month < 1 || month > 12 || day < 1 || day > days[month - 1] ||
        Number(match[4]) > 23 || Number(match[5]) > 59 ||
        (match[6] !== undefined && Number(match[6]) > 59)) {
        throw new Error("due_date contains an invalid calendar date or time");
    }
    const offset = match[8];
    if (offset !== "Z" && (Number(offset.slice(1, 3)) > 23 || Number(offset.slice(4, 6)) > 59)) {
        throw new Error("due_date contains an invalid UTC offset");
    }
    const timestamp = Date.parse(value);
    if (!Number.isSafeInteger(timestamp))
        throw new Error("due_date is not a supported instant");
    return timestamp;
}
/** Validates persistent records rather than silently replacing corrupt plugin storage. */
function validateReminders(reminders) {
    const ids = new Set();
    for (const record of reminders) {
        if (record === null || typeof record !== "object")
            throw new Error("Invalid reminder record");
        const reminder = record;
        requireText(reminder.id, "reminder.id");
        requireText(reminder.title, "reminder.title");
        requireText(reminder.description, "reminder.description");
        if (parseDueDate(reminder.due_date) !== reminder.due_timestamp ||
            !Number.isFinite(Date.parse(reminder.created_at)) || ids.has(reminder.id) ||
            !["pending", "delivering", "delivered", "cancelled", "failed"].includes(reminder.status) ||
            (reminder.delivered_at !== null && !Number.isFinite(Date.parse(reminder.delivered_at))) ||
            (reminder.error !== null && typeof reminder.error !== "string")) {
            throw new Error(`Invalid stored reminder: ${reminder.id}`);
        }
        ids.add(reminder.id);
    }
}
/** Validates stored reminders and records interrupted deliveries before the first read. */
async function initialize(opened) {
    const reminders = (await opened.data()).entries()
        .filter(([key]) => key.startsWith(reminderKeyPrefix))
        .map(([, value]) => value);
    validateReminders(reminders);
    for (const reminder of reminders) {
        if (reminder.status === "delivering") {
            await opened.edit(preferences => {
                preferences.set(reminderKeyPrefix + reminder.id, {
                    ...reminder, status: "failed", error: interruptedDeliveryError,
                });
            });
        }
    }
    return opened;
}
/** Opens the plugin's native preference store exactly once per runtime. */
function load() {
    if (persistenceFailure !== null)
        return Promise.reject(persistenceFailure);
    if (store === null) {
        store = Tools.Storage.dataStore.open({ path: `${ToolPkg.getSpaceDataDir()}/reminders.preferences.json` })
            .then(opened => opened.withSchema(1, () => { }))
            .then(initialize);
    }
    return store;
}
/** Stops this owner after a storage failure instead of delivering uncommitted reminders. */
async function persistRecord(opened, reminder) {
    try {
        await opened.edit(preferences => {
            preferences.set(reminderKeyPrefix + reminder.id, reminder);
        });
    }
    catch (error) {
        persistenceFailure = new Error(`Reminder storage failed; this runtime has stopped delivery: ${String(error)}`);
        throw persistenceFailure;
    }
}
/** Returns a detached reminder record so callers cannot mutate plugin storage. */
function copy(reminder) {
    return { ...reminder };
}
/** Executes validated reminder operations inside the main-runtime owner. */
async function dispatch(request) {
    const opened = await load();
    switch (request.action) {
        case "create": {
            const title = requireText(request.title, "title");
            const description = requireText(request.description, "description");
            const dueTimestamp = parseDueDate(request.due_date);
            const now = Date.now();
            if (dueTimestamp <= now)
                throw new Error("due_date must be in the future");
            const reminder = {
                id: `reminder-${now}-${Math.random().toString(36).slice(2, 12)}`,
                title, description, due_date: new Date(dueTimestamp).toISOString(),
                due_timestamp: dueTimestamp, created_at: new Date(now).toISOString(),
                status: "pending", delivered_at: null, error: null,
            };
            const existing = (await opened.data()).contains(reminderKeyPrefix + reminder.id);
            if (existing)
                throw new Error("Reminder ID collision");
            await persistRecord(opened, reminder);
            return copy(reminder);
        }
        case "list": {
            const snapshot = await opened.data();
            const reminders = snapshot.entries()
                .filter(([key]) => key.startsWith(reminderKeyPrefix))
                .map(([, value]) => value);
            validateReminders(reminders);
            return reminders.map(copy).sort((left, right) => left.due_timestamp - right.due_timestamp);
        }
        case "cancel": {
            const id = requireText(request.id, "id");
            const snapshot = await opened.data();
            const stored = snapshot.entries().find(([key]) => key === reminderKeyPrefix + id);
            if (stored === undefined)
                throw new Error(`Reminder not found: ${id}`);
            const reminder = stored[1];
            validateReminders([reminder]);
            if (reminder.status !== "pending")
                throw new Error(`Cannot cancel reminder in ${reminder.status} state: ${id}`);
            const cancelled = { ...reminder, status: "cancelled" };
            await persistRecord(opened, cancelled);
            return copy(cancelled);
        }
        default:
            throw new Error("Unknown daily_life reminder operation");
    }
}
/** Routes reminder IPC through the same serialized owner used by delivery. */
function receiveReminder(request) {
    return owned(() => dispatch(request));
}
/** Claims due reminders durably and records the actual notification outcome once. */
async function deliver() {
    const opened = await load();
    const snapshot = await opened.data();
    const stored = snapshot.entries()
        .filter(([key]) => key.startsWith(reminderKeyPrefix))
        .map(([, value]) => value);
    validateReminders(stored);
    const due = stored.filter(reminder => reminder.status === "pending" && reminder.due_timestamp <= Date.now());
    const failures = [];
    for (const reminder of due) {
        const delivering = { ...reminder, status: "delivering" };
        await persistRecord(opened, delivering);
        try {
            await Tools.System.sendNotification(delivering.description, delivering.title);
        }
        catch (error) {
            const message = String(error);
            await persistRecord(opened, { ...delivering, status: "failed", error: message });
            failures.push(`${delivering.id}: ${message}`);
            continue;
        }
        await persistRecord(opened, { ...delivering, status: "delivered", delivered_at: new Date().toISOString() });
    }
    if (failures.length > 0)
        throw new Error(`Reminder delivery failed: ${failures.join("; ")}`);
}
/** Processes persisted due reminders when the existing Host emits a lifecycle or clock event. */
function deliverReminders() {
    return owned(deliver);
}
/** Serializes tool mutations and scheduled delivery without retrying failed operations. */
function owned(operation) {
    const result = ownerQueue.then(operation);
    ownerQueue = result.then(() => undefined, () => undefined);
    return result;
}
