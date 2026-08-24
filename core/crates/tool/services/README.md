# operit-tools

`operit-tools` owns the Operit2 tool system. Built-in tools, ToolPkg, MCP,
skill packages, permission metadata, tool result data, and package execution
surfaces live here.

## Responsibilities

- Register and execute built-in tools through `AIToolHandler`.
- Define tool result data, tool execution parsing, permission metadata, and
  tool access rules.
- Own ToolPkg package loading, package manager behavior, package hook models,
  and package resource helpers.
- Isolate every ToolPkg main-script registration in a temporary JavaScript engine so one package
  cannot retain registration state or block later package scans.
- Own MCP package models, MCP runtime repository code, MCP bridge helpers, and
  MCP tool execution.
- Own skill package models, skill manager behavior, and skill repository code.
- Define child-crate interfaces for runtime-owned services in
  `runtime_support.rs`.
- Define the JS execution trait used by ToolPkg without owning the concrete JS
  engine implementation.

## Main Modules

- `src/ToolExecutionManager.rs`: parses AI tool calls and coordinates tool
  access checks.
- `src/ConversationMarkupManager.rs`: parses and renders assistant/tool markup.
- `src/runtime_support.rs`: tool-side contract implemented by
  `operit-runtime`.
- `src/tools/AIToolHandler.rs`: tool registry and tool invocation path.
- `src/tools/ToolRegistration.rs`: built-in public and internal tool
  registration.
- `src/tools/defaultTool/`: standard file, HTTP, terminal, memory, browser,
  system, Bluetooth, music, and web visit tools.
- `src/tools/packTool/`: ToolPkg models, parsing, loading, manager, and hook
  support.
- `src/tools/mcp/` and `src/tools/mcp_runtime/`: MCP tool and runtime support.
- `src/tools/skill/` and `src/tools/skill_runtime/`: skill package support.
- `src/tools/javascript.rs`: `JsExecutionHost` support, package runtime adapters,
  and the parent-injected `JsExecutionProvider` binding.

## Runtime Support

When tool code needs runtime-owned behavior, the interface is declared in this
crate and implemented by `operit-runtime`. Examples include character-card tool
access, bundled assets, environment variables, skill visibility, MCP
description generation, chat operations, memory bindings, and structured file
edits.

## Boundary

MCP and skill are part of this crate. Provider APIs are not tools and belong in
`operit-providers`. The concrete JavaScript engine belongs in
`operit-js-bridge`. This crate implements the SDK `JsExecutionHost` contract;
the top-level runtime injects an SDK `JsExecutionProvider`.

See `core/CRATE_BOUNDARIES.md` for the full dependency direction.
