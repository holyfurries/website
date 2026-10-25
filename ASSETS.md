# Assets

Artwork, screenshots, logos, favicons, and derived images are excluded from the
root MIT license. No permission to reuse the artwork or Holy Furries Studio
branding is granted. Third-party materials retain their original licenses.

## Sources

Branding and the shared splash live in
[holyfurries/brand-assets](https://github.com/holyfurries/brand-assets). Website
copies live under `static/`; project images live in `static/images/`.

| Image                       | Source                                                    |
| --------------------------- | --------------------------------------------------------- |
| `crowns-adrift-island.webp` | Current client, seed 1, `build 3` capture                 |
| `crowns-adrift-detail.webp` | Current client, seed 1, `city 7` capture                  |
| `cold-orbit-flight.webp`    | `../space-game/assets/contrast-lighting-burn-preview.png` |
| `spice-cad-editor.webp`     | `../spice-rack/docs/images/workspace.png`                 |

Crowns captures use `../tribute/scripts/shot.sh` at revision `1f29de6`. The
detail is a `1600x900+550+500` crop of a 3200 × 1800 capture.

Crowns illustrations use lossless WebP conversions from
`../tribute/crates/client/assets/`: `buildings/{fields,granary,foundry}.png` and
`units/{levy,commander}.png`. Source records are the `SOURCE.txt` files in those
directories.

`studio-social.png` is a 1200 × 630 export of `studio-social.svg`.

## Regeneration

- Screenshots: `cwebp`, quality 86 for games and 90 for CAD. Crop the top
  64-pixel debug strip from Cold Orbit first.
- Blur previews: ImageMagick `-resize 64x -strip -quality 45`, saved as
  `*-preview.webp`.
- Responsive variants: `nix build .#image-variants`, then copy `result/*.webp`
  to `static/images/`.

Datastar 1.0.4 uses the pinned build in `nix/datastar.nix`. Run
`nix build .#datastar` and copy `result/datastar.js` to
`static/vendor/datastar-1.0.4.js`. Its MIT license is at
`static/vendor/datastar-LICENSE.txt`.
