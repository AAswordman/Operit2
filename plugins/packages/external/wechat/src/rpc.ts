import { CHANNEL, type Request, type Response } from "./model.js";

/** Routes tools and Compose actions to the single package main runtime. */
export function call(request: Request): Promise<Response> {
  return ToolPkg.ipc.call<Request, Response>(CHANNEL, request, {
    targetRuntime: "main",
  });
}
