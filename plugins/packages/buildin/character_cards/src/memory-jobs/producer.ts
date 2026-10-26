import { assertObject, requireId } from "../validation";

/** Reads the actual executing CoreNode identity without inferring ownership from reachability. */
export async function memoryJobProducer(): Promise<string> {
  const state: unknown = JSON.parse(await toolCall("list_core_nodes", {}));
  assertObject(state, "CoreNode discovery");
  return requireId(state.currentNodeId, "current CoreNode id");
}
