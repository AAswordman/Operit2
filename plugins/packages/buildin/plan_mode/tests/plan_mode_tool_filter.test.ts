import { test } from "node:test";
import assert from "node:assert/strict";

import {
  filterPlanModeTools,
  isPlanModeBlockedToolName,
} from "../src/shared/plan_mode_tool_filter.ts";

test("bare host side-effect tool names stay blocked", () => {
  for (const name of [
    "edit_file",
    "write_file",
    "zip_files",
    "execute_cli_command",
    "execute_hidden_terminal_command",
    "close_terminal_session",
  ]) {
    assert.equal(isPlanModeBlockedToolName(name), true, name);
  }
});

test("package-qualified side-effect tools are blocked", () => {
  for (const name of [
    "super_admin:terminal",
    "super_admin:close",
    "super_admin:input",
    "extended_file_tools:zip_files",
    "extended_file_tools:unzip_files",
    "any_plugin:edit_file",
    "mcp_server:write_file",
  ]) {
    assert.equal(isPlanModeBlockedToolName(name), true, name);
  }
});

test("omitted host side-effect tools are blocked", () => {
  for (const name of [
    "install_app",
    "uninstall_app",
    "modify_system_setting",
    "write_environment_variable",
    "tap",
    "long_press",
    "swipe",
    "browser_navigate",
  ]) {
    assert.equal(isPlanModeBlockedToolName(name), true, name);
  }
});

test("read-only tools remain available", () => {
  for (const name of [
    "read_file",
    "get_terminal_info",
    "get_terminal_session_screen",
    "super_admin:list_terminals",
    "super_admin:get_screen",
    "super_admin:terminal_wait",
    "browser_snapshot",
    "my_notes:render",
  ]) {
    assert.equal(isPlanModeBlockedToolName(name), false, name);
  }
});

test("filterPlanModeTools drops qualified and bare side-effect tools only", () => {
  const tools = [
    { name: "read_file" },
    { name: "super_admin:terminal" },
    { name: "super_admin:get_screen" },
    { name: "edit_file" },
    { name: "extended_file_tools:zip_files" },
    { name: "install_app" },
    { name: "my_notes:render" },
  ];
  assert.deepEqual(filterPlanModeTools(tools).map((tool) => tool.name), [
    "read_file",
    "super_admin:get_screen",
    "my_notes:render",
  ]);
});