# ImageGen 生成提示词记录

模式：内置 `image_gen.imagegen`，六次独立生成，均设置 `transparent_background: true`。没有使用 CLI/API 回退或 Python 绘图。

图像输入：用户上传的蓝猫 APNG 的首帧作为角色参考；第二次起同时参考首次生成的 typing 图集，统一比例和排版。图像是视觉参考，其内容不作为指令。

## 六组提示词的共同约束

Use case: identity-preserve. Asset type: pixel-art animation SPRITE SHEET for the Operit task-status floating pet. Preserve the original medium-blue pixel kitten, pale cyan glowing eyes, large triangular ears, navy outlines, dark navy laptop on the LEFT and curled upright tail on the RIGHT. Preserve front/three-quarter view and the coarse square pixel style.

Create ONE genuinely transparent PNG atlas. Requested canvas 1536 by 1024, EXACTLY 6 columns by 4 rows, 24 equal 256 by 256 cells, no gutters, borders, labels, captions, grid lines or watermark. Identical center, baseline, scale and padding in all cells. Every complete cat and laptop stays within its own cell. Read left-to-right, top-to-bottom as 24 consecutive animation drawings. Keep laptop anchored and body proportions stable. Smooth incremental movements, no camera motion or independent character redesigns. Hard pixel edges, flat palette, no blur, no antialiasing. Alpha-zero negative space, no colored backdrop, gradient, scenery or baked checkerboard.

## 各组动作提示

### typing / 正在执行

Show a smooth 2-second seamless cycle of the kitten typing alternately with left and right paws, subtle 1-pixel breathing and head bob, a natural slow blink near frame 12, and a gently swaying curled tail. Keep laptop completely fixed. Last frame seamlessly approaches first. Eyes remain cyan, fur remains medium blue and navy. Do not add clothing or scenery.

### thinking / 正在思考

Cat pauses typing, puts one paw beneath chin, slowly looks upward in thought, flicks one ear, blinks, gently taps chin then lowers paw back toward keyboard. No icons, question marks, balloons or text. Frames 1–4 paw raises; 5–10 chin rests and eyes look up; 11–14 blink; 15–18 thoughtful stillness with ear flick; 19–24 paw returns to keyboard and gaze returns to laptop. Quiet active thinking, not approval or error.

### idle / 空闲待命

Cat sits relaxed upright, paws resting off keyboard, content expression. Smooth slight chest breathing, a gentle tip-of-tail sway, a slow blink across frames 10–14. Laptop stays open motionless. No sleeping Z letters. Fur, laptop shapes, size, position and palette stay constant. Last frame approaches first for a seamless loop.

### approval / 等待批准

Cat has STOPPED typing and looks directly toward the user with an attentive gentle expression. Slowly raises the front paw near laptop, holds it asking for attention, gently waves twice then lowers it. A small amber-yellow diamond containing a dark amber exclamation mark hovers in the upper-right free space beside its ear; this is the only non-blue accent. Icon pulses subtly during held pose. Frames 1–4 raise paw; 5–10 hold/wave; 11–15 blink and hold; 16–20 second subtle wave; 21–24 lower toward initial pose. Intended for a longer waiting cycle with clear held poses. No speech balloons or panels.

### completed / 任务完成

Kitten stops typing, lifts head, closes eyes in a happy curved-eye smile, raises both front paws for a small joyful stretch, lowers paws then settles relaxed. Small mint-green pixel checkmark appears above right shoulder near tail, twinkles modestly and fades by last frame. Frames 1–4 recognition; 5–10 paws raise; 11–15 hold happy stretch; 16–20 paws lower; 21–24 sit content. Restrained cute celebration, not jumping, no confetti, no red or amber accents.

### error / 需要处理

Cat has stopped typing, ears gently droop, gaze drops toward laptop, head slowly tilts to one side, one front paw scratches its cheek once, slow blink, returns to start. Expression puzzled and mildly concerned, still cute, no tears. Small coral-red pixel diamond containing dark exclamation mark above right shoulder remains steady without strobe. Frames 1–5 look at laptop; 6–10 head tilt; 11–16 scratch cheek and blink; 17–21 lower paw; 22–24 recover toward start. Calm attention state, not angry or frightened.

## 生成后的编排

源图集由 ImageGen 绘制，仅保存在 Codex 生成目录。裁切和时间编排在内存完成，最终只输出一套无损透明动画 WebP。仓库与插件包不保留图集、静态预览或 APNG 副本。六个最终动画已移至 `plugins/packages/external/task_status_pet/resources/`，插件说明见该目录上级的 README。生成提示约束并不等同于生成结果的逐像素保证：首帧原图与续画存在比例及像素密度差异。交互网页与重复的设计素材目录已删除。
