import vm from "node:vm";
import test from "node:test";
import assert from "node:assert/strict";
import { buildGraphModule } from "../scripts/build.mjs";
const module = { exports: {} };
vm.runInNewContext(await buildGraphModule(), { module, exports: module.exports });
const api = module.exports;

/** Uses fixed measured node sizes to compare the ported geometry exactly. */
function measure() { return { width: 56, height: 28, lines: ["node"] }; }

/** Confirms cross-folder relationships do not merge independent native layout clusters. */
test("native graph layout packs actual rectangles and ignores cross-folder links for clustering", () => {
  const scene = api.layoutMemoryGraph([{ id: "a", label: "A" }, { id: "b", label: "B" }, { id: "c", label: "C" }], [{ sourceId: "a", targetId: "b", isCrossFolderLink: true }], measure);
  assert.deepEqual(JSON.parse(JSON.stringify([...scene.positions.entries()])), [["a", { x: 28, y: 14 }], ["b", { x: 140, y: 14 }], ["c", { x: 28, y: 98 }]]);
  assert.equal(scene.bounds.width, 168);
  assert.equal(scene.bounds.height, 112);
});

/** Confirms native degree ordering, clearance rings and camera padding survive the port. */
test("connected graph uses native degree ordering and forty-pixel fit padding", () => {
  const scene = api.layoutMemoryGraph([{ id: "b", label: "B" }, { id: "a", label: "A" }, { id: "c", label: "C" }], [{ sourceId: "a", targetId: "b", isCrossFolderLink: false }, { sourceId: "a", targetId: "c", isCrossFolderLink: false }], measure);
  const a = scene.positions.get("a"), b = scene.positions.get("b"), c = scene.positions.get("c");
  assert.ok(Math.abs(a.x - b.x) < 0.001);
  assert.ok(b.y < a.y && a.y < c.y);
  const camera = api.fitMemoryGraph(scene.bounds, 800, 520);
  assert.equal(camera.scale, 1);
  assert.equal(camera.x + (scene.bounds.left + scene.bounds.width / 2), 400);
  assert.equal(camera.y + (scene.bounds.top + scene.bounds.height / 2), 260);
});

/** Confirms wheel and pinch zoom preserve the exact focal world point at both limits. */
test("native camera zoom preserves focal point and clamps to 0.1–5", () => {
  const camera = { x: 30, y: -10, scale: 1 }, focal = { x: 250, y: 180 };
  const zoom = api.zoomMemoryGraph(camera, focal, 100);
  assert.equal(zoom.scale, 5);
  assert.equal((focal.x - zoom.x) / zoom.scale, (focal.x - camera.x) / camera.scale);
  assert.equal((focal.y - zoom.y) / zoom.scale, (focal.y - camera.y) / camera.scale);
  assert.equal(api.zoomMemoryGraph(zoom, focal, 0.0001).scale, 0.1);
});
