import { workspaceChats, chatWorkspace } from "./workspaces.js";
import type { Responses } from "./api.js";
import { feedbackItem, projectSummaries } from "./dashboard.js";
import { ChannelCoordinator } from "./channels.js";
import { CHANNELS } from "./dependencies/connector_hub/api.js";
import { AppRecovery, appMessage, isStatusQuestion } from "./app-recovery.js";
import { CaptureCoordinator } from "./capture.js";
import { MaintenanceEngine, type Agent, type Workers } from "./engine.js";
import { CHANNEL, required, type Request, type Ticket } from "./model.js";
import { Repository } from "./storage.js";
import { GitVerifier } from "./git-verifier.js";
import { inspect } from "./inspection.js";
import { prepareRetry } from "./retry-check.js";

interface AgentsResponse { timed_out: boolean; agents: Agent[] }
const workers: Workers = {
  async spawn(parentChatId, name, task) {
    return ToolPkg.callDependency("com.operit.subagents", "agents.request", { action: "spawn", parentChatId, name, task, participantId: null });
  },
  async cleanup(parentChatId, agentId) {
    // The completion event can precede native observer cleanup by a few ticks.
    let failure: unknown = null;
    for (let attempt = 0; attempt < 10; attempt++) {
      try {
        const result = await ToolPkg.callDependency<unknown, {closed: string[]; retained: string[]}>("com.operit.subagents", "agents.request", {action:"release_idle_terminals", parentChatId, agentId});
        if (!result || !Array.isArray(result.closed) || !Array.isArray(result.retained) ||
            !result.closed.every(id => typeof id === "string") || !result.retained.every(id => typeof id === "string")) throw new Error("Invalid terminal cleanup result");
        return result;
      } catch (error) {
        failure = error;
        if (attempt < 9) await new Promise<void>(resolve => setTimeout(resolve, 100));
      }
    }
    throw failure;
  },
  async safeToRetry(parentChatId, agentId) {
    return prepareRetry(parentChatId, agentId, {
      list: parent => ToolPkg.callDependency("com.operit.subagents", "agents.request", {action:"list", parentChatId:parent}),
      close: (parent, id) => ToolPkg.callDependency("com.operit.subagents", "agents.request", {action:"close_if_completed", parentChatId:parent, agentId:id}),
      status: id => Tools.Chat.agentStatus(id),
    });
  },
  async wait(parentChatId, agentId) {
    while (true) {
      const result = await ToolPkg.callDependency<unknown, AgentsResponse>("com.operit.subagents", "agents.request", { action: "wait", parentChatId, agentIds: [agentId], timeoutMs: 30000 });
      if (!result.timed_out) {
        if (!Array.isArray(result.agents) || result.agents.length !== 1) throw new Error("Subagent returned an invalid terminal result");
        return result.agents[0];
      }
    }
  },
};
const repository = new Repository();
const engine = new MaintenanceEngine(repository, workers, {
  async validateWorkspace(chatId, path) {
    if (await chatWorkspace(chatId) !== path) throw Error("聊天的工作区已更换，历史工单不会转移到新仓库。请在原工作区继续处理，或提交新反馈。");
  },
  now: () => Date.now(), id: () => `${Date.now().toString(36)}-${CryptoJS.binary.randomHex(16)}`,
  report: error => console.error("[project_maintenance]", error),
  git: new GitVerifier(command => Tools.System.terminal.hiddenExec(command, {executorKey: "project-maintenance-git", timeoutMs: 15000})),
});
const capture = new CaptureCoordinator(repository, engine, () => Date.now(), error => console.error("[project_maintenance.capture]", error));
const channels = new ChannelCoordinator({chats:workspaceChats,binding:chatId=>repository.chatBinding(chatId),
  resetBinding:(chatId,path)=>repository.bindChat(chatId,path,Date.now()),workspace: chatWorkspace, capture: (parentChatId, projectPath, message) => capture.capture(parentChatId, projectPath, message),
  notices: (channel, config) => repository.channelNotifications(channel, config.parentChatId, config.projectPath, config.conversationId, config.notifySince),
  status: scope => receive({action: "channel_status", ...scope})});
const recovery = new AppRecovery(repository, capture, error => console.error("[project_maintenance.app_recovery]", error), chatWorkspace, chatId=>channels.isBotChat(chatId));
let timer: number | null = null;
let recoveryTimer: number | null = null;
export async function receive(request: Request): Promise<Responses[Request["action"]]> {
  if (request === null || typeof request !== "object") throw new Error("Invalid maintenance request");
  switch (request.action) {
    case "prepare_channel_chat": return channels.prepare(request.channel,required(request.conversationKey,"conversationKey"));
    case "dashboard": {
      const [list, bindings, sources, reports, chats] = await Promise.all([engine.list(), repository.chatBindings(), repository.dashboardSources(), channels.status(), workspaceChats()]);
      const connectedBindings = bindings.filter(b => b.projectPath !== null).map(binding => ({...binding,
        projectPath: chats.find(chat => chat.id === binding.chatId)?.workspacePath ?? null}));
      return {observedAt: Date.now(), workspaceChats: chats, tickets: list, bindings: connectedBindings, channels: reports,
        feedbackTotal: sources.captures.length,
        feedback: sources.captures.sort((a,b) => b.receivedAt-a.receivedAt).slice(0, 300).map(feedbackItem),
        projects: projectSummaries(list, connectedBindings, reports)};
    }

    case "conversation_status": {
      const chatId = required(request.chatId, "chatId");
      const bound = await repository.chatProject(chatId) !== null;
      const relevant = (await engine.list()).filter(ticket => ticket.parentChatId === chatId);
      const counts: Record<string, number> = {};
      for (const ticket of relevant) counts[ticket.stage] = (counts[ticket.stage] ?? 0) + 1;
      return { bound, total: relevant.length, counts, tickets: relevant.slice(0, 5).map(ticket => ({id: ticket.id, stage: ticket.stage, updatedAt: ticket.updatedAt,
        gitVerified: ticket.notes.some(note => note.kind === "delivery_verification"), verifiedPullRequest: verifiedPullRequest(ticket)})) };
    }
    case "channel_status": {
      if (!CHANNELS.includes(request.channel)) throw new Error("Invalid channel");
      const scope = {channel: request.channel, parentChatId: required(request.parentChatId, "parentChatId"), projectPath: required(request.projectPath, "projectPath"),
        accountId: required(request.accountId, "accountId"), conversationId: required(request.conversationId, "conversationId"), authorId: required(request.authorId, "authorId")};
      const source = await repository.channelStatus(scope), ids = new Set(source.ids);
      const relevant = (await engine.list()).filter(ticket => ids.has(ticket.id) && ticket.parentChatId === scope.parentChatId && ticket.projectPath === scope.projectPath && ticket.source === scope.channel);
      const counts: Record<string, number> = {};
      for (const ticket of relevant) counts[ticket.stage] = (counts[ticket.stage] ?? 0) + 1;
      return {pending: source.pending, total: relevant.length, counts, tickets: relevant.slice(0, 5).map(ticket => ({id:ticket.id, stage:ticket.stage, updatedAt:ticket.updatedAt}))};
    }
    case "available_chats": {
      return workspaceChats();
    }
    case "list_bindings": {
      const chats = await workspaceChats();
      return (await repository.chatBindings()).filter(binding => binding.projectPath !== null).map(binding => ({...binding,
        projectPath: chats.find(chat => chat.id === binding.chatId)?.workspacePath ?? null}));
    }
    case "bind_chat": {
      const chatId = required(request.chatId, "chatId");
      if (typeof request.enabled !== "boolean") throw Error("enabled must be boolean");
      await repository.bindChat(chatId, request.enabled ? await chatWorkspace(chatId) : null, Date.now());
      await channels.sync();
      return { chatId, projectPath: await repository.chatProject(chatId) };
    }
    case "app_message": {
      const chatId = required(request.chatId, "chatId");
      if (!Number.isSafeInteger(request.timestamp) || request.timestamp < 0 || typeof request.text !== "string") throw new Error("Invalid persisted chat message");
      if (await repository.chatProject(chatId) === null || !request.text.trim() || isStatusQuestion(request.text)) return { saved: false };
      if(await channels.isBotChat(chatId))return {saved:false};
      const projectPath = await chatWorkspace(chatId);
      if (await repository.chatProject(chatId) !== projectPath) await repository.bindChat(chatId, projectPath, Date.now());
      return capture.capture(chatId, projectPath, appMessage(chatId, request.timestamp, request.text));
    }
    case "channel_notifications": return repository.channelNotifications(request.channel, required(request.parentChatId, "parentChatId"), required(request.projectPath, "projectPath"), required(request.conversationId, "conversationId"), request.since);
    case "capture": return capture.capture(request.parentChatId, await chatWorkspace(required(request.parentChatId, "parentChatId")), request.message);
    case "create": return engine.create(request.parentChatId, await chatWorkspace(required(request.parentChatId, "parentChatId")), request.source, request.feedback);
    case "create_from_binding": {
      const chatId = required(request.chatId, "chatId");
      if (await repository.chatProject(chatId) === null) throw new Error("反馈入口已停用，请先启用该工作区聊天的反馈收集。");
      return engine.create(chatId, await chatWorkspace(chatId), "panel", request.feedback);
    }
    case "list": return engine.list();
    case "get": return engine.get(required(request.id, "id"));
    case "inspect": return inspect(await engine.get(required(request.id, "id")), {
      claim: path => repository.projectClaim(path),
      list: parentChatId => ToolPkg.callDependency("com.operit.subagents", "agents.request", {action: "list", parentChatId}),
      status: chatId => Tools.Chat.agentStatus(chatId), now: () => Date.now(),
    });
    case "start": return engine.start(required(request.id, "id"));
    case "add_note": return engine.addNote(required(request.id, "id"), request.text);
    default: throw new Error("Unknown maintenance request");
  }
}
/** Only host verification evidence, not the delivery agent's report, can establish a PR. */
function verifiedPullRequest(ticket: Ticket): string | null {
  if (ticket.stage !== "completed") return null;
  const note = [...ticket.notes].reverse().find(item => item.kind === "delivery_verification");
  try {
    const pr = note === undefined ? null : JSON.parse(note.text).pullRequest;
    return pr && pr.state === "OPEN" && typeof pr.url === "string" ? pr.url : null;
  } catch { return null; }
}
export function registerService(): void {
  channels.start();
  ToolPkg.ipc.on<Request, unknown>(CHANNEL, receive);
  if (timer === null) {
    const run = () => { void capture.tick().catch(error => console.error("[project_maintenance.capture]", error)); };
    timer = setInterval(run, 3000); run();
  }
  if (recoveryTimer === null) {
    const run = () => { void recovery.run().catch(error => console.error("[project_maintenance.app_recovery]", error)); };
    recoveryTimer = setInterval(run, 30000); run();
  }
}

export function channelConversationStatus(chatId: string): Promise<unknown | null> { return channels.conversationStatus(chatId); }
