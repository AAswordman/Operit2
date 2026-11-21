/* Slice generated sources in memory and retain only one animated asset per state. */
const fs = require('node:fs');
const path = require('node:path');
const sharp = require('C:/Users/12809/.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/sharp');

const root = __dirname;
const generated = 'C:/Users/12809/.codex/generated_images/01a1fea9-524d-7b90-a1b3-cb16ebd36fa0';
const sources = {
  typing: 'call_16f4d29692254163885164c793568521.png',
  thinking: 'call_5c53f134b36242f0a010aecb0a9bdfcb.png',
  idle: 'call_3fff92ac2e294639a5adedd9828813ce.png',
  approval: 'call_f175a655e7be412b87cd11e1712fadff.png',
  completed: 'call_6f722f27ff414fb3b921d25dc4183a7f.png',
  error: 'call_2ec1b1f42966407784de6fdaae636bfb.png',
};

/** Expand drawing exposures into a nominal twelve-frame-per-second timeline. */
function exposures(atlas, counts) {
  return counts.flatMap((count, frame) => Array.from({ length: count }, () => ({ atlas, frame })));
}

/** Return every drawing in atlas reading order. */
function sequence(atlas) {
  return exposures(atlas, Array(24).fill(1));
}

const timelines = {
  working: [...sequence('typing'), ...sequence('thinking'), ...sequence('typing'), ...sequence('idle')],
  thinking: exposures('thinking', [2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 1, 1, 1, 1, 3, 3, 2, 2, 2, 2, 1, 1, 1, 2]),
  idle: exposures('idle', [6, 6, 6, 6, 6, 6, 4, 4, 4, 4, 1, 1, 1, 1, 6, 6, 6, 6, 4, 4, 2, 2, 2, 2]),
  approval: exposures('approval', [1, 1, 1, 1, 4, 5, 5, 5, 5, 5, 3, 1, 1, 1, 8, 8, 7, 7, 7, 7, 4, 4, 3, 2]),
  completed: exposures('completed', [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 3, 3, 3, 3, 1, 1, 1, 1, 1, 1, 1, 1, 3, 3]),
  error: exposures('error', [2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3, 1, 1, 1, 3, 3, 3, 3, 6, 6, 6, 5]),
};

const labels = {
  working: '正在执行', thinking: '正在思考', idle: '空闲待命',
  approval: '等待批准', completed: '任务完成', error: '需要处理',
};

/** Encode lossless transparent animations, letting WebP merge identical held frames. */
async function main() {
  fs.mkdirSync(path.join(root, 'animations'), { recursive: true });
  const frameBank = {};
  for (const [name, source] of Object.entries(sources)) {
    const input = path.join(generated, source);
    const metadata = await sharp(input).metadata();
    if (metadata.width !== 1536 || metadata.height !== 1024 || !metadata.hasAlpha) throw new Error(`Unexpected source: ${name}`);
    frameBank[name] = await Promise.all(Array.from({ length: 24 }, (_, frame) => sharp(input).extract({ left: (frame % 6) * 256, top: Math.floor(frame / 6) * 256, width: 256, height: 256 }).ensureAlpha().raw().toBuffer()));
  }
  const manifest = { version: 2, format: 'animated-webp', nominalFps: 12, width: 256, height: 256, compression: 'lossless', states: {} };
  for (const [state, timeline] of Object.entries(timelines)) {
    // Millisecond frame boundaries preserve the total duration of twelve ticks per second.
    const delay = timeline.map((_, index) => Math.round((index + 1) * 1000 / 12) - Math.round(index * 1000 / 12));
    const file = `animations/${state}.webp`;
    const output = path.join(root, file);
    const raw = Buffer.concat(timeline.map(({ atlas, frame }) => frameBank[atlas][frame]));
    await sharp(raw, { raw: { width: 256, height: 256 * timeline.length, channels: 4, pageHeight: 256 } })
      .webp({ lossless: true, effort: 4, loop: state === 'completed' ? 1 : 0, delay })
      .toFile(output);
    const metadata = await sharp(output, { animated: true }).metadata();
    const durationMs = metadata.delay.reduce((sum, value) => sum + value, 0);
    if (!metadata.hasAlpha || metadata.width !== 256 || metadata.pageHeight !== 256 || durationMs !== Math.round(timeline.length * 1000 / 12)) throw new Error(`Invalid animation: ${state}`);
    manifest.states[state] = { label: labels[state], file, playbackTicks: timeline.length, encodedFrames: metadata.pages, durationSeconds: durationMs / 1000, loop: state !== 'completed', nextState: state === 'completed' ? 'idle' : null, bytes: fs.statSync(output).size };
  }
  fs.writeFileSync(path.join(root, 'manifest.json'), `${JSON.stringify(manifest, null, 2)}\n`);
  fs.writeFileSync(path.join(root, 'asset-checks.json'), `${JSON.stringify({ compression: 'lossless', alphaPreserved: true, totalAnimationBytes: Object.values(manifest.states).reduce((sum, state) => sum + state.bytes, 0), states: manifest.states }, null, 2)}\n`);
  console.log(JSON.stringify(manifest));
}

main().catch((error) => { console.error(error); process.exitCode = 1; });
