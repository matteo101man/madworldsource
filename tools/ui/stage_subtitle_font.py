"""Stage the game's subtitle typeface as one labelled font for added subtitles (the Mathilda variant's intro).

python -I tools/ui/stage_subtitle_font.py
The original event / menu message fonts (subscr/*.mtd, the ev00 MESS font) are one typeface, white bold sans with a
black outline. Each font file holds only the glyphs its own messages use and records no character codes (glyph ids are
0x1000 + index). subscr/subscr200_us.mtd holds the whole alphabet, upper and lower case, the digits and the common
punctuation; its 80 glyphs were read by eye from an indexed contact sheet (LABELS below). Output:
assets/native/ui/subtitle-font/{font-0.png, catalog.json} in the mw-native-message-catalog/2 layout (glyphs, pages,
baseAdvance from the font's own metrics) plus `chars` (character -> glyph id). Glyph pixels and metrics are unchanged.
"""
import hashlib, json, struct, sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
EXPORT = Path('D:/Games/gamesfordecomp/exports/madworld')
sys.path.insert(0, str(EXPORT / 'builders'))
from native import tpl  # noqa: E402

SOURCE = 'subscr/subscr200_us.mtd'
# glyph index -> character, read from the indexed sheet of subscr200_us.mtd
LABELS = ['[', 'L', 'I', 'V', 'E', 'F', 'D', ',', 'X', 'C', 'T', ']', 'A', 's', 'y', 'o',
          'u', 'c', 'a', 'n', 'e', 'l', 'i', 'g', 'h', 't', 'b', 'r', 'd', 'J', 'f', 'v',
          '.', 'R', 'N', 'G', 'O', 'M', 'P', 'W', 'm', 'k', 'w', 'Y', "'", 'p', 'B', ':',
          '-', '1', '0', 'S', 'K', 'H', 'z', '(', ')', 'U', '?', '!', 'j', 'Z', 'q', '3',
          '"', 'Q', '2', 'x', ';', '\u2019', '9', '7', '=', '5', '$', '8', '4', '%', '6', '}']
OUT = ROOT / 'assets/native/ui/subtitle-font'


def main():
    data = (EXPORT / 'extracted/disc/DATA/files' / SOURCE).read_bytes()
    metrics, first, count = struct.unpack_from('>3I', data)
    _, advance, line = struct.unpack_from('>3I', data, metrics)
    if count != len(LABELS): raise ValueError('label table does not match the font')
    at = data.find(bytes.fromhex('0020af30'), metrics)
    images = [im for im, _ in tpl(data[at:])]
    OUT.mkdir(parents=True, exist_ok=True)
    pages = []
    for i, im in enumerate(images):
        path = OUT / f'font-{i}.png'; im.save(path)
        pages.append(dict(image=f'native/ui/subtitle-font/font-{i}.png', width=im.width, height=im.height, sha256=hashlib.sha256(path.read_bytes()).hexdigest()))
    glyphs, chars = [], {}
    for i in range(count):
        index, page, *uv = struct.unpack_from('>HH4f', data, 12 + i * 20)
        _, _, w, h = struct.unpack_from('>HH2f', data, metrics + 12 + i * 12)
        W, H = images[page].size
        rect = [round(uv[0] * W), round(uv[1] * H), round(uv[2] * W), round(uv[3] * H)]
        glyphs.append(dict(id=0x1000 + i, page=page, rect=rect, size=[w, h]))
        chars.setdefault(LABELS[i], 0x1000 + i)
    catalog = dict(schema='mw-native-message-catalog/2', source=dict(file=SOURCE, sha256=hashlib.sha256(data).hexdigest()),
                   pages=pages, glyphs=glyphs, messages=[], baseAdvance=float(advance), lineHeight=line, chars=chars,
                   scope='Original glyphs and metrics of subscr200_us.mtd; character labels read by eye (tools/ui/stage_subtitle_font.py)')
    (OUT / 'catalog.json').write_text(json.dumps(catalog, indent=1) + '\n', encoding='utf-8')
    for old in OUT.glob('labels.png'): old.unlink()
    print(count, 'glyphs,', len(chars), 'characters; advance', advance, 'line', line)


if __name__ == '__main__':
    main()
