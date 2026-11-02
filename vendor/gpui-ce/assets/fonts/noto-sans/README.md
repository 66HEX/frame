# Noto Sans width fixtures

The variable fixture contains printable ASCII, U+0020 through U+007E, with all
layout features retained. Its `wdth` axis covers 62.5 to 100 and its `wght` axis
covers 100 to 900. It includes the example text `hello gpui-ce` and ligature test
text such as `office`.

Source: [Noto Sans 2.015 in Google Fonts](https://github.com/google/fonts/blob/2984c575fdce412ee02b2baaba67672b9a9434d8/ofl/notosans/NotoSans%5Bwdth%2Cwght%5D.ttf),
commit `2984c575fdce412ee02b2baaba67672b9a9434d8`.
The original font's SHA-256 is
`bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d`.
[OFL.txt](OFL.txt) contains its license with trailing whitespace removed.

`WidthFixture-Regular.ttf` and `WidthFixture-Condensed.ttf` are static instances
at weight 400 and widths 100 and 75. They share the test family `GPUI Width Fixture`
and use OS/2 width classes 5 and 3. They have no variable axes and are registered
without the variable face when testing static face matching.

Generate the fixtures with Python and FontTools 4.62.1. Run this command from the
repository root. FontTools is needed only to prepare the assets.

```sh
python3 - <<'PY'
from hashlib import sha256
from io import BytesIO
from pathlib import Path
from urllib.request import urlopen

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont

commit = '2984c575fdce412ee02b2baaba67672b9a9434d8'
source_url = f'https://raw.githubusercontent.com/google/fonts/{commit}/ofl/notosans/'
source = urlopen(source_url + 'NotoSans%5Bwdth%2Cwght%5D.ttf').read()
assert sha256(source).hexdigest() == 'bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d'

asset_dir = Path('assets/fonts/noto-sans')
asset_dir.mkdir(parents=True, exist_ok=True)
license_text = urlopen(source_url + 'OFL.txt').read().decode()
(asset_dir / 'OFL.txt').write_text('\n'.join(line.rstrip() for line in license_text.splitlines()) + '\n')

font = TTFont(BytesIO(source), recalcTimestamp=False)
options = subset.Options()
options.layout_features = ['*']
options.name_IDs = ['*']
options.name_legacy = True
options.name_languages = ['*']
subsetter = subset.Subsetter(options=options)
subsetter.populate(unicodes=range(0x20, 0x7f))
subsetter.subset(font)
font.save(asset_dir / 'NotoSans[wdth,wght].subset.ttf')

for label, width, width_class in [('Regular', 100, 5), ('Condensed', 75, 3)]:
    instance = instantiateVariableFont(font, {'wdth': width, 'wght': 400}, inplace=False)
    names = {
        1: 'GPUI Width Fixture',
        2: label,
        3: f'GPUIWidthFixture-{label}',
        4: f'GPUI Width Fixture {label}',
        6: f'GPUIWidthFixture-{label}',
        16: 'GPUI Width Fixture',
        17: label,
    }
    for record in instance['name'].names:
        if record.nameID in names:
            record.string = names[record.nameID].encode(record.getEncoding())
    instance['OS/2'].usWidthClass = width_class
    instance.save(asset_dir / f'WidthFixture-{label}.ttf')
PY
```

The bundled files have these SHA-256 checksums:

| File | SHA-256 |
| --- | --- |
| `NotoSans[wdth,wght].subset.ttf` | `b5ce4ae4ae9b223481b148cde4fd21e5659b9e92e747c885b69013a59db6458e` |
| `WidthFixture-Regular.ttf` | `cb02cdb7ee355d5900120073cb05d0095579807a26ed2441cd77bb50db33e2ad` |
| `WidthFixture-Condensed.ttf` | `67a4a224d7bf97c1ce28dd7997b7073da4568cf72f918d03f62a078625719eed` |
