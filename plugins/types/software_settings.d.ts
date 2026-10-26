// Generated from operit-plugin-sdk Rust declarations.

import type { EnvironmentVariableReadResultData, EnvironmentVariableWriteResultData } from "./results";

/**
 * Manages environment variables and executes core commands.
 */
export namespace SoftwareSettings {
  /**
   * Execute a core command with CLI-style arguments.
   * @param args - Command arguments, for example ['plugin', 'list']
   */
  function exec(args: string[]): Promise<string>;
  /**
   * Read current value of an environment variable.
   * @param package_name - Installed package or subpackage name
   * @param key - Environment variable key
   */
  function readEnvironmentVariable(package_name: string, key: string): Promise<EnvironmentVariableReadResultData>;
  /**
   * Write an environment variable; empty value clears the variable.
   * @param package_name - Installed package or subpackage name
   * @param key - Environment variable key
   * @param value - Variable value (empty string clears)
   */
  function writeEnvironmentVariable(package_name: string, key: string, value?: string): Promise<EnvironmentVariableWriteResultData>;
}
