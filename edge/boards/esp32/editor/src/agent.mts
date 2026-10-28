#!/usr/bin/env node
/* Dependency-free adapter for Agents on any host with Node.js and HTTP access. */
import {isRecord, errorMessage} from './layout/project-model.mts';

const base = process.env.OPERIT_UI_URL || 'http://127.0.0.1:8766';

interface ToolDefinition {
  name: string;
  description: string;
  inputSchema: {
    type: 'object';
    properties: Record<string, unknown>;
    required?: string[];
    additionalProperties: false;
  };
}

interface JsonRpcRequest {
  jsonrpc?: string;
  id?: string | number | null;
  method?: string;
  params?: {
    name?: string;
    arguments?: Record<string, unknown>;
  };
}

const definitions: ToolDefinition[] = [
  {
    name: 'ui_component_source',
    description:
      'Locate a component in the actual layout file: line range, JSON pointer, revision, excerpt, event binding and implementation source entrypoints. Use ID and re-read before coding.',
    inputSchema: {
      type: 'object',
      properties: {id: {type: 'string'}, revision: {type: 'string'}},
      required: ['id'],
      additionalProperties: false,
    },
  },
  {
    name: 'ui_read',
    description: 'Read shared UI JSON and revision. Read before editing.',
    inputSchema: {type: 'object', properties: {}, additionalProperties: false},
  },
  {
    name: 'ui_components',
    description: 'List self-drawn UI components, click/long-press route capabilities, support status and resource limits.',
    inputSchema: {type: 'object', properties: {}, additionalProperties: false},
  },
  {
    name: 'ui_patch',
    description: 'Apply layout operations with optimistic revision checking. Saves shared source and triggers preview + firmware build.',
    inputSchema: {
      type: 'object',
      properties: {revision: {type: 'string'}, operations: {type: 'array', items: {type: 'object'}}},
      required: ['revision', 'operations'],
      additionalProperties: false,
    },
  },
  {
    name: 'ui_validate',
    description: 'Validate a candidate document without changing files.',
    inputSchema: {
      type: 'object',
      properties: {document: {type: 'object'}},
      required: ['document'],
      additionalProperties: false,
    },
  },
  {
    name: 'ui_build_status',
    description: 'Read shared source and firmware build status.',
    inputSchema: {type: 'object', properties: {}, additionalProperties: false},
  },
  {
    name: 'ui_build',
    description: 'Build browser preview and ESP32 firmware. Never flashes hardware.',
    inputSchema: {type: 'object', properties: {}, additionalProperties: false},
  },
];

const toolRoutes: Record<string, [string, string]> = {
  ui_component_source: ['', 'GET'],
  ui_read: ['/api/layout', 'GET'],
  ui_components: ['/api/components', 'GET'],
  ui_patch: ['/api/layout', 'PATCH'],
  ui_validate: ['/api/layout/validate', 'POST'],
  ui_build_status: ['/api/build', 'GET'],
  ui_build: ['/api/build', 'POST'],
};

/** Invokes one editor HTTP tool and returns the parsed JSON body. */
async function invoke(name: string, args: Record<string, unknown> = {}): Promise<unknown> {
  if (name === 'ui_component_source') {
    const id = typeof args.id === 'string' ? args.id : '';
    const params = new URLSearchParams({id});
    if (typeof args.revision === 'string') params.set('revision', args.revision);
    toolRoutes.ui_component_source[0] = '/api/component-source?' + params;
  }
  const route = toolRoutes[name];
  if (!route) throw new Error('Unknown tool ' + name);
  const [path, method] = route;
  const res = await fetch(base + path, {
    method,
    headers: {'Content-Type': 'application/json'},
    ...(method === 'GET' ? {} : {body: JSON.stringify(args)}),
  });
  const data: unknown = await res.json();
  if (!res.ok) {
    const detail = isRecord(data) && data.error !== undefined ? data.error : data;
    throw new Error(`${res.status}: ${typeof detail === 'string' ? detail : JSON.stringify(data)}`);
  }
  return data;
}

if (process.argv.includes('--mcp')) {
  const {createInterface} = await import('node:readline');
  const lines = createInterface({input: process.stdin});
  for await (const line of lines) {
    let request: JsonRpcRequest | undefined;
    try {
      request = JSON.parse(line) as JsonRpcRequest;
      if (request.id === undefined) continue;
      let result: unknown;
      if (request.method === 'initialize') {
        result = {
          protocolVersion: '2024-11-05',
          capabilities: {tools: {}},
          serverInfo: {name: 'operit-shared-ui', version: '1.0.0'},
        };
      } else if (request.method === 'ping') {
        result = {};
      } else if (request.method === 'tools/list') {
        result = {tools: definitions};
      } else if (request.method === 'tools/call') {
        try {
          result = {
            content: [
              {
                type: 'text',
                text: JSON.stringify(await invoke(request.params?.name ?? '', request.params?.arguments ?? {})),
              },
            ],
          };
        } catch (error) {
          result = {isError: true, content: [{type: 'text', text: errorMessage(error)}]};
        }
      } else {
        process.stdout.write(
          JSON.stringify({jsonrpc: '2.0', id: request.id, error: {code: -32601, message: 'Unknown method'}}) + '\n',
        );
        continue;
      }
      process.stdout.write(JSON.stringify({jsonrpc: '2.0', id: request.id, result}) + '\n');
    } catch (error) {
      process.stdout.write(
        JSON.stringify({jsonrpc: '2.0', id: request?.id ?? null, error: {code: -32700, message: errorMessage(error)}}) +
          '\n',
      );
    }
  }
} else {
  try {
    let args: Record<string, unknown> = {};
    if (process.argv[3]) {
      const input = process.argv[3];
      args = JSON.parse(
        input.startsWith('@') ? await (await import('node:fs/promises')).readFile(input.slice(1), 'utf8') : input,
      ) as Record<string, unknown>;
    }
    console.log(JSON.stringify(await invoke(process.argv[2] || 'ui_read', args), null, 2));
  } catch (error) {
    console.error(errorMessage(error));
    process.exitCode = 1;
  }
}
