/** Host-observed Git facts. This does not establish that tests or PR claims are true. */
export interface ReviewedSnapshot { head: string; tree: string }
export interface GitSnapshot { head: string; parent: string; reviewedTree: string; trackedClean: boolean; untrackedClean: boolean }
type Run = (command: string) => Promise<{output: string; exitCode: number; timedOut: boolean}>;

/** POSIX shell quoting; project paths are data and cannot become shell syntax. */
function quote(value: string): string { return "'" + value.replace(/'/g, "'\\''") + "'"; }
function objectId(value: string): boolean { return /^(?:[0-9a-f]{40}|[0-9a-f]{64})$/.test(value); }
export class GitVerifier {
  constructor(private readonly run: Run) {}
  private queue: Promise<void> = Promise.resolve();
  private async execute(command: string): Promise<string> {
    // Hidden executors may use an interactive shell. Delimit output and serialize
    // commands sharing its session so prompts and concurrent output cannot mix.
    const begin = "OPERIT_MAINTENANCE_GIT_BEGIN", end = "OPERIT_MAINTENANCE_GIT_END";
    const wrapped = `printf '\\n%s\\n' '${begin}'; ${command}; opm_git_status=$?; printf '\\n%s\\n' '${end}'; test "$opm_git_status" -eq 0`;
    const pending = this.queue.then(() => this.run(wrapped));
    this.queue = pending.then(() => {}, () => {});
    const result = await pending;
    if (result.timedOut || result.exitCode !== 0) throw new Error("Git verification command failed or timed out");
    const normalized = result.output.replace(/\r\n/g, "\n");
    const match = new RegExp(`(?:^|\\n)${begin}\\n([\\s\\S]*?)\\n${end}(?:\\n|$)`).exec(normalized);
    if (match === null) throw new Error("Git verification output markers are missing");
    return match[1].trim();
  }
  private path(path: string): string {
    if (!path.startsWith("/") || path.includes("\0") || path.includes("\n") || path.includes("\r")) throw new Error("Git verification requires an absolute POSIX project path");
    return quote(path);
  }
  private async git(path: string, args: string): Promise<string> { return this.execute(`git -C ${this.path(path)} ${args}`); }
  async head(path: string): Promise<string> {
    const head = await this.git(path, "rev-parse --verify 'HEAD^{commit}'");
    if (!objectId(head)) throw new Error("Git returned an invalid commit ID");
    return head;
  }
  /** Refuse to begin with work belonging to someone else or from a subdirectory. */
  async cleanHead(path: string): Promise<string> {
    if (await this.git(path, "rev-parse --show-prefix") !== "") throw new Error("目标路径必须是 Git 仓库根目录");
    const before = await this.head(path);
    const status = await this.git(path, "status --porcelain=v1 --untracked-files=all --ignore-submodules=none");
    if (status !== "") throw new Error("仓库有已跟踪或未跟踪的未提交改动，请先妥善保存并清理工作区，再重试");
    if (await this.head(path) !== before) throw new Error("检查仓库基线时 Git HEAD 发生变化");
    return before;
  }
  /** Snapshot the worktree including non-ignored new files, without changing the real index. */
  async snapshot(path: string): Promise<ReviewedSnapshot> {
    const directory = this.path(path);
    const output = await this.execute(`( set -e; opm_dir=$(mktemp -d "${'${TMPDIR:-/tmp}'}/operit-review.XXXXXXXX"); trap 'rm -rf "$opm_dir"' EXIT; export GIT_INDEX_FILE="$opm_dir/index"; git -C ${directory} rev-parse --verify 'HEAD^{commit}'; git -C ${directory} read-tree HEAD; git -C ${directory} add -A -- .; git -C ${directory} write-tree )`);
    const parts = output.split("\n");
    if (parts.length !== 2 || !parts.every(objectId)) throw new Error("Git returned an invalid worktree snapshot");
    return {head: parts[0], tree: parts[1]};
  }
  /** Verify a reported GitHub PR against the configured origin and the verified commit. */
  async verifyPullRequest(path: string, url: string, head: string): Promise<{url: string; number: number; head: string; state: "OPEN"}> {
    if (!objectId(head)) throw new Error("Invalid verified Git commit ID");
    const match = /^https:\/\/github\.com\/([A-Za-z0-9_.-]+)\/([A-Za-z0-9_.-]+)\/pull\/([1-9][0-9]*)\/?$/.exec(url);
    if (match === null || match[1] === "." || match[1] === ".." || match[2] === "." || match[2] === "..") throw new Error("Unsupported or invalid GitHub pull request URL");
    const number = Number(match[3]);
    if (!Number.isSafeInteger(number)) throw new Error("Invalid pull request number");
    const origin = await this.git(path, "remote get-url origin");
    const remote = /^(?:https:\/\/github\.com\/|git@github\.com:|ssh:\/\/git@github\.com\/)([A-Za-z0-9_.-]+)\/([A-Za-z0-9_.-]+?)(?:\.git)?\/?$/.exec(origin);
    if (remote === null || remote[1].toLowerCase() !== match[1].toLowerCase() || remote[2].toLowerCase() !== match[2].toLowerCase()) throw new Error("Pull request repository does not match the configured GitHub origin");
    const repository = `github.com/${match[1]}/${match[2]}`;
    const data = await this.execute(`(cd ${this.path(path)} && gh pr view ${number} --repo ${quote(repository)} --json url,number,state,headRefOid)`);
    let value: unknown;
    try { value = JSON.parse(data); } catch { throw new Error("GitHub returned an invalid pull request response"); }
    if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error("GitHub returned an invalid pull request response");
    const result = value as Record<string, unknown>;
    if (typeof result.url !== "string" || result.url.replace(/\/$/, "").toLowerCase() !== url.replace(/\/$/, "").toLowerCase() || result.number !== number || result.state !== "OPEN" || result.headRefOid !== head) throw new Error("Pull request is not open at the verified commit");
    return {url: result.url, number, head, state: "OPEN"};
  }
  async verify(path: string, reviewed: ReviewedSnapshot): Promise<GitSnapshot> {
    if (!objectId(reviewed.head) || !objectId(reviewed.tree)) throw new Error("Invalid reviewed Git snapshot");
    const head = await this.head(path);
    if (head === reviewed.head) throw new Error("Git HEAD did not change during delivery");
    const lineage = (await this.git(path, "rev-list --parents -n 1 HEAD")).split(" ");
    if (lineage.length !== 2 || lineage[0] !== head || lineage[1] !== reviewed.head) throw new Error("Delivered commit is not a direct single-parent child of the reviewed HEAD");
    const tree = await this.git(path, "rev-parse --verify 'HEAD^{tree}'");
    if (tree !== reviewed.tree) throw new Error("Delivered commit does not match the independently reviewed worktree");
    const status = await this.git(path, "status --porcelain=v1 --untracked-files=all --ignore-submodules=none");
    if (status !== "") throw new Error("Tracked or untracked changes remain after delivery");
    return {head, parent: reviewed.head, reviewedTree: tree, trackedClean: true, untrackedClean: true};
  }
}
