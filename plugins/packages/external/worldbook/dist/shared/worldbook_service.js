"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.toWorldBookListEntry = toWorldBookListEntry;
exports.listWorldBookEntries = listWorldBookEntries;
exports.getWorldBookEntry = getWorldBookEntry;
exports.createWorldBookEntry = createWorldBookEntry;
exports.importWorldBookEntries = importWorldBookEntries;
exports.updateWorldBookEntry = updateWorldBookEntry;
exports.deleteWorldBookEntry = deleteWorldBookEntry;
exports.toggleWorldBookEntry = toggleWorldBookEntry;
exports.listWorldBookCharacterCards = listWorldBookCharacterCards;
exports.listWorldBookGroups = listWorldBookGroups;
exports.createWorldBookGroup = createWorldBookGroup;
exports.deleteWorldBookGroup = deleteWorldBookGroup;
exports.copyEntryToGroup = copyEntryToGroup;
exports.updateWorldBookGroup = updateWorldBookGroup;
const index_js_1 = require("../i18n/index.js");
const worldbook_storage_js_1 = require("./worldbook_storage.js");
function generateId() {
    return `wb_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
}
function splitKeywords(raw) {
    if (!raw) {
        return [];
    }
    return raw
        .split(/[,，]/)
        .map((keyword) => keyword.trim())
        .filter((keyword) => keyword.length > 0);
}
function normalizeNumber(value, fallbackValue) {
    const numeric = Number(value);
    return Number.isFinite(numeric) ? numeric : fallbackValue;
}
function normalizeInjectTarget(value) {
    if (value === "user") {
        return "user";
    }
    if (value === "assistant") {
        return "assistant";
    }
    return "system";
}
function normalizeInjectPosition(value) {
    if (value === "prepend") {
        return "prepend";
    }
    if (value === "at_depth") {
        return "at_depth";
    }
    return "append";
}
function worldBookError(code, message, details) {
    return { code, message, details };
}
function isRecord(value) {
    return !!value && typeof value === "object" && !Array.isArray(value);
}
function hasOwn(record, key) {
    return Object.prototype.hasOwnProperty.call(record, key);
}
function isNullish(value) {
    return value == null;
}
function toKeywordArray(value) {
    if (Array.isArray(value)) {
        return value
            .map((item) => String(item || "").trim())
            .filter((item) => item.length > 0);
    }
    if (typeof value === "string") {
        return splitKeywords(value);
    }
    return [];
}
function requireStringField(record, key, fieldLabel) {
    const value = record[key];
    if (typeof value !== "string") {
        throw worldBookError("INVALID_FIELD_TYPE", (0, index_js_1.currentWorldBookI18n)().service.fieldString(fieldLabel));
    }
    return value.trim();
}
function readOptionalBooleanField(record, key, defaultValue, fieldLabel) {
    if (!hasOwn(record, key)) {
        return defaultValue;
    }
    const value = record[key];
    if (isNullish(value)) {
        return defaultValue;
    }
    if (typeof value !== "boolean") {
        throw worldBookError("INVALID_FIELD_TYPE", (0, index_js_1.currentWorldBookI18n)().service.fieldBoolean(fieldLabel));
    }
    return value;
}
function readOptionalNumberField(record, key, defaultValue, fieldLabel) {
    if (!hasOwn(record, key)) {
        return defaultValue;
    }
    const value = record[key];
    if (isNullish(value)) {
        return defaultValue;
    }
    if (typeof value !== "number" || !Number.isFinite(value)) {
        throw worldBookError("INVALID_FIELD_TYPE", (0, index_js_1.currentWorldBookI18n)().service.fieldNumber(fieldLabel));
    }
    return value;
}
function readOptionalKeywordArrayField(record, key, fieldLabel) {
    if (!hasOwn(record, key)) {
        return [];
    }
    const value = record[key];
    if (isNullish(value)) {
        return [];
    }
    if (!Array.isArray(value)) {
        throw worldBookError("INVALID_FIELD_TYPE", (0, index_js_1.currentWorldBookI18n)().service.fieldArray(fieldLabel));
    }
    const keywords = value.map((item) => {
        if (typeof item !== "string") {
            throw worldBookError("INVALID_FIELD_TYPE", (0, index_js_1.currentWorldBookI18n)().service.fieldArray(fieldLabel));
        }
        return item.trim();
    });
    return keywords.filter((item) => item.length > 0);
}
function addWarning(warnings, warning) {
    if (!warnings.includes(warning)) {
        warnings.push(warning);
    }
}
function addTemplateCompatibilityWarnings(content, warnings) {
    if (/\{\{\s*get_preset_variable::/i.test(content)) {
        addWarning(warnings, (0, index_js_1.currentWorldBookI18n)().service.presetRead);
    }
    if (/\{\{\s*format_preset_variable::/i.test(content)) {
        addWarning(warnings, (0, index_js_1.currentWorldBookI18n)().service.presetFormat);
    }
    if (/\{\{\s*(?:set|inc|dec)(?:global)?var::/i.test(content)) {
        addWarning(warnings, (0, index_js_1.currentWorldBookI18n)().service.inlineWrite);
    }
    if (/\{\{\s*[.$][A-Za-z][A-Za-z0-9_-]*\s*(?:\+\+|--|\+=|-=|\|\|=?|\?\?=?|==|!=|>=|<=|>|<|=)[^}]*\}\}/.test(content)) {
        addWarning(warnings, (0, index_js_1.currentWorldBookI18n)().service.inlineExpression);
    }
}
function resolveCharacterBookRoleTarget(rawRole, warnings) {
    if (rawRole == null) {
        return "system";
    }
    if (rawRole === 0 || rawRole === "system") {
        return "system";
    }
    if (rawRole === 1 || rawRole === "user") {
        return "user";
    }
    if (rawRole === 2 || rawRole === "assistant" || rawRole === "char") {
        return "assistant";
    }
    addWarning(warnings, (0, index_js_1.currentWorldBookI18n)().service.unknownRole(String(rawRole)));
    return "system";
}
function resolveCharacterBookPlacement(rawEntry, extensions, warnings) {
    const position = typeof rawEntry.position === "string" ? rawEntry.position.trim().toLowerCase() : "";
    const roleValue = hasOwn(rawEntry, "role") ? rawEntry.role : extensions.role;
    const depthValue = hasOwn(rawEntry, "depth") ? rawEntry.depth : extensions.depth;
    switch (position) {
        case "":
            return {
                inject_target: "system",
                inject_position: "append",
                insertion_depth: 0
            };
        case "before_char":
            return {
                inject_target: "system",
                inject_position: "prepend",
                insertion_depth: 0
            };
        case "after_char":
            return {
                inject_target: "system",
                inject_position: "append",
                insertion_depth: 0
            };
        case "top_an":
            addWarning(warnings, (0, index_js_1.currentWorldBookI18n)().service.topAuthorNote);
            return {
                inject_target: "system",
                inject_position: "prepend",
                insertion_depth: 0
            };
        case "bottom_an":
        case "after_examples":
            addWarning(warnings, (0, index_js_1.currentWorldBookI18n)().service.mappedPosition(String(position)));
            return {
                inject_target: "system",
                inject_position: "append",
                insertion_depth: 0
            };
        case "at_depth":
        case "in_chat":
            return {
                inject_target: resolveCharacterBookRoleTarget(roleValue, warnings),
                inject_position: "at_depth",
                insertion_depth: normalizeNumber(depthValue, 0)
            };
        default:
            addWarning(warnings, (0, index_js_1.currentWorldBookI18n)().service.unsupportedPosition(String(position)));
            return {
                inject_target: "system",
                inject_position: "append",
                insertion_depth: 0
            };
    }
}
function buildImportedEntry(normalized, characterCardId, now) {
    return {
        id: generateId(),
        name: normalized.name,
        content: normalized.content,
        keywords: normalized.keywords,
        is_regex: normalized.is_regex,
        case_sensitive: normalized.case_sensitive,
        always_active: normalized.always_active,
        enabled: normalized.enabled,
        priority: normalized.priority,
        scan_depth: normalized.scan_depth,
        inject_target: normalized.inject_target,
        inject_position: normalizeInjectPosition(normalized.inject_position),
        insertion_depth: normalizeNumber(normalized.insertion_depth, 0),
        character_card_id: characterCardId,
        created_at: now,
        updated_at: now
    };
}
function parseOperitEntryArray(rawEntries) {
    const warnings = [];
    const entries = [];
    for (const rawEntry of rawEntries) {
        if (!isRecord(rawEntry)) {
            continue;
        }
        const name = requireStringField(rawEntry, "name", (0, index_js_1.currentWorldBookI18n)().service.field("Operit", "name"));
        const content = requireStringField(rawEntry, "content", (0, index_js_1.currentWorldBookI18n)().service.field("Operit", "content"));
        if (!name || !content) {
            continue;
        }
        entries.push({
            name,
            content,
            keywords: readOptionalKeywordArrayField(rawEntry, "keywords", (0, index_js_1.currentWorldBookI18n)().service.field("Operit", "keywords")),
            is_regex: readOptionalBooleanField(rawEntry, "is_regex", false, (0, index_js_1.currentWorldBookI18n)().service.field("Operit", "is_regex")),
            case_sensitive: readOptionalBooleanField(rawEntry, "case_sensitive", false, (0, index_js_1.currentWorldBookI18n)().service.field("Operit", "case_sensitive")),
            always_active: readOptionalBooleanField(rawEntry, "always_active", false, (0, index_js_1.currentWorldBookI18n)().service.field("Operit", "always_active")),
            enabled: readOptionalBooleanField(rawEntry, "enabled", true, (0, index_js_1.currentWorldBookI18n)().service.field("Operit", "enabled")),
            priority: readOptionalNumberField(rawEntry, "priority", 50, (0, index_js_1.currentWorldBookI18n)().service.field("Operit", "priority")),
            scan_depth: readOptionalNumberField(rawEntry, "scan_depth", 0, (0, index_js_1.currentWorldBookI18n)().service.field("Operit", "scan_depth")),
            inject_target: normalizeInjectTarget(rawEntry.inject_target),
            inject_position: normalizeInjectPosition(rawEntry.inject_position),
            insertion_depth: readOptionalNumberField(rawEntry, "insertion_depth", 0, (0, index_js_1.currentWorldBookI18n)().service.field("Operit", "insertion_depth"))
        });
    }
    if (entries.length === 0) {
        return null;
    }
    return {
        sourceType: "operit_entries",
        warnings,
        entries
    };
}
function parseSillyTavernRecord(rawEntry, warnings) {
    const name = typeof rawEntry.name === "string"
        ? rawEntry.name.trim()
        : typeof rawEntry.comment === "string"
            ? rawEntry.comment.trim()
            : typeof rawEntry.id === "string"
                ? rawEntry.id.trim()
                : "";
    const content = requireStringField(rawEntry, "content", (0, index_js_1.currentWorldBookI18n)().service.field("SillyTavern", "content"));
    if (!name || !content) {
        return null;
    }
    addTemplateCompatibilityWarnings(content, warnings);
    const keywords = readOptionalKeywordArrayField(rawEntry, "key", (0, index_js_1.currentWorldBookI18n)().service.field("SillyTavern", "key"));
    const alwaysActive = readOptionalBooleanField(rawEntry, "constant", false, (0, index_js_1.currentWorldBookI18n)().service.field("SillyTavern", "constant"));
    if (keywords.length === 0 && !alwaysActive) {
        addWarning(warnings, (0, index_js_1.currentWorldBookI18n)().service.missingKeywords);
        return null;
    }
    const secondaryKeys = readOptionalKeywordArrayField(rawEntry, "keysecondary", (0, index_js_1.currentWorldBookI18n)().service.field("SillyTavern", "keysecondary"));
    if (secondaryKeys.length > 0) {
        addWarning(warnings, (0, index_js_1.currentWorldBookI18n)().service.secondaryKeys);
    }
    if (readOptionalBooleanField(rawEntry, "selective", false, (0, index_js_1.currentWorldBookI18n)().service.field("SillyTavern", "selective"))) {
        addWarning(warnings, (0, index_js_1.currentWorldBookI18n)().service.selective);
    }
    if (rawEntry.role != null) {
        addWarning(warnings, (0, index_js_1.currentWorldBookI18n)().service.unmappedRole);
    }
    return {
        name,
        content,
        keywords,
        is_regex: readOptionalBooleanField(rawEntry, "use_regex", false, (0, index_js_1.currentWorldBookI18n)().service.field("SillyTavern", "use_regex")),
        case_sensitive: readOptionalBooleanField(rawEntry, "caseSensitive", false, (0, index_js_1.currentWorldBookI18n)().service.field("SillyTavern", "caseSensitive")),
        always_active: alwaysActive,
        enabled: !readOptionalBooleanField(rawEntry, "disable", false, (0, index_js_1.currentWorldBookI18n)().service.field("SillyTavern", "disable")),
        priority: hasOwn(rawEntry, "display_index")
            ? readOptionalNumberField(rawEntry, "display_index", 50, (0, index_js_1.currentWorldBookI18n)().service.field("SillyTavern", "display_index"))
            : readOptionalNumberField(rawEntry, "order", 50, (0, index_js_1.currentWorldBookI18n)().service.field("SillyTavern", "order")),
        scan_depth: hasOwn(rawEntry, "scanDepth")
            ? readOptionalNumberField(rawEntry, "scanDepth", 0, (0, index_js_1.currentWorldBookI18n)().service.field("SillyTavern", "scanDepth"))
            : readOptionalNumberField(rawEntry, "depth", 0, (0, index_js_1.currentWorldBookI18n)().service.field("SillyTavern", "depth")),
        inject_target: "system",
        inject_position: "append",
        insertion_depth: 0
    };
}
function parseSillyTavernWorldBook(raw) {
    if (!isRecord(raw.entries)) {
        return null;
    }
    const warnings = [];
    const entries = [];
    for (const rawEntry of Object.values(raw.entries)) {
        if (!isRecord(rawEntry)) {
            continue;
        }
        const parsedEntry = parseSillyTavernRecord(rawEntry, warnings);
        if (parsedEntry) {
            entries.push(parsedEntry);
        }
    }
    if (entries.length === 0) {
        return null;
    }
    entries.sort((left, right) => right.priority - left.priority);
    return {
        sourceType: "sillytavern_worldbook",
        warnings,
        entries
    };
}
function parseCharacterBookEntry(rawEntry, warnings) {
    const name = typeof rawEntry.comment === "string"
        ? rawEntry.comment.trim()
        : typeof rawEntry.name === "string"
            ? rawEntry.name.trim()
            : typeof rawEntry.id === "string"
                ? rawEntry.id.trim()
                : "";
    const content = requireStringField(rawEntry, "content", (0, index_js_1.currentWorldBookI18n)().service.field("character_book", "content"));
    if (!name || !content) {
        return null;
    }
    addTemplateCompatibilityWarnings(content, warnings);
    const keywords = readOptionalKeywordArrayField(rawEntry, "keys", (0, index_js_1.currentWorldBookI18n)().service.field("character_book", "keys"));
    const alwaysActive = readOptionalBooleanField(rawEntry, "constant", false, (0, index_js_1.currentWorldBookI18n)().service.field("character_book", "constant"));
    if (keywords.length === 0 && !alwaysActive) {
        addWarning(warnings, (0, index_js_1.currentWorldBookI18n)().service.cardMissingKeywords);
        return null;
    }
    const secondaryKeys = readOptionalKeywordArrayField(rawEntry, "secondary_keys", (0, index_js_1.currentWorldBookI18n)().service.field("character_book", "secondary_keys"));
    if (secondaryKeys.length > 0) {
        addWarning(warnings, (0, index_js_1.currentWorldBookI18n)().service.cardSecondaryKeys);
    }
    if (readOptionalBooleanField(rawEntry, "selective", false, (0, index_js_1.currentWorldBookI18n)().service.field("character_book", "selective"))) {
        addWarning(warnings, (0, index_js_1.currentWorldBookI18n)().service.cardSelective);
    }
    const extensions = isRecord(rawEntry.extensions) ? rawEntry.extensions : {};
    const placement = resolveCharacterBookPlacement(rawEntry, extensions, warnings);
    return {
        name,
        content,
        keywords,
        is_regex: readOptionalBooleanField(rawEntry, "use_regex", false, (0, index_js_1.currentWorldBookI18n)().service.field("character_book", "use_regex")),
        case_sensitive: readOptionalBooleanField(extensions, "case_sensitive", false, (0, index_js_1.currentWorldBookI18n)().service.field("character_book", "extensions.case_sensitive")),
        always_active: alwaysActive,
        enabled: readOptionalBooleanField(rawEntry, "enabled", true, (0, index_js_1.currentWorldBookI18n)().service.field("character_book", "enabled")),
        priority: readOptionalNumberField(rawEntry, "insertion_order", 50, (0, index_js_1.currentWorldBookI18n)().service.field("character_book", "insertion_order")),
        scan_depth: readOptionalNumberField(extensions, "depth", 0, (0, index_js_1.currentWorldBookI18n)().service.field("character_book", "extensions.depth")),
        inject_target: placement.inject_target,
        inject_position: placement.inject_position,
        insertion_depth: placement.insertion_depth
    };
}
function parseCharacterBook(rawBook) {
    if (!Array.isArray(rawBook.entries)) {
        return null;
    }
    const warnings = [];
    const entries = [];
    for (const rawEntry of rawBook.entries) {
        if (!isRecord(rawEntry)) {
            continue;
        }
        const parsedEntry = parseCharacterBookEntry(rawEntry, warnings);
        if (parsedEntry) {
            entries.push(parsedEntry);
        }
    }
    if (entries.length === 0) {
        return null;
    }
    entries.sort((left, right) => right.priority - left.priority);
    return {
        sourceType: "character_book",
        warnings,
        entries
    };
}
function parseImportedWorldBookPayload(raw) {
    if (Array.isArray(raw)) {
        const parsedOperit = parseOperitEntryArray(raw);
        if (parsedOperit) {
            return parsedOperit;
        }
    }
    if (!isRecord(raw)) {
        throw worldBookError("INVALID_WORLD_BOOK_JSON", (0, index_js_1.currentWorldBookI18n)().service.invalidStructure);
    }
    const embeddedCharacterBook = isRecord(raw.character_book)
        ? parseCharacterBook(raw.character_book)
        : null;
    if (embeddedCharacterBook) {
        return embeddedCharacterBook;
    }
    const wrappedOriginalData = isRecord(raw.originalData) ? raw.originalData : null;
    if (wrappedOriginalData) {
        const parsedOriginalCharacterBook = parseCharacterBook(wrappedOriginalData);
        if (parsedOriginalCharacterBook) {
            return parsedOriginalCharacterBook;
        }
        const parsedOriginalWorldBook = parseSillyTavernWorldBook(wrappedOriginalData);
        if (parsedOriginalWorldBook) {
            return parsedOriginalWorldBook;
        }
    }
    const parsedWorldBook = parseSillyTavernWorldBook(raw);
    if (parsedWorldBook) {
        return parsedWorldBook;
    }
    const parsedCharacterBook = parseCharacterBook(raw);
    if (parsedCharacterBook) {
        return parsedCharacterBook;
    }
    if (Array.isArray(raw.entries)) {
        const parsedEntries = parseCharacterBook(raw);
        if (parsedEntries) {
            return parsedEntries;
        }
    }
    throw worldBookError("UNSUPPORTED_WORLD_BOOK_FORMAT", (0, index_js_1.currentWorldBookI18n)().service.unsupportedFormat);
}
async function loadEntries() {
    return await (0, worldbook_storage_js_1.readWorldBookEntries)();
}
async function saveEntries(entries) {
    await (0, worldbook_storage_js_1.writeWorldBookEntries)(entries);
}
function requireEntryId(id) {
    const normalizedId = String(id || "").trim();
    if (!normalizedId) {
        throw worldBookError("INVALID_ENTRY_ID", (0, index_js_1.currentWorldBookI18n)().service.entryIdRequired);
    }
    return normalizedId;
}
function findEntryIndex(entries, id) {
    const targetId = requireEntryId(id);
    const index = entries.findIndex((entry) => entry.id === targetId);
    if (index === -1) {
        throw worldBookError("ENTRY_NOT_FOUND", (0, index_js_1.currentWorldBookI18n)().service.entryNotFound(targetId));
    }
    return index;
}
function toWorldBookListEntry(entry) {
    return {
        id: entry.id,
        name: entry.name,
        enabled: entry.enabled,
        always_active: entry.always_active,
        priority: entry.priority,
        keywords: entry.keywords || [],
        is_regex: entry.is_regex || false,
        scan_depth: entry.scan_depth ?? 0,
        inject_target: entry.inject_target || "system",
        inject_position: normalizeInjectPosition(entry.inject_position),
        insertion_depth: normalizeNumber(entry.insertion_depth, 0),
        character_card_id: entry.character_card_id || ""
    };
}
async function listWorldBookEntries() {
    const entries = await loadEntries();
    return entries
        .map(toWorldBookListEntry)
        .sort((left, right) => right.priority - left.priority);
}
async function getWorldBookEntry(id) {
    const entries = await loadEntries();
    return entries[findEntryIndex(entries, id)];
}
async function createWorldBookEntry(params) {
    const entries = await loadEntries();
    const now = new Date().toISOString();
    const injectTarget = normalizeInjectTarget(params.inject_target);
    const injectPosition = injectTarget === "assistant" ? "at_depth" : normalizeInjectPosition(params.inject_position);
    const entry = {
        id: generateId(),
        name: String(params.name || ""),
        content: String(params.content || ""),
        keywords: splitKeywords(params.keywords),
        is_regex: params.is_regex === true,
        case_sensitive: params.case_sensitive === true,
        always_active: params.always_active === true,
        enabled: params.enabled !== false,
        priority: normalizeNumber(params.priority, 50),
        scan_depth: normalizeNumber(params.scan_depth, 0),
        inject_target: injectTarget,
        inject_position: injectPosition,
        insertion_depth: normalizeNumber(params.insertion_depth, 0),
        character_card_id: String(params.character_card_id || "").trim(),
        created_at: now,
        updated_at: now
    };
    entries.push(entry);
    await saveEntries(entries);
    return entry;
}
async function importWorldBookEntries(params) {
    const rawContent = String(params.content || "").trim();
    let sourceContent = rawContent;
    if (!sourceContent) {
        const path = String(params.path || "").trim();
        if (!path) {
            throw worldBookError("IMPORT_PATH_REQUIRED", (0, index_js_1.currentWorldBookI18n)().service.importPathRequired);
        }
        const fileResult = await Tools.Files.read(path);
        sourceContent = String(fileResult?.content || "").trim();
        if (!sourceContent) {
            throw worldBookError("IMPORT_FILE_EMPTY", (0, index_js_1.currentWorldBookI18n)().service.importEmpty);
        }
    }
    let parsedJson;
    try {
        parsedJson = JSON.parse(sourceContent);
    }
    catch (error) {
        throw worldBookError("INVALID_JSON", (0, index_js_1.currentWorldBookI18n)().service.importJsonInvalid(error.message));
    }
    const parsedPayload = parseImportedWorldBookPayload(parsedJson);
    if (parsedPayload.entries.length === 0) {
        throw worldBookError("NO_IMPORTABLE_ENTRIES", (0, index_js_1.currentWorldBookI18n)().service.noEntries);
    }
    const entries = await loadEntries();
    const now = new Date().toISOString();
    const characterCardId = String(params.character_card_id || "").trim();
    const importedEntries = parsedPayload.entries.map((entry) => buildImportedEntry(entry, characterCardId, now));
    entries.push(...importedEntries);
    await saveEntries(entries);
    return {
        source_type: parsedPayload.sourceType,
        imported_count: importedEntries.length,
        warning_count: parsedPayload.warnings.length,
        warnings: parsedPayload.warnings,
        entries: importedEntries
    };
}
async function updateWorldBookEntry(params) {
    const entries = await loadEntries();
    const index = findEntryIndex(entries, params.id);
    const nextEntry = { ...entries[index] };
    if (params.name != null) {
        nextEntry.name = String(params.name);
    }
    if (params.content != null) {
        nextEntry.content = String(params.content);
    }
    if (params.keywords != null) {
        nextEntry.keywords = splitKeywords(params.keywords);
    }
    if (params.is_regex != null) {
        nextEntry.is_regex = params.is_regex === true;
    }
    if (params.case_sensitive != null) {
        nextEntry.case_sensitive = params.case_sensitive === true;
    }
    if (params.always_active != null) {
        nextEntry.always_active = params.always_active === true;
    }
    if (params.enabled != null) {
        nextEntry.enabled = params.enabled !== false;
    }
    if (params.priority != null) {
        nextEntry.priority = normalizeNumber(params.priority, nextEntry.priority);
    }
    if (params.scan_depth != null) {
        nextEntry.scan_depth = normalizeNumber(params.scan_depth, nextEntry.scan_depth);
    }
    if (params.inject_target != null) {
        nextEntry.inject_target = normalizeInjectTarget(params.inject_target);
    }
    if (params.inject_position != null) {
        nextEntry.inject_position = normalizeInjectPosition(params.inject_position);
    }
    if (params.insertion_depth != null) {
        nextEntry.insertion_depth = normalizeNumber(params.insertion_depth, nextEntry.insertion_depth ?? 0);
    }
    if (nextEntry.inject_target === "assistant") {
        nextEntry.inject_position = "at_depth";
    }
    if (params.character_card_id != null) {
        nextEntry.character_card_id = String(params.character_card_id || "").trim();
    }
    nextEntry.updated_at = new Date().toISOString();
    entries[index] = nextEntry;
    await saveEntries(entries);
    return nextEntry;
}
async function deleteWorldBookEntry(id) {
    const entries = await loadEntries();
    const index = findEntryIndex(entries, id);
    const [removed] = entries.splice(index, 1);
    await saveEntries(entries);
    // Clean up group references
    const groups = await (0, worldbook_storage_js_1.readWorldBookGroups)();
    let groupsDirty = false;
    for (const group of groups) {
        const idx = group.entry_ids.indexOf(id);
        if (idx !== -1) {
            group.entry_ids.splice(idx, 1);
            groupsDirty = true;
        }
    }
    if (groupsDirty) {
        await (0, worldbook_storage_js_1.writeWorldBookGroups)(groups);
    }
    return removed;
}
async function toggleWorldBookEntry(id) {
    const entries = await loadEntries();
    const index = findEntryIndex(entries, id);
    const nextEntry = {
        ...entries[index],
        enabled: !entries[index].enabled,
        updated_at: new Date().toISOString()
    };
    entries[index] = nextEntry;
    await saveEntries(entries);
    return toWorldBookListEntry(nextEntry);
}
async function listWorldBookCharacterCards() {
    const cards = await ToolPkg.callDependency("com.operit.character_cards", "character.list", {});
    if (!Array.isArray(cards))
        throw new Error((0, index_js_1.currentWorldBookI18n)().service.characterDirectoryInvalid);
    return cards
        .map((card) => ({
        id: String(card?.id || "").trim(),
        name: String(card?.name || "").trim(),
        description: String(card?.description || "").trim(),
        isDefault: card?.isDefault === true
    }))
        .filter((card) => !!card.id);
}
function generateGroupId() {
    return `wg_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
}
function normalizeWorldBookGroupName(name) {
    const normalized = name.trim();
    if (normalized.length === 0) {
        throw worldBookError("GROUP_NAME_REQUIRED", (0, index_js_1.currentWorldBookI18n)().service.groupNameRequired);
    }
    return normalized;
}
async function listWorldBookGroups() {
    const groups = await (0, worldbook_storage_js_1.readWorldBookGroups)();
    return groups.sort((a, b) => a.name.localeCompare(b.name));
}
async function createWorldBookGroup(params) {
    const groups = await (0, worldbook_storage_js_1.readWorldBookGroups)();
    const now = new Date().toISOString();
    const group = {
        id: generateGroupId(),
        name: normalizeWorldBookGroupName(params.name),
        entry_ids: params.entry_ids,
        character_card_ids: params.character_card_ids,
        created_at: now,
        updated_at: now
    };
    groups.push(group);
    await (0, worldbook_storage_js_1.writeWorldBookGroups)(groups);
    return group;
}
async function deleteWorldBookGroup(id, deleteEntries) {
    const groups = await (0, worldbook_storage_js_1.readWorldBookGroups)();
    const index = groups.findIndex((g) => g.id === id);
    if (index === -1)
        throw worldBookError("GROUP_NOT_FOUND", (0, index_js_1.currentWorldBookI18n)().service.groupNotFound);
    const [removedGroup] = groups.splice(index, 1);
    await (0, worldbook_storage_js_1.writeWorldBookGroups)(groups);
    let removedEntries = [];
    if (deleteEntries) {
        const entries = await loadEntries();
        const remaining = [];
        for (const entry of entries) {
            if (removedGroup.entry_ids.includes(entry.id)) {
                removedEntries.push(entry);
            }
            else {
                remaining.push(entry);
            }
        }
        await saveEntries(remaining);
    }
    return { removedGroup, removedEntries };
}
async function copyEntryToGroup(entryId, targetGroupId) {
    const entries = await loadEntries();
    const sourceIndex = findEntryIndex(entries, entryId);
    const originalEntry = entries[sourceIndex];
    const now = new Date().toISOString();
    const newEntry = {
        ...originalEntry,
        id: `wb_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`,
        name: (0, index_js_1.currentWorldBookI18n)().service.copyName(originalEntry.name),
        created_at: now,
        updated_at: now
    };
    entries.push(newEntry);
    await saveEntries(entries);
    if (targetGroupId) {
        const groups = await (0, worldbook_storage_js_1.readWorldBookGroups)();
        const targetIndex = groups.findIndex((g) => g.id === targetGroupId);
        if (targetIndex !== -1) {
            groups[targetIndex].entry_ids.push(newEntry.id);
            groups[targetIndex].updated_at = now;
            await (0, worldbook_storage_js_1.writeWorldBookGroups)(groups);
        }
    }
    return { newEntry };
}
async function updateWorldBookGroup(params) {
    const groups = await (0, worldbook_storage_js_1.readWorldBookGroups)();
    const index = groups.findIndex((g) => g.id === params.id);
    if (index === -1)
        throw worldBookError("GROUP_NOT_FOUND", (0, index_js_1.currentWorldBookI18n)().service.groupNotFound);
    const now = new Date().toISOString();
    if (params.name != null) {
        groups[index].name = normalizeWorldBookGroupName(params.name);
    }
    if (params.entry_ids != null) {
        groups[index].entry_ids = params.entry_ids;
    }
    if (params.character_card_ids != null) {
        groups[index].character_card_ids = params.character_card_ids;
    }
    groups[index].updated_at = now;
    await (0, worldbook_storage_js_1.writeWorldBookGroups)(groups);
    return groups[index];
}
