"use strict";
var __defProp = Object.defineProperty;
var __getOwnPropDesc = Object.getOwnPropertyDescriptor;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, { get: all[name], enumerable: true });
};
var __copyProps = (to, from, except, desc) => {
  if (from && typeof from === "object" || typeof from === "function") {
    for (let key of __getOwnPropNames(from))
      if (!__hasOwnProp.call(to, key) && key !== except)
        __defProp(to, key, { get: () => from[key], enumerable: !(desc = __getOwnPropDesc(from, key)) || desc.enumerable });
  }
  return to;
};
var __toCommonJS = (mod) => __copyProps(__defProp({}, "__esModule", { value: true }), mod);
var __async = (__this, __arguments, generator) => {
  return new Promise((resolve, reject) => {
    var fulfilled = (value) => {
      try {
        step(generator.next(value));
      } catch (e) {
        reject(e);
      }
    };
    var rejected = (value) => {
      try {
        step(generator.throw(value));
      } catch (e) {
        reject(e);
      }
    };
    var step = (x) => x.done ? resolve(x.value) : Promise.resolve(x.value).then(fulfilled, rejected);
    step((generator = generator.apply(__this, __arguments)).next());
  });
};

// src/ui/workspace.ui.ts
var workspace_ui_exports = {};
__export(workspace_ui_exports, {
  default: () => Workspace
});
module.exports = __toCommonJS(workspace_ui_exports);

// src/model.ts
function decodeXml(value) {
  return value.replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&").replace(/&quot;/g, '"').replace(/&apos;/g, "'");
}
function metadata(value) {
  if (value === void 0 || value.length === 0) return null;
  const text = /^\s*\{\s*&quot;/.test(value) ? decodeXml(value) : value;
  const decoded = JSON.parse(text);
  return decoded !== null && typeof decoded === "object" && !Array.isArray(decoded) ? decoded : null;
}
function lineStats(diff) {
  let additions = 0;
  let deletions = 0;
  for (const line of diff.split("\n")) {
    if (line.startsWith("+") && !line.startsWith("+++")) additions++;
    if (line.startsWith("-") && !line.startsWith("---")) deletions++;
  }
  return { additions, deletions };
}
function envelopeDiffs(text) {
  const changes = [];
  for (const match of text.matchAll(/<file-diff\b([^>]*)>([\s\S]*?)<\/file-diff>/gi)) {
    const path = /\bpath="([^"]*)"/i.exec(match[1]);
    const cdata = /<!\[CDATA\[([\s\S]*?)\]\]>/i.exec(match[2]);
    changes.push({
      path: path === null ? "" : decodeXml(path[1]),
      diffContent: decodeXml((cdata === null ? match[2] : cdata[1]).trim())
    });
  }
  return changes;
}
function reviewForMessage(message) {
  const files = /* @__PURE__ */ new Map();
  for (const part of message.parts) {
    if (part.kind !== "tool_result") continue;
    const display = metadata(part.attributes.display);
    if (display === null || display.resultRenderer !== "fileDiff" || display.label === null || typeof display.label !== "object" || Array.isArray(display.label) || Object.values(display.label).some((value) => typeof value !== "string")) continue;
    let changes;
    if (part.attributes.result_data === void 0 || part.attributes.result_data === "") {
      changes = envelopeDiffs(part.content);
    } else {
      const data = metadata(part.attributes.result_data);
      const operation = data?.operation;
      if (operation === null || typeof operation !== "object" || Array.isArray(operation)) throw new Error("File diff requires a typed operation");
      const typed = operation;
      if (typeof typed.path !== "string" || typeof data?.diffContent !== "string" || typed.details !== void 0 && typed.details !== null && typeof typed.details !== "string") throw new Error("Invalid typed file diff");
      changes = data.diffContent.length === 0 ? [] : [{ path: typed.path, diffContent: data.diffContent }];
    }
    for (const file of changes) {
      const previous = files.get(file.path);
      files.set(file.path, previous === void 0 ? file.diffContent : previous + "\n" + file.diffContent);
    }
  }
  return [...files].map(([path, diffContent]) => ({ path, diffContent, ...lineStats(diffContent) }));
}
function fileIdentity(path) {
  const normalized = path.replace(/\\/g, "/");
  const index = normalized.lastIndexOf("/");
  return {
    name: index < 0 || index === normalized.length - 1 ? path : normalized.slice(index + 1),
    directory: index < 0 ? "" : normalized.slice(0, index + 1)
  };
}

// src/diff.ts
function changedRanges(oldText, newText) {
  const shared = Math.min(oldText.length, newText.length);
  let prefix = 0;
  while (prefix < shared && oldText[prefix] === newText[prefix]) prefix++;
  let suffix = 0;
  while (suffix < shared - prefix && oldText[oldText.length - 1 - suffix] === newText[newText.length - 1 - suffix]) suffix++;
  const oldEnd = oldText.length - suffix, newEnd = newText.length - suffix;
  return [oldEnd > prefix ? [{ start: prefix, end: oldEnd }] : [], newEnd > prefix ? [{ start: prefix, end: newEnd }] : []];
}
function emphasize(rows) {
  let index = 0;
  while (index < rows.length) {
    if (rows[index].kind !== "remove") {
      index++;
      continue;
    }
    const removed = index;
    while (index < rows.length && rows[index].kind === "remove") index++;
    const added = index;
    while (index < rows.length && rows[index].kind === "add") index++;
    for (let pair = 0; pair < Math.min(added - removed, index - added); pair++) {
      const [left, right] = changedRanges(rows[removed + pair].text, rows[added + pair].text);
      rows[removed + pair].segments = left;
      rows[added + pair].segments = right;
    }
  }
  return rows;
}
function visualWidth(text) {
  let width = 0;
  for (const character of text) width += character === "	" ? 4 : character.codePointAt(0) > 127 ? 2 : 1;
  return width;
}
function parseDiff(diff, allowImplicitHunks = false) {
  const hunks = [];
  let hunk = null;
  let oldLine = 1, newLine = 1;
  function flush() {
    if (hunk !== null && hunk.rows.length > 0) {
      hunk.rows = emphasize(hunk.rows);
      hunks.push(hunk);
    }
  }
  for (const raw of diff.split("\n")) {
    const line = raw.endsWith("\r") ? raw.slice(0, -1) : raw;
    const header = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@.*/.exec(line);
    if (header !== null) {
      flush();
      oldLine = Number(header[1]);
      newLine = Number(header[3]);
      hunk = {
        header: line,
        oldStart: oldLine,
        oldCount: header[2] === void 0 ? 1 : Number(header[2]),
        newStart: newLine,
        newCount: header[4] === void 0 ? 1 : Number(header[4]),
        rows: []
      };
      continue;
    }
    if (allowImplicitHunks && hunk === null && !line.startsWith("+++") && !line.startsWith("---") && (line.startsWith("+") || line.startsWith("-"))) {
      hunk = { header: "@@", oldStart: 1, oldCount: 0, newStart: 1, newCount: 0, rows: [] };
      oldLine = 1;
      newLine = 1;
    }
    if (hunk === null || line.startsWith("\\")) continue;
    if (line.startsWith("+")) {
      hunk.rows.push({ lineNumber: newLine, newLineNumber: newLine++, oldLineNumber: null, text: line.slice(1), kind: "add", segments: [] });
    } else if (line.startsWith("-")) {
      hunk.rows.push({ lineNumber: oldLine, oldLineNumber: oldLine++, newLineNumber: null, text: line.slice(1), kind: "remove", segments: [] });
    } else if (line.startsWith(" ")) {
      hunk.rows.push({ lineNumber: newLine, oldLineNumber: oldLine++, newLineNumber: newLine++, text: line.slice(1), kind: "context", segments: [] });
    }
  }
  flush();
  const rows = hunks.flatMap((item) => item.rows);
  return {
    hunks,
    additions: rows.filter((row) => row.kind === "add").length,
    deletions: rows.filter((row) => row.kind === "remove").length,
    maxLineLength: rows.reduce((maximum, row) => Math.max(maximum, visualWidth(row.text)), 0)
  };
}
function gapRows(lines, oldStart, newStart, count) {
  if (newStart < 1 || newStart + count - 1 > lines.length) return null;
  return Array.from({ length: count }, (_, offset) => ({
    lineNumber: newStart + offset,
    oldLineNumber: oldStart + offset,
    newLineNumber: newStart + offset,
    text: lines[newStart + offset - 1],
    kind: "context",
    segments: []
  }));
}
function diffItems(document, split, fileLines, expandedGaps) {
  const items = [];
  let oldEnd = 1, newEnd = 1, first = true;
  for (const hunk of document.hunks) {
    const count = first ? hunk.oldStart - 1 : hunk.oldStart - oldEnd;
    const oldStartLine = first ? 1 : oldEnd, newStartLine = first ? 1 : newEnd;
    if (count > 0) {
      const resolved = fileLines !== null && (expandedGaps === null || expandedGaps.has(newStartLine)) ? gapRows(fileLines, oldStartLine, newStartLine, count) : null;
      if (resolved !== null && resolved.length > 0) {
        for (const row of resolved) items.push(split ? { type: "pair", left: row, right: row } : { type: "line", row });
      } else {
        items.push({ type: "gap", count, oldStartLine, newStartLine, expandable: fileLines !== null });
      }
    }
    first = false;
    oldEnd = hunk.oldStart + hunk.oldCount;
    newEnd = hunk.newStart + hunk.newCount;
    if (!split) {
      if (hunk.header !== "@@") items.push({ type: "hunk", header: hunk.header });
      for (const row of hunk.rows) items.push({ type: "line", row });
      continue;
    }
    let index = 0;
    while (index < hunk.rows.length) {
      const row = hunk.rows[index];
      if (row.kind === "context") {
        items.push({ type: "pair", left: row, right: row });
        index++;
        continue;
      }
      const removed = index;
      while (index < hunk.rows.length && hunk.rows[index].kind === "remove") index++;
      const added = index;
      while (index < hunk.rows.length && hunk.rows[index].kind === "add") index++;
      for (let pair = 0; pair < Math.max(added - removed, index - added); pair++) {
        items.push({
          type: "pair",
          left: pair < added - removed ? hunk.rows[removed + pair] : null,
          right: pair < index - added ? hunk.rows[added + pair] : null
        });
      }
    }
  }
  return items;
}

// src/git.ts
function runCommand(workspace, program, args) {
  return __async(this, null, function* () {
    return JSON.parse(yield Tools.SoftwareSettings.exec(["--json", "workspace", "exec", workspace, program, ...args]));
  });
}
function resolvePath(path, native = false) {
  return __async(this, null, function* () {
    return JSON.parse(yield Tools.SoftwareSettings.exec(["--json", "workspace", "resolve-path", ...native ? ["--native"] : [], path]));
  });
}
function resolveResource(workspace, path) {
  return __async(this, null, function* () {
    return JSON.parse(yield Tools.SoftwareSettings.exec(["--json", "workspace", "resolve-path", "--from", workspace, path]));
  });
}
function openPath(_0, _1) {
  return __async(this, arguments, function* (path, action, programArgs = []) {
    yield Tools.SoftwareSettings.exec(["--json", "workspace", "open-path", path, action, ...programArgs]);
  });
}
function git(run, workspace, args) {
  return __async(this, null, function* () {
    const result = yield run(workspace, "git", ["--no-optional-locks", ...args]);
    if (result.exitCode !== 0) throw new Error(`git ${args[0]} (${result.exitCode}): ${result.stderr.trim()}`);
    return result.stdout;
  });
}
function parseStatus(text) {
  const records = text.split("\0");
  const result = [];
  for (let index = 0; index < records.length; index++) {
    const record = records[index];
    if (record.length === 0) continue;
    if (record.length < 4 || record[2] !== " ") throw new Error("Invalid git status record");
    const flags = record.slice(0, 2);
    const renamed = /[RC]/.test(flags);
    const originalPath = renamed ? records[++index] : null;
    if (renamed && !originalPath) throw new Error("Missing git rename source");
    const status = flags === "??" || flags === "AA" || flags === "DD" || /U/.test(flags) ? "U" : /A/.test(flags) ? "A" : /D/.test(flags) ? "D" : renamed ? "R" : "M";
    result.push({ path: record.slice(3), status, untracked: flags === "??", originalPath });
  }
  return result;
}
function parseNameStatus(text) {
  const records = text.split("\0");
  const result = [];
  for (let index = 0; index < records.length; index++) {
    const flag = records[index];
    if (flag === "") continue;
    if (!/^[ACDMRTUXB]\d*$/.test(flag)) throw new Error("Invalid git name-status record");
    const originalPath = /^[RC]/.test(flag) ? records[++index] : null;
    const path = records[++index];
    if (!path || originalPath === "") throw new Error("Missing git name-status path");
    result.push({ path, status: flag[0] === "C" ? "R" : flag[0], untracked: false, originalPath });
  }
  return result;
}
function parseNumstat(text) {
  const records = text.split("\0"), result = /* @__PURE__ */ new Map();
  for (let index = 0; index < records.length; index++) {
    if (records[index] === "") continue;
    const match = /^(\d+|-)\t(\d+|-)\t([\s\S]*)$/.exec(records[index]);
    if (match === null) throw new Error("Invalid git numstat record");
    let path = match[3];
    if (path === "") {
      index++;
      path = records[++index];
    }
    if (!path) throw new Error("Missing git numstat path");
    result.set(path, { additions: match[1] === "-" ? 0 : Number(match[1]), deletions: match[2] === "-" ? 0 : Number(match[2]) });
  }
  return result;
}
function comparisonBase(workspace, run) {
  return __async(this, null, function* () {
    const inventory = yield git(run, workspace, ["for-each-ref", "--format=%(refname)%00%(HEAD)%00%(upstream)"]);
    const refs = /* @__PURE__ */ new Set();
    let upstream = null;
    for (const record of inventory.split("\n")) {
      if (record === "") continue;
      const [name, current, tracking] = record.replace(/\r$/, "").split("\0");
      refs.add(name);
      if (current === "*" && tracking !== "") upstream = tracking;
    }
    if (upstream !== null) {
      if (!refs.has(upstream)) throw new Error(`The configured upstream does not exist: ${upstream}`);
      return upstream;
    }
    const conventional = ["refs/remotes/origin/main", "refs/heads/main", "refs/remotes/origin/master", "refs/heads/master"].find((ref) => refs.has(ref));
    if (conventional !== void 0) return conventional;
    const history = (yield git(run, workspace, ["rev-list", "--max-count=2", "HEAD"])).trim().split("\n");
    if (history.length < 2) throw new Error("This repository has no branch comparison base");
    return history[1];
  });
}
function loadChanges(_0, _1, _2) {
  return __async(this, arguments, function* (workspace, source, base, run = runCommand) {
    const root = (yield git(run, workspace, ["rev-parse", "--show-toplevel"])).replace(/\r?\n$/, "");
    const head = yield run(workspace, "git", ["--no-optional-locks", "rev-parse", "--verify", "--quiet", "HEAD"]);
    if (head.exitCode !== 0 && head.exitCode !== 1) throw new Error(`Cannot inspect HEAD: ${head.stderr}`);
    const unborn = head.exitCode === 1;
    if (source === "branch" && unborn) return [];
    const selectedBase = source === "branch" ? base ?? (yield comparisonBase(workspace, run)) : null;
    const revision = source === "branch" ? `${selectedBase}...HEAD` : "HEAD";
    const [status, numstat] = yield Promise.all([
      git(run, workspace, ["-C", root, ...source === "branch" ? ["diff", "--name-status", "-z", revision, "--"] : ["status", "--porcelain=v1", "-uall", "-z"]]),
      unborn ? Promise.resolve("") : git(run, workspace, ["-C", root, "diff", "--numstat", "-z", revision, "--"])
    ]);
    const stats = parseNumstat(numstat);
    const entries = source === "branch" ? parseNameStatus(status) : parseStatus(status);
    return entries.map((entry) => ({
      ...entry,
      root,
      workspace,
      base: selectedBase,
      unborn,
      diffContent: "",
      additions: stats.get(entry.path)?.additions ?? 0,
      deletions: stats.get(entry.path)?.deletions ?? 0
    }));
  });
}
function loadWorkspaceChanges(workspace, source, base) {
  return __async(this, null, function* () {
    const listing = yield Tools.Files.list(workspace);
    const ownRepository = listing.entries.some((entry) => entry.name === ".git");
    const children = listing.entries.filter((entry) => entry.isDirectory && !entry.name.startsWith("."));
    const candidates = yield Promise.all(children.map((entry) => __async(null, null, function* () {
      const path = `${workspace.replace(/\/$/, "")}/${entry.name}`;
      return (yield Tools.Files.list(path)).entries.some((entry2) => entry2.name === ".git") ? path : null;
    })));
    const mounted = candidates.filter((path) => path !== null);
    const paths = ownRepository || mounted.length === 0 ? [workspace, ...mounted] : mounted;
    const repositories = yield Promise.all(paths.map((path) => loadChanges(path, source, base)));
    const seen = /* @__PURE__ */ new Set();
    return repositories.flatMap((files) => {
      if (files.length === 0) return [];
      const root = files[0].root;
      if (seen.has(root)) return [];
      seen.add(root);
      return files;
    });
  });
}
function loadFileDiff(_0, _1, _2) {
  return __async(this, arguments, function* (workspace, file, readUntracked, run = runCommand) {
    let diffContent;
    if (file.untracked || file.unborn) {
      const content = yield readUntracked();
      const lines = content.length === 0 ? [] : content.replace(/\n$/, "").split("\n");
      diffContent = lines.length === 0 ? "" : `--- /dev/null
+++ b/${file.path}
@@ -0,0 +1,${lines.length} @@
${lines.map((line) => `+${line}`).join("\n")}`;
    } else {
      diffContent = yield git(run, file.workspace, [
        "-C",
        file.root,
        "diff",
        "--unified=3",
        "--no-ext-diff",
        "--no-textconv",
        file.base === null ? "HEAD" : `${file.base}...HEAD`,
        "--",
        ...file.originalPath === null ? [] : [file.originalPath],
        file.path
      ]);
    }
    return { ...file, diffContent, ...lineStats(diffContent), loaded: true };
  });
}

// src/icons.ts
var icons = {
  "VsCode": '<svg role="img" viewBox="0 0 24 24" fill="#007ACC" xmlns="http://www.w3.org/2000/svg">\n<path d="M23.15 2.587L18.21.21a1.494 1.494 0 0 0-1.705.29l-9.46 8.63-4.12-3.128a.999.999 0 0 0-1.276.057L.327 7.261A1 1 0 0 0 .326 8.74L3.899 12 .326 15.26a1 1 0 0 0 .001 1.479L1.65 17.94a.999.999 0 0 0 1.276.057l4.12-3.128 9.46 8.63a1.492 1.492 0 0 0 1.704.29l4.942-2.377A1.5 1.5 0 0 0 24 20.06V3.939a1.5 1.5 0 0 0-.85-1.352zm-5.146 14.861L10.826 12l7.178-5.448v10.896z"/>\n</svg>',
  "Terminal": '<svg role="img" viewBox="0 0 24 24" fill="#5391FE" xmlns="http://www.w3.org/2000/svg">\n<path d="M24 18.064c0 1.34-1.077 2.42-2.408 2.42H2.408C1.077 20.484 0 19.404 0 18.064V5.936C0 4.596 1.077 3.516 2.408 3.516h19.184C22.923 3.516 24 4.596 24 5.936v12.128zm-11.455-6.07l-6.86-5.474a.81.81 0 0 0-1.12.115.815.815 0 0 0 .114 1.127l5.966 4.76-5.966 4.761a.813.813 0 0 0-.114 1.127.81.81 0 0 0 1.12.115l6.86-5.474a.816.816 0 0 0 0-1.057zm7.42 4.417h-6.286a.81.81 0 1 0 0 1.621h6.286a.81.81 0 1 0 0-1.621z"/>\n</svg>',
  "TreeHierarchy": '<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">\n<rect x="9" y="3" width="6" height="5" rx="1"/>\n<rect x="3" y="15" width="6" height="5" rx="1"/>\n<rect x="15" y="15" width="6" height="5" rx="1"/>\n<path d="M12 8v4M6 15v-3h12v3"/>\n</svg>',
  "LangPython": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<path d="M11.91 2c-5.06 0-4.73 2.19-4.73 2.19l.01 2.27h4.81v.68H5.21S2 6.77 2 11.88s2.8 5 2.8 5h1.67v-2.33s-.09-2.8 2.76-2.8h4.74s2.66.04 2.66-2.6V4.6s.4-2.6-4.72-2.6zm-2.58 1.48a.86.86 0 1 1 0 1.72.86.86 0 0 1 0-1.72z" fill="#3776AB"/>\n<path d="M12.09 22c5.06 0 4.73-2.19 4.73-2.19l-.01-2.27h-4.81v-.68h6.78s3.21.37 3.21-4.74-2.8-5-2.8-5h-1.67v2.33s.09 2.8-2.76 2.8H10.1s-2.66-.04-2.66 2.6V19.4s-.4 2.6 4.72 2.6zm2.58-1.48a.86.86 0 1 1 0-1.72.86.86 0 0 1 0 1.72z" fill="#FFD43B"/>\n</svg>',
  "LangRust": '<svg viewBox="0 0 24 24" fill="#DEA584" xmlns="http://www.w3.org/2000/svg">\n<path d="M23.83 12c0-.29-.02-.57-.05-.84l-1.92-.32a9.98 9.98 0 0 0-.58-1.41l1.33-1.44a11.96 11.96 0 0 0-1.23-1.9l-1.84.66a9.97 9.97 0 0 0-1.08-1.08l.66-1.84a11.97 11.97 0 0 0-1.9-1.23l-1.44 1.33a10.03 10.03 0 0 0-1.41-.59l-.32-1.92A11.75 11.75 0 0 0 12 .17c-.29 0-.57.02-.84.05l-.32 1.92a9.98 9.98 0 0 0-1.41.58l-1.44-1.33a11.96 11.96 0 0 0-1.9 1.23l.66 1.84a9.98 9.98 0 0 0-1.08 1.08l-1.84-.66a11.97 11.97 0 0 0-1.23 1.9l1.33 1.44c-.23.45-.43.93-.59 1.41l-1.92.32a11.77 11.77 0 0 0-.05.85c0 .28.02.56.05.84l1.92.32c.16.49.36.96.59 1.41l-1.33 1.44a11.96 11.96 0 0 0 1.23 1.9l1.84-.66c.34.4.7.76 1.08 1.08l-.66 1.84a11.97 11.97 0 0 0 1.9 1.23l1.44-1.33c.45.23.93.43 1.41.59l.32 1.92c.28.03.56.05.84.05.29 0 .57-.02.84-.05l.32-1.92a10.02 10.02 0 0 0 1.41-.59l1.44 1.33a11.96 11.96 0 0 0 1.9-1.23l-.66-1.84a9.98 9.98 0 0 0 1.08-1.08l1.84.66a11.97 11.97 0 0 0 1.23-1.9l-1.33-1.44c.23-.45.43-.93.59-1.41l1.92-.32c.03-.28.05-.56.05-.84zm-11.83 6.64a6.64 6.64 0 1 1 0-13.28 6.64 6.64 0 0 1 0 13.28zm-2.02-9.61h2.24c1.17 0 1.91.56 1.91 1.48 0 .76-.48 1.28-1.24 1.43l1.43 2.7h-1.28l-1.28-2.48h-.6v2.48H9.98V9.03zm1.18 2.11h.96c.55 0 .9-.25.9-.66 0-.42-.35-.66-.9-.66h-.96v1.32z"/>\n</svg>',
  "LangC": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<path d="M12 2.5l8.23 4.75v9.5L12 21.5l-8.23-4.75v-9.5L12 2.5z" fill="#00599C"/>\n<path d="M14.8 8.8a4.2 4.2 0 0 0-5.6 0 4 4 0 0 0 0 6.4 4.2 4.2 0 0 0 5.6 0" stroke="#FFFFFF" stroke-width="2.4" stroke-linecap="round"/>\n</svg>',
  "LangCpp": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<path d="M12 2.5l8.23 4.75v9.5L12 21.5l-8.23-4.75v-9.5L12 2.5z" fill="#004482"/>\n<path d="M12 9.2a3.5 3.5 0 0 0-4.8 0 3.4 3.4 0 0 0 0 5.6 3.5 3.5 0 0 0 4.8 0" stroke="#FFFFFF" stroke-width="2.2" stroke-linecap="round"/>\n<path d="M14 12h2.5M15.25 10.75v2.5M17.5 12h2.5M18.75 10.75v2.5" stroke="#659AD2" stroke-width="1.5" stroke-linecap="round"/>\n</svg>',
  "LangDart": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<path d="M4.1 3.5L3 13.2l5.7 5.7 8.2-8.2L4.1 3.5z" fill="#0175C2"/>\n<path d="M16.9 10.7l-8.2 8.2 2.1 2.1c1.2 1.2 3.1 1.2 4.3 0l5.8-5.8-4-4.5z" fill="#02569B"/>\n<path d="M12.9 2.5c-1.2-1.2-3.1-1.2-4.3 0L3 8.1l1.1 5.1 12.8-2.5-4-8.2z" fill="#00B4AB"/>\n<path d="M16.9 10.7l4 4.5 1.1-5.1-5.1-7.6-4 8.2 4 0z" fill="#29B6F6"/>\n</svg>',
  "LangTs": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<rect width="22" height="22" x="1" y="1" rx="4" fill="#3178C6"/>\n<path d="M11 9H5v2.2h2V18h2.3v-6.8H11V9zm3.8 4.7c.8.6 1.7.9 2.7.9 1 0 1.6-.4 1.6-1 0-.6-.5-.9-1.8-1.4-1.7-.6-2.8-1.5-2.8-3 0-1.8 1.4-3.2 3.6-3.2 1.3 0 2.3.4 3.1 1l-1 1.8c-.7-.5-1.4-.7-2.1-.7-.8 0-1.3.4-1.3.9 0 .6.5.8 1.7 1.3 1.9.7 2.9 1.6 2.9 3.2 0 1.9-1.4 3.3-3.8 3.3-1.6 0-2.8-.5-3.8-1.2l1-1.9z" fill="#FFFFFF"/>\n</svg>',
  "LangJs": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<rect width="22" height="22" x="1" y="1" rx="4" fill="#F7DF1E"/>\n<path d="M7.5 17.5c1 .6 2.1.9 3.2.9 1.8 0 2.8-.9 2.8-2.5V9H11v6.9c0 .6-.3.9-.9.9-.4 0-.8-.1-1.2-.3l-1.4 1zM15 17.2c1 .6 2.1 1 3.4 1 2.2 0 3.6-1.1 3.6-2.8 0-1.6-1-2.4-2.6-3-1.1-.5-1.6-.8-1.6-1.3 0-.5.4-.8 1.2-.8.8 0 1.5.3 2.1.7l1.1-1.8c-.9-.6-2-1-3.2-1-2.1 0-3.5 1.2-3.5 2.8 0 1.5 1 2.4 2.5 3 1.1.5 1.7.8 1.7 1.4 0 .6-.5.9-1.4.9-.9 0-1.8-.3-2.5-.8l-1.3 1.9z" fill="#000000"/>\n</svg>',
  "LangGo": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<rect width="22" height="22" x="1" y="1" rx="5" fill="#00ADD8"/>\n<path d="M10.8 12.3H8.3v1.8h4.5c-.4 2.3-2 3.6-4.5 3.6-2.9 0-5.1-2.1-5.1-5.4s2.2-5.4 5.1-5.4c1.8 0 3.1.8 3.9 1.8l-1.6 1.4c-.6-.7-1.3-1.2-2.3-1.2-1.7 0-3 1.4-3 3.4s1.3 3.4 3 3.4c1.2 0 2-.6 2.4-1.5H10.8v-1.9zm8.4-.3c0 3.3-2.3 5.7-5.5 5.7s-5.5-2.4-5.5-5.7 2.3-5.7 5.5-5.7 5.5 2.4 5.5 5.7zm-2.4 0c0-2-1.3-3.6-3.1-3.6s-3.1 1.6-3.1 3.6 1.3 3.6 3.1 3.6 3.1-1.6 3.1-3.6z" fill="#FFFFFF"/>\n</svg>',
  "LangJava": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<path d="M4 10h12v6a4 4 0 0 1-4 4H8a4 4 0 0 1-4-4v-6zm12 2h2a2 2 0 0 1 2 2v0a2 2 0 0 1-2 2h-2v-4z" stroke="#EA2D2E" stroke-width="2" stroke-linecap="round"/>\n<path d="M7 6c0-1.5 1-2.5 1-4M10 6c0-1.5 1-2.5 1-4M13 6c0-1.5 1-2.5 1-4" stroke="#5382A1" stroke-width="1.8" stroke-linecap="round"/>\n<path d="M2 21h18" stroke="#EA2D2E" stroke-width="2" stroke-linecap="round"/>\n</svg>',
  "LangKotlin": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<rect width="22" height="22" x="1" y="1" rx="4" fill="#7F52FF"/>\n<path d="M4 20L12 12 4 4v16zM12 12l8 8H12v-8zM12 12l8-8H12v8z" fill="#FFFFFF" fill-opacity="0.85"/>\n</svg>',
  "LangSwift": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<rect width="22" height="22" x="1" y="1" rx="5" fill="#F05138"/>\n<path d="M19.2 17.5c-3.1 2.2-7.3 1.8-10.4-.9 4.2 1.4 8.7-.3 10.4-.9-2.3-1.6-4.5-4-5.2-6.8 1.8 1.4 4.1 2 6.4 1.4-1.9-2.6-4.9-5-8.5-5.8 4.2.9 7.8 3.5 9.7 7.2-.6-1.5-1.5-2.8-2.6-3.9 3.2 2.7 4.1 7.1.2 9.7z" fill="#FFFFFF"/>\n</svg>',
  "LangHtml": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<path d="M4 3l1.5 16.5L12 21.5l6.5-2L20 3H4z" fill="#E34F26"/>\n<path d="M12 4.5v15.2l5.1-1.5 1.2-13.7H12z" fill="#EF652A"/>\n<path d="M12 7.8H8.2l.2 2.5h3.6V7.8zm0 5h-2.1l-.1-1.3H8.3l.3 3.6h3.4v-2.3zm0-5v2.5h3.4l-.3 3.5-3.1.8v2.4l5.3-1.5.6-7.7H12z" fill="#FFFFFF"/>\n</svg>',
  "LangCss": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<path d="M4 3l1.5 16.5L12 21.5l6.5-2L20 3H4z" fill="#1572B6"/>\n<path d="M12 4.5v15.2l5.1-1.5 1.2-13.7H12z" fill="#33A9DC"/>\n<path d="M12 7.8H8.2l.2 2.5h3.6V7.8zm0 5h-3.5l.3 3.6 3.2.9v-2.4l-1.4-.4-.1-1.7H12v-0zm0-5v2.5h3.4l-.3 3.5-3.1.8v2.4l5.3-1.5.6-7.7H12z" fill="#FFFFFF"/>\n</svg>',
  "LangJson": '<svg viewBox="0 0 24 24" fill="none" stroke="#CB85E8" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">\n<path d="M9 5c-1.5 0-2 1-2 2v3c0 1-1 2-2 2 1 0 2 1 2 2v3c0 1 .5 2 2 2M15 5c1.5 0 2 1 2 2v3c0 1 1 2 2 2-1 0-2 1-2 2v3c0 1-.5 2-2 2"/>\n</svg>',
  "LangYaml": '<svg viewBox="0 0 24 24" fill="none" stroke="#E5C07B" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">\n<path d="M4 21v-7M4 10V3M12 21v-9M12 8V3M20 21v-5M20 12V3M1 14h6M9 8h6M17 16h6"/>\n</svg>',
  "LangMarkdown": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<rect width="22" height="16" x="1" y="4" rx="3" stroke="#79B8FF" stroke-width="2"/>\n<path d="M4 15V9l2.5 3L9 9v6M13 12h3M14.5 9.5v5.5m0 0l-2-2m2 2l2-2" stroke="#79B8FF" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round"/>\n</svg>',
  "LangShell": '<svg viewBox="0 0 24 24" fill="none" stroke="#5391FE" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">\n<polyline points="4 17 10 11 4 5"/>\n<line x1="12" y1="19" x2="20" y2="19"/>\n</svg>',
  "LangSql": '<svg viewBox="0 0 24 24" fill="none" stroke="#E5C07B" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">\n<ellipse cx="12" cy="5" rx="9" ry="3"/>\n<path d="M21 12c0 1.66-4 3-9 3s-9-1.34-9-3M3 5v14c0 1.66 4 3 9 3s9-1.34 9-3V5"/>\n</svg>',
  "LangDocker": '<svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">\n<path d="M2.5 12.5h19c0 4.5-3.5 8-8 8-4 0-7.2-2.8-7.9-6.5l-3.1-1.5z" fill="#0db7ed"/>\n<rect x="5" y="9" width="2" height="2" fill="#0db7ed"/>\n<rect x="8" y="9" width="2" height="2" fill="#0db7ed"/>\n<rect x="11" y="9" width="2" height="2" fill="#0db7ed"/>\n<rect x="8" y="6" width="2" height="2" fill="#0db7ed"/>\n<rect x="11" y="6" width="2" height="2" fill="#0db7ed"/>\n<rect x="14" y="9" width="2" height="2" fill="#0db7ed"/>\n</svg>',
  "LangGit": '<svg viewBox="0 0 24 24" fill="none" stroke="#F05032" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">\n<circle cx="6" cy="6" r="3"/>\n<circle cx="6" cy="18" r="3"/>\n<circle cx="18" cy="9" r="3"/>\n<line x1="6" y1="9" x2="6" y2="15"/>\n<path d="M6 12a9 9 0 0 0 9-3"/>\n</svg>',
  "LangLock": '<svg viewBox="0 0 24 24" fill="none" stroke="#8B949E" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">\n<rect x="3" y="11" width="18" height="11" rx="2" ry="2"/>\n<path d="M7 11V7a5 5 0 0 1 10 0v4"/>\n</svg>',
  "LangDefault": '<svg viewBox="0 0 24 24" fill="none" stroke="#8B949E" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">\n<path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>\n<polyline points="14 2 14 8 20 8"/>\n</svg>'
};
function fileIcon(name) {
  const lower = name.toLowerCase();
  if (lower === "dockerfile" || lower.startsWith("dockerfile.")) return icons.LangDocker;
  if (lower.startsWith(".git")) return icons.LangGit;
  const extension = name.indexOf(".") < 0 ? "" : name.split(".").pop().toLowerCase();
  switch (extension) {
    case "c":
    case "h":
      return icons.LangC;
    case "cpp":
    case "cc":
    case "cxx":
    case "hpp":
    case "hxx":
      return icons.LangCpp;
    case "py":
    case "pyw":
    case "ipynb":
      return icons.LangPython;
    case "rs":
      return icons.LangRust;
    case "dart":
      return icons.LangDart;
    case "ts":
    case "tsx":
    case "mts":
    case "cts":
      return icons.LangTs;
    case "js":
    case "jsx":
    case "mjs":
    case "cjs":
      return icons.LangJs;
    case "go":
      return icons.LangGo;
    case "java":
    case "class":
    case "jar":
      return icons.LangJava;
    case "kt":
    case "kts":
      return icons.LangKotlin;
    case "swift":
      return icons.LangSwift;
    case "html":
    case "htm":
      return icons.LangHtml;
    case "css":
    case "scss":
    case "less":
      return icons.LangCss;
    case "json":
    case "jsonc":
      return icons.LangJson;
    case "yaml":
    case "yml":
    case "toml":
      return icons.LangYaml;
    case "md":
    case "markdown":
      return icons.LangMarkdown;
    case "sh":
    case "bash":
    case "zsh":
    case "ps1":
    case "bat":
    case "cmd":
      return icons.LangShell;
    case "sql":
      return icons.LangSql;
    case "lock":
      return icons.LangLock;
    default:
      return icons.LangDefault;
  }
}

// src/change_tree.ts
function fileKey(file) {
  return typeof file.root === "string" ? JSON.stringify([file.root, file.path]) : file.path;
}
function buildChangeTree(files) {
  const roots = /* @__PURE__ */ new Map();
  const directories = /* @__PURE__ */ new Map();
  for (const file of files) {
    const root = typeof file.root === "string" ? file.root : "";
    let parent = roots.get(root);
    if (parent === void 0) {
      parent = { kind: "directory", id: `change-root:${JSON.stringify(root)}`, name: root.replace(/\\/g, "/").split("/").filter(Boolean).pop() ?? ".", path: root, children: [] };
      roots.set(root, parent);
    }
    const segments = file.path.replace(/\\/g, "/").split("/").filter(Boolean);
    for (let index = 0; index < segments.length - 1; index++) {
      const path = segments.slice(0, index + 1).join("/");
      const id = `change-dir:${JSON.stringify([root, path])}`;
      let directory = directories.get(id);
      if (directory === void 0) {
        directory = { kind: "directory", id, name: segments[index], path, children: [] };
        directories.set(id, directory);
        parent.children.push(directory);
      }
      parent = directory;
    }
    parent.children.push({ kind: "file", id: fileKey(file), name: segments[segments.length - 1], path: file.path, file });
  }
  function sort(nodes) {
    nodes.sort((a, b) => Number(b.kind === "directory") - Number(a.kind === "directory") || a.name.localeCompare(b.name));
    for (const node of nodes) if (node.kind === "directory") sort(node.children);
    return nodes;
  }
  return sort(roots.size === 1 ? [...roots.values()][0].children : [...roots.values()]);
}
function changeTreeRows(nodes, collapsed, depth = 0, parents = []) {
  return nodes.flatMap((node) => [
    { node, depth, parents },
    ...node.kind === "directory" && !collapsed.has(node.id) ? changeTreeRows(node.children, collapsed, depth + 1, [...parents, node.id]) : []
  ]);
}

// src/ui/workspace.ui.ts
var svgVsCode = '\r\n<svg role="img" viewBox="0 0 24 24" fill="#007ACC" xmlns="http://www.w3.org/2000/svg">\r\n<path d="M23.15 2.587L18.21.21a1.494 1.494 0 0 0-1.705.29l-9.46 8.63-4.12-3.128a.999.999 0 0 0-1.276.057L.327 7.261A1 1 0 0 0 .326 8.74L3.899 12 .326 15.26a1 1 0 0 0 .001 1.479L1.65 17.94a.999.999 0 0 0 1.276.057l4.12-3.128 9.46 8.63a1.492 1.492 0 0 0 1.704.29l4.942-2.377A1.5 1.5 0 0 0 24 20.06V3.939a1.5 1.5 0 0 0-.85-1.352zm-5.146 14.861L10.826 12l7.178-5.448v10.896z"/>\r\n</svg>\r\n';
var svgTerminal = '\r\n<svg role="img" viewBox="0 0 24 24" fill="#5391FE" xmlns="http://www.w3.org/2000/svg">\r\n<path d="M24 18.064c0 1.34-1.077 2.42-2.408 2.42H2.408C1.077 20.484 0 19.404 0 18.064V5.936C0 4.596 1.077 3.516 2.408 3.516h19.184C22.923 3.516 24 4.596 24 5.936v12.128zm-11.455-6.07l-6.86-5.474a.81.81 0 0 0-1.12.115.815.815 0 0 0 .114 1.127l5.966 4.76-5.966 4.761a.813.813 0 0 0-.114 1.127.81.81 0 0 0 1.12.115l6.86-5.474a.816.816 0 0 0 0-1.057zm7.42 4.417h-6.286a.81.81 0 1 0 0 1.621h6.286a.81.81 0 1 0 0-1.621z"/>\r\n</svg>\r\n';
var svgTreeHierarchy = '\r\n<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" xmlns="http://www.w3.org/2000/svg">\r\n<rect x="9" y="3" width="6" height="5" rx="1"/>\r\n<rect x="3" y="15" width="6" height="5" rx="1"/>\r\n<rect x="15" y="15" width="6" height="5" rx="1"/>\r\n<path d="M12 8v4M6 15v-3h12v3"/>\r\n</svg>\r\n';
function join(root, child) {
  if (child.startsWith("/")) return child;
  return `${root.replace(/\/$/, "")}/${child}`;
}
function Workspace(ctx) {
  const { UI, Modifier: M } = ctx, c = ctx.MaterialTheme.colorScheme;
  const text = (props) => UI.Text({ inheritStyle: props.style === void 0, ...props });
  const [input] = ctx.useState("input", {});
  const [messages] = ctx.useState("messages", []);
  const [workspace] = ctx.useState("workspacePath", null);
  const [locale] = ctx.useState("locale", "zh");
  const dark = ctx.Theme.getCurrent().brightness === "dark";
  const lastTurn = ctx.useMemo("turn-files", () => {
    if (input.files !== void 0) return input.files;
    const conversation = input.messages === void 0 ? messages : input.messages;
    for (let index = conversation.length - 1; index >= 0; index--) {
      const message = conversation[index];
      if (message.sender !== "ai") continue;
      const files = reviewForMessage(message);
      if (files.length > 0) return files;
    }
    return [];
  }, [input, messages]);
  const [source, setSource] = ctx.useState("source", lastTurn.length > 0 ? "lastTurn" : "git");
  const [pane, setPane] = ctx.useState("pane", "changes");
  const previousPane = ctx.useRef("previous-pane", "changes");
  const [split, setSplit] = ctx.useState("split", false);
  const [wrap, setWrap] = ctx.useState("wrap", false);
  const [full, setFull] = ctx.useState("full", false);
  const [query, setQuery] = ctx.useState("query", "");
  const [base] = ctx.useState("branch-base", null);
  const [listWidth, setListWidth] = ctx.useState("list-width", 220);
  const listDrag = ctx.useRef("list-drag-width", listWidth);
  const [collapsedChanges, setCollapsedChanges] = ctx.useState("collapsed-changes", []);
  const [splitRatio, setSplitRatio] = ctx.useState("split-ratio", 0.5);
  const splitDrag = ctx.useRef("split-drag-ratio", splitRatio);
  const [size, setSize] = ctx.useState("size", { width: 800, height: 600 });
  const [catalog, setCatalog] = ctx.useState("catalog", []);
  const [loading, setLoading] = ctx.useState("loading", false);
  const [error, setError] = ctx.useState("error", "");
  const [selectedPath, setSelectedPath] = ctx.useState("selected", "");
  const [loadingDiff, setLoadingDiff] = ctx.useState("loading-diff", false);
  const [fileLines, setFileLines] = ctx.useState("file-lines", null);
  const [gaps, setGaps] = ctx.useState("gaps", []);
  const [tree, setTree] = ctx.useState("tree", {});
  const treeCache = ctx.useRef("tree-cache", {});
  const [expanded, setExpanded] = ctx.useState("expanded", []);
  const [preview, setPreview] = ctx.useState("preview", null);
  const [previewLoading, setPreviewLoading] = ctx.useState("preview-loading", false);
  const [previewError, setPreviewError] = ctx.useState("preview-error", "");
  const [treeLoading, setTreeLoading] = ctx.useState("tree-loading", false);
  const [treeError, setTreeError] = ctx.useState("tree-error", "");
  const treeRequest = ctx.useRef("tree-request", 0);
  const previewRequest = ctx.useRef("preview-request", 0);
  const [opener, setOpener] = ctx.useState("opener", 1);
  const [refresh, setRefresh] = ctx.useState("refresh", 0);
  const sourceNames = ["Git 变更", "分支变更", "上一轮变更"];
  const sources = ["git", "branch", "lastTurn"];
  const green = "#FF3FB950", red = "#FFF85149";
  const divider = dark ? "#FF262626" : c.outlineVariant.copy({ alpha: 0.35 });
  const all = source === "lastTurn" ? lastTurn : catalog;
  const rows = ctx.useMemo("filtered-changes", () => all.filter((file) => query.trim() === "" || file.path.toLowerCase().includes(query.trim().toLowerCase())), [all, query]);
  const currentIndex = Math.max(0, rows.findIndex((file) => fileKey(file) === selectedPath));
  const current = rows[currentIndex] ?? null;
  const effective = current;
  const totals = all.reduce((sum, file) => ({ additions: sum.additions + file.additions, deletions: sum.deletions + file.deletions }), { additions: 0, deletions: 0 });
  const availableListWidth = Math.max(0, Math.min(500, size.width - 161));
  const minimumListWidth = Math.min(120, availableListWidth);
  const sidebarWidth = Math.min(Math.max(minimumListWidth, listWidth), availableListWidth);
  const mainWidth = Math.max(1, size.width - (pane === "closed" ? 0 : sidebarWidth + 1));
  const changeTree = ctx.useMemo("change-tree", () => buildChangeTree(rows), [rows]);
  const visibleChanges = changeTreeRows(changeTree, new Set(query.trim() === "" ? collapsedChanges : []));
  ctx.useEffect("review-input", () => {
    setSource(lastTurn.length > 0 ? "lastTurn" : "git");
    return () => {
    };
  }, [lastTurn]);
  ctx.useEffect("catalog-load", () => {
    let active = true;
    ++previewRequest.current;
    setError("");
    setCatalog([]);
    setSelectedPath("");
    setPreview(null);
    if (source === "lastTurn") {
      setLoading(false);
      return () => {
        active = false;
      };
    }
    if (workspace === null || workspace === "") {
      setLoading(false);
      return () => {
        active = false;
      };
    }
    setLoading(true);
    loadWorkspaceChanges(workspace, source, base).then((files) => {
      if (active) {
        setCatalog(files);
        setLoading(false);
      }
    }, (failure) => {
      if (active) {
        setError(String(failure));
        setLoading(false);
      }
    });
    return () => {
      active = false;
    };
  }, [source, workspace, base, refresh]);
  function filePath(file) {
    return __async(this, null, function* () {
      if (source === "lastTurn") return (yield resolveResource(workspace ?? "", file.path)).vfsPath;
      if (workspace === null || workspace === "") throw new Error("当前对话未绑定工作区");
      return (yield resolvePath(join(file.root, file.path), true)).vfsPath;
    });
  }
  ctx.useEffect("selected-file", () => {
    let active = true;
    setError("");
    setFileLines(null);
    setGaps([]);
    setLoadingDiff(false);
    if (current === null || source === "lastTurn" || workspace === null) return () => {
      active = false;
    };
    if (current.loaded) return () => {
      active = false;
    };
    setLoadingDiff(true);
    loadFileDiff(workspace, current, () => __async(null, null, function* () {
      return (yield Tools.Files.read(yield filePath(current))).content;
    })).then((file) => {
      if (active) {
        setCatalog(catalog.map((entry) => fileKey(entry) === fileKey(file) ? file : entry));
        setLoadingDiff(false);
      }
    }, (failure) => {
      if (active) {
        setError(String(failure));
        setLoadingDiff(false);
      }
    });
    return () => {
      active = false;
    };
  }, [source, workspace, current === null ? null : fileKey(current), refresh]);
  ctx.useEffect("full-file", () => {
    let active = true;
    if (!full || current === null) {
      setFileLines(null);
      setGaps([]);
      return () => {
        active = false;
      };
    }
    filePath(current).then((path) => Tools.Files.read(path)).then((result) => {
      if (active) setFileLines(result.content.split(/\r?\n/));
    }, (failure) => {
      if (active) setError(String(failure));
    });
    return () => {
      active = false;
    };
  }, [full, current === null ? null : fileKey(current), source, workspace]);
  function loadDirectory(path, depth, generation) {
    return __async(this, null, function* () {
      const listing = yield Tools.Files.list(path);
      const children = listing.entries.filter((entry) => !entry.name.startsWith(".")).sort((a, b) => {
        if (a.isDirectory !== b.isDirectory) return a.isDirectory ? -1 : 1;
        return a.name.toLowerCase().localeCompare(b.name.toLowerCase());
      }).map((entry) => ({ ...entry, path: join(path, entry.name), depth }));
      if (treeRequest.current !== generation) return;
      treeCache.current = { ...treeCache.current, [path]: children };
      setTree(treeCache.current);
    });
  }
  ctx.useEffect("tree-root", () => {
    let active = true;
    const generation = ++treeRequest.current;
    if (!workspace) return () => {
      active = false;
    };
    setTreeError("");
    setTreeLoading(true);
    setExpanded([]);
    Tools.Files.list(workspace).then((listing) => {
      if (active && treeRequest.current === generation) {
        treeCache.current = { [workspace]: listing.entries.filter((entry) => !entry.name.startsWith(".")).sort((a, b) => a.isDirectory !== b.isDirectory ? a.isDirectory ? -1 : 1 : a.name.toLowerCase().localeCompare(b.name.toLowerCase())).map((entry) => ({ ...entry, path: join(workspace, entry.name), depth: 0 })) };
        setTree(treeCache.current);
        setTreeLoading(false);
      }
    }, (failure) => {
      if (active) {
        setTreeError(String(failure));
        setTreeLoading(false);
      }
    });
    return () => {
      active = false;
      ++treeRequest.current;
      ++previewRequest.current;
    };
  }, [workspace, refresh]);
  function icon(name, tooltip, onClick, active = false, enabled = true) {
    return UI.Tooltip({ message: tooltip }, UI.InkWell(
      { onClick: enabled ? onClick : void 0, shape: { cornerRadius: 4 } },
      UI.Box(
        { width: 22, height: 22, contentAlignment: "center", modifier: active ? M.background(c.primary.copy({ alpha: 0.18 }), { cornerRadius: 4 }) : void 0 },
        UI.Icon({ name, size: 14, tint: enabled ? active ? c.primary : c.onSurfaceVariant : c.onSurfaceVariant.copy({ alpha: 0.38 }) })
      )
    ));
  }
  function stat(value, addition, fontSize = 11) {
    return text({ text: `${addition ? "+" : "-"}${value}`, color: addition ? green : red, fontSize, fontFamily: "monospace", fontWeight: fontSize === 11 ? "700" : void 0, maxLines: 1 });
  }
  function line(color = c.outlineVariant) {
    return UI.Divider({ height: 1, thickness: 1, color });
  }
  function menu(anchor, labels, selected, onSelect, tooltip) {
    return UI.MenuAnchor({ anchor, tooltip, itemHeight: 32, onSelect }, labels.map((label, index) => UI.Row({ verticalAlignment: "center" }, [
      text({ text: label, fontSize: 12, weight: 1 }),
      ...index === selected ? [UI.Icon({ name: "check", size: 14 })] : []
    ])));
  }
  function sidebarHeader() {
    const sourceAnchor = UI.Box(
      { modifier: M.height(24).background(c.surface.copy({ alpha: 0.8 }), { cornerRadius: 5 }).border(1, c.outlineVariant.copy({ alpha: 0.3 }), { cornerRadius: 5 }).padding(8, 1), contentAlignment: "center" },
      UI.Row({ verticalAlignment: "center" }, [text({ text: sourceNames[sources.indexOf(source)], style: "labelSmall", fontSize: 11.5, letterSpacing: 0.5, fontWeight: "600" }), UI.Spacer({ width: 2 }), UI.Icon({ name: "arrow_drop_down", size: 15, tint: c.onSurfaceVariant })])
    );
    return UI.Box({ height: 32, modifier: M.padding(8, 0), contentAlignment: "start" }, UI.ScrollArea({ axis: "horizontal", height: 32 }, UI.Row({ modifier: M.widthIn(sidebarWidth - 16), verticalAlignment: "center", horizontalArrangement: "spaceBetween" }, [
      UI.Row({ verticalAlignment: "center" }, [
        UI.Tooltip({ message: pane === "tree" ? "切换至变更文件列表" : "切换至项目工作树" }, UI.InkWell(
          { shape: { cornerRadius: 5 }, onClick: () => setPane(pane === "tree" ? "changes" : "tree") },
          UI.Box({ modifier: M.width(25).height(25).background(pane === "tree" ? c.primary.copy({ alpha: 0.18 }) : c.surface.copy({ alpha: 0.8 }), { cornerRadius: 5 }).border(1, pane === "tree" ? c.primary.copy({ alpha: 0.5 }) : c.outlineVariant.copy({ alpha: 0.3 }), { cornerRadius: 5 }).padding(5.5) }, UI.Svg({ source: svgTreeHierarchy, tint: pane === "tree" ? c.primary : c.onSurfaceVariant }))
        )),
        UI.Spacer({ width: 6 }),
        menu(sourceAnchor, sourceNames, sources.indexOf(source), (index) => setSource(sources[index]), "切换变更对比源")
      ]),
      UI.Spacer({ width: 6 }),
      UI.Row({ verticalAlignment: "center" }, [stat(totals.additions, true), UI.Spacer({ width: 5 }), stat(totals.deletions, false)])
    ])));
  }
  function select(path) {
    ++previewRequest.current;
    setSelectedPath(path);
    setPreview(null);
    const row = changeTreeRows(changeTree, /* @__PURE__ */ new Set()).find((entry) => entry.node.id === path);
    if (row !== void 0) setCollapsedChanges(collapsedChanges.filter((id) => !row.parents.includes(id)));
  }
  function changeRow({ node, depth }) {
    const folder = node.kind === "directory", selected = !folder && current !== null && node.id === fileKey(current);
    const opened = query.trim() !== "" || !collapsedChanges.includes(node.id);
    return UI.Tooltip({ message: node.path }, UI.InkWell({ key: node.id, onClick: () => {
      if (node.kind === "directory") setCollapsedChanges(opened ? [...collapsedChanges, node.id] : collapsedChanges.filter((id) => id !== node.id));
      else select(node.id);
    } }, UI.Row({ height: 26, modifier: M.background(selected ? c.primary.copy({ alpha: 0.15 }) : "transparent").borderSides(selected ? { start: { width: 2.5, color: c.primary } } : {}).padding(8 + depth * 14 - (selected ? 2.5 : 0), 0, 8, 0), verticalAlignment: "center" }, [
      folder ? UI.Icon({ name: opened ? "keyboard_arrow_down" : "keyboard_arrow_right", size: 13, tint: c.outline }) : UI.Spacer({ width: 13 }),
      UI.Spacer({ width: 3 }),
      folder ? UI.Icon({ name: opened ? "folder_open" : "folder", size: 14, tint: "#FFEBCB8B" }) : UI.Svg({ source: fileIcon(node.name), width: 14, height: 14 }),
      UI.Spacer({ width: 6 }),
      text({ text: node.name, weight: 1, maxLines: 1, overflow: "ellipsis", color: selected ? c.primary : c.onSurface, fontWeight: selected ? "700" : "500", fontSize: 11.5 }),
      ...node.kind === "file" && node.file.additions > 0 ? [stat(node.file.additions, true, 10.5), UI.Spacer({ width: 3 })] : [],
      ...node.kind === "file" && node.file.deletions > 0 ? [stat(node.file.deletions, false, 10.5)] : []
    ])));
  }
  function dragSidebar(deltaX) {
    listDrag.current = Math.min(availableListWidth, Math.max(minimumListWidth, listDrag.current + deltaX));
    setListWidth(listDrag.current);
  }
  function selectTree(entry) {
    return __async(this, null, function* () {
      if (entry.isDirectory) {
        if (expanded.includes(entry.path)) setExpanded(expanded.filter((path) => path !== entry.path));
        else {
          const generation = treeRequest.current;
          try {
            if (treeCache.current[entry.path] === void 0) yield loadDirectory(entry.path, entry.depth + 1, generation);
            if (treeRequest.current === generation) setExpanded([...expanded, entry.path]);
          } catch (failure) {
            if (treeRequest.current === generation) setTreeError(String(failure));
          }
        }
      } else {
        const generation = ++previewRequest.current;
        setPreview({ path: entry.path, content: "" });
        setPreviewLoading(true);
        setPreviewError("");
        try {
          const result = yield Tools.Files.read(entry.path);
          if (previewRequest.current === generation) {
            setPreview({ path: entry.path, content: result.content });
            setPreviewLoading(false);
          }
        } catch (failure) {
          if (previewRequest.current === generation) {
            setPreviewError(String(failure));
            setPreviewLoading(false);
          }
        }
      }
    });
  }
  function treeRows(path) {
    return (tree[path] ?? []).flatMap((entry) => {
      const opened = expanded.includes(entry.path), selected = preview?.path === entry.path;
      return [UI.InkWell({ key: entry.path, onClick: () => selectTree(entry) }, UI.Row({ height: 24, modifier: M.background(selected ? c.primary.copy({ alpha: 0.12 }) : "transparent").padding(6 + entry.depth * 14, 0, 6, 0), verticalAlignment: "center" }, [
        entry.isDirectory ? UI.Icon({ name: opened ? "keyboard_arrow_down" : "keyboard_arrow_right", size: 13, tint: c.outline }) : UI.Spacer({ width: 13 }),
        UI.Spacer({ width: 3 }),
        entry.isDirectory ? UI.Icon({ name: opened ? "folder_open" : "folder", size: 14, tint: selected ? c.primary : "#FFEBCB8B" }) : UI.Svg({ source: fileIcon(entry.name), width: 14, height: 14 }),
        UI.Spacer({ width: 6 }),
        text({ text: entry.name, fontSize: 11.5, color: selected ? c.primary : void 0, fontWeight: selected ? "700" : void 0, weight: 1, maxLines: 1, overflow: "ellipsis" })
      ])), ...opened ? treeRows(entry.path) : []];
    });
  }
  function openTarget(index) {
    return __async(this, null, function* () {
      const path = preview?.path ?? (current === null ? workspace : yield filePath(current));
      if (!path) return;
      const resolved = yield resolvePath(path);
      if (index === 3) {
        yield Tools.System.setClipboardText(resolved.nativePath);
        ctx.showToast(`已复制路径: ${resolved.nativePath}`);
      } else if (index === 0) yield openPath(path, "reveal");
      else if (index === 1) yield openPath(path, "program", ["code", "-g"]);
      else if (index === 2) yield openPath(path, "terminal");
      else throw new Error(`Unknown external opener: ${index}`);
    });
  }
  function toolbar() {
    function navigation(name, tooltip, onClick, enabled = true, tint) {
      return UI.IconButton({ tooltip, width: 40, height: 30, enabled, onClick }, UI.Icon({ name, size: 14, tint }));
    }
    function toggle(name, tooltip, onClick, active, enabled = true) {
      return UI.Tooltip({ message: tooltip }, UI.InkWell({ onClick: enabled ? onClick : void 0 }, UI.Box({ modifier: M.height(22).background(active ? c.primary.copy({ alpha: 0.15 }) : c.surface.copy({ alpha: 0.6 }), { cornerRadius: 4 }).border(1, active ? c.primary.copy({ alpha: 0.5 }) : c.outlineVariant.copy({ alpha: 0.25 }), { cornerRadius: 4 }).padding(6, 1), contentAlignment: "center" }, UI.Icon({ name, size: 14, tint: !enabled ? c.outlineVariant : active ? c.primary : c.outline }))));
    }
    function openerIcon(index, size2 = 14) {
      return index === 0 ? UI.Icon({ name: "folder", size: size2, tint: "#FFFFA000" }) : index === 3 ? UI.Icon({ name: "copy", size: size2 }) : UI.Svg({ source: index === 1 ? svgVsCode : svgTerminal, width: size2, height: size2 });
    }
    const labels = ["文件资源管理器", "Visual Studio Code", "终端 (Terminal)", "复制路径"];
    const openerMenu = UI.MenuAnchor({
      tooltip: "选择打开方式",
      itemHeight: 32,
      items: [{ enabled: false, height: 26 }, {}, {}, {}, { divider: true, height: 6 }, { height: 30 }],
      onSelect: (index) => {
        const chosen = index === 5 ? 3 : index - 1;
        if (chosen !== 3) setOpener(chosen);
        openTarget(chosen).catch(ctx.reportError);
      },
      anchor: UI.Box({ modifier: M.height(22).background(c.surface.copy({ alpha: 0.6 }), { cornerRadius: 4 }).border(1, c.outlineVariant.copy({ alpha: 0.3 }), { cornerRadius: 4 }).padding(7, 1), contentAlignment: "center" }, UI.Row({ verticalAlignment: "center" }, [openerIcon(opener), UI.Spacer({ width: 4 }), UI.Icon({ name: "arrow_drop_down", size: 14, tint: c.onSurfaceVariant })]))
    }, [text({ text: "打开方式", fontSize: 10.5, fontWeight: "700", color: "#FF9E9E9E" }), ...[0, 1, 2].map((index) => UI.Row({ verticalAlignment: "center" }, [openerIcon(index, 15), UI.Spacer({ width: 8 }), text({ text: labels[index], fontSize: 12 }), ...opener === index ? [UI.Box({ weight: 1 }), UI.Icon({ name: "check", size: 14 })] : []])), UI.Spacer({ height: 0 }), UI.Row({ verticalAlignment: "center" }, [openerIcon(3), UI.Spacer({ width: 8 }), text({ text: labels[3], fontSize: 12 })])]);
    const layout = UI.Box({ modifier: M.height(22).background(c.surface.copy({ alpha: 0.6 }), { cornerRadius: 4 }).border(1, c.outlineVariant.copy({ alpha: 0.25 }), { cornerRadius: 4 }).padding(2.5), contentAlignment: "center" }, UI.Row({ verticalAlignment: "center" }, [false, true].map((value) => UI.Tooltip({ message: value ? "分栏视图 (Split)" : "统一视图 (Unified)" }, UI.InkWell({ onClick: () => setSplit(value) }, UI.Box({ modifier: M.background(split === value ? c.primary.copy({ alpha: 0.15 }) : "transparent", { cornerRadius: 3 }).padding(5, 0), contentAlignment: "center" }, UI.Icon({ name: value ? "vertical_split_outlined" : "table_rows_outlined", size: 14, tint: split === value ? c.primary : c.outline })))))));
    return UI.Box(
      { height: 30, modifier: M.background(c.surfaceContainerHighest.copy({ alpha: 0.25 })).padding(6, 0) },
      UI.ScrollArea({ axis: "horizontal" }, UI.Row({ height: 30, width: Math.max(410, mainWidth - 12), verticalAlignment: "center", horizontalArrangement: "spaceBetween" }, [
        UI.Row({ verticalAlignment: "center" }, [
          navigation("view_sidebar_outlined", pane === "closed" ? "展开文件区" : "收起文件区", () => {
            if (pane === "closed") setPane(previousPane.current);
            else {
              previousPane.current = pane;
              setPane("closed");
            }
          }, true, pane === "closed" ? c.primary : c.onSurfaceVariant),
          UI.VerticalDivider({ width: 8, height: 14 }),
          navigation("arrow_back", "上一个改动 (P)", () => select(fileKey(rows[Math.max(0, currentIndex - 1)])), rows.length > 0),
          navigation("arrow_forward", "下一个改动 (N)", () => select(fileKey(rows[Math.min(rows.length - 1, currentIndex + 1)])), rows.length > 0),
          UI.VerticalDivider({ width: 8, height: 14 }),
          openerMenu
        ]),
        UI.Spacer({ width: 8 }),
        layout,
        UI.Spacer({ width: 6 }),
        toggle(full ? "unfold_more_outlined" : "compress_outlined", current === null ? "请选择文件后再切换完整视图" : full ? "仅显示改动附近的行 (收起未更改区域)" : "完整展示整个文件 (展开未更改区域)", () => {
          setError("");
          setFull(!full);
        }, full, current !== null),
        UI.Spacer({ width: 6 }),
        toggle("wrap_text_outlined", wrap ? "关闭换行 (不换行)" : "开启换行 (自动换行)", () => setWrap(!wrap), wrap)
      ]))
    );
  }
  function empty(message) {
    return UI.Box({ fillMaxSize: true, contentAlignment: "center" }, UI.Column({ horizontalAlignment: "center" }, [
      UI.Box({ width: 32, height: 32, contentAlignment: "center", modifier: M.border(1.2, c.outline.copy({ alpha: dark ? 0.35 : 0.5 }), { cornerRadius: 6 }) }, UI.Icon({ name: "add", size: 16, tint: c.outline.copy({ alpha: dark ? 0.65 : 0.8 }) })),
      UI.Spacer({ height: 12 }),
      text({ text: "尚无文件变更", fontSize: 13, fontWeight: "600", color: c.onSurface.copy({ alpha: dark ? 0.88 : 0.92 }) }),
      UI.Spacer({ height: 4 }),
      text({ text: message, fontSize: 11.5, color: c.outline.copy({ alpha: dark ? 0.65 : 0.8 }) })
    ]));
  }
  function ribbon(file) {
    const identity = fileIdentity(file.path);
    const status = "status" in file ? file.status : "M";
    const statusColor = status === "U" || status === "A" ? green : status === "D" ? red : status === "R" ? "#FFD29922" : "#FF58A6FF";
    return UI.Row({ height: 32, modifier: M.background(c.surface.copy({ alpha: 0.35 })).padding(10, 0), verticalAlignment: "center" }, [
      UI.Box({ modifier: M.background({ value: statusColor, alpha: 0.15 }, { cornerRadius: 3 }).padding(4, 1) }, text({ text: status, color: statusColor, fontSize: 10.5, fontWeight: "700" })),
      UI.Spacer({ width: 7 }),
      UI.Svg({ source: fileIcon(identity.name), width: 14, height: 14 }),
      UI.Spacer({ width: 7 }),
      UI.RichText({ weight: 1, maxLines: 1, overflow: "ellipsis", spans: [
        { text: identity.name, fontWeight: "700", fontSize: 12, color: c.onSurface },
        ...identity.directory === "" ? [] : [{ text: "  " }, { text: identity.directory, fontSize: 11.5, color: c.outline.copy({ alpha: 0.7 }) }]
      ] }),
      UI.Spacer({ width: 8 }),
      stat(file.additions, true),
      UI.Spacer({ width: 4 }),
      stat(file.deletions, false)
    ]);
  }
  function lineText(row) {
    const accent = row.kind === "add" ? dark ? "#FF3FB950" : "#FF1A7F37" : dark ? "#FFF85149" : "#FFCF222E";
    return UI.CodeText({
      text: row.text,
      filePath: current?.path ?? "",
      emphasis: row.segments.map((segment) => ({ ...segment, color: { value: accent, alpha: dark ? 0.4 : 0.32 }, fontWeight: "700" })),
      fontSize: 12,
      fontFamily: "Consolas",
      lineHeight: 1,
      softWrap: wrap,
      maxLines: wrap ? void 0 : 1,
      color: row.kind === "context" ? c.onSurface.copy({ alpha: 0.82 }) : c.onSurface
    });
  }
  function diffLine(row, width, added) {
    if (row === null) return UI.Box({ width, height: wrap ? void 0 : 22, modifier: M.heightIn(22).background({ ...c.surfaceContainerHighest.copy({ alpha: dark ? 0.2 : 0.12 }), over: c.surface }) });
    const kind = row.kind, changed = kind !== "context";
    const accent = kind === "add" ? dark ? "#FF3FB950" : "#FF1A7F37" : kind === "remove" ? dark ? "#FFF85149" : "#FFCF222E" : "transparent";
    const background = { value: accent, alpha: dark ? 0.2 : 0.12, over: c.surface };
    const number = added === false ? row.oldLineNumber : added === true ? row.newLineNumber : kind === "remove" ? row.oldLineNumber : row.newLineNumber;
    const gutter = UI.Box(
      { contentAlignment: "end", modifier: (wrap ? M.width(42) : M.width(42).height(22)).background(changed ? { value: accent, alpha: dark ? 0.1 : 0.07, over: c.surface } : { ...c.onSurface.copy({ alpha: dark ? 0.055 : 0.045 }), over: c.surface }).padding(0, 0, 8, 0) },
      text({ textAlign: "right", text: String(number ?? ""), fontSize: 11, lineHeight: 1, fontFamily: "Consolas", fontFamilyFallback: ["Cascadia Mono", "monospace"], color: changed ? { value: accent, alpha: dark ? 0.85 : 0.9 } : c.outline.copy({ alpha: 0.65 }) })
    );
    const content = UI.Row({ verticalAlignment: wrap ? "topStart" : "center", fillMaxWidth: true }, [
      ...!wrap ? [UI.Box({ width: 3, height: 22, modifier: M.background(accent) })] : [],
      gutter,
      UI.Spacer({ width: added === void 0 ? 8 : 6 }),
      UI.Box({ weight: 1, modifier: wrap ? M.padding(0, 3) : void 0 }, lineText(row))
    ]);
    if (wrap) return UI.Box({ modifier: M.width(width).heightIn(22).background(changed ? background : "transparent").borderSides({ start: { width: 3, color: accent } }).padding(3, 0, 0, 0) }, content);
    return UI.Box({ width, height: 22 }, [
      ...changed ? [UI.Canvas({ width, height: 22, commands: [{ type: "Rect", x: 0, y: 0, width, height: 23, color: background }] })] : [],
      content
    ]);
  }
  function expandGap(start) {
    setGaps([...gaps, start]);
  }
  function diffBody(file) {
    const document = ctx.useMemo("document", () => parseDiff(file.diffContent, source === "lastTurn"), [file.diffContent, source]);
    const items = ctx.useMemo("items", () => diffItems(document, split, fileLines, full ? null : new Set(gaps)), [document, split, fileLines, full, gaps]);
    if (items.length === 0) return empty("没有可显示的行级变更");
    const textWidth = document.maxLineLength * 7.8 + 48;
    const width = wrap ? mainWidth : Math.max(mainWidth, split ? 2 * (51 + textWidth) : 53 + textWidth);
    const usableWidth = width - 1, minimumRatio = Math.min(96 / usableWidth, 0.5);
    const ratio = Math.min(1 - minimumRatio, Math.max(minimumRatio, splitRatio));
    const leftWidth = usableWidth * ratio, rightWidth = usableWidth - leftWidth;
    function item(index) {
      const entry = items[index];
      if (entry.type === "line") return diffLine(entry.row, width);
      if (entry.type === "pair") return UI.Row({ width, verticalAlignment: wrap ? "topStart" : "center" }, [diffLine(entry.left, leftWidth, false), UI.Box({ width: 1, height: wrap ? 0 : 22, modifier: M.background(c.outlineVariant.copy({ alpha: 0.32 })) }), diffLine(entry.right, rightWidth, true)]);
      if (entry.type === "hunk") return UI.Box(
        { modifier: M.width(width).height(22).background({ ...c.onSurface.copy({ alpha: dark ? 0.055 : 0.045 }), over: c.surface }).padding(53, 0, 0, 0), contentAlignment: "start" },
        text({ text: entry.header, fontSize: 11, lineHeight: 1, color: c.outline.copy({ alpha: 0.9 }), fontFamily: "Consolas", fontFamilyFallback: ["Cascadia Mono", "monospace"], maxLines: 1 })
      );
      return UI.InkWell({ onClick: entry.expandable ? () => expandGap(entry.newStartLine) : void 0 }, UI.Row({ width, modifier: M.padding(0, 6), verticalAlignment: "center" }, [
        UI.Box({ weight: 1 }, line(c.outlineVariant.copy({ alpha: 0.4 }))),
        UI.Spacer({ width: 10 }),
        UI.Box(
          { modifier: M.background(c.surfaceContainerHighest.copy({ alpha: 0.7 }), { cornerRadius: 20 }).border(1, c.outlineVariant.copy({ alpha: 0.4 }), { cornerRadius: 20 }).padding(11, 4) },
          UI.Row({ verticalAlignment: "center" }, [UI.Icon({ name: entry.expandable ? "unfold_more" : "unfold_less", size: 12, tint: entry.expandable ? c.primary : c.outline }), UI.Spacer({ width: 5 }), text({ text: `${entry.count} 行未更改`, fontSize: 10.5, color: entry.expandable ? c.primary : c.outline })])
        ),
        UI.Spacer({ width: 10 }),
        UI.Box({ weight: 1 }, line(c.outlineVariant.copy({ alpha: 0.4 })))
      ]));
    }
    const lazy = ctx.lazyItems({ key: `diff:${source}:${file.path}:${split}:${wrap}`, itemCount: items.length, itemExtent: 22, initialItemCount: 48, variableExtent: true }, item);
    const list = UI.LazyColumn({ width, height: Math.max(1, size.height - 64), contentPadding: { top: 4, bottom: 24 }, ...lazy });
    const content = split ? UI.Box({ width, height: Math.max(1, size.height - 64), contentAlignment: "topStart" }, [
      list,
      UI.Box({ key: "diff-splitter", width: 12, fillMaxHeight: true, modifier: M.offset(leftWidth - 5.5, 0).cursor("resizeColumn").dragGestures({
        onDragStart: () => {
          splitDrag.current = ratio;
        },
        onDrag: (event) => {
          splitDrag.current = Math.min(1 - minimumRatio, Math.max(minimumRatio, splitDrag.current + event.deltaX / usableWidth));
          setSplitRatio(splitDrag.current);
        }
      }) })
    ]) : list;
    return width > mainWidth ? UI.ScrollArea({ axis: "horizontal", thumbVisibility: true, trackVisibility: true }, content) : content;
  }
  function filePreview() {
    if (preview === null) throw new Error("A preview file must be selected");
    const relative = workspace === null ? preview.path : preview.path.slice(workspace.replace(/\/$/, "").length + 1);
    return UI.Column({ fillMaxSize: true }, [
      UI.Column({ fillMaxWidth: true, horizontalAlignment: "start", modifier: M.padding(12, 8) }, [
        UI.Row({ verticalAlignment: "center", fillMaxWidth: true }, [UI.Icon({ name: "description_outlined", size: 16, tint: c.primary }), UI.Spacer({ width: 7 }), text({ text: fileIdentity(preview.path).name, style: "titleSmall", weight: 1, maxLines: 1, overflow: "ellipsis" })]),
        UI.Spacer({ height: 2 }),
        text({ text: relative, style: "bodySmall", color: c.outline, maxLines: 1, overflow: "ellipsis" })
      ]),
      line(),
      UI.Box({ weight: 1, fillMaxWidth: true }, previewLoading ? UI.Box({ fillMaxSize: true, contentAlignment: "center" }, UI.CircularProgressIndicator({ width: 18, height: 18, strokeWidth: 2 })) : previewError !== "" ? UI.Box({ modifier: M.padding(12) }, text({ text: previewError })) : UI.ScrollArea({}, UI.Box({ modifier: M.padding(12) }, UI.SelectionContainer({}, UI.CodeText({ text: preview.content, filePath: preview.path, style: "bodySmall", fontFamily: "monospace" })))))
    ]);
  }
  const noChanges = error || (!workspace ? "项目变更将显示在此处" : source === "git" ? "当前分支工作区无未提交的 Git 改动" : source === "branch" ? "当前分支与基准分支无差异或未检测到上游基准" : "项目变更将显示在此处");
  const body = preview !== null ? filePreview() : effective === null ? empty(noChanges) : loadingDiff ? UI.Box({ fillMaxSize: true, contentAlignment: "center" }, UI.CircularProgressIndicator({ width: 22, height: 22, strokeWidth: 2 })) : error !== "" ? empty(error) : diffBody(effective);
  const sidebar = pane === "tree" ? treeError !== "" ? UI.Box({ modifier: M.padding(12) }, text({ text: treeError })) : treeLoading ? UI.Box({ fillMaxSize: true, contentAlignment: "center" }, UI.CircularProgressIndicator({ width: 18, height: 18, strokeWidth: 2 })) : workspace === null || (tree[workspace] ?? []).length === 0 ? empty("工作区暂无目录或文件") : UI.LazyColumn({ contentPadding: { vertical: 4 } }, treeRows(workspace)) : rows.length === 0 ? empty(error || "没有匹配的改动文件") : UI.ScrollArea({ axis: "horizontal", thumbVisibility: true }, UI.LazyColumn({
    width: visibleChanges.reduce((width, { node, depth }) => Math.max(width, 70 + depth * 14 + node.name.length * 12 + (node.kind === "file" ? 24 + String(node.file.additions).length * 7 + String(node.file.deletions).length * 7 : 0)), sidebarWidth),
    height: Math.max(1, size.height - 80),
    contentPadding: { vertical: 2 }
  }, visibleChanges.map(changeRow)));
  return UI.Box({ modifier: M.fillMaxSize().onSizeChanged((next) => {
    if (next.width !== size.width || next.height !== size.height) setSize(next);
  }), contentAlignment: "topStart" }, [
    UI.Row({ fillMaxSize: true }, [
      ...pane === "closed" ? [] : [UI.Column({ width: sidebarWidth, fillMaxHeight: true, modifier: M.background(c.surfaceContainerHighest.copy({ alpha: dark ? 0.12 : 0.04 })) }, [
        sidebarHeader(),
        line(),
        UI.Box({ modifier: M.padding(8, 6) }, UI.Box({ modifier: M.height(27).background(c.surfaceContainerHighest.copy({ alpha: 0.35 }), { cornerRadius: 4 }).border(1, c.outlineVariant.copy({ alpha: 0.28 }), { cornerRadius: 4 }).padding(1) }, UI.Row({ verticalAlignment: "center" }, [
          UI.Box({ modifier: M.padding(7, 0, 5, 0) }, UI.Icon({ name: "search", size: 13, tint: c.outline.copy({ alpha: 0.7 }) })),
          UI.TextField({ weight: 1, value: query, onValueChange: setQuery, singleLine: true, cursorWidth: 1.2, cursorHeight: 13, placeholder: "筛选文件...", style: { inheritStyle: true, fontSize: 11.5, color: c.onSurface }, decoration: { isDense: true, borderless: true, contentPadding: { vertical: 5 }, hintStyle: { inheritStyle: true, fontSize: 11.5, color: c.outline.copy({ alpha: 0.5 }) } } }),
          ...query === "" ? [] : [UI.InkWell({ onClick: () => setQuery(""), shape: { cornerRadius: 3 } }, UI.Box({ modifier: M.padding(6, 4) }, UI.Icon({ name: "close", size: 12, tint: c.outline.copy({ alpha: 0.7 }) })))]
        ]))),
        line(),
        UI.Box({ weight: 1, fillMaxWidth: true }, sidebar)
      ]), UI.Box({ width: 1, fillMaxHeight: true, modifier: M.background(divider) })],
      UI.Column({ weight: 1, fillMaxHeight: true }, [toolbar(), line(), ...effective === null || preview !== null ? [] : [ribbon(effective), line(c.outlineVariant.copy({ alpha: 0.25 }))], UI.Box({ weight: 1, fillMaxWidth: true }, body)])
    ]),
    ...pane === "closed" ? [] : [UI.Box({ key: "sidebar-splitter", width: 12, fillMaxHeight: true, modifier: M.offset(sidebarWidth - 5.5, 0).cursor("resizeColumn").dragGestures({
      onDragStart: () => {
        listDrag.current = sidebarWidth;
      },
      onDrag: (event) => dragSidebar(event.deltaX)
    }) })]
  ]);
}
