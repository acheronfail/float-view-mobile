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
written by `sips`. The output includes the shared Apple icon layer, macOS ICNS, and Android
density/adaptive-icon WebP assets used by the native targets.

The centered loading screen uses the rider without the icon background. Native
and early HTML loading screens use the same stroke-free silhouette as the app
icon, without an outline or shadow. Light loading screens use Fresh teal
(`#14b8a6`); dark loading screens use bright teal (`#2dd4bf`). Light loading PNGs retain their original
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
(`#121418`), regardless of system light/dark appearance. The app icon uses
filled paths without a stroke so all platform silhouettes have the same weight.

`FloatView.icon/` provides the shared layered iOS/macOS 26 and later icon. Its rider SVG is generated
from `icon.svg`, and `icon.json` applies the dark palette as the default.
Glass and specular highlights let Apple render reflective edges and lighting;
these effects are not baked into the source artwork.
`backends/apple/package-macos.sh` requires Xcode 26 or later and compiles it
into `Assets.car` plus an ICNS fallback before signing. Packaged apps retain
system icon tinting. macOS 14/15 and bare SwiftPM launches use the dark fallback.

Android uses the same dark artwork in default density and adaptive-icon assets.
The monochrome layer supports Android 13+ themed icons, whose colours follow
the launcher's wallpaper/theme settings. No launcher aliases or component
switching are used.

The iOS target includes the same `FloatView.icon` document as a resource and
selects `FloatView` as its app icon. Xcode compiles the layered icon and
generates compatibility images for older iOS versions; no separate AppIcon PNG
is maintained. System Home Screen tinting remains under the user's control.
