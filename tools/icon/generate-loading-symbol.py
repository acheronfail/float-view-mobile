#!/usr/bin/env python3
"""Generate themed loading artwork and the separate map silhouette."""
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
# The map uses the same silhouette with theme-aware CSS colours.
(root / 'frontend/src/lib/ui/rider-symbol.svg').write_text(
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="14 0 75 101" '
    'fill="currentColor" aria-hidden="true">\n'
    + ''.join(f'  <path d="{path}"/>\n' for path in paths)
    + '</svg>\n'
)
with tempfile.TemporaryDirectory(prefix='float-view-symbol-') as directory:
    source_path = Path(directory) / 'symbol.svg'
    output_path = Path(directory) / 'symbol.png'
    source_path.write_text(svg.replace('currentColor', 'white'))
    subprocess.run([
        'magick', '-background', 'none', '-density', '288', str(source_path),
        '-resize', '216x288!', '-define', 'png:exclude-chunk=all', str(output_path),
    ], check=True)
    # Use the same stroke-free geometry and viewport in both appearances.
    light_svg = svg.replace('currentColor', '#14b8a6').replace(
        '<svg ', '<svg class="loading-symbol-light" ', 1,
    )
    light_path = Path(directory) / 'loading-light.svg'
    light_path.write_text(light_svg)
    light_png = Path(directory) / 'loading-light.png'
    subprocess.run([
        'rsvg-convert', '-w', '216', '-h', '288', str(light_path),
        '-o', str(light_png),
    ], check=True)
    for destination in [
        'backends/apple/ios/float-view/Assets.xcassets/LoadingSymbol-Light.imageset/LoadingSymbol-Light.png',
        'backends/apple/macos/Resources/LoadingSymbol-Light.png',
        'backends/android/app/src/main/res/drawable-nodpi/loading_symbol_light.png',
    ]:
        asset = root / destination
        asset.parent.mkdir(parents=True, exist_ok=True)
        asset.write_bytes(light_png.read_bytes())

    html_path = root / 'frontend/src/app.html'
    html = html_path.read_text()
    dark_svg = svg.replace('<svg ', '<svg class="loading-symbol-dark" ', 1)
    # The inline artwork paints before frontend scripts or asset requests.
    symbol = (
        '<span id="loading-symbol" aria-hidden="true">'
        + light_svg + dark_svg + '</span>'
    )
    html, count = re.subn(
        r'<span id="loading-symbol"[^>]*>[\s\S]*?</span>',
        lambda _: symbol, html, count=1,
    )
    if count != 1:
        raise RuntimeError('Expected one early HTML loading symbol')
    html_path.write_text(html)

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

(root / 'backends/apple/ios/float-view/Assets.xcassets/LoadingSymbol-Light.imageset/Contents.json').write_text(
    json.dumps({
        'images': [{'filename': 'LoadingSymbol-Light.png', 'idiom': 'universal'}],
        'info': {'author': 'xcode', 'version': 1},
        'properties': {'template-rendering-intent': 'original'},
    }, indent=2) + '\n'
)
