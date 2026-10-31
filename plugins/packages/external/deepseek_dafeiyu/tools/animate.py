"""Rebuild local animated WebP assets from the attributed three-view PNGs.
Requires Pillow (no network or generated-image service).
"""
from pathlib import Path
from math import sin, cos, pi
from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1] / 'resources'
FRONT = Image.open(ROOT / 'source/front.png').convert('RGBA')
SIDE = Image.open(ROOT / 'source/side.png').convert('RGBA')
BACK = Image.open(ROOT / 'source/back.png').convert('RGBA')
SIZE = (272, 352)


def frame(sprite, phase, mood):
    canvas = Image.new('RGBA', SIZE)
    bounce = sin(phase * 2 * pi)
    scale_y = 1 + 0.012 * bounce
    bob = 1.5 * bounce
    angle = 0.7 * sin(phase * 2 * pi)
    if mood == 'happy':
        bob = -24 * max(0, sin(phase * pi))
        angle = 3 * sin(phase * 2 * pi)
    elif mood == 'eat':
        scale_y = 1 + 0.025 * sin(phase * 6 * pi)
        angle = 1.5 * sin(phase * 4 * pi)
    elif mood == 'walk':
        bob = -4 * abs(sin(phase * 4 * pi))
        angle = 2 * sin(phase * 4 * pi)
    resized = sprite.resize((sprite.width, round(sprite.height * scale_y)), Image.Resampling.LANCZOS)
    rotated = resized.rotate(angle, resample=Image.Resampling.BICUBIC, expand=True)
    x = (SIZE[0] - rotated.width) // 2
    y = round(SIZE[1] - rotated.height - 12 + bob)
    canvas.alpha_composite(rotated, (x, y))
    if mood == 'eat':
        draw = ImageDraw.Draw(canvas)
        fx = round(SIZE[0] / 2 + 14 * cos(phase * 2 * pi))
        fy = round(228 + 3 * bounce)
        draw.ellipse((fx - 12, fy - 5, fx + 10, fy + 5), fill='#89C9FF', outline='#34548E', width=2)
        draw.polygon([(fx + 9, fy), (fx + 18, fy - 7), (fx + 18, fy + 7)], fill='#89C9FF')
        draw.ellipse((fx - 7, fy - 2, fx - 4, fy + 1), fill='#24366C')
    return canvas


frame(FRONT, 0, 'idle').save(ROOT / 'still.png')
for mood, count, duration in [('idle', 24, 160), ('happy', 20, 54), ('eat', 24, 98), ('walk', 24, 87)]:
    frames = [frame(SIDE if mood == 'walk' else FRONT, i / count, mood) for i in range(count)]
    frames[0].save(ROOT / f'{mood}.webp', save_all=True, append_images=frames[1:],
                   duration=duration, loop=0, lossless=False, quality=88, method=4)
    print(mood, (ROOT / f'{mood}.webp').stat().st_size)
