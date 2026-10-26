# Float View icons

`icon.svg` is the source artwork for the native application icons. Regenerate
all platform assets from the repository root with:

```sh
tools/icon/generate.sh
```

The rider artwork is adapted from [“onewheel” by Luis Prado, Noun Project](https://thenounproject.com/icon/onewheel-4260841/),
image 4260841, under [CC BY 3.0](https://creativecommons.org/licenses/by/3.0/).
Colours, background, silhouettes and formats have been adapted. Preserve the
credit and separate artwork licence when regenerating app/loading/map assets;
see [NOTICE](../../NOTICE) and [licensing details](../../docs/licensing.md).

The generator requires `rsvg-convert`, ImageMagick's `magick`, and Apple's
`sips` (supplied by macOS), plus Python 3. When a working `iconutil` is available, it is used
for the macOS ICNS; otherwise the generator falls back to a valid 512px ICNS
written by `sips`. The output includes the iOS default AppIcon PNG, macOS ICNS, and
Android density/adaptive-icon WebP assets used by the native targets.

The centered loading screen uses the rider without the icon background. Native
and early HTML loading screens use the same Fresh teal (`#14b8a6`) artwork,
outline and shadow derived by `generate-light-artwork.py`. The dark loading screen retains its
bright teal (`#2dd4bf`) silhouette. Light loading PNGs retain their original
colours rather than being tinted as template images. Regenerate the early HTML,
iOS/macOS PNGs, Android light PNG and dark vector, and the frontend map's
separate SVG silhouette with Python 3, ImageMagick and `rsvg-convert`:

```sh
python3 tools/icon/generate-loading-symbol.py
```

macOS uses `macos-mask.svg` to round the artwork, with an 824px face centered
on a transparent 1024px canvas so its Dock size matches other macOS icons.
The mask and padding are applied before generating every ICNS resolution,
including the `sips` fallback. This dark ICNS also gives unbundled SwiftPM launches
an icon. iOS and Android use their existing platform shapes.

All platforms default to the original bright teal (`#2dd4bf`) rider on charcoal
(`#121418`), regardless of system light/dark appearance.

`FloatView.icon/` provides the layered macOS 26 icon. Its rider SVG is generated
from `icon.svg`, and `icon.json` applies the dark palette as the default.
`backends/apple/package-macos.sh` requires Xcode 26 or later and compiles it
into `Assets.car` plus an ICNS fallback before signing. Packaged apps retain
system icon tinting. macOS 14/15 and bare SwiftPM launches use the dark fallback.

Android uses the same dark artwork in default density and adaptive-icon assets.
The monochrome layer supports Android 13+ themed icons, whose colours follow
the launcher's wallpaper/theme settings. No launcher aliases or component
switching are used.

iOS uses one default AppIcon without a separate dark variant. Regenerate only
this asset with `tools/icon/generate.sh --ios-only`. System Home Screen tinting
remains under the user's control.

`generate-light-artwork.py` remains in use for light loading screens. It derives
a Fresh teal (`#14b8a6`) rider with a thin ink teal (`#073832`) outline and a
small dark teal (`#052e2b`) shadow. Dark loading, map and Android monochrome
symbols retain their existing silhouettes.
