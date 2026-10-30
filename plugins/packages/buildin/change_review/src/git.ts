import { lineStats, type ReviewFile } from './model';

/** Captures the unmodified output of the host's argument-vector executor. */
export interface CommandOutput { exitCode: number | null; stdout: string; stderr: string; workingDirectory: string }
export type RunCommand = (workspace: string, program: string, args: string[]) => Promise<CommandOutput>;
export interface ResolvedPath { vfsPath: string; nativePath: string }
export interface GitFile extends ReviewFile { root: string; workspace: string; status: string; untracked: boolean; originalPath: string | null; base: string | null; unborn: boolean; loaded?: boolean }

/** Runs through the existing authenticated core-command transport, never a local UI subprocess. */
export async function runCommand(workspace: string, program: string, args: string[]): Promise<CommandOutput> {
  return JSON.parse(await Tools.SoftwareSettings.exec(['--json', 'workspace', 'exec', workspace, program, ...args])) as CommandOutput;
}

/** Resolves locator types explicitly through the runtime mapper, including registered mounts. */
export async function resolvePath(path: string, native = false): Promise<ResolvedPath> {
  return JSON.parse(await Tools.SoftwareSettings.exec(['--json', 'workspace', 'resolve-path', ...(native ? ['--native'] : []), path])) as ResolvedPath;
}

/** Resolves tool-result resource locators at the runtime boundary, including relative and native paths. */
export async function resolveResource(workspace: string, path: string): Promise<ResolvedPath> {
  return JSON.parse(await Tools.SoftwareSettings.exec(['--json', 'workspace', 'resolve-path', '--from', workspace, path])) as ResolvedPath;
}

/** Delivers a platform-neutral external-open intent to the runtime's registered host. */
export async function openPath(path: string, action: 'reveal' | 'terminal' | 'program', programArgs: string[] = []): Promise<void> {
  await Tools.SoftwareSettings.exec(['--json', 'workspace', 'open-path', path, action, ...programArgs]);
}

/** Rejects unsuccessful commands without swallowing diagnostics or trying another executable. */
export async function git(run: RunCommand, workspace: string, args: string[]): Promise<string> {
  const result = await run(workspace, 'git', ['--no-optional-locks', ...args]);
  if (result.exitCode !== 0) throw new Error(`git ${args[0]} (${result.exitCode}): ${result.stderr.trim()}`);
  return result.stdout;
}

/** Preserves NUL-delimited filenames and the destination-first porcelain rename format. */
export function parseStatus(text: string): { path: string; status: string; untracked: boolean; originalPath: string | null }[] {
  const records = text.split('\0');
  const result: { path: string; status: string; untracked: boolean; originalPath: string | null }[] = [];
  for (let index = 0; index < records.length; index++) {
    const record = records[index];
    if (record.length === 0) continue;
    if (record.length < 4 || record[2] !== ' ') throw new Error('Invalid git status record');
    const flags = record.slice(0, 2);
    const renamed = /[RC]/.test(flags);
    const originalPath = renamed ? records[++index] : null;
    if (renamed && !originalPath) throw new Error('Missing git rename source');
    const status = flags === '??' || flags === 'AA' || flags === 'DD' || /U/.test(flags) ? 'U'
      : /A/.test(flags) ? 'A' : /D/.test(flags) ? 'D' : renamed ? 'R' : 'M';
    result.push({ path: record.slice(3), status, untracked: flags === '??', originalPath });
  }
  return result;
}

/** Reads branch name-status output without interpreting tabs or newlines inside paths. */
export function parseNameStatus(text: string): { path: string; status: string; untracked: boolean; originalPath: string | null }[] {
  const records = text.split('\0');
  const result: { path: string; status: string; untracked: boolean; originalPath: string | null }[] = [];
  for (let index = 0; index < records.length; index++) {
    const flag = records[index];
    if (flag === '') continue;
    if (!/^[ACDMRTUXB]\d*$/.test(flag)) throw new Error('Invalid git name-status record');
    const originalPath = /^[RC]/.test(flag) ? records[++index] : null;
    const path = records[++index];
    if (!path || originalPath === '') throw new Error('Missing git name-status path');
    result.push({ path, status: flag[0] === 'C' ? 'R' : flag[0], untracked: false, originalPath });
  }
  return result;
}

/** Parses machine-readable numstat records, including both paths of a rename. */
export function parseNumstat(text: string): Map<string, { additions: number; deletions: number }> {
  const records = text.split('\0'), result = new Map<string, { additions: number; deletions: number }>();
  for (let index = 0; index < records.length; index++) {
    if (records[index] === '') continue;
    const match = /^(\d+|-)\t(\d+|-)\t([\s\S]*)$/.exec(records[index]);
    if (match === null) throw new Error('Invalid git numstat record');
    let path = match[3];
    if (path === '') { index++; path = records[++index]; }
    if (!path) throw new Error('Missing git numstat path');
    result.set(path, { additions: match[1] === '-' ? 0 : Number(match[1]), deletions: match[2] === '-' ? 0 : Number(match[2]) });
  }
  return result;
}

/** Selects the declared upstream or conventional base from one complete ref inventory, without command retries. */
export async function comparisonBase(workspace: string, run: RunCommand): Promise<string> {
  const inventory = await git(run, workspace, ['for-each-ref', '--format=%(refname)%00%(HEAD)%00%(upstream)']);
  const refs = new Set<string>();
  let upstream: string | null = null;
  for (const record of inventory.split('\n')) {
    if (record === '') continue;
    const [name, current, tracking] = record.replace(/\r$/, '').split('\0');
    refs.add(name);
    if (current === '*' && tracking !== '') upstream = tracking;
  }
  if (upstream !== null) {
    if (!refs.has(upstream)) throw new Error(`The configured upstream does not exist: ${upstream}`);
    return upstream;
  }
  const conventional = ['refs/remotes/origin/main', 'refs/heads/main', 'refs/remotes/origin/master', 'refs/heads/master'].find(ref => refs.has(ref));
  if (conventional !== undefined) return conventional;
  const history = (await git(run, workspace, ['rev-list', '--max-count=2', 'HEAD'])).trim().split('\n');
  if (history.length < 2) throw new Error('This repository has no branch comparison base');
  return history[1];
}

/** Loads one repository catalog with explicit handling of its initial, unborn HEAD state. */
export async function loadChanges(workspace: string, source: 'git' | 'branch', base: string | null, run: RunCommand = runCommand): Promise<GitFile[]> {
  const root = (await git(run, workspace, ['rev-parse', '--show-toplevel'])).replace(/\r?\n$/, '');
  const head = await run(workspace, 'git', ['--no-optional-locks', 'rev-parse', '--verify', '--quiet', 'HEAD']);
  if (head.exitCode !== 0 && head.exitCode !== 1) throw new Error(`Cannot inspect HEAD: ${head.stderr}`);
  const unborn = head.exitCode === 1;
  if (source === 'branch' && unborn) return [];
  const selectedBase = source === 'branch' ? base ?? await comparisonBase(workspace, run) : null;
  const revision = source === 'branch' ? `${selectedBase}...HEAD` : 'HEAD';
  const [status, numstat] = await Promise.all([
    git(run, workspace, ['-C', root, ...(source === 'branch' ? ['diff', '--name-status', '-z', revision, '--'] : ['status', '--porcelain=v1', '-uall', '-z'])]),
    unborn ? Promise.resolve('') : git(run, workspace, ['-C', root, 'diff', '--numstat', '-z', revision, '--']),
  ]);
  const stats = parseNumstat(numstat);
  const entries = source === 'branch' ? parseNameStatus(status) : parseStatus(status);
  return entries.map(entry => ({ ...entry, root, workspace, base: selectedBase, unborn, diffContent: '',
    additions: stats.get(entry.path)?.additions ?? 0, deletions: stats.get(entry.path)?.deletions ?? 0 }));
}

/** Discovers immediate mounted repositories through VFS entries and deduplicates their canonical Git roots. */
export async function loadWorkspaceChanges(workspace: string, source: 'git' | 'branch', base: string | null): Promise<GitFile[]> {
  const listing = await Tools.Files.list(workspace);
  const ownRepository = listing.entries.some(entry => entry.name === '.git');
  const children = listing.entries.filter(entry => entry.isDirectory && !entry.name.startsWith('.'));
  const candidates = await Promise.all(children.map(async entry => {
    const path = `${workspace.replace(/\/$/, '')}/${entry.name}`;
    return (await Tools.Files.list(path)).entries.some(entry => entry.name === '.git') ? path : null;
  }));
  const mounted = candidates.filter((path): path is string => path !== null);
  const paths = ownRepository || mounted.length === 0 ? [workspace, ...mounted] : mounted;
  const repositories = await Promise.all(paths.map(path => loadChanges(path, source, base)));
  const seen = new Set<string>();
  return repositories.flatMap(files => {
    if (files.length === 0) return [];
    const root = files[0].root;
    if (seen.has(root)) return [];
    seen.add(root);
    return files;
  });
}

/** Loads the exact selected comparison and keeps untracked files independent of platform null devices. */
export async function loadFileDiff(workspace: string, file: GitFile, readUntracked: () => Promise<string>, run: RunCommand = runCommand): Promise<GitFile> {
  let diffContent: string;
  if (file.untracked || file.unborn) {
    const content = await readUntracked();
    const lines = content.length === 0 ? [] : content.replace(/\n$/, '').split('\n');
    diffContent = lines.length === 0 ? '' : `--- /dev/null\n+++ b/${file.path}\n@@ -0,0 +1,${lines.length} @@\n${lines.map(line => `+${line}`).join('\n')}`;
  } else {
    diffContent = await git(run, file.workspace, ['-C', file.root, 'diff', '--unified=3', '--no-ext-diff', '--no-textconv',
      file.base === null ? 'HEAD' : `${file.base}...HEAD`, '--', ...(file.originalPath === null ? [] : [file.originalPath]), file.path]);
  }
  return { ...file, diffContent, ...lineStats(diffContent), loaded: true };
}
