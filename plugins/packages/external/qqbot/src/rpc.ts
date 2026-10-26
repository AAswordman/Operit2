import { CHANNEL, type Request, type Response } from "./model.js";

/** Routes every tool and UI command to the package main runtime. */
export function call(request: Request): Promise<Response> {
  return ToolPkg.ipc.call<Request, Response>(CHANNEL, request, { targetRuntime: "main" });
}
