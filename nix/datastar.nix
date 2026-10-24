{
  runCommand,
  fetchFromGitHub,
  esbuild,
}:
let
  source = fetchFromGitHub {
    owner = "starfederation";
    repo = "datastar";
    rev = "v1.0.4";
    hash = "sha256-tWcN2oOuMxOHzw1xKrdsVBhngkkTFkrfBR3StAelQvQ=";
  };
in
runCommand "datastar-1.0.4" { nativeBuildInputs = [ esbuild ]; } ''
  cat > entry.js <<'EOF'
  import '@plugins/actions/fetch'
  import '@plugins/attributes/init'
  import '@plugins/watchers/patchElements'
  EOF
  mkdir -p "$out"
  esbuild entry.js --bundle --format=esm --target=es2021 --minify \
    --define:ALIAS=null --tsconfig=${source}/library/tsconfig.json \
    --outfile="$out/datastar.js"
  cp ${source}/LICENSE.md "$out/LICENSE.md"
''
