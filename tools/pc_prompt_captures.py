"""Turn headless PC-prompt XFB captures (native_prompts::CaptureProbe) into PNG evidence.

Each capture moment has three frames: keyboard/mouse glyphs, gamepad glyphs and the
original (no patches). Writes <dir>/png/*.png, one side-by-side sheet per moment
(<dir>/sheet-<t>-kind<k>.png) and <dir>/index.json. Removes the PPMs after conversion.
usage: python tools/pc_prompt_captures.py evidence/pc-prompts/<label>
"""
import json, re, sys
from pathlib import Path
from PIL import Image, ImageChops, ImageDraw


def main(folder):
    folder = Path(folder)
    (folder / 'png').mkdir(exist_ok=True)
    groups = {}
    for ppm in sorted(folder.glob('*.ppm')):
        m = re.fullmatch(r't(\d+\.\d)-kind([0-9a-f]{2})-(kbm|pad|wii)\.ppm', ppm.name)
        if not m:
            continue
        png = folder / 'png' / ppm.with_suffix('.png').name
        Image.open(ppm).save(png)
        ppm.unlink()
    for png in sorted((folder / 'png').glob('*.png')):
        m = re.fullmatch(r't(\d+\.\d)-kind([0-9a-f]{2})-(kbm|pad|wii)\.png', png.name)
        if m:
            groups.setdefault(m.group(2), []).append((float(m.group(1)), m.group(3), png))
    index = []
    for kind, frames in groups.items():
        frames.sort()
        # A moment = consecutive kbm, pad, wii frames.
        for i in range(0, len(frames) - 2, 3):
            trio = frames[i:i + 3]
            if [f[1] for f in trio] != ['kbm', 'pad', 'wii']:
                continue
            images = [Image.open(f[2]).convert('RGB') for f in trio]
            w, h = images[0].size
            sheet = Image.new('RGB', (w * 3, h + 18), (30, 30, 30))
            draw = ImageDraw.Draw(sheet)
            for j, (img, (_, label, _)) in enumerate(zip(images, trio)):
                sheet.paste(img, (j * w, 18))
                draw.text((j * w + 4, 3), {'kbm': 'keyboard/mouse', 'pad': 'gamepad', 'wii': 'original (off)'}[label], fill=(255, 255, 0))
            name = f'sheet-{trio[0][0]:06.1f}-kind{kind}.png'
            sheet.save(folder / name)
            diff = [ImageChops.difference(images[k], images[2]).getbbox() for k in (0, 1)]
            index.append(dict(seconds=trio[0][0], kind=int(kind, 16), sheet=name,
                              frames=[str(f[2].relative_to(folder)) for f in trio],
                              kbmDiffersFromOriginal=diff[0], padDiffersFromOriginal=diff[1]))
    index.sort(key=lambda e: e['seconds'])
    (folder / 'index.json').write_text(json.dumps(dict(
        scope='Headless XFB captures; glyph sets switched by the probe, frames ~0.5 s apart, so the game moves between them',
        moments=index), indent=1) + '\n', encoding='utf-8')
    print(json.dumps(dict(moments=len(index), withPrompt=sum(e['kind'] != 0 for e in index))))


if __name__ == '__main__':
    main(sys.argv[1])
