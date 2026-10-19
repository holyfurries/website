{
  lib,
  crane_lib,
  nats-server,
}:
let
  common_args = {
    pname = "holyfurries";
    version = "0.1.0";
    src = lib.fileset.toSource {
      root = ../.;
      fileset = lib.fileset.unions [
        ../Cargo.toml
        ../Cargo.lock
        ../src
        ../templates
        ../static
      ];
    };
    strictDeps = true;
  };
  cargo_artifacts = crane_lib.buildDepsOnly common_args;
in
crane_lib.buildPackage (
  common_args
  // {
    cargoArtifacts = cargo_artifacts;
    nativeCheckInputs = [ nats-server ];
    meta.mainProgram = "holyfurries";
  }
)
