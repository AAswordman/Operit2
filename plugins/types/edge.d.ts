// Generated from operit-plugin-sdk Rust declarations.

import type { EdgePortResultData } from "./results";

/**
 * Identifies one native service endpoint on the selected Edge device.
 */
export interface EdgeInterfaceInfo {
  /**
   * Native device service target in the edge namespace.
   */
  target: string;
  /**
   * Native service method handled by the device application.
   */
  method: string;
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
 * Calls native device services on an explicit node.
 */
export interface DeviceEdge {
  /**
   * Executes a native service method through authenticated node routing without adding application-level retries.
   */
  execute(interfaceInfo: EdgeInterfaceInfo, args?: any): Promise<EdgePortResultData>;
}

/**
 * Executes hardware I/O through authenticated node routing, not the host's local serial port.
 */
export interface DeviceIo {
  /**
   * Executes one supported port operation. Missing ports reject instead of pretending success.
   */
  execute(interfaceInfo: IoInterfaceInfo, args?: any): Promise<EdgePortResultData>;
}

/**
 * Opens the two directions of one native protocol with the same session arguments.
 */
export interface DeviceTransportEndpoint {
  target: string;
  /**
   * ReverseStream method, carried by Push.
   */
  method: string;
  /**
   * Stream property, carried by Watch.
   */
  property: string;
  args?: any;
}

/**
 * Carries the original Watch event kind and its vendor-defined value.
 */
export interface DeviceTransportEvent {
  kind: string;
  /**
   * Structured values may contain Uint8Array byte payloads at any depth.
   */
  value: any;
}

/**
 * Represents a duplex channel; its receive direction is also an async iterable.
 */
export interface DeviceTransportChannel extends AsyncIterable<DeviceTransportEvent> {
  send(value: any): Promise<void>;
  /**
   * Completes ReverseStream input while keeping Stream output readable.
   */
  finish(): Promise<void>;
  /**
   * Closes both directions and wakes pending I/O.
   */
  close(): Promise<void>;
}

/**
 * Transports structured values and bytes without interpreting their business meaning.
 */
export interface DeviceTransport {
  call(target: string, method: string, args?: any): Promise<any>;
  open(endpoint: DeviceTransportEndpoint): Promise<DeviceTransportChannel>;
}
