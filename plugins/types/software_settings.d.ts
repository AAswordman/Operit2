/**
 * Software settings type definitions for Assistance Package Tools
 */

import {
    EnvironmentVariableReadResultData,
    EnvironmentVariableWriteResultData
} from './results';

/**
 * Software settings operations namespace
 */
export namespace SoftwareSettings {
    /**
     * Read current value of an environment variable.
     * @param key - Environment variable key
     */
    function readEnvironmentVariable(key: string): Promise<EnvironmentVariableReadResultData>;

    /**
     * Write an environment variable; empty value clears the variable.
     * @param key - Environment variable key
     * @param value - Variable value (empty string clears)
     */
    function writeEnvironmentVariable(key: string, value?: string): Promise<EnvironmentVariableWriteResultData>;

    /**
     * Execute a core command with CLI-style arguments.
     * @param args - Command arguments, for example ['plugin', 'list']
     */
    function exec(args: string[]): Promise<string>;
}
