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
export namespace edge {
  /**
   * Executes a declared action through authenticated node routing without adding application-level retries.
   */
  function execute(nodeId: string, interfaceInfo: EdgeInterfaceInfo, args?: any): Promise<EdgePortResultData>;
}

/**
 * Executes hardware I/O through authenticated node routing, not the host's local serial port.
 */
export namespace io {
  /**
   * Executes one supported port operation. Missing ports reject instead of pretending success.
   */
  function execute(nodeId: string, interfaceInfo: IoInterfaceInfo, args?: any): Promise<EdgePortResultData>;
}
