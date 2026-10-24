type LogPayloadValue = string | number | boolean | null | undefined;

/** Writes plan-plugin call diagnostics at the shared V3 depth. */
export function logPlanModeVerbose(message: string, payload?: Record<string, LogPayloadValue>): void {
  console.verbose(3, "[plan_mode] " + message + (payload === undefined ? "" : " " + JSON.stringify(payload)));
}

/** Keeps failed plan-plugin operations visible at Error severity. */
export function logPlanModeError(message: string, payload?: Record<string, LogPayloadValue>): void {
  console.error("[plan_mode] " + message + (payload === undefined ? "" : " " + JSON.stringify(payload)));
}
