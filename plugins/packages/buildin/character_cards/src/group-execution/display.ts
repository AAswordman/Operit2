import type { ChatConfigurationDisplayResult, ChatConfigurationExecutionResult, ChatConfigurationRequest, ChatDisplayParticipant, CharacterRecord, GroupRecord } from "../api";
import { parseChatSelection } from "../chat-bindings";
import { decodeChatMarker } from "../chat-markers";
import type { ChatConfigurationService } from "../domain";

/** Accepts an explicit execution request only when the plugin actually authors an initial message. */
type InitialMessageExecution = (request: ChatConfigurationRequest, service: ChatConfigurationService) => Promise<ChatConfigurationExecutionResult>;

/** Projects only an actual persisted participant's identity, without resolving execution resources. */
function identity(card: CharacterRecord): ChatDisplayParticipant {
  if (card.id.trim().length === 0 || card.name.trim().length === 0) throw new Error("Display participant identity is empty");
  return { id: card.id, name: card.name, avatarUri: card.avatarUri };
}

/** Reads each genuine member in its saved order and rejects deleted or duplicate references. */
export async function orderedGroupParticipants(group: GroupRecord, service: ChatConfigurationService): Promise<ChatDisplayParticipant[]> {
  if (group.members.length === 0) throw new Error("Selected group has no participants: " + group.id);
  const members = [...group.members].sort(
    /** Uses the saved order without choosing an execution participant. */
    (left, right) => left.orderIndex - right.orderIndex,
  );
  const ids = new Set<string>(), participants: ChatDisplayParticipant[] = [];
  for (const member of members) {
    if (ids.has(member.characterCardId)) throw new Error("Duplicate group participant: " + member.characterCardId);
    ids.add(member.characterCardId);
    const card = await service.dispatchDomain("character.get", { id: member.characterCardId });
    if (card.id !== member.characterCardId) throw new Error("Group participant read returned a different identity");
    participants.push(identity(card));
  }
  return participants;
}

/** Resolves presentation without selecting a group executor or reading model, voice and memory catalogs. */
export async function resolveChatDisplay(
  request: ChatConfigurationRequest, service: ChatConfigurationService, executeInitialMessage: InitialMessageExecution,
): Promise<ChatConfigurationDisplayResult> {
  if (request.purpose !== "display" || request.participantId !== null || request.messageExtension !== null) throw new Error("Display configuration cannot request an execution participant or message snapshot");
  const marker = decodeChatMarker(request.chatExtension), selection = parseChatSelection(marker.selection);
  if (selection.kind === "group") {
    const group = await service.dispatchDomain("group.get", { id: selection.id });
    if (group.id !== selection.id || group.name.trim().length === 0) throw new Error("Display group identity is invalid");
    return { contextKey: marker.selection, identity: { title: group.name, avatarUri: null }, participants: await orderedGroupParticipants(group, service), initialMessages: [] };
  }
  const card = await service.dispatchDomain("character.get", { id: selection.id });
  if (card.id !== selection.id) throw new Error("Display character read returned a different identity");
  const result: ChatConfigurationDisplayResult = { contextKey: marker.selection, identity: { title: card.name, avatarUri: card.avatarUri }, participants: [identity(card)], initialMessages: [] };
  if (card.openingStatement.trim().length !== 0) {
    const execution = await executeInitialMessage({ ...request, purpose: "execution", participantId: card.id }, service);
    if (execution.profile.id !== card.id) throw new Error("Initial message author does not match its explicit participant");
    result.initialMessages.push({ content: card.openingStatement, displayName: execution.profile.name, messageExtension: execution.messageExtension });
  }
  return result;
}
