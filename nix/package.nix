{
  lib,
  crane_lib,
  nats-server,
  chromium,
  chromedriver,
  firefox,
  geckodriver,
  makeFontsConf,
  dejavu_fonts,
}:
let
  common_args = {
    pname = "holyfurries";
    version = (builtins.fromTOML (builtins.readFile ../Cargo.toml)).package.version;
    src = lib.fileset.toSource {
      root = ../.;
      fileset = lib.fileset.unions [
        ../Cargo.toml
        ../Cargo.lock
        ../build.rs
        ../src
        ../templates
        ../static
        ../tests
      ];
    };
    strictDeps = true;
    NATS_SERVER = lib.getExe nats-server;
  };
  cargo_artifacts = crane_lib.buildDepsOnly common_args;
in
crane_lib.buildPackage (
  common_args
  // {
    cargoArtifacts = cargo_artifacts;
    postInstall = ''
      install -Dm644 ${nats-server.src}/LICENSE "$out/share/licenses/nats-server/LICENSE"
    '';
    meta.mainProgram = "holyfurries";
    passthru.browserCheck = crane_lib.cargoTest (
      common_args
      // {
        cargoArtifacts = cargo_artifacts;
        cargoTestExtraArgs = "--test browser -- --ignored";
        nativeCheckInputs = [
          chromium
          chromedriver
          firefox
          geckodriver
        ];
        preCheck = ''
          export HOME="$TMPDIR/browser-home"
          mkdir -p "$HOME"
          export FONTCONFIG_FILE=${makeFontsConf { fontDirectories = [ dejavu_fonts ]; }}
          export BROWSER_CHROMIUM=${lib.getExe chromium}
          export BROWSER_FIREFOX=${lib.getExe firefox}
          export MOZ_DISABLE_CONTENT_SANDBOX=1
        '';
      }
    );
  }
)
