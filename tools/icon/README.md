# Float View icons

`icon.svg` is the source artwork for the native application icons. Regenerate
all platform assets from the repository root with:

```sh
tools/icon/generate.sh
```

The generator requires `rsvg-convert`, ImageMagick's `magick`, and Apple's
`sips` (supplied by macOS). When a working `iconutil` is available, it is used
for the macOS ICNS; otherwise the generator falls back to a valid 512px ICNS
written by `sips`. The output includes the iOS AppIcon PNG, macOS ICNS, and
Android density/adaptive-icon WebP assets used by the native targets.

macOS uses `macos-mask.svg` to round the artwork, with an 824px face centered
on a transparent 1024px canvas so its Dock size matches other macOS icons.
The mask and padding are applied before generating every ICNS resolution,
including the `sips` fallback. iOS and Android use their existing platform shapes.
