{
  lib,
  runCommand,
  imagemagick,
}:
let
  source = lib.fileset.toSource {
    root = ../static/images;
    fileset = lib.fileset.unions [
      ../static/images/crowns-adrift-island.webp
      ../static/images/crowns-adrift-detail.webp
      ../static/images/cold-orbit-flight.webp
      ../static/images/spice-cad-editor.webp
    ];
  };
in
runCommand "website-image-variants" { nativeBuildInputs = [ imagemagick ]; } ''
  mkdir -p "$out"
  for name in crowns-adrift-island crowns-adrift-detail cold-orbit-flight spice-cad-editor; do
    quality=86
    if [ "$name" = spice-cad-editor ]; then quality=90; fi
    for width in 480 800 1200; do
      magick ${source}/"$name.webp" -resize "''${width}x" -strip \
        -quality "$quality" "$out/$name-$width.webp"
    done
  done
''
