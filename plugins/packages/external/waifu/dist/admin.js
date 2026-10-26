"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.admin = admin;
exports.callAdmin = callAdmin;
const mode_js_1 = require("./mode.js");
const settings_js_1 = require("./settings.js");
let nextImage = 0;
/** Routes native authoring commands through the single provider runtime. */
async function admin(request) {
    switch (request.action) {
        case "get": return (0, settings_js_1.readSettings)(request.profile);
        case "set": return (0, settings_js_1.writeSettings)(request.profile, request.settings);
        case "mode.get": return (0, mode_js_1.modeContext)();
        case "mode.set": return (0, mode_js_1.saveMode)(request.selection, request.mode);
        case "emoji.previews": {
            const result = {};
            for (const [category, images] of Object.entries(request.emojis)) {
                result[category] = [];
                for (const source of images) {
                    const resource = /^resource:([a-z0-9_]+)$/.exec(source);
                    result[category].push(resource === null ? source : await ToolPkg.readResource(resource[1]));
                }
            }
            return result;
        }
        case "emoji.import": {
            const directory = `${ToolPkg.getSpaceDataDir()}/emojis`;
            await Tools.Files.mkdir(directory, true);
            const paths = [];
            for (const file of request.files) {
                const extension = /\.(png|jpe?g|gif|webp|avif)$/i.exec(file.name);
                if (extension === null)
                    throw new Error(`Unsupported emoji image: ${file.name}`);
                const destination = `${directory}/${Date.now()}-${++nextImage}.${extension[1].toLowerCase()}`;
                await Tools.Files.copy(file.path, destination);
                paths.push(destination);
            }
            return paths;
        }
    }
}
/** Calls the private main-runtime settings channel from tools and Compose screens. */
function callAdmin(request) {
    return ToolPkg.ipc.call("waifu.settings", request, { targetRuntime: "main" });
}
