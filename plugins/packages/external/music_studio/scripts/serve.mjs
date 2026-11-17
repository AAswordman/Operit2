import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
const port = Number(process.env.PORT || 4178);
createServer(async (req, res) => { if (req.url?.split("?")[0] !== "/") { res.writeHead(404); res.end(); return; } try { const html = await readFile("resources/studio.html"); res.writeHead(200, { "Content-Type": "text/html; charset=utf-8", "Cache-Control": "no-store" }); res.end(html); } catch { res.writeHead(503); res.end("Run npm run build first."); } }).listen(port, "127.0.0.1", () => console.log(`Studio preview: http://127.0.0.1:${port}`));
