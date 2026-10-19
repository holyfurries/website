# Holy Furries Studio

The website for [holyfurries.com](https://holyfurries.com): games, mods, and
software tools.

Rust, Axum, and Askama serve HTML with embedded templates and assets. Datastar
updates the mod catalog over SSE, and NATS carries release metadata from
Thunderstore. Pages and install links work without browser JavaScript.

## Run

Enter the development shell in each terminal:

```console
nix develop
```

Start NATS in one terminal:

```console
nats-server --addr 127.0.0.1 --port 4222
```

Run the website in another:

```console
cargo run --locked
```

Open http://127.0.0.1:8080. `LISTEN_ADDRESS` and `NATS_URL` override the
defaults. NATS must be available at startup. `/healthz` returns 503 if its
connection goes down. The application needs no runtime asset directory.

The flake uses flake-parts, devshell, crane, and rust-overlay. The shell and
package build share a pinned Rust toolchain.

## Development

```console
nix develop --command cargo fmt --check
nix develop --command cargo test --locked
nix develop --command cargo clippy --locked --all-targets -- -D warnings
nix build
```

The packaged executable is `./result/bin/holyfurries`. Tests start their own
local NATS server and cover routes, assets, release validation, and SSE updates
and shutdown.

Formatting uses rustfmt, nixfmt, Biome for JavaScript and CSS, SuperHTML for
HTML, Taplo for TOML, mdformat with its GFM plugin for Markdown, and OpenTofu
for Terraform. Keep generated files and vendored code unchanged.

- `src/`: HTTP service, release metadata, and tests.
- `templates/`: Askama pages and shared HTML fragments.
- `static/`: styles, browser code, images, and vendored Datastar.
- `nix/`: package and NixOS modules.
- `ops/`: Cloudflare DNS configuration and mocked OpenTofu tests.
- [ASSETS.md](ASSETS.md): image provenance and asset licensing.

Colors use OKLCH. Typography, spacing, and gutters use
[Utopia](https://utopia.fyi/) fluid clamps. Flex wrapping and auto-fit grids
adapt to available space without layout breakpoints. Images show small blurred
previews while loading and fade in after decoding.

## Deployment

Add this repository as an input to the host flake:

```nix
inputs.holyfurries.url = "github:holyfurries/website";
```

Import its generic service module in the NixOS configuration:

```nix
{
  imports = [ inputs.holyfurries.nixosModules.default ];
  services.holyfurries.enable = true;
}
```

The module runs the website and a private NATS service on loopback. Configure
public routing and TLS in the host configuration. The deployment-specific
`nixosModules.atlas` export contains the existing site's tunnel routing.

`ops/` manages the site's Cloudflare DNS records and HTTPS redirect. Supply
`TF_VAR_zone_id` and `CLOUDFLARE_API_TOKEN` through the environment or a secret
manager. Validate changes with:

```console
tofu -chdir=ops init -backend=false
tofu -chdir=ops validate
tofu -chdir=ops test
```

Terraform state, plans, local credentials, and validation artifacts are ignored
by Git.

## Releases

GitHub Actions checks builds and formatting on pushes to `main` and pull
requests. The Release workflow builds an `x86_64-linux` archive for the server's
architecture, then imports it on a fresh runner and starts the website with NATS
to check its health and public routes.

Running Release manually produces downloadable workflow artifacts. Pushing a tag
matching the Cargo version, such as `v0.1.0`, also publishes the verified
archives and SHA-256 checksums to GitHub Releases.

Each archive contains the application's complete Nix runtime closure, its store
path, documentation, and license notices. It requires Nix on the matching Linux
architecture and a running NATS service. For an x86-64 release:

```console
sha256sum --check holyfurries-x86_64-linux.tar.gz.sha256
mkdir release
tar -xzf holyfurries-x86_64-linux.tar.gz -C release
sudo "$(command -v nix-store)" --import < release/closure.nar
"$(cat release/store-path)/bin/holyfurries"
```

## License

Source code and documentation are licensed under [MIT](LICENSE). Project
artwork, screenshots, logos, and their derived previews are excluded from that
license. See [ASSETS.md](ASSETS.md) for asset terms and third-party notices.
