import { createRequire } from "node:module";
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const require = createRequire(new URL("../../workflow/package.json", import.meta.url));
const ts = require("typescript");
const declarations = new Map();
const integerFields = new Set([
  "createdAt", "updatedAt", "lastAccessedAt", "orderIndex", "displayOrder", "chunkIndex",
  "triggerMessageTimestamp", "triggerVariantIndex", "attemptCount", "autoSaveIntervalMinutes",
  "nextAutoSaveRunAtMs", "totalChats", "completedChats", "totalWindows", "completedWindows",
  "totalSourceMessages", "processedSourceMessages", "failedWindows", "nextWindow",
  "sourceMessageCount", "timestamp", "variantIndex", "variantCount",
]);
for (const file of ["model.ts", "api.ts"]) {
  const source = ts.createSourceFile(file, readFileSync(new URL("../src/" + file, import.meta.url), "utf8"), ts.ScriptTarget.Latest, true);
  for (const node of source.statements) if (ts.isInterfaceDeclaration(node) || ts.isTypeAliasDeclaration(node)) declarations.set(node.name.text, node);
}

/** Resolves persisted domain types into explicit columns and ordered relation tables. */
function schema(node) {
  switch (node.kind) {
    case ts.SyntaxKind.StringKeyword: return { kind: "text" };
    case ts.SyntaxKind.NumberKeyword: return { kind: "number" };
    case ts.SyntaxKind.BooleanKeyword: return { kind: "boolean" };
  }
  if (ts.isLiteralTypeNode(node) && ts.isStringLiteral(node.literal)) return { kind: "text" };
  if (ts.isArrayTypeNode(node)) return { kind: "array", item: schema(node.elementType) };
  if (ts.isUnionTypeNode(node)) {
    const types = node.types.filter(type => !(ts.isLiteralTypeNode(type) && type.literal.kind === ts.SyntaxKind.NullKeyword));
    if (types.length !== node.types.length) {
      if (types.length !== 1) throw new Error("Unsupported nullable union: " + node.getText());
      return { ...schema(types[0]), nullable: true };
    }
    if (types.every(type => ts.isLiteralTypeNode(type) && ts.isStringLiteral(type.literal))) return { kind: "text" };
    throw new Error("Unsupported persisted union: " + node.getText());
  }
  if (ts.isTypeReferenceNode(node)) {
    const name = node.typeName.getText();
    if (name === "Record") return { kind: "map", item: schema(node.typeArguments[1]) };
    const definition = declarations.get(name);
    if (definition === undefined) throw new Error("Unknown persisted type: " + name);
    return ts.isInterfaceDeclaration(definition) ? object(definition.members) : schema(definition.type);
  }
  if (ts.isTypeLiteralNode(node)) return object(node.members);
  if (ts.isIndexedAccessTypeNode(node) && node.objectType.getText() === "CharacterRecord" && node.indexType.getText() === '"toolAccessConfig"') return named("ToolAccessConfig");
  throw new Error("Unsupported persisted type: " + node.getText());
}

/** Preserves every declared object field without introducing an opaque payload column. */
function object(members) {
  const fields = {};
  for (const member of members) {
    if (!ts.isPropertySignature(member) || member.type === undefined) throw new Error("Persisted fields must be typed properties");
    fields[member.name.getText()] = schema(member.type);
    if (fields[member.name.getText()].kind === "number" && integerFields.has(member.name.getText())) fields[member.name.getText()].kind = "integer";
    if (member.questionToken !== undefined) fields[member.name.getText()].optional = true;
  }
  return { kind: "object", fields };
}

/** Resolves one explicitly selected persisted domain interface. */
function named(name) {
  const definition = declarations.get(name);
  if (definition === undefined || !ts.isInterfaceDeclaration(definition)) throw new Error("Missing persisted interface: " + name);
  return object(definition.members);
}

const collections = {};
for (const [collection, name] of Object.entries({ cards: "Card", groups: "Group", tags: "Tag", stores: "Store", conversationGroups: "ConversationGroupRecord" })) collections[collection] = named(name);
collections.owners = named("MemorySpace");
for (const field of ["memories", "links", "chunks", "candidates", "embeddings"]) delete collections.owners.fields[field];
for (const [collection, name] of Object.entries({ memories: "Memory", links: "MemoryLink", chunks: "DocumentChunk", candidates: "MemoryCandidate", embeddings: "MemoryEmbedding" })) {
  collections[collection] = { kind: "object", fields: { ownerKey: { kind: "text" }, record: named(name) } };
}
collections.embeddings.fields.record.fields.vector = { kind: "vector" };
collections.meta = { kind: "object", fields: {
  version: { kind: "integer" }, nextId: { kind: "text" },
  active: { kind: "object", nullable: true, fields: {
    CharacterCard: { kind: "object", optional: true, fields: { id: { kind: "text" } } },
    CharacterGroup: { kind: "object", optional: true, fields: { id: { kind: "text" } } },
  } },
  memoryPreferences: { ...named("GlobalMemoryPreferences"), optional: true },
  migrationStatus: { kind: "text", optional: true }, migrationSource: { kind: "text", optional: true },
} };
collections.chat_bindings = { kind: "object", fields: { version: { kind: "integer" }, selection: { kind: "text" } } };
collections.chat_drafts = collections.chat_bindings;
collections.message_snapshots = { kind: "object", fields: {
  version: { kind: "integer" }, selection: { kind: "text" }, promptFunctionType: { kind: "text" }, primaryOwnerKey: { kind: "text" },
  profile: named("ChatParticipantProfile"), participants: { kind: "array", item: named("ChatParticipantProfile") },
} };
const target = new URL("../src/storage/schema.json", import.meta.url);
writeFileSync(target, JSON.stringify({ version: 1, collections }, null, 2) + "\n");
console.log("Generated relational storage schema: " + fileURLToPath(target));
