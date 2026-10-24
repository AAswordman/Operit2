import {
  AbilityAware, AbilityPluginBinding, FlutterPlugin, FlutterPluginBinding,
  MethodChannel, MethodCallHandler, MethodCall, MethodResult
} from '@ohos/flutter_ohos';
import common from '@ohos.app.ability.common';
import deviceInfo from '@ohos.deviceInfo';
import backgroundTaskManager from '@ohos.resourceschedule.backgroundTaskManager';
import wantAgent, { WantAgent } from '@ohos.app.ability.wantAgent';
import notification from '@ohos.notification';
import notificationManager from '@ohos.notificationManager';

/** Defines display fields without depending on application chat objects. */
interface WorkPresentationItem {
  workId: string; activation: Record<string, string>; title: string; subtitle: string; elapsedText: string; status: string;
  canStop: boolean; openLabel: string; stopLabel: string;
  startedAtMillis: number; endedAtMillis: number | null;
}

/** Carries one complete display revision from the shared Flutter protocol. */
interface WorkPresentationPayload {
  revision: number; summary: string; activeCount: number; waitingCount: number; items: WorkPresentationItem[];
}

/** Carries an exact action through a notification Want. */
interface WorkActivation { workId: string; activation: Record<string, string>; action: string; }

/** Owns computing-task residency on supported OHOS devices through the common protocol. */
export default class OperitKeepAlivePlugin implements FlutterPlugin, AbilityAware, MethodCallHandler {
  private channel: MethodChannel | null = null;
  private context: common.UIAbilityContext | null = null;
  private protectionContext: common.UIAbilityContext | null = null;
  private leases: Set<string> = new Set<string>();
  private operations: Promise<void> = Promise.resolve();
  private initialized: boolean = false;
  private workRevision: number = -1;
  private actionsReady: boolean = false;
  private static workOwner: OperitKeepAlivePlugin | null = null;
  private static pendingActions: WorkActivation[] = [];

  /** Queues cold-start and existing-ability actions without depending on Flutter readiness. */
  static receiveWorkActivation(encoded: string): void {
    const action = JSON.parse(encoded) as WorkActivation;
    if (typeof action.workId !== 'string' || action.workId.length === 0 ||
        typeof action.activation !== 'object' || action.activation === null ||
        (action.action !== 'open' && action.action !== 'stop')) throw new Error('Invalid work activation');
    OperitKeepAlivePlugin.pendingActions.push(action);
    OperitKeepAlivePlugin.workOwner?.flushWorkActions();
  }

  /** Delivers queued actions after the process-level application receiver is installed. */
  private flushWorkActions(): void {
    if (!this.actionsReady || this.channel === null) return;
    const actions = OperitKeepAlivePlugin.pendingActions;
    OperitKeepAlivePlugin.pendingActions = [];
    actions.forEach((action: WorkActivation): void => {
      const value = new Map<string, Object>();
      value.set('workId', action.workId);
      const activation = new Map<string, Object>();
      Object.keys(action.activation).forEach((key: string): void => { activation.set(key, action.activation[key]); });
      value.set('activation', activation);
      value.set('action', action.action);
      this.channel?.invokeMethod('workAction', value);
    });
  }

  /** Creates an explicit application activation for one work item and action. */
  private async workWant(work: WorkPresentationItem, action: string): Promise<WantAgent> {
    if (this.context === null) throw new Error('Work notification ability is unavailable');
    const activation: WorkActivation = { workId: work.workId, activation: work.activation, action: action };
    return wantAgent.getWantAgent({
      wants: [{ bundleName: this.context.abilityInfo.bundleName, abilityName: this.context.abilityInfo.name,
        parameters: { operitKeepAliveWorkAction: JSON.stringify(activation) } }],
      operationType: wantAgent.OperationType.START_ABILITY,
      requestCode: action === 'open' ? 24020 : 24021,
      wantAgentFlags: [wantAgent.WantAgentFlags.UPDATE_PRESENT_FLAG]
    });
  }

  /** Replaces the process work notification using actual lifecycle and application labels. */
  private async publishWork(call: MethodCall): Promise<void> {
    const payload = JSON.parse(this.string(call, 'json')) as WorkPresentationPayload;
    if (!Number.isInteger(payload.revision) || !Array.isArray(payload.items)) throw new Error('Invalid work presentation');
    if (payload.revision <= this.workRevision) return;
    if (payload.items.length === 0) {
      this.workRevision = payload.revision;
      return;
    }
    const work = payload.items[0];
    const open = await this.workWant(work, 'open');
    const buttons: notificationManager.NotificationActionButton[] = [{ title: work.openLabel, wantAgent: open }];
    if (work.canStop) buttons.push({ title: work.stopLabel, wantAgent: await this.workWant(work, 'stop') });
    const request: notificationManager.NotificationRequest = {
      id: 2402,
      slotType: notification.SlotType.SERVICE_INFORMATION,
      content: { contentType: notification.ContentType.NOTIFICATION_CONTENT_BASIC_TEXT,
        normal: { title: work.title, text: work.subtitle + " · " + work.elapsedText, additionalText: payload.summary } },
      wantAgent: open, actionButtons: buttons, isOngoing: work.endedAtMillis === null,
      isUnremovable: false, isAlertOnce: true,
      deliveryTime: work.endedAtMillis === null ? work.startedAtMillis : work.endedAtMillis
    };
    if (work.endedAtMillis !== null) request.autoDeletedTime = work.endedAtMillis + 60000;
    await notificationManager.publish(request);
    this.workRevision = payload.revision;
  }

  private running: boolean = false;
  private subscribed: boolean = false;
  private detail: string = 'OHOS computing residency requires a 2-in-1 device; ordinary mobile applications cannot request indefinite generic computation.';

  /** Publishes cancellation without restarting work rejected by the operating system. */
  private cancelled = (info: backgroundTaskManager.ContinuousTaskCancelInfo): void => {
    this.running = false;
    this.protectionContext = null;
    this.detail = `OHOS cancelled the computing task, reason=${info.reason}`;
    this.channel?.invokeMethod('protectionChanged', this.report());
  };

  /** Returns the exact plugin name used by generated registration. */
  getUniqueClassName(): string { return 'OperitKeepAlivePlugin'; }

  /** Registers transport without starting work for detached Flutter engines. */
  onAttachedToEngine(binding: FlutterPluginBinding): void {
    this.channel = new MethodChannel(binding.getBinaryMessenger(), 'operit/keep_alive');
    this.channel.setMethodCallHandler(this);
  }

  /** Supplies the actual application ability used by the notification return action. */
  onAttachedToAbility(binding: AbilityPluginBinding): void {
    this.context = binding.getAbility().context;
  }

  /** Releases the activity reference while preserving existing background ownership. */
  onDetachedFromAbility(): void { this.context = null; }

  /** Ends owned resources after pending method operations complete. */
  async onDetachedFromEngine(binding: FlutterPluginBinding): Promise<void> {
    this.channel?.setMethodCallHandler(null);
    await this.operations;
    if (this.subscribed) {
      backgroundTaskManager.off('continuousTaskCancel', this.cancelled);
      this.subscribed = false;
    }
    if (this.running && this.protectionContext !== null) {
      await backgroundTaskManager.stopBackgroundRunning(this.protectionContext);
      this.running = false;
      this.protectionContext = null;
    }
    if (OperitKeepAlivePlugin.workOwner === this) OperitKeepAlivePlugin.workOwner = null;
    this.actionsReady = false;
    this.channel = null;
  }

  /** Validates exact nonempty protocol fields. */
  private string(call: MethodCall, key: string): string {
    const value: string | null = call.argument(key);
    if (typeof value !== 'string' || value.trim().length === 0) {
      throw new Error(`Missing keep-alive argument: ${key}`);
    }
    return value;
  }

  /** Reports only protection actually granted by the operating system. */
  private report(): Map<string, Object> {
    const report = new Map<string, Object>();
    report.set('protection', this.running ? 'continuous' : 'unavailable');
    report.set('closeToTray', false);
    report.set('activeLeases', this.leases.size);
    report.set('detail', this.detail);
    return report;
  }

  /** Requests computing protection using the real ability and an explicit task category. */
  private async start(): Promise<void> {
    if (this.context === null) throw new Error('OHOS foreground ability is unavailable');
    const context = this.context;
    const agent = await wantAgent.getWantAgent({
      wants: [{ bundleName: context.abilityInfo.bundleName, abilityName: context.abilityInfo.name }],
      operationType: wantAgent.OperationType.START_ABILITY,
      requestCode: 2401,
      wantAgentFlags: [wantAgent.WantAgentFlags.UPDATE_PRESENT_FLAG]
    });
    await backgroundTaskManager.startBackgroundRunning(context,
      backgroundTaskManager.BackgroundMode.TASK_KEEPING, agent);
    this.protectionContext = context;
    this.running = true;
    this.detail = 'OHOS computing continuous task is active; cancellation by the user or system ends protection.';
  }

  /** Serializes native lease changes so first and final transitions cannot race. */
  onMethodCall(call: MethodCall, result: MethodResult): void {
    this.operations = this.operations.then(async (): Promise<void> => {
      try {
        switch (call.method) {
          case 'workCapabilities': {
            const capabilities = new Map<string, Object>();
            capabilities.set('updatesOwner', 'client');
            capabilities.set('surface', await notificationManager.isNotificationEnabled() ? 'notification' : 'none');
            capabilities.set('liveUpdates', false);
            result.success(capabilities);
            return;
          }
          case 'workActionsReady':
            this.actionsReady = true;
            this.flushWorkActions();
            result.success(null);
            return;
          case 'publishWork':
            await this.publishWork(call);
            result.success(null);
            return;
          case 'initialize':
            if (OperitKeepAlivePlugin.workOwner !== null && OperitKeepAlivePlugin.workOwner !== this) throw new Error('Another engine owns work presentation');
            OperitKeepAlivePlugin.workOwner = this;
            if (!this.initialized && deviceInfo.deviceType === '2in1') {
              backgroundTaskManager.on('continuousTaskCancel', this.cancelled);
              this.subscribed = true;
              this.detail = 'OHOS computing residency is available after an execution lease is acquired.';
            }
            this.initialized = true;
            break;
          case 'acquire': {
            if (!this.initialized) throw new Error('Keep-alive is not initialized');
            const id = this.string(call, 'id');
            this.string(call, 'reason');
            if (this.leases.has(id)) throw new Error('Duplicate keep-alive lease');
            if (deviceInfo.deviceType === '2in1' && !this.running) {
              if (this.leases.size > 0) throw new Error('OHOS cancelled protection for the active Core task');
              await this.start();
            }
            this.leases.add(id);
            break;
          }
          case 'release': {
            const id = this.string(call, 'id');
            if (!this.leases.has(id)) throw new Error('Unknown keep-alive lease');
            if (this.leases.size === 1 && this.running) {
              if (this.protectionContext === null) throw new Error('OHOS task ownership is unavailable');
              await backgroundTaskManager.stopBackgroundRunning(this.protectionContext);
              this.running = false;
              this.protectionContext = null;
              this.detail = 'The final OHOS execution lease has finished.';
            }
            this.leases.delete(id);
            break;
          }
          default: result.notImplemented(); return;
        }
        result.success(this.report());
      } catch (error) {
        const failure = error as Error;
        result.error('KEEP_ALIVE_ERROR', failure.message, null);
      }
    });
  }
}
