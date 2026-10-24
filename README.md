# Holy Furries Studio

The website for [holyfurries.com](https://holyfurries.com): games, mods, and
software tools.

Rust, Axum, and Askama serve HTML with embedded templates and assets. Datastar
updates the mod catalog over SSE, and NATS carries release metadata from
Thunderstore. Latest versions, descriptions, and total downloads across all
versions refresh every 30 minutes. Failed requests retain the current metadata;
written descriptions remain the fallback before the first successful fetch.
Pages and install links work without browser JavaScript.

Datastar loads only on the homepage. Its pinned Nix build includes fetch,
initialization, and element patching. Each homepage response uses a fresh
cryptographic nonce for Datastar's CSP mode, so dynamic JavaScript evaluation is
disabled. The browser removes the nonce attribute after initialization.

## Run

Enter the development shell in each terminal:

```console
nix develop
```

Run the website:

```console
cargo run --locked
```

Open http://127.0.0.1:8080. `LISTEN_ADDRESS` overrides the HTTP address. The
website starts its own NATS child on an automatically assigned loopback port,
waits for it to become ready, and stops and reaps it on shutdown. If NATS exits,
the website exits with an error so its service manager can restart both.
`/healthz` checks the private NATS connection. The application needs no runtime
asset directory or separate NATS service.

The flake uses flake-parts, devshell, crane, and rust-overlay. The shell and
package build share a pinned Rust toolchain. The shell provides `nats-server`;
the packaged website uses its bundled, pinned server binary directly.

## Development

```console
nix develop --command cargo fmt --check
nix develop --command cargo test --locked
nix develop --command cargo clippy --locked --all-targets -- -D warnings
nix build
nix build .#checks.x86_64-linux.browser
```

The packaged executable is `./result/bin/holyfurries`. Tests start their own
local NATS server and cover routes, assets, release validation, and SSE updates
and shutdown.

The browser check runs Chromium and Firefox with Nix-pinned drivers. It checks
navigation, constrained layouts, enlarged text, image loading, reduced motion,
and live catalog updates. CI and tagged releases run it through
`nix flake check`. Tests use local NATS messages rather than external API
responses.

`Cargo.toml` is the version source for the Nix package and catalog user agent.
`Cargo.lock` records the generated package metadata.

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
adapt to available space, with a container query keeping the mobile header
compact. Images show small blurred previews while loading and fade in after
decoding. The navigation stays visible while scrolling, and section links
account for its current height. Pages enter with a quick upward movement.
Browsers with cross-document view transitions also animate internal page
navigation. The homepage fox enters with the shared studio splash's halo bounce,
glance, and blink. Reduced-motion mode disables these animations.

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

The module runs one website service, including its private NATS child. Configure
public routing and TLS in the host configuration. The deployment-specific
`nixosModules.atlas` export contains the existing site's tunnel routing.

The Atlas module enables automatic deployment of published stable releases.
Every five minutes Atlas checks `holyfurries/website`, verifies the archive's
SHA-256 checksum, imports its Nix closure, and switches only the website's
profile. A failed restart or health check restores the previous package. No
GitHub credentials or inbound deployment connection are required.

Push a `vX.Y.Z` tag matching `Cargo.toml` to release. The Release workflow runs
the full CI checks, builds the package, and tests the exported archive on a
fresh runner before publishing it. Ordinary pushes to `main` run CI without
deploying. Run `systemctl start holyfurries-update` on Atlas to check
immediately; inspect deployments with `journalctl -u holyfurries-update`.

For a manual rollback, stop `holyfurries-update.timer`, run
`nix-env --profile /nix/var/nix/profiles/holyfurries --rollback`, and restart
`holyfurries`. Resume the timer once the release issue is resolved. These
commands require root. The current release is recorded in
`/var/lib/holyfurries-update/deployed`.

The generic module defaults to the package pinned by the host flake. Enable
`services.holyfurries.autoUpdate.enable` to opt into release deployment on
`x86_64-linux`. The initial profile uses the pinned package; subsequent releases
persist across reboots and NixOS rebuilds. Updating NixOS still updates the
service module and updater executable.

For hosts running the older separate NATS service, deploy the website release
with the owned NATS child first. Then update the host's module to remove the
older service dependency. This keeps the currently deployed website working
through the transition. Rolling back across this change also requires restoring
the previous host module so the older website has its separate NATS service
available again.

To update the host's website input and service configuration, replace `HOST`
with the NixOS configuration name:

```console
nix flake update holyfurries --flake path:/etc/nixos
nix build path:/etc/nixos#nixosConfigurations.HOST.config.system.build.toplevel
sudo ./result/bin/switch-to-configuration dry-activate
sudo nixos-rebuild switch --flake path:/etc/nixos#HOST
```

Importing a matching release archive first lets the rebuild reuse the tested
package. Review the dry activation before switching. To return to the previous
NixOS generation, use `sudo nixos-rebuild switch --rollback`; automatic release
profiles have their own rollback described above.

`ops/` manages the Holy Furries zone's DNS records, DNSSEC, redirects, TLS, and
caching. HTTP redirects to HTTPS, and `www` redirects permanently to
`holyfurries.com` while preserving paths and query strings. The TLS minimum is
1.2. HSTS lasts 30 days, with `includeSubDomains` and preload disabled.
Cloudflare adds the policy to HTTPS responses, including redirects. Only static
GET and HEAD responses are eligible for caching. Edge and browser lifetimes
follow the origin headers: content-hashed assets use a one-year immutable
lifetime. Pages, live mod updates, and health checks bypass the edge cache.
Supply `TF_VAR_zone_id` and `CLOUDFLARE_API_TOKEN` through the environment or a
secret manager. Validate changes with:

```console
tofu -chdir=ops init -backend=false
tofu -chdir=ops validate
tofu -chdir=ops test
```

The token needs DNS edit, Zone Settings edit, Single Redirect edit, and Cache
Rules edit permissions for the Holy Furries zone. Before the first plan, inspect
existing redirect and cache phase rulesets in Cloudflare. If present, import
them into `cloudflare_ruleset.redirects` and `cloudflare_ruleset.cache` using
`zones/<zone_id>/<ruleset_id>`, and reconcile their rules before applying.
Import existing DNSSEC settings into `cloudflare_zone_dnssec.website` using the
zone ID. Review `tofu -chdir=ops plan` before applying.

Holy Furries uses Cloudflare Registrar, which publishes the DS record
automatically after DNSSEC is enabled. This can take one to two days. Inspect
the expected record with `tofu -chdir=ops output -raw dnssec_ds_record` and
verify the parent DS record and signed DNS responses before considering DNSSEC
complete. For rollback, disable DNSSEC at the registrar and wait for the parent
DS record to expire before disabling zone signing. The DNSSEC resource prevents
accidental destruction.

Terraform state, plans, local credentials, and validation artifacts are ignored
by Git.

## Releases

GitHub Actions checks builds and formatting on pushes to `main` and pull
requests. The Release workflow builds an `x86_64-linux` archive for the server's
architecture, then imports it on a fresh runner and starts the website alone to
check its owned NATS startup, health, and public routes.

Running Release manually produces downloadable workflow artifacts. Pushing a tag
matching the Cargo version, such as `v0.1.0`, also publishes the verified
archives and SHA-256 checksums to GitHub Releases.

Each archive contains the application's complete Nix runtime closure, its store
path, documentation, and license notices. It requires Nix on the matching Linux
architecture. The archive includes the NATS server runtime. For an x86-64
release:

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
