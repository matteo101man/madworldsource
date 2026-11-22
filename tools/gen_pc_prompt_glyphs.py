"""Generate PC prompt glyphs (keyboard/mouse and generic gamepad) for the native frontend.

Every glyph is drawn here from plain shapes and a system sans font: key caps, a generic
mouse, generic face buttons/bumpers/triggers/sticks/D-pad. No game or brand art is used.
Glyphs follow the original prompt style (white ink, black fill, transparent outside) at
the original sprite sizes, so the game's own tint and animation still apply.

Writes native-host-data/pc-prompts/{glyphs.rgba,glyphs.json,pages.json} and a preview
sheet research/ui-source/pc-prompt-glyphs.png. crates/mw-game/src/native_prompts.rs
composes them per binding (pc_config.rs input names) and last-used device at runtime.
"""
import hashlib, json, re, struct
from pathlib import Path
import xxhash
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
DISC = Path('D:/Games/gamesfordecomp/exports/madworld/extracted/disc/DATA/files/etc/core_us.idd')
STAGED = ROOT / 'staged/pc-ui/core_us.idd'
OUT = ROOT / 'native-host-data/pc-prompts'
SS = 8  # supersampling
FONT = 'C:/Windows/Fonts/arialbd.ttf'
WHITE, BLACK = (255, 255, 255, 255), (0, 0, 0, 255)
SIZES = (18, 20, 28, 36, 46)

# Prompt slots in etc/core_us.idd (research/ui-live-core-layout.json): sprite id -> role.
SLOTS = {
    1: 'attack', 47: 'attack', 2: 'chainsaw', 49: 'chainsaw', 51: 'lock_on', 54: 'jump',
    42: 'plus', 43: 'pause', 44: 'move', 50: 'pickup', 52: 'pickup', 53: 'pickup',
    64: 'remote', 65: 'remote', 66: 'nunchuk', 67: 'nunchuk',
}


def input_names():
    src = (ROOT / 'crates/mw-game/src/pc_config.rs').read_text(encoding='utf-8')
    keys = re.search(r'pub const KEYS: &\[KeyCode\] = keys!\[(.*?)\];', src, re.S).group(1)
    names = [k.strip() for k in keys.replace('\n', ' ').split(',') if k.strip()]
    for table in ('MOUSE', 'PAD', 'STICK'):
        block = re.search(rf'const {table}: &\[\(&str, \w+\)\] = &\[(.*?)\];', src, re.S).group(1)
        names += re.findall(r'\("(\w+)"', block)
    return names + ['PadLStick', 'MoveKeys', 'WordShake', 'WordPlus']


def font(px):
    return ImageFont.truetype(FONT, max(1, int(px)))


def text_center(d, box, label, fill, max_h):
    x0, y0, x1, y1 = box
    size = max_h
    while size > 4:
        f = font(size)
        l, t, r, b = d.textbbox((0, 0), label, font=f)
        if r - l <= (x1 - x0) and b - t <= (y1 - y0):
            break
        size -= SS // 2
    d.text(((x0 + x1 - (r - l)) / 2 - l, (y0 + y1 - (b - t)) / 2 - t), label, font=f, fill=fill)


def arrow(d, cx, cy, s, direction, fill):
    """Solid arrow (triangle head + shaft) of half-size s pointing up/down/left/right."""
    pts = {'up': [(0, -1), (.9, -.05), (.35, -.05), (.35, .95), (-.35, .95), (-.35, -.05), (-.9, -.05)]}
    rot = {'up': lambda x, y: (x, y), 'down': lambda x, y: (x, -y), 'left': lambda x, y: (y, x), 'right': lambda x, y: (-y, x)}[direction]
    d.polygon([(cx + rot(x, y)[0] * s, cy + rot(x, y)[1] * s) for x, y in pts['up']], fill=fill)


def keycap(d, n, label=None, draw=None):
    """Key cap: white rim, black face, a thicker bottom edge so it reads as a key."""
    m, rim = n * .04, max(SS, n * .07)
    d.rounded_rectangle((m, m, n - m, n - m), radius=n * .18, fill=WHITE)
    face = (m + rim, m + rim, n - m - rim, n - m - rim * 2.2)
    d.rounded_rectangle(face, radius=n * .12, fill=BLACK)
    if draw:
        draw(d, face)
    elif label:
        pad = (face[2] - face[0]) * .1
        text_center(d, (face[0] + pad, face[1] + pad, face[2] - pad, face[3] - pad), label, WHITE, int((face[3] - face[1]) * (.8 if len(label) == 1 else .55)))


def key_glyph(d, n, name):
    labels = {'Space': 'Space', 'Enter': 'Enter', 'Escape': 'Esc', 'Tab': 'Tab', 'Backspace': 'Bksp',
              'ShiftLeft': 'Shift', 'ShiftRight': 'Shift', 'ControlLeft': 'Ctrl', 'ControlRight': 'Ctrl',
              'AltLeft': 'Alt', 'AltRight': 'Alt', 'CapsLock': 'Caps', 'Backquote': '`', 'Minus': '-', 'Equal': '=',
              'BracketLeft': '[', 'BracketRight': ']', 'Backslash': '\\', 'Semicolon': ';', 'Quote': "'",
              'Comma': ',', 'Period': '.', 'Slash': '/', 'Insert': 'Ins', 'Delete': 'Del', 'Home': 'Home', 'End': 'End',
              'PageUp': 'PgUp', 'PageDown': 'PgDn', 'NumpadEnter': 'Enter', 'NumpadAdd': '+', 'NumpadSubtract': '-',
              'NumpadMultiply': '*', 'NumpadDivide': '/', 'NumpadDecimal': '.'}
    if name.startswith('Arrow'):
        direction = name[5:].lower()
        keycap(d, n, draw=lambda d, f: arrow(d, (f[0] + f[2]) / 2, (f[1] + f[3]) / 2, (f[3] - f[1]) * .36, direction, WHITE))
        return
    if name == 'Space':
        def bar(d, f):
            w, h = f[2] - f[0], f[3] - f[1]
            x0, x1, y = f[0] + w * .18, f[2] - w * .18, f[1] + h * .62
            t = max(SS, h * .1)
            d.rectangle((x0, y, x1, y + t), fill=WHITE)
            d.rectangle((x0, y - h * .22, x0 + t, y + t), fill=WHITE)
            d.rectangle((x1 - t, y - h * .22, x1, y + t), fill=WHITE)
        keycap(d, n, draw=bar)
        return
    label = labels.get(name) or re.sub(r'^(Key|Digit|Numpad)', '', name)
    keycap(d, n, label)


def mouse_glyph(d, n, name):
    w, h = n * .62, n * .9
    x0, y0 = (n - w) / 2, (n - h) / 2
    rim = max(SS, n * .07)
    d.rounded_rectangle((x0, y0, x0 + w, y0 + h), radius=w * .48, fill=WHITE)
    d.rounded_rectangle((x0 + rim, y0 + rim, x0 + w - rim, y0 + h - rim), radius=w * .4, fill=BLACK)
    split = y0 + h * .42
    d.rectangle((x0 + rim, split, x0 + w - rim, split + rim * .8), fill=WHITE)
    d.rectangle((n / 2 - rim * .4, y0 + rim, n / 2 + rim * .4, split), fill=WHITE)
    side = {'MouseLeft': 0, 'MouseRight': 1}.get(name)
    if side is not None:
        body = Image.new('L', (n, n), 0)
        ImageDraw.Draw(body).rounded_rectangle((x0 + rim, y0 + rim, x0 + w - rim, y0 + h - rim), radius=w * .4, fill=255)
        half = Image.new('L', (n, n), 0)
        hx0, hx1 = (0, n / 2 - rim * .4) if side == 0 else (n / 2 + rim * .4, n)
        ImageDraw.Draw(half).rectangle((hx0, 0, hx1, split), fill=255)
        from PIL import ImageChops
        d._image.paste(WHITE, (0, 0), ImageChops.multiply(body, half))
    elif name == 'MouseMiddle':
        d.rounded_rectangle((n / 2 - rim * 1.1, y0 + h * .12, n / 2 + rim * 1.1, y0 + h * .34), radius=rim, fill=WHITE)
    else:
        text_center(d, (x0 + rim, split + rim, x0 + w - rim, y0 + h - rim), {'MouseBack': '4', 'MouseForward': '5'}[name], WHITE, int(h * .3))


def pad_glyph(d, n, name):
    rim = max(SS, n * .08)
    face = {'PadA': 'A', 'PadB': 'B', 'PadX': 'X', 'PadY': 'Y'}
    if name in face:
        d.ellipse((n * .04, n * .04, n * .96, n * .96), fill=WHITE)
        d.ellipse((n * .04 + rim, n * .04 + rim, n * .96 - rim, n * .96 - rim), fill=BLACK)
        text_center(d, (n * .25, n * .25, n * .75, n * .75), face[name], WHITE, int(n * .58))
    elif name in ('PadLB', 'PadRB'):
        d.rounded_rectangle((n * .02, n * .2, n * .98, n * .8), radius=n * .2, fill=WHITE)
        d.rounded_rectangle((n * .02 + rim, n * .2 + rim, n * .98 - rim, n * .8 - rim), radius=n * .14, fill=BLACK)
        text_center(d, (n * .14, n * .3, n * .86, n * .7), name[3:], WHITE, int(n * .4))
    elif name in ('PadLT', 'PadRT'):
        d.rounded_rectangle((n * .12, n * .02, n * .88, n * .98), radius=n * .3, fill=WHITE)
        d.rounded_rectangle((n * .12 + rim, n * .02 + rim, n * .88 - rim, n * .98 - rim), radius=n * .24, fill=BLACK)
        text_center(d, (n * .2, n * .3, n * .8, n * .8), name[3:], WHITE, int(n * .4))
    elif name in ('PadSelect', 'PadStart'):
        d.rounded_rectangle((n * .02, n * .18, n * .98, n * .82), radius=n * .3, fill=WHITE)
        d.rounded_rectangle((n * .02 + rim, n * .18 + rim, n * .98 - rim, n * .82 - rim), radius=n * .24, fill=BLACK)
        if name == 'PadStart':  # three lines
            for i in range(3):
                y = n * (.36 + .14 * i)
                d.rectangle((n * .3, y - rim * .45, n * .7, y + rim * .45), fill=WHITE)
        else:  # two overlapping squares
            s = n * .22
            d.rectangle((n * .3, n * .32, n * .3 + s, n * .32 + s), outline=WHITE, width=int(rim * .8))
            d.rectangle((n * .46, n * .44, n * .46 + s, n * .44 + s), fill=WHITE)
    elif name in ('PadUp', 'PadDown', 'PadLeft', 'PadRight'):
        a, t = n * .04, n * .33
        cross = [(n / 2 - t / 2 - rim, a), (n / 2 + t / 2 + rim, a), (n / 2 + t / 2 + rim, n / 2 - t / 2 - rim),
                 (n - a, n / 2 - t / 2 - rim), (n - a, n / 2 + t / 2 + rim), (n / 2 + t / 2 + rim, n / 2 + t / 2 + rim),
                 (n / 2 + t / 2 + rim, n - a), (n / 2 - t / 2 - rim, n - a), (n / 2 - t / 2 - rim, n / 2 + t / 2 + rim),
                 (a, n / 2 + t / 2 + rim), (a, n / 2 - t / 2 - rim), (n / 2 - t / 2 - rim, n / 2 - t / 2 - rim)]
        d.polygon(cross, fill=WHITE)
        b = a + rim
        d.rectangle((n / 2 - t / 2, b, n / 2 + t / 2, n - b), fill=BLACK)
        d.rectangle((b, n / 2 - t / 2, n - b, n / 2 + t / 2), fill=BLACK)
        arm = {'PadUp': (n / 2 - t / 2, b, n / 2 + t / 2, n / 2 - t / 2), 'PadDown': (n / 2 - t / 2, n / 2 + t / 2, n / 2 + t / 2, n - b),
               'PadLeft': (b, n / 2 - t / 2, n / 2 - t / 2, n / 2 + t / 2), 'PadRight': (n / 2 + t / 2, n / 2 - t / 2, n - b, n / 2 + t / 2)}[name]
        g = rim * .7
        d.rectangle((arm[0] + g, arm[1] + g, arm[2] - g, arm[3] - g), fill=WHITE)
    else:  # sticks: PadL3/PadR3, PadLStick, PadRStick<dir>
        side = 'L' if name in ('PadL3', 'PadLStick') else 'R'
        direction = name.removeprefix('PadRStick').lower()
        off = {'up': (0, -1), 'down': (0, 1), 'left': (-1, 0), 'right': (1, 0)}.get(direction, (0, 0))
        small = direction in ('up', 'down', 'left', 'right', 'cw', 'ccw')
        r = n * (.33 if small else .46)
        cx, cy = n / 2 - off[0] * n * .14, n / 2 - off[1] * n * .14
        if direction in ('cw', 'ccw'):
            cy = n * .56
        d.ellipse((cx - r, cy - r, cx + r, cy + r), fill=WHITE)
        d.ellipse((cx - r + rim, cy - r + rim, cx + r - rim, cy + r - rim), fill=BLACK)
        if not small:
            inner = r * .62
            d.ellipse((cx - inner, cy - inner, cx + inner, cy + inner), outline=WHITE, width=int(rim * .7))
        label = side + ('3' if name in ('PadL3', 'PadR3') else '')
        text_center(d, (cx - r * .55, cy - r * .55, cx + r * .55, cy + r * .55), label, WHITE, int(r * (1.1 if small else .8)))
        if direction in ('up', 'down', 'left', 'right'):
            # Outlined arrow beside the stick, on the flicked side.
            ax, ay = n / 2 + off[0] * n * .34, n / 2 + off[1] * n * .34
            arrow(d, ax, ay, n * .17, direction, BLACK)
            arrow(d, ax, ay, n * .13, direction, WHITE)
        elif direction in ('cw', 'ccw'):
            ar = r + rim * 1.6
            box = (cx - ar, cy - ar, cx + ar, cy + ar)
            d.arc(box, 190, 350, fill=BLACK, width=int(rim * 2.4))
            d.arc(box, 195, 345, fill=WHITE, width=int(rim * 1.1))
            tip = cx + ar * (.92 if direction == 'cw' else -.92)
            arrow(d, tip, cy - ar * .15, n * .11, 'down', BLACK)
            arrow(d, tip, cy - ar * .15, n * .08, 'down', WHITE)
        elif name == 'PadLStick':
            for direction, (ox, oy) in {'up': (0, -1), 'down': (0, 1), 'left': (-1, 0), 'right': (1, 0)}.items():
                arrow(d, n / 2 + ox * n * .4, n / 2 + oy * n * .4, n * .1, direction, WHITE)


def move_keys(d, n):
    """WASD cluster: one cap on top, three below."""
    c = n * .34
    for x, y, label in ((n / 2 - c / 2, n * .14, 'W'), (n / 2 - c * 1.5, n * .14 + c, 'A'), (n / 2 - c / 2, n * .14 + c, 'S'), (n / 2 + c / 2, n * .14 + c, 'D')):
        img = Image.new('RGBA', (int(c), int(c)), (0, 0, 0, 0))
        keycap(ImageDraw.Draw(img), int(c), label)
        d._image.alpha_composite(img, (int(x), int(y)))


def render(name, size, width=None):
    w = width or size
    n = w * SS
    img = Image.new('RGBA', (n, size * SS), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)
    if name == 'WordShake':
        text_center(d, (0, 0, n, size * SS), 'SHAKE', WHITE, size * SS)
        shadow = Image.new('RGBA', img.size, (0, 0, 0, 0))
        # Black outline so the word reads on any background.
        alpha = img.getchannel('A')
        for dx in (-SS, 0, SS):
            for dy in (-SS, 0, SS):
                shadow.paste(BLACK, (dx, dy), alpha)
        shadow.alpha_composite(img)
        img = shadow
    elif name == 'WordPlus':
        t = n * .14
        d.rectangle((n * .2, n / 2 - t, n * .8, n / 2 + t), fill=BLACK)
        d.rectangle((n / 2 - t, n * .2, n / 2 + t, n * .8), fill=BLACK)
        t2 = t * .5
        d.rectangle((n * .26, n / 2 - t2, n * .74, n / 2 + t2), fill=WHITE)
        d.rectangle((n / 2 - t2, n * .26, n / 2 + t2, n * .74), fill=WHITE)
    elif name == 'MoveKeys':
        move_keys(d, n)
    elif name.startswith('Mouse'):
        mouse_glyph(d, n, name)
    elif name.startswith('Pad'):
        pad_glyph(d, n, name)
    else:
        key_glyph(d, n, name)
    return img.resize((w, size), Image.LANCZOS)


def texture_names(raw, layout_pages):
    names = {}
    for p in layout_pages:
        if p['id'] not in (1, 2, 3):
            continue
        n = p['width'] * p['height'] // 2
        data = raw[p['dataOffset']:p['dataOffset'] + n]
        idx = set()
        for b in data:
            idx.add(b & 15); idx.add(b >> 4)
        pal = raw[p['paletteOffset']:p['paletteOffset'] + 32]
        ph = xxhash.xxh64(pal[min(idx) * 2:(max(idx) + 1) * 2]).hexdigest()
        names[p['id']] = f"tex1_{p['width']}x{p['height']}_{xxhash.xxh64(data).hexdigest()}_{ph}_8"
    return names


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    blob, index = bytearray(), {}
    names = input_names()
    for name in names:
        for size in SIZES:
            img = render(name, size, width=size)
            if name == 'WordShake':
                img = render(name, max(8, size // 3), width=size)
            index.setdefault(name, {})[str(size)] = dict(offset=len(blob), width=img.width, height=img.height)
            blob += img.tobytes()
    (OUT / 'glyphs.rgba').write_bytes(blob)
    (OUT / 'glyphs.json').write_text(json.dumps(dict(
        scope='Generated PC prompt glyphs (plain shapes + system sans font); RGBA8 rows, see native_prompts.rs',
        font=FONT, supersample=SS, sizes=SIZES, glyphs=index), indent=1) + '\n', encoding='utf-8')

    layout = json.loads((ROOT / 'research/ui-live-core-layout.json').read_text(encoding='utf-8'))
    pages = {}
    for label, path in (('disc', DISC), ('staged', STAGED)):
        if path.is_file():
            for page, name in texture_names(path.read_bytes(), layout['pages']).items():
                pages.setdefault(str(page), []).append(dict(source=label, name=name))
    slots = [dict(sprite=s['id'], page=s['texturePage'], rect=s['rect'], role=SLOTS[s['id']])
             for s in layout['sprites'] if s['id'] in SLOTS]
    (OUT / 'pages.json').write_text(json.dumps(dict(
        scope='Texture names of etc/core_us.idd pages 1-3 (disc and staged DI variants) and the prompt sprite slots',
        pages=pages, slots=slots), indent=1) + '\n', encoding='utf-8')

    # Preview: every glyph at 28 and 46 px, scaled 3x, on mid grey.
    cols, cell = 12, 50
    rows = (len(names) + cols - 1) // cols
    sheet = Image.new('RGBA', (cols * cell * 2 * 3 // 2, rows * cell * 3 // 2 + 10), (96, 96, 96, 255))
    for i, name in enumerate(names):
        x, y = (i % cols) * cell * 3, (i // cols) * cell * 3 // 2
        for j, size in enumerate((28, 46)):
            g = index[name][str(size)]
            img = Image.frombytes('RGBA', (g['width'], g['height']), bytes(blob[g['offset']:g['offset'] + g['width'] * g['height'] * 4]))
            img = img.resize((img.width * 3 // 2, img.height * 3 // 2), Image.NEAREST)
            sheet.alpha_composite(img, (x + j * 38 * 3 // 2 // 1, y))
    sheet.save(ROOT / 'research/ui-source/pc-prompt-glyphs.png')
    print(json.dumps(dict(glyphs=len(names), bytes=len(blob), pages={k: len(v) for k, v in pages.items()}, slots=len(slots))))


if __name__ == '__main__':
    main()
