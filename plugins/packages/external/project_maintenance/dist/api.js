"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.request = request;
/** Calls the provider's explicit public method with an inferred response type. */
function request(payload) {
    return ToolPkg.callDependency("com.operit.project_maintenance", "maintenance.request", payload);
}
