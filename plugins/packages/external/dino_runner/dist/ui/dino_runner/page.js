"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ARCADE_BONUS_TOKEN_BASE64 = exports.ARCADE_RESOURCE_SPECS = exports.ARCADE_ROUTES = exports.ARCADE_BASE_URL = exports.ARCADE_HOST_INTERFACE_NAME = void 0;
exports.resolveArcadePathname = resolveArcadePathname;
exports.buildScoreCsv = buildScoreCsv;
exports.buildArcadeNoticeHtml = buildArcadeNoticeHtml;
exports.buildInlineBattleHtml = buildInlineBattleHtml;
exports.ARCADE_HOST_INTERFACE_NAME = "ArcadeHost";
exports.ARCADE_BASE_URL = "https://arcade.local";
const ARCADE_BASE_PREFIX = `${exports.ARCADE_BASE_URL}/`;
exports.ARCADE_ROUTES = {
    lobby: `${exports.ARCADE_BASE_URL}/arcade/lobby`,
    stage1: `${exports.ARCADE_BASE_URL}/arcade/stage-1`,
    legacyStage: `${exports.ARCADE_BASE_URL}/arcade/old-stage`,
    lockedDoor: `${exports.ARCADE_BASE_URL}/arcade/locked-door`,
    bossRush: `${exports.ARCADE_BASE_URL}/arcade/boss-rush`,
    offlineGallery: `${exports.ARCADE_BASE_URL}/offline/gallery`,
    scoreCsv: `${exports.ARCADE_BASE_URL}/downloads/scores.csv`,
    externalHub: "https://www.bilibili.com/",
};
exports.ARCADE_RESOURCE_SPECS = {
    appHtml: {
        key: "arcade_web_app_html",
        outputName: "arcade_app.html",
    },
    themeCss: {
        key: "arcade_web_theme_css",
        outputName: "arcade_theme.css",
    },
    consoleJs: {
        key: "arcade_web_console_js",
        outputName: "arcade_console.js",
    },
    spritePng: {
        key: "arcade_web_sprite_png",
        outputName: "chrome_dino_sprite.png",
    },
};
exports.ARCADE_BONUS_TOKEN_BASE64 = "PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHdpZHRoPSI5NiIgaGVpZ2h0PSI5NiIgdmlld0JveD0iMCAwIDk2IDk2IiBmaWxsPSJub25lIj4KICA8cmVjdCB3aWR0aD0iOTYiIGhlaWdodD0iOTYiIHJ4PSIyNCIgZmlsbD0iIzFGMUEzQSIvPgogIDxyZWN0IHg9IjE4IiB5PSIzNCIgd2lkdGg9IjYwIiBoZWlnaHQ9IjMwIiByeD0iMTUiIGZpbGw9IiM2RUU3RjkiIGZpbGwtb3BhY2l0eT0iMC4xOCIgc3Ryb2tlPSIjNkVFN0Y5IiBzdHJva2Utd2lkdGg9IjQiLz4KICA8Y2lyY2xlIGN4PSIzNCIgY3k9IjQ5IiByPSI2IiBmaWxsPSIjRkZEMTY2Ii8+CiAgPHJlY3QgeD0iMzEiIHk9IjQxIiB3aWR0aD0iNiIgaGVpZ2h0PSIxNiIgcng9IjMiIGZpbGw9IiNGRkQxNjYiLz4KICA8cmVjdCB4PSIyNiIgeT0iNDYiIHdpZHRoPSIxNiIgaGVpZ2h0PSI2IiByeD0iMyIgZmlsbD0iI0ZGRDE2NiIvPgogIDxjaXJjbGUgY3g9IjYwIiBjeT0iNDUiIHI9IjUiIGZpbGw9IiNGRjdBQTIiLz4KICA8Y2lyY2xlIGN4PSI2OSIgY3k9IjU0IiByPSI1IiBmaWxsPSIjQTc4QkZBIi8+CiAgPHBhdGggZD0iTTQxIDI2QzQzLjUgMjEgNDYgMTggNDggMThDNTAgMTggNTIuNSAyMSA1NSAyNiIgc3Ryb2tlPSIjNkVFN0Y5IiBzdHJva2Utd2lkdGg9IjQiIHN0cm9rZS1saW5lY2FwPSJyb3VuZCIvPgo8L3N2Zz4=";
function escapeHtml(value) {
    return String(value ?? "")
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#39;");
}
function resolveArcadePathname(url) {
    const trimmed = String(url || "").trim();
    if (!trimmed) {
        return null;
    }
    if (trimmed === exports.ARCADE_BASE_URL) {
        return "/";
    }
    if (!trimmed.startsWith(ARCADE_BASE_PREFIX)) {
        return null;
    }
    const suffix = trimmed.slice(exports.ARCADE_BASE_URL.length) || "/";
    const withoutHash = suffix.split("#", 1)[0] || "/";
    const withoutQuery = withoutHash.split("?", 1)[0] || "/";
    return withoutQuery.startsWith("/") ? withoutQuery : `/${withoutQuery}`;
}
function buildScoreCsv() {
    return [
        "lane,score,combo,rank",
        "大厅试玩,18,7,B",
        "第一关,29,11,A",
        "Boss 冲刺,41,14,S",
    ].join("\n");
}
function buildArcadeNoticeHtml(title, message) {
    return `
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="utf-8" />
    <meta
      name="viewport"
      content="width=device-width, initial-scale=1, viewport-fit=cover"
    />
    <title>${escapeHtml(title)}</title>
    <style>
      :root {
        color-scheme: dark;
        --bg: #0a1020;
        --panel: rgba(18, 24, 46, 0.92);
        --line: rgba(89, 208, 255, 0.22);
        --text: #f5f7ff;
        --muted: #b8c2e3;
        --accent: #59d0ff;
      }
      * { box-sizing: border-box; }
      body {
        margin: 0;
        min-height: 100vh;
        display: grid;
        place-items: center;
        padding: 20px;
        font-family: "Microsoft YaHei", "Segoe UI", sans-serif;
        color: var(--text);
        background:
          radial-gradient(circle at top left, rgba(89, 208, 255, 0.18), transparent 26%),
          radial-gradient(circle at bottom right, rgba(255, 166, 119, 0.15), transparent 24%),
          linear-gradient(180deg, #141a34, var(--bg));
      }
      .panel {
        width: min(760px, 100%);
        border-radius: 28px;
        border: 1px solid var(--line);
        background: var(--panel);
        padding: 28px;
        box-shadow: 0 28px 70px rgba(0, 0, 0, 0.38);
      }
      .eyebrow {
        color: var(--accent);
        letter-spacing: 0.18em;
        text-transform: uppercase;
        font-size: 12px;
        font-weight: 700;
      }
      h1 {
        margin: 12px 0 0;
        font-size: 32px;
      }
      p {
        margin: 14px 0 0;
        color: var(--muted);
        line-height: 1.7;
      }
      .nav {
        margin-top: 22px;
        display: flex;
        flex-wrap: wrap;
        gap: 10px;
      }
      .nav a {
        color: var(--text);
        text-decoration: none;
        border-radius: 999px;
        padding: 10px 14px;
        background: rgba(89, 208, 255, 0.12);
        border: 1px solid rgba(89, 208, 255, 0.18);
      }
    </style>
  </head>
  <body>
    <section class="panel">
      <div class="eyebrow">小游戏浏览器</div>
      <h1>${escapeHtml(title)}</h1>
      <p>${escapeHtml(message)}</p>
      <nav class="nav">
        <a href="${exports.ARCADE_ROUTES.lobby}">回到大厅</a>
        <a href="${exports.ARCADE_ROUTES.legacyStage}">试试旧入口改写</a>
        <a href="${exports.ARCADE_ROUTES.externalHub}">去外部站点</a>
      </nav>
    </section>
  </body>
</html>
  `.trim();
}
function buildInlineBattleHtml(interfaceName) {
    const interfaceNameJson = JSON.stringify(interfaceName);
    return `
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="utf-8" />
    <meta
      name="viewport"
      content="width=device-width, initial-scale=1, viewport-fit=cover"
    />
    <title>加时挑战页</title>
    <style>
      :root {
        color-scheme: dark;
        --bg: #0e1730;
        --panel: rgba(17, 24, 51, 0.94);
        --line: rgba(89, 208, 255, 0.2);
        --text: #f5f7ff;
        --muted: #b3bfde;
        --accent: #59d0ff;
        --accent2: #ffb457;
      }
      * { box-sizing: border-box; }
      body {
        margin: 0;
        padding: 18px;
        min-height: 100vh;
        font-family: "Microsoft YaHei", "Segoe UI", sans-serif;
        color: var(--text);
        background:
          radial-gradient(circle at top left, rgba(89, 208, 255, 0.16), transparent 24%),
          linear-gradient(180deg, #151d39, var(--bg));
      }
      .panel {
        max-width: 900px;
        margin: 0 auto;
        border-radius: 26px;
        border: 1px solid var(--line);
        background: var(--panel);
        padding: 22px;
      }
      .eyebrow {
        color: var(--accent);
        text-transform: uppercase;
        letter-spacing: 0.16em;
        font-size: 12px;
        font-weight: 700;
      }
      h1 {
        margin: 10px 0 0;
        font-size: 28px;
      }
      p {
        margin: 12px 0 0;
        color: var(--muted);
        line-height: 1.7;
      }
      textarea {
        width: 100%;
        margin-top: 16px;
        min-height: 140px;
        border-radius: 18px;
        border: 1px solid rgba(255, 255, 255, 0.1);
        background: rgba(255, 255, 255, 0.04);
        color: var(--text);
        padding: 14px 16px;
        font: inherit;
        resize: vertical;
      }
      .row {
        margin-top: 14px;
        display: flex;
        flex-wrap: wrap;
        gap: 10px;
      }
      button {
        border: 0;
        border-radius: 999px;
        padding: 11px 16px;
        font: inherit;
        font-weight: 700;
        cursor: pointer;
        color: #11172f;
        background: linear-gradient(135deg, var(--accent), #8a9bff);
      }
      button.alt {
        background: linear-gradient(135deg, var(--accent2), #ff8a7a);
      }
      pre {
        margin-top: 14px;
        border-radius: 16px;
        padding: 14px;
        background: rgba(255, 255, 255, 0.04);
        border: 1px solid rgba(255, 255, 255, 0.06);
        white-space: pre-wrap;
        word-break: break-word;
        color: var(--muted);
      }
    </style>
  </head>
  <body>
    <section class="panel">
      <div class="eyebrow">加时挑战</div>
      <h1>这一页是通过 loadHtml 临时插进来的</h1>
      <p>
        它不走资源目录里的主页面文件，但仍然能继续使用同一个宿主注入对象。
      </p>
      <textarea id="noteInput" placeholder="记一句加时赛笔记，比如“中路补刀最稳”。"></textarea>
      <div class="row">
        <button id="saveButton">把战报发给宿主</button>
        <button id="pingButton" class="alt">同步一条页面日志</button>
      </div>
      <pre id="resultBox">还没有触发任何加时页动作。</pre>
    </section>
    <script>
      (function() {
        var host = null;
        var noteInput = document.getElementById("noteInput");
        var resultBox = document.getElementById("resultBox");

        function setResult(value) {
          if (!resultBox) {
            return;
          }
          if (typeof value === "string") {
            resultBox.textContent = value;
            return;
          }
          try {
            resultBox.textContent = JSON.stringify(value, null, 2);
          } catch (_error) {
            resultBox.textContent = String(value);
          }
        }

        function describeError(error) {
          if (error && typeof error === "object" && typeof error.message === "string" && error.message) {
            return error.message;
          }
          var text = String(error || "").trim();
          return text || "未知错误";
        }

        function resolveHost() {
          var candidate = window[${interfaceNameJson}] || null;
          host = candidate || null;
          return host;
        }

        function hasHostApi(target) {
          return !!(
            target &&
            typeof target.savePlayerNote === "function" &&
            typeof target.pageLog === "function"
          );
        }

        function getSnapshot() {
          var noteLength =
            noteInput && "value" in noteInput ? String(noteInput.value || "").length : 0;
          return {
            lane: "加时挑战页",
            noteLength: noteLength
          };
        }

        setResult(
          hasHostApi(resolveHost())
            ? "宿主对象已注入，可直接调用方法。"
            : "宿主对象暂不可用"
        );

        document.getElementById("saveButton")?.addEventListener("click", function() {
          var activeHost = resolveHost();
          if (!activeHost || typeof activeHost.savePlayerNote !== "function") {
            setResult("宿主对象不可用");
            return;
          }
          try {
            setResult(
              activeHost.savePlayerNote({
                lane: "加时挑战页",
                note: noteInput && "value" in noteInput ? String(noteInput.value || "") : ""
              })
            );
          } catch (error) {
            setResult(describeError(error));
          }
        });

        document.getElementById("pingButton")?.addEventListener("click", function() {
          var activeHost = resolveHost();
          if (!activeHost || typeof activeHost.pageLog !== "function") {
            setResult("宿主对象不可用");
            return;
          }
          try {
            setResult(
              activeHost.pageLog({
                  lane: "加时挑战页",
                  noteLength: noteInput && "value" in noteInput ? String(noteInput.value || "").length : 0
                })
            );
          } catch (error) {
            setResult(describeError(error));
          }
        });
      })();
    </script>
  </body>
</html>
  `.trim();
}
