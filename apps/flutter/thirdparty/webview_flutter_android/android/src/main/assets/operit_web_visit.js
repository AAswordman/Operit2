(function () {
  "use strict";
  const clean = value => String(value || "").replace(/\u00a0/g, " ")
    .split(/\r?\n/).map(line => line.trim()).filter(Boolean).join("\n");
  const excluded = new Set(["SCRIPT", "STYLE", "NOSCRIPT", "TEMPLATE", "SVG", "CANVAS"]);
  const blocks = new Set(["ADDRESS", "ARTICLE", "ASIDE", "BLOCKQUOTE", "BR", "DIV", "DL", "DT", "DD",
    "FIGCAPTION", "FIGURE", "FOOTER", "FORM", "H1", "H2", "H3", "H4", "H5", "H6", "HEADER",
    "HR", "LI", "MAIN", "NAV", "OL", "P", "PRE", "SECTION", "TABLE", "TR", "UL"]);
  // DOM traversal does not require painted frames or innerText's rendering-dependent behavior.
  const text = root => {
    const chunks = [];
    const walk = node => {
      if (node.nodeType === 3) { chunks.push(node.nodeValue); return; }
      if (node.nodeType !== 1 || excluded.has(node.tagName) || node.hidden ||
          node.getAttribute("aria-hidden") === "true") return;
      const style = getComputedStyle(node);
      if (style.display === "none" || style.visibility === "hidden" || style.visibility === "collapse") return;
      const block = blocks.has(node.tagName);
      if (block) chunks.push("\n");
      for (const child of node.childNodes) walk(child);
      if (block) chunks.push("\n");
      else if (node.tagName === "TD" || node.tagName === "TH") chunks.push("\t");
    };
    if (root) walk(root);
    return clean(chunks.join(""));
  };
  const absolute = value => {
    if (!value) return "";
    try {
      const url = new URL(value, document.baseURI);
      return url.protocol === "http:" || url.protocol === "https:" ? url.href : "";
    } catch (_) { return ""; }
  };
  const content = text(document.body);
  const metadata = {};
  for (const meta of document.querySelectorAll("meta")) {
    const key = meta.getAttribute("name") || meta.getAttribute("property");
    const value = meta.getAttribute("content");
    if (key && value && !Object.prototype.hasOwnProperty.call(metadata, key))
      Object.defineProperty(metadata, key, { value, enumerable: true });
  }
  const links = [], seen = new Set();
  for (const anchor of document.querySelectorAll("a[href]")) {
    const url = absolute(anchor.getAttribute("href"));
    const label = text(anchor) || clean(anchor.getAttribute("aria-label") || anchor.getAttribute("title"));
    if (!url || !label || seen.has(url + "\n" + label)) continue;
    seen.add(url + "\n" + label);
    links.push({ url, text: label });
  }
  const imageLinks = [], images = new Set();
  for (const image of document.querySelectorAll("img")) {
    const url = absolute(image.currentSrc || image.getAttribute("src") || image.getAttribute("data-src"));
    if (url && !images.has(url)) { images.add(url); imageLinks.push(url); }
  }
  return JSON.stringify({
    ready: document.readyState === "complete" && !!document.body,
    hasDocument: !!document.body,
    captcha: !!document.querySelector('iframe[src*="recaptcha"], iframe[src*="hcaptcha"], iframe[src*="challenges.cloudflare.com"], .g-recaptcha, .h-captcha, .cf-turnstile, #challenge-form, #challenge-running'),
    result: { url: location.href, title: clean(document.title), content, metadata, links, imageLinks }
  });
})()
