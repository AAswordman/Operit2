"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.registerToolPkg = registerToolPkg;
exports.on_edge_event = on_edge_event;
exports.test_connection = test_connection;
exports.test_tool_call = test_tool_call;
const pet_1 = require("./pet");
ToolPkg.ipc.on('edge.pet', pet_1.command);
function registerToolPkg() { return true; }
/** One generic listener; this plugin's action handlers consume display.scene events. */
function on_edge_event(event) {
    return (0, pet_1.onEdge)(event.eventPayload);
}
/** Existing Core diagnostic exports: capability read only, no display lease. */
async function diagnostic() {
    const capabilities = await (0, pet_1.command)({ operation: 'test' });
    return { passed: capabilities.protocol === 1 && capabilities.events?.push === true,
        message: 'Edge scene v1 and event delivery capabilities' };
}
function test_connection() { return diagnostic(); }
function test_tool_call() { return diagnostic(); }
