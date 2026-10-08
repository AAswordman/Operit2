import { parseChatSelection } from "../chat-bindings";
import type { ChatConfigurationService } from "../domain";
import type { GroupTurnTransport } from "./contracts";
import { orderedGroupParticipants } from "./display";
import { GroupExecutionController } from "./executor";
import { planGroupResponse } from "./planner";

/** Uses the sole shared domain service and native-authored execution snapshots, without a second configuration-request host interface. */
export function createGroupExecutionController(service: ChatConfigurationService, transport: GroupTurnTransport): GroupExecutionController {
  return new GroupExecutionController({
    /** Reads only the actual conversation's owner-isolated namespace, never global selection or a file mirror. */
    async readSelection(chatId: string): Promise<string> {
      return (await service.dispatchDomain("chat.configuration.binding.read", { chatId })).selection;
    },
    /** Requires real saved group members in their explicit domain order before planning or executing. */
    async readParticipants(selection: string) {
      const parsed = parseChatSelection(selection);
      if (parsed.kind !== "group") throw new Error("Group planner requires a group selection");
      return orderedGroupParticipants(await service.dispatchDomain("group.get", { id: parsed.id }), service);
    },
    plan: planGroupResponse,
    transport,
  });
}
