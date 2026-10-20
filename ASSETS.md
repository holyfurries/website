# Asset provenance

The website uses local project artwork, not stock screenshots or invented
product mockups.

## Asset terms

Project artwork, screenshots, logos, favicons, and their derived previews are
excluded from the root MIT license. That license grants no permission to reuse
these assets or the Holy Furries Studio branding. Third-party materials retain
their original licenses.

## Sources

The shared product splash and canonical studio branding live in
[holyfurries/brand-assets](https://github.com/holyfurries/brand-assets). The
website keeps local copies of the mark and favicon for serving its pages. The
inline hero mark and its entrance animation follow the shared splash's artwork
and timing.

| Website asset                      | Source                                                                                |
| ---------------------------------- | ------------------------------------------------------------------------------------- |
| `static/images/crowns-adrift.webp` | `../tribute/crates/client/assets/menu/harbour.png`                                    |
| `static/images/cold-orbit.webp`    | `../space-game/assets/contrast-lighting-burn-preview.png`                             |
| `static/images/spice-cad.webp`     | `../spice-rack/output/native/ui-pass-verified-qa/desktop.png`                         |
| `static/mark.svg`                  | Original geometric fox and halo drawn for this website                                |
| `static/favicon.svg`               | Simplified fox and halo with heavier features and a solid background for browser tabs |

WebP conversions use `cwebp` at quality 86 for games and 90 for the CAD
interface. Originals remain in their source projects. The Crowns Adrift title
art’s existing provenance is recorded in
`../tribute/crates/client/assets/menu/SOURCE.txt`. These are development images
and are identified as such on the project pages.

Each `*-preview.webp` is a 64-pixel-wide copy of the corresponding website
image, resized with ImageMagick (`-resize 64x -strip -quality 45`) and blurred
by CSS. CSS displays these while the full images load. The small browser module
fades in decoded images and retains the preview if loading fails. Full images
remain visible when JavaScript is disabled. Image dimensions reserve layout
space on both the homepage and project pages.

Datastar 1.0.4 is vendored from
https://cdn.jsdelivr.net/gh/starfederation/datastar@v1.0.4/bundles/datastar.js.
Its MIT license is included at `static/vendor/datastar-LICENSE.txt`. Update the
pinned filename, embedded asset route, base template, and tests together when
upgrading.
