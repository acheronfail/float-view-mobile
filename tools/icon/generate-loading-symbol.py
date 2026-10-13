#!/usr/bin/env python3
"""Generate loading silhouettes from the app icon's rider paths (requires magick)."""
from pathlib import Path
import json
import re
import subprocess
import tempfile
import xml.etree.ElementTree as ET

root = Path(__file__).resolve().parents[2]
source = ET.parse(root / 'tools/icon/icon.svg')
paths = [path.attrib['d'] for path in source.iter('{http://www.w3.org/2000/svg}path')]
svg = (
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="14 0 75 101" '
    'width="72" height="96" fill="currentColor" aria-hidden="true">'
    + ''.join(f'<path d="{path}"/>' for path in paths)
    + '</svg>'
)
html_path = root / 'frontend/src/app.html'
html = html_path.read_text()
def update_html_symbol(match):
    index = 0

    def update_path(path_match):
        nonlocal index
        result = path_match[1] + paths[index] + path_match[2]
        index += 1
        return result

    result = re.sub(r'(<path\s+d=")[^"]*(")', update_path, match[0])
    if index != len(paths):
        raise RuntimeError('Early HTML loading symbol path count differs from the icon')
    return result


html, count = re.subn(
    r'(<div id="app-loading"[^>]*>\s*)<svg\b[\s\S]*?</svg>',
    update_html_symbol,
    html,
    count=1,
)
if count != 1:
    raise RuntimeError('Expected one early HTML loading symbol')
html_path.write_text(html)

with tempfile.TemporaryDirectory(prefix='float-view-symbol-') as directory:
    source_path = Path(directory) / 'symbol.svg'
    output_path = Path(directory) / 'symbol.png'
    source_path.write_text(svg.replace('currentColor', 'white'))
    subprocess.run([
        'magick', '-background', 'none', '-density', '288', str(source_path),
        '-resize', '216x288!', '-define', 'png:exclude-chunk=all', str(output_path),
    ], check=True)
    for destination in [
        'backends/apple/ios/float-view/Assets.xcassets/LoadingSymbol.imageset/LoadingSymbol.png',
        'backends/apple/macos/Resources/LoadingSymbol.png',
    ]:
        asset = root / destination
        asset.parent.mkdir(parents=True, exist_ok=True)
        asset.write_bytes(output_path.read_bytes())

(root / 'backends/apple/ios/float-view/Assets.xcassets/LoadingSymbol.imageset/Contents.json').write_text(
    json.dumps({
        'images': [{'filename': 'LoadingSymbol.png', 'idiom': 'universal'}],
        'info': {'author': 'xcode', 'version': 1},
        'properties': {'template-rendering-intent': 'template'},
    }, indent=2) + '\n'
)
vector = (
    '<vector xmlns:android="http://schemas.android.com/apk/res/android" '
    'android:width="72dp" android:height="96dp" '
    'android:viewportWidth="75" android:viewportHeight="101">\n'
    '  <group android:translateX="-14">\n'
    + ''.join(f'    <path android:fillColor="#FFFFFFFF" android:pathData="{path}" />\n' for path in paths)
    + '  </group>\n</vector>\n'
)
(root / 'backends/android/app/src/main/res/drawable/loading_symbol.xml').write_text(vector)
