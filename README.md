# Holy Furries Studio

The website for [holyfurries.com](https://holyfurries.com), built with Rust,
Axum, Askama, and Datastar. The process owns its NATS server and refreshes mod
metadata from Thunderstore every 30 minutes.

## Development

```console
nix develop
cargo run --locked
```

Open http://127.0.0.1:8080. Override the address with `LISTEN_ADDRESS`.

```console
nix flake check
nix develop --command cargo clippy --locked --all-targets -- -D warnings
```

Checks include Chromium and Firefox. `nix build` produces
`result/bin/holyfurries`. Keep generated assets and vendored code unformatted.
See [ASSETS.md](ASSETS.md) for sources and regeneration.

## Deployment

Update the version in `Cargo.toml` and regenerate `Cargo.lock`. Push a matching
`vX.Y.Z` tag. GitHub checks, builds, and verifies the release before publishing.
Atlas checks every five minutes and rolls back automatically if deployment
fails. Pushes to `main` run CI without deploying.

On Atlas, as root:

```console
systemctl start holyfurries-update
journalctl -u holyfurries-update
cat /var/lib/holyfurries-update/deployed
```

Manual rollback:

```console
systemctl stop holyfurries-update.timer
nix-env --profile /nix/var/nix/profiles/holyfurries --rollback
systemctl restart holyfurries
```

Resume the timer after resolving the release issue. NixOS service configuration
lives in `nix/service.nix`; Atlas routing lives in `nix/atlas.nix`.

## Cloudflare

`ops/` manages DNS, DNSSEC, HTTPS redirects, TLS, HSTS, and caching. Set
`TF_VAR_zone_id` and `CLOUDFLARE_API_TOKEN`. The token needs DNS Edit, Zone
Settings Edit, Single Redirect Edit, and Cache Rules Edit for this zone.

```console
nix develop
tofu -chdir=ops init
tofu -chdir=ops test
tofu -chdir=ops plan -out=website.tfplan
tofu -chdir=ops apply website.tfplan
```

Keep the local state. Import existing resources before managing them. Cloudflare
Registrar publishes the DNSSEC DS record automatically; verify the parent record
before treating DNSSEC as active. To disable DNSSEC, remove the parent DS record
and let it expire before disabling zone signing.

## License

Code and documentation: [MIT](LICENSE). Artwork and branding are excluded; see
[ASSETS.md](ASSETS.md).
