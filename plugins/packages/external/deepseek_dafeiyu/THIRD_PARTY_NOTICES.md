# Third-party animation assets

## Animated clips

- Project / author repository: PC2005-cloud/dsh-pet
- Source: https://github.com/PC2005-cloud/dsh-pet
- Pinned revision: 38ae8465cd69ba15e8c8a29e7208997a0baa3451
- Asset terms stated by upstream: “素材（动画/提示词/源视频）：允许开源使用，禁止商用”. These assets are **not covered by the upstream code's MIT license**.
- Upstream additionally requests that public derivatives credit https://github.com/PC2005-cloud/dsh-pet in their GitHub repository, product site, and other public channels. Keep this notice and the attribution in the settings page when distributing this plugin.
- The pinned README's complete license section is preserved verbatim in `resources/source/UPSTREAM-LICENSE.md`.

| Local source | Original path at the pinned revision | Used for |
| --- | --- | --- |
| `resources/source/idle.webm` | `dsh-pet/assets/webm/待机呼吸休闲.webm` | Blinking, looking around, idle |
| `resources/source/happy.webm` | `dsh-pet/assets/webm/点击回应-元气挥手.webm` | Waving greeting |
| `resources/source/eat.webm` | `dsh-pet/assets/webm/吃白饭.webm` | Eating rice from a bowl |
| `resources/source/walk.webm` | `dsh-pet/assets/webm/原地漂浮踏步.webm` | Stepping in place |
| `resources/source/talk.webm` | `dsh-pet/assets/webm/碎碎念-对屏碎碎念.webm` | Mouth, facial expression and hand movement |

Each original file's SHA-256 is recorded in `resources/source/provenance.json`. Original VP9-alpha WebM clips are preserved there for offline rebuilding. Local modifications: removing near-zero alpha compression residue (alpha ≤ 4), one shared crop/resize/padding transform for all frames, conversion to transparent animated WebP at the original 24 fps, and a still PNG extracted from idle. No rotation, sway, bounce, generated mouth frames, or synthetic motion was added. Conversion tool: `tools/convert_animations.py`.

## Character design

As attributed in the pinned upstream README:

- Original character “溟月”: Bilibili @上善无形 — https://space.bilibili.com/4456176
- DeepSeek fan design (maid whale girl): Bilibili @ZipZipPipe — https://space.bilibili.com/4168597
- Character license: CC BY-NC-SA 4.0 (Attribution, NonCommercial, ShareAlike).
- Public character license statement linked by upstream: https://www.bilibili.com/opus/1247156596114456581
- License: https://creativecommons.org/licenses/by-nc-sa/4.0/
- Full license text: `resources/source/CC-BY-NC-SA-4.0.txt`.

The converted character assets retain these noncommercial and attribution restrictions; share adaptations under CC BY-NC-SA 4.0. They cannot be used for a commercial distribution without separately obtaining the necessary permissions. These asset terms do not relicense unrelated Operit code.

This is an unofficial community fan character, not an official DeepSeek character or product. No affiliation or endorsement is asserted.
