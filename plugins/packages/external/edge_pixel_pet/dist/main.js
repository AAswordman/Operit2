"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.registerToolPkg = registerToolPkg;
exports.test_connection = test_connection;
exports.test_tool_call = test_tool_call;
const pet_1 = require("./pet");
ToolPkg.ipc.on('edge.pet', pet_1.command);
function registerToolPkg() { return true; }
/** Configuration editor diagnostics only; the device owns runtime behavior. */
async function diagnostic() {
    const capabilities = await (0, pet_1.command)({ operation: 'inspect' });
    return { passed: capabilities.protocol === 1 && capabilities.input?.local === true,
        message: 'Native scene v1 with device-local interaction' };
}
function test_connection() { return diagnostic(); }
function test_tool_call() { return diagnostic(); }
