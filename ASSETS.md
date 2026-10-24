# Asset provenance

The website uses local project artwork, not stock screenshots or invented
product mockups.

## Asset terms

Project artwork, screenshots, logos, favicons, and their derived previews are
excluded from the root MIT license. That license grants no permission to reuse
these assets or the Holy Furries Studio branding. Third-party materials retain
their original licenses.

## Sources

The Rust build generates SHA-256 URLs for all served assets, including image
references inside CSS. Hashed URLs use a one-year immutable cache lifetime.
Pages, including the 404 page, use the current build's generated URLs.

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

Screenshots also have 480, 800, and 1200-pixel-wide WebP variants. `srcset` lets
the browser choose a size for the displayed width and pixel density, retaining
the full-size image for larger screens. Lazy images use `sizes="auto"` with a
viewport-based fallback. Generate the variants with
`nix build .#image-variants`, then copy `result/*.webp` to `static/images/`. The
flake checks that these generated files match their sources.

Datastar 1.0.4 is built from the pinned upstream source by `nix/datastar.nix`.
The bundle includes fetch actions, `data-init`, and element patching. Its MIT
license is included at `static/vendor/datastar-LICENSE.txt`. Rebuild the
vendored artifact with `nix build .#datastar`, then copy `result/datastar.js` to
`static/vendor/datastar-1.0.4.js`. Keep this generated file unformatted.

## Crowns Adrift project page

The expanded game page uses a second gameplay capture,
`crowns-adrift-detail.webp`, from `../tribute/docs/art-study-zoom.png`. It shows
the palace and riverbank with the live game interface.

The following illustrations are lossless WebP conversions of the current in-game
sprites under `../tribute/crates/client/assets/`:

| Website asset                  | Source                  |
| ------------------------------ | ----------------------- |
| `crowns-adrift-fields.webp`    | `buildings/fields.png`  |
| `crowns-adrift-granary.webp`   | `buildings/granary.png` |
| `crowns-adrift-foundry.webp`   | `buildings/foundry.png` |
| `crowns-adrift-levy.webp`      | `units/levy.png`        |
| `crowns-adrift-commander.webp` | `units/commander.png`   |

These are renders of the project's authored low-poly models, rather than the
historical pixel-art sheets retained in that repository. The original sprite
source records are `assets/buildings/SOURCE.txt` and `assets/units/SOURCE.txt`
relative to the game client. Each illustration and the second gameplay capture
has a matching 64-pixel loading preview. The page labels the images as
in-development artwork.
