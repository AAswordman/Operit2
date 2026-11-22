// Planning forbids side effects: block every filesystem-mutating and terminal-execution
// tool. Read-only getters (get_terminal_info / get_terminal_session_screen) stay available.
const PLAN_MODE_BLOCKED_TOOL_NAMES = new Set([
  "apply_file",
  "create_file",
  "edit_file",
  "delete_file",
  "write_file",
  "write_file_binary",
  "make_directory",
  "move_file",
  "copy_file",
  "download_file",
  "zip_files",
  "unzip_files",
  "create_terminal_session",
  "execute_in_terminal_session",
  "execute_in_terminal_session_streaming",
  "execute_hidden_terminal_command",
  "execute_cli_command",
  "input_in_terminal_session",
  "close_terminal_session",
  // super_admin package equivalents of the terminal tools above.
  "terminal",
  "close",
  "input",
  // Host system-mutating and UI-automation tools.
  "install_app",
  "uninstall_app",
  "modify_system_setting",
  "write_environment_variable",
  "tap",
  "long_press",
  "swipe",
  "browser_navigate",
]);

/** Tests whether a tool name is blocked in planning mode. */
export function isPlanModeBlockedToolName(name: string): boolean {
  if (PLAN_MODE_BLOCKED_TOOL_NAMES.has(name)) {
    return true;
  }
  // Package and MCP tools are exposed as pkg:tool; match the bare tool name too.
  const separator = name.lastIndexOf(":");
  return separator >= 0 && PLAN_MODE_BLOCKED_TOOL_NAMES.has(name.slice(separator + 1));
}

/** Removes filesystem-mutating and terminal-execution tools while the chat is in planning mode. */
export function filterPlanModeTools<T extends { name: string }>(
  availableTools: T[]
): T[] {
  return availableTools.filter((tool) => !isPlanModeBlockedToolName(tool.name));
}