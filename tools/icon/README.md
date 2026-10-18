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
written by `sips`. The output includes the iOS light/default and dark AppIcon PNGs, macOS ICNS, and
Android density/adaptive-icon WebP assets used by the native targets.

The centered loading screen uses only the rider silhouette, without the icon
background. Native and early HTML loading screens tint it teal in light mode and
mint in dark mode to match the app icon artwork. Regenerate its early HTML,
iOS/macOS PNG and Android vector assets
and the frontend map’s SVG symbol independently on any platform with Python 3 and ImageMagick:

```sh
python3 tools/icon/generate-loading-symbol.py
```

macOS uses `macos-mask.svg` to round the artwork, with an 824px face centered
on a transparent 1024px canvas so its Dock size matches other macOS icons.
The mask and padding are applied before generating every ICNS resolution,
including the `sips` fallback. This light ICNS also gives unbundled SwiftPM launches
an icon. iOS and Android use their existing platform shapes.

`FloatView.icon/` provides the macOS 26 appearance variants. Its rider SVG is
generated from `icon.svg`; `icon.json` sets the pale slate/teal default and the
charcoal/mint dark appearance. `backends/apple/package-macos.sh` requires Xcode 26
or later and compiles it into `Assets.car` plus a light ICNS fallback before
signing. Packaged apps let macOS select the icon appearance, including system
tinting; they do not replace it with a static Dock image. macOS 14/15 and bare
SwiftPM launches use the light fallback.

Android uses the light palette in its default density and adaptive-icon assets,
with the original dark palette in `mipmap-night-*` and `drawable-night-nodpi`.
System night mode selects those resources; launcher caching can delay refreshing
the displayed icon. The existing monochrome layer supports Android 13+ themed
icons, whose colours follow the launcher's wallpaper/theme settings instead of
our teal/mint palettes. No launcher aliases or component switching are used.

iOS derives its light icon from the same SVG geometry with a pale slate background
and teal rider; the existing artwork supplies the explicit dark appearance.
Regenerate only these iOS assets with `tools/icon/generate.sh --ios-only`.
On iOS 18 and later, Home Screen appearance set to Automatic selects the variant
with the system appearance. An explicit Light, Dark or Tinted Home Screen choice
remains under the user's control. Older iOS versions use the light/default icon.
