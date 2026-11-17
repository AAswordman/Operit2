"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.receive = receive;
const service_1 = require("./service");
let config;
const service = new service_1.StudioService({
    async read() { config = await PluginConfig.use("music-studio-v1", { database: null }); return config.database; },
    async write(value) { if (!config)
        throw new Error("Storage not initialized"); const before = config.database; config.database = value; try {
        await PluginConfig.flush(config);
    }
    catch (error) {
        config.database = before;
        try {
            await PluginConfig.flush(config);
        }
        catch { /* Preserve the original durable write error. */ }
        throw error;
    } },
});
function receive(request) { return service.request(request); }
