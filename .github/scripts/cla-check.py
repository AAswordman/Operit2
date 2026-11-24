#!/usr/bin/env python3
"""Operit2 CLA check bot.

Verifies that every commit author of a PR has signed per CLA.md section 7:
a standalone comment by the author containing the declaration
("我已阅读并同意 Operit2 贡献者许可协议（CLA）"), the CLA text commit SHA,
the signing scope and the signer name, with the SHA equal to the latest
commit touching CLA.md on main.

Actions:
- upserts a sticky status comment on the PR (marked with <!-- operit-cla-check -->);
- sets a commit status on the PR head (context: CLA). A failure state is
  advisory only; making it merge-blocking requires branch protection on main.
"""

import json
import os
import re
import sys
import urllib.error
import urllib.parse
import urllib.request

API = "https://api.github.com"
REPO = os.environ.get("REPO", "AAswordman/Operit2")
TOKEN = os.environ.get("GH_TOKEN", "")
EVENT_NAME = os.environ.get("EVENT_NAME", "")
PR_NUMBER = int(os.environ.get("PR_NUMBER", "0") or 0)
COMMENT_ID = os.environ.get("COMMENT_ID", "")
DRY_RUN = os.environ.get("DRY_RUN", "0") == "1"

MARKER = "<!-- operit-cla-check -->"
SIGN_PHRASE = "我已阅读并同意"
OWNER = REPO.split("/")[0]
SHA_RE = re.compile(r"CLA 文本提交[:：]\s*([0-9a-f]{40})")
EXCLUDED_LOGINS = {"web-flow"}


def api(method, path, data=None, params=None):
    url = API + path
    if params:
        url += "?" + urllib.parse.urlencode(params)
    req = urllib.request.Request(url, method=method)
    req.add_header("Authorization", f"Bearer {TOKEN}")
    req.add_header("Accept", "application/vnd.github+json")
    req.add_header("X-GitHub-Api-Version", "2022-11-28")
    body = None
    if data is not None:
        body = json.dumps(data).encode()
        req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req) as resp:
            raw = resp.read()
            return json.loads(raw) if raw else None
    except urllib.error.HTTPError as exc:
        detail = exc.read().decode("utf-8", "replace")
        print(f"API {method} {path} -> {exc.code}: {detail}", file=sys.stderr)
        raise


def paged(path, params=None):
    out, page = [], 1
    while True:
        p = dict(params or {})
        p["per_page"] = 100
        p["page"] = page
        batch = api("GET", path, params=p)
        if not batch:
            break
        out.extend(batch)
        if len(batch) < 100:
            break
        page += 1
    return out


def is_bot(login):
    return (not login) or login.endswith("[bot]") or login in EXCLUDED_LOGINS


def find_signature(comments, login):
    """Find login's valid signature comment. Returns SHA, "" (malformed), or None."""
    for c in comments:
        if c.get("user", {}).get("login") != login:
            continue
        body = c.get("body") or ""
        if SIGN_PHRASE not in body:
            continue
        if "CLA 文本提交" not in body or "签署范围" not in body or "签署人" not in body:
            continue
        m = SHA_RE.search(body)
        return m.group(1) if m else ""
    return None


def prior_signature(login, cla_sha):
    """Look for login's earlier signature in PRs they opened themselves.
    Returns (status, pr_number). Does not use the search API: the comment
    search index lags (the #515 signature comment was still unindexed 30
    minutes after posting), while reading the author's own PR list and
    checking comments directly is reliable and real-time."""
    try:
        issues = paged(f"/repos/{REPO}/issues",
                       params={"creator": login, "state": "all"})
    except Exception:
        return ("unknown", None)
    first_outdated = first_malformed = None
    scanned = 0
    for issue in issues:
        if "pull_request" not in issue:
            continue
        num = issue["number"]
        if num == PR_NUMBER:
            continue
        scanned += 1
        if scanned > 30:
            break
        try:
            comments = paged(f"/repos/{REPO}/issues/{num}/comments")
        except Exception:
            continue
        sha = find_signature(comments, login)
        if sha is None:
            continue
        if sha == "":
            if first_malformed is None:
                first_malformed = num
            continue
        if cla_sha and sha != cla_sha:
            if first_outdated is None:
                first_outdated = num
            continue
        return ("signed", num)
    if first_outdated is not None:
        return ("outdated", first_outdated)
    if first_malformed is not None:
        return ("malformed", first_malformed)
    return ("missing", None)


def label(status, where):
    if status == "signed":
        return "✅ 已签署（本 PR）" if where is None else f"✅ 已签署（历史记录，PR #{where}）"
    if status == "outdated":
        return "⚠️ 已签署旧版本，需按当前 CLA 重新签署" if where is None else f"⚠️ 已签署旧版本（PR #{where}），需按当前 CLA 重新签署"
    if status == "malformed":
        return "⚠️ 签署评论格式不完整（缺声明、SHA、范围或签署人）"
    if status == "unknown":
        return "❓ 历史签署记录查询失败，请人工核对"
    return "❌ 未签署"


def main():
    if not TOKEN:
        print("GH_TOKEN is required", file=sys.stderr)
        return 1
    if not PR_NUMBER:
        print("PR_NUMBER is required", file=sys.stderr)
        return 1

    # issue_comment 事件：只响应人类在 PR 上的新评论，忽略机器人与本机器人自己的评论。
    if EVENT_NAME == "issue_comment" and COMMENT_ID:
        try:
            comment = api("GET", f"/repos/{REPO}/issues/comments/{COMMENT_ID}")
        except Exception:
            return 0
        user = comment.get("user", {}).get("login", "")
        body = comment.get("body") or ""
        if is_bot(user) or MARKER in body:
            print("ignoring bot/own comment")
            return 0
        try:
            issue = api("GET", f"/repos/{REPO}/issues/{PR_NUMBER}")
        except Exception:
            return 0
        if "pull_request" not in issue:
            print("comment is not on a PR")
            return 0

    pr = api("GET", f"/repos/{REPO}/pulls/{PR_NUMBER}")
    head_sha = pr["head"]["sha"]
    pr_url = pr["html_url"]

    cla_sha = ""
    try:
        commits = api("GET", f"/repos/{REPO}/commits",
                      params={"path": "CLA.md", "sha": "main", "per_page": 1})
        if commits:
            cla_sha = commits[0]["sha"]
    except Exception:
        pass

    authors = set()
    unlinked = set()
    if not is_bot(pr["user"]["login"]) and pr["user"]["login"] != OWNER:
        authors.add(pr["user"]["login"])
    pr_commits = paged(f"/repos/{REPO}/pulls/{PR_NUMBER}/commits")
    for c in pr_commits:
        login = (c.get("author") or {}).get("login")
        if login:
            if not is_bot(login) and login != OWNER:
                authors.add(login)
        else:
            name = (c.get("commit") or {}).get("author", {}).get("name")
            if name:
                unlinked.add(name)

    comments = paged(f"/repos/{REPO}/issues/{PR_NUMBER}/comments")

    results = {}
    for login in sorted(authors):
        sha = find_signature(comments, login)
        if sha is None:
            status, ref = prior_signature(login, cla_sha)
            results[login] = (status, ref)
        elif sha == "":
            results[login] = ("malformed", None)
        elif cla_sha and sha != cla_sha:
            results[login] = ("outdated", None)
        else:
            results[login] = ("signed", None)

    lines = [MARKER, "## CLA 签署状态", ""]
    if cla_sha:
        lines.append(f"当前 CLA 版本：`{cla_sha[:7]}`（main 上 `CLA.md` 的最新提交）")
    else:
        lines.append("当前 CLA 版本：无法读取（请检查 `CLA.md` 是否存在）")
    lines.append("")
    if not results and not unlinked:
        lines.append("无需签署：本 PR 的作者为仓库维护者或机器人。")
    else:
        lines.append("| 作者 | 状态 |")
        lines.append("|---|---|")
        for login in sorted(results):
            status, where = results[login]
            lines.append(f"| @{login} | {label(status, where)} |")
        if unlinked:
            lines.append("")
            lines.append("无法关联 GitHub 账号的提交作者（请人工核对是否已签署）：")
            for name in sorted(unlinked):
                lines.append(f"- {name}")
        lines.append("")
        lines.append("### 如何签署")
        lines.append(f"按 [CLA.md](https://github.com/{REPO}/blob/main/CLA.md) 第 7 条，在本 PR 发表一条独立评论：")
        lines.append("")
        lines.append("```text")
        lines.append("我已阅读并同意 Operit2 贡献者许可协议（CLA）1.0，确认有权作出其中的授权，并同意其适用于下列贡献及我此后向项目提交的贡献。")
        lines.append(f"CLA 文本提交：{cla_sha or '填写包含所阅读 CLA.md 的完整 Git 提交 SHA'}")
        lines.append("签署范围：填写当前 PR 链接；如包含历史贡献，请逐项列出 PR 链接或提交 SHA")
        lines.append("签署人：填写姓名；代表组织签署时，同时填写组织全称及代表身份")
        lines.append("```")
        lines.append("")
        lines.append("签署后本机器人会自动重新核验；签署记录以评论为准，勾选 PR 模板不代替签署。")
    body = "\n".join(lines) + "\n"

    all_signed = all(s == "signed" for s, _ in results.values())
    state = "success" if all_signed and not unlinked else "failure"
    if not results and not unlinked:
        desc = "无需签署 CLA"
    elif unlinked and all_signed:
        desc = "存在无法关联账号的提交作者，请人工核对"
    elif all_signed:
        desc = "所有作者均已签署 CLA"
    else:
        n = sum(1 for s, _ in results.values() if s != "signed")
        desc = f"{n} 位作者尚未签署 CLA"

    if DRY_RUN:
        print(f"=== status: {state} | {desc} ===")
        print(body)
        return 0

    existing = None
    for c in comments:
        if MARKER in (c.get("body") or "") and (c.get("user", {}) or {}).get("login", "").endswith("[bot]"):
            existing = c
            break
    if existing:
        api("PATCH", f"/repos/{REPO}/issues/comments/{existing['id']}", {"body": body})
    else:
        api("POST", f"/repos/{REPO}/issues/{PR_NUMBER}/comments", {"body": body})
    api("POST", f"/repos/{REPO}/statuses/{head_sha}",
        {"state": state, "context": "CLA", "description": desc[:140], "target_url": pr_url})
    print(f"CLA check done: {state} ({desc})")
    return 0


if __name__ == "__main__":
    sys.exit(main())
