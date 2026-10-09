// Generated from operit-plugin-sdk Rust declarations.

import type { EdgePortResultData } from "./results";

/**
 * Identifies one firmware-registered Edge plugin action, not a Core RPC target.
 */
export interface EdgeInterfaceInfo {
  /**
   * Stable Edge native plugin ID, as declared by the firmware.
   */
  pluginId: string;
  /**
   * Action explicitly declared by that native plugin.
   */
  action: string;
}

/**
 * Identifies an I/O port family and operation. Unknown ports fail explicitly.
 */
export interface IoInterfaceInfo {
  /**
   * Currently `gpio`; serial/UART ports are not implemented yet.
   */
  port: string;
  /**
   * Currently `read` or `write`; GPIO arguments carry `pin` and optionally `level`.
   */
  operation: string;
}

/**
 * Calls only firmware-declared native plugin actions on an explicit node.
 */
export class Edge {
  private constructor();
  /**
   * Executes a declared action through authenticated node routing without adding application-level retries.
   */
  static execute(nodeId: string, interfaceInfo: EdgeInterfaceInfo, args?: any): Promise<EdgePortResultData>;
}

/**
 * Executes hardware I/O through authenticated node routing, not the host's local serial port.
 */
export class Io {
  private constructor();
  /**
   * Executes one supported port operation. Missing ports reject instead of pretending success.
   */
  static execute(nodeId: string, interfaceInfo: IoInterfaceInfo, args?: any): Promise<EdgePortResultData>;
}

/**
 * One received device action. The action identifies business meaning, not a callable method.
 */
export interface EdgeActionEvent {
  /**
   * Monotonic producer sequence, safe as a JavaScript integer.
   */
  seq: number;
  /**
   * Examples: target.tap, gpio.changed, sensor.sample, serial.message.
   */
  action: string;
  /**
   * Producer-defined JSON object; validated by that action's handler.
   */
  data: object;
}

/**
 * Generic bounded event batch shared by all device event producers.
 */
export interface EdgeEventBatch {
  /**
   * Protocol version, currently 1.
   */
  v: number;
  /**
   * Firmware service that produced this batch.
   */
  source: string;
  /**
   * Producer stream identity; renewed after reset/re-subscription.
   */
  stream: string;
  /**
   * Up to four ordered action events.
   */
  events: EdgeActionEvent[];
  /**
   * Last sequence in this batch; acknowledge only after persistence.
   */
  next: number;
  /**
   * Last irrecoverably lost sequence; gaps are explicit.
   */
  lostBefore: number;
}

/**
 * Core attaches routing context to a generic Edge event batch.
 */
export interface EdgeEventPayload {
  /**
   * Existing authorized chat Binding, never implicitly created.
   */
  chatId: string;
  /**
   * Device identity authenticated against the route origin.
   */
  nodeId: string;
  /**
   * Device-independent action envelope.
   */
  batch: EdgeEventBatch;
}

/**
 * Argument of the fixed on_edge_event export in the existing ToolPkg main runtime.
 */
export interface EdgeEventHookEvent {
  /**
   * Dedicated generic event discriminator: edge_event.
   */
  event: string;
  /**
   * Repeats the event discriminator.
   */
  eventName: string;
  /**
   * Trusted route context and bounded device events.
   */
  eventPayload: EdgeEventPayload;
}

/**
 * Return after applying/persisting this batch; never acknowledge merely on reception.
 */
export interface EdgeEventAck {
  /**
   * False rejects this receiver/source/stream or unhandled action.
   */
  accepted: boolean;
  /**
   * Required for acceptance; must equal the delivered batch next.
   */
  next?: number;
}
