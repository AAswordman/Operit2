"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.initializeWorldBookStorage = initializeWorldBookStorage;
exports.readWorldBookEntries = readWorldBookEntries;
exports.writeWorldBookEntries = writeWorldBookEntries;
exports.readWorldBookGroups = readWorldBookGroups;
exports.writeWorldBookGroups = writeWorldBookGroups;
const LEGACY_WORLD_BOOK_DIR = "/sdcard/Download/Operit/worldbook";
const LEGACY_WORLD_BOOK_FILE = `${LEGACY_WORLD_BOOK_DIR}/entries.json`;
async function deleteLegacyWorldBookStorage() {
    try {
        const legacyFileExists = await Tools.Files.exists(LEGACY_WORLD_BOOK_FILE);
        if (legacyFileExists?.exists) {
            await Tools.Files.deleteFile(LEGACY_WORLD_BOOK_FILE);
        }
    }
    catch (_error) {
    }
    try {
        const legacyDirExists = await Tools.Files.exists(LEGACY_WORLD_BOOK_DIR);
        if (legacyDirExists?.exists) {
            await Tools.Files.deleteFile(LEGACY_WORLD_BOOK_DIR, true);
        }
    }
    catch (_error) {
    }
}
async function initializeStorage(worldBookDir) {
    const worldBookFile = `${worldBookDir}/entries.json`;
    const worldBookGroupsFile = `${worldBookDir}/groups.json`;
    const variablesFile = `${worldBookDir}/variables.json`;
    await Tools.Files.mkdir(worldBookDir, true);
    const currentFileExists = await Tools.Files.exists(worldBookFile);
    if (!currentFileExists?.exists) {
        const legacyFileExists = await Tools.Files.exists(LEGACY_WORLD_BOOK_FILE);
        if (legacyFileExists?.exists) {
            const legacyFile = await Tools.Files.read(LEGACY_WORLD_BOOK_FILE);
            const migratedContent = String(legacyFile?.content || "").trim() || "[]";
            await Tools.Files.write(worldBookFile, migratedContent, false);
        }
        else {
            await Tools.Files.write(worldBookFile, "[]", false);
        }
    }
    const groupFileExists = await Tools.Files.exists(worldBookGroupsFile);
    if (!groupFileExists?.exists) {
        await Tools.Files.write(worldBookGroupsFile, "[]", false);
    }
    const variablesExist = await Tools.Files.exists(variablesFile);
    if (!variablesExist?.exists) {
        await Tools.Files.write(variablesFile, JSON.stringify({
            global_variables: {}, character_variables: {}, chats: {}
        }), false);
    }
    await deleteLegacyWorldBookStorage();
    return { entriesFile: worldBookFile, groupsFile: worldBookGroupsFile, variablesFile };
}
// One initialization per retained plugin owner and storage root. A failed open is
// retained too: ordinary reads must not silently restart migration on every hook.
let storageRoot;
let initialization;
function initializeWorldBookStorage() {
    const root = ToolPkg.getConfigDir();
    if (!initialization || storageRoot !== root) {
        storageRoot = root;
        initialization = initializeStorage(root);
    }
    return initialization;
}
async function readWorldBookEntries() {
    const storage = await initializeWorldBookStorage();
    try {
        const fileResult = await Tools.Files.read(storage.entriesFile);
        if (!fileResult?.content) {
            return [];
        }
        const parsed = JSON.parse(fileResult.content);
        return Array.isArray(parsed) ? parsed : [];
    }
    catch (_error) {
        return [];
    }
}
async function writeWorldBookEntries(entries) {
    const storage = await initializeWorldBookStorage();
    await Tools.Files.write(storage.entriesFile, JSON.stringify(entries, null, 2));
}
async function readWorldBookGroups() {
    const storage = await initializeWorldBookStorage();
    const fileResult = await Tools.Files.read(storage.groupsFile);
    if (fileResult?.content == null) {
        throw new Error("世界书分组文件读取失败");
    }
    const parsed = JSON.parse(fileResult.content);
    if (!Array.isArray(parsed)) {
        throw new Error("世界书分组文件格式无效");
    }
    return parsed;
}
async function writeWorldBookGroups(groups) {
    const storage = await initializeWorldBookStorage();
    await Tools.Files.write(storage.groupsFile, JSON.stringify(groups, null, 2));
}
