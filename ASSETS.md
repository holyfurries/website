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

| Website asset                             | Source                                                                                |
| ----------------------------------------- | ------------------------------------------------------------------------------------- |
| `static/images/crowns-adrift-island.webp` | `../tribute/docs/art-study-board.png`                                                 |
| `static/images/cold-orbit-flight.webp`    | `../space-game/assets/contrast-lighting-burn-preview.png`                             |
| `static/images/spice-cad-editor.webp`     | `../spice-rack/docs/images/workspace.png`                                             |
| `static/mark.svg`                         | Original geometric fox and halo drawn for this website                                |
| `static/favicon.svg`                      | Simplified fox and halo with heavier features and a solid background for browser tabs |
| `static/images/studio-social.svg`         | Studio mark and wordmark arranged for shared-link previews                            |

WebP conversions use `cwebp` at quality 86 for games and 90 for the CAD
interface. Originals remain in their source projects. The Crowns Adrift image is
a 1600 by 900 gameplay capture from the current client showing its low-poly
island terrain, rendered buildings, resource panel, and cards. Spice CAD shows
its updated typed Luau editor, rounded-cube model, and parameter controls. These
are development images and are identified as such on the project pages.

The Cold Orbit capture has its top 64-pixel debug strip cropped before WebP
conversion. This removes the FPS/frame-time overlay while retaining the ship and
gameplay controls.

`studio-social.png` is a 1200 × 630 browser-rendered export of
`studio-social.svg`, with an opaque cream background for shared-link previews.
Open Graph and Twitter metadata use this studio artwork independently of the
project screenshots.

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
