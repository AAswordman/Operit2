"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.CHANNEL = void 0;
exports.required = required;
exports.errorText = errorText;
exports.CHANNEL = "project_maintenance.service";
function required(value, name) {
    if (typeof value !== "string" || !value.trim())
        throw new Error(name + " must be non-empty text");
    return value.trim();
}
function errorText(error) { return error instanceof Error ? error.message : String(error); }
