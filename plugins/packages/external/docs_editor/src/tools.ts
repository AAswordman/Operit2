import { channel, natural, record, text, type Command } from "./protocol";

/* METADATA
{
    "api_version": "2.0.0",
  "name": "office",
  "enabled_by_default": true,
  "display_name": {"zh":"Office 文档","en":"Office documents"},
  "description": "Read and modify live DOCX, PPTX and PDF documents open in the current conversation's workspace. Start with list, then read. Edits are visible immediately and are not saved until save is called.",
  "tools": [
    {"name":"list","description":"List live Office editors and their document ids for this conversation.","parameters":[]},
    {"name":"read","description":"Inspect an open document. Returns its revision. DOCX sections: body (default), selection, headers, images, or image:<entry> (base64 chunks). PPTX sections: body (slides), selection, tools, or shapes:<zero-based slide index>. PDF: body (page count), selection, or page:<one-based page number>. Use offset for pagination.","parameters":[{"name":"id","type":"string","required":true,"description":"Identifier from office:list"},{"name":"section","type":"string","required":false,"description":"Section to read"},{"name":"offset","type":"number","required":false,"description":"Pagination offset"}]},
    {"name":"edit","description":"Update the open editor in place. Use the revision returned by read. DOCX: JSON {op:'replace',expected,text,occurrence?} or {op:'format',expected,occurrence?,bold?,italic?,underline?,align?}; expected is exact source text and occurrence is zero-based for duplicate matches. Apply alignment separately. PDF: JSON {op:'note',page,text,x?,y?} (page is one-based). PPTX: JSON {op:'pptx',name,input} with the schema from read section tools. Does not write the file.","parameters":[{"name":"id","type":"string","required":true,"description":"Document identifier"},{"name":"revision","type":"number","required":true,"description":"Exact revision from read"},{"name":"json","type":"string","required":true,"description":"One structured change as JSON"}]},
    {"name":"save","description":"Save current edits back to the original workspace file. Fails if it changed outside the editor.","parameters":[{"name":"id","type":"string","required":true,"description":"Document identifier"},{"name":"revision","type":"number","required":true,"description":"Exact revision from read or edit"}]}
  ]
}
*/

function execute(id: string, command: Command): Promise<unknown> {
  return ToolPkg.ipc.call(channel.registry, { kind: "command", chatId: getChatId() || null, id: text(id, "id", 128), command });
}

export function list(): Promise<unknown> {
  return ToolPkg.ipc.call(channel.registry, { kind: "list", chatId: getChatId() || null });
}

export function read(params: { id: string; section?: string; offset?: number }): Promise<unknown> {
  return execute(params.id, { kind: "read", section: text(params.section ?? "body", "section", 240), offset: natural(params.offset ?? 0, "offset") });
}

export function edit(params: { id: string; revision: number; json: string }): Promise<unknown> {
  const change = record(JSON.parse(text(params.json, "change", 48000)));
  return execute(params.id, { kind: "edit", revision: natural(params.revision, "revision"), change });
}

export function save(params: { id: string; revision: number }): Promise<unknown> {
  return execute(params.id, { kind: "save", revision: natural(params.revision, "revision") });
}
