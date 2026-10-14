# Float View icons

`icon.svg` is the source artwork for the native application icons. Regenerate
all platform assets from the repository root with:

```sh
tools/icon/generate.sh
```

The generator requires `rsvg-convert`, ImageMagick's `magick`, and Apple's
`sips` (supplied by macOS), plus Python 3. When a working `iconutil` is available, it is used
for the macOS ICNS; otherwise the generator falls back to a valid 512px ICNS
written by `sips`. The output includes the iOS AppIcon PNG, macOS ICNS, and
Android density/adaptive-icon WebP assets used by the native targets.

The centered loading screen uses only the rider silhouette, without the icon
background. Regenerate its early HTML, iOS/macOS PNG and Android vector assets
and the frontend map’s SVG symbol independently on any platform with Python 3 and ImageMagick:

```sh
python3 tools/icon/generate-loading-symbol.py
```

macOS uses `macos-mask.svg` to round the artwork, with an 824px face centered
on a transparent 1024px canvas so its Dock size matches other macOS icons.
The mask and padding are applied before generating every ICNS resolution,
including the `sips` fallback. iOS and Android use their existing platform shapes.
