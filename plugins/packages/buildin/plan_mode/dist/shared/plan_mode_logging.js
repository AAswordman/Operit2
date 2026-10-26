"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.logPlanModeVerbose = logPlanModeVerbose;
exports.logPlanModeError = logPlanModeError;
/** Writes plan-plugin call diagnostics at the shared V3 depth. */
function logPlanModeVerbose(message, payload) {
    console.verbose(3, "[plan_mode] " + message + (payload === undefined ? "" : " " + JSON.stringify(payload)));
}
/** Keeps failed plan-plugin operations visible at Error severity. */
function logPlanModeError(message, payload) {
    console.error("[plan_mode] " + message + (payload === undefined ? "" : " " + JSON.stringify(payload)));
}
