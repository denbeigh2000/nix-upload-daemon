{ lib, binding, target, workers, package, ... }:

let
  inherit (builtins) isNull toString;
  inherit (lib) optionalString;
in

''
${package}/bin/nix-upload-daemon \
  --bind "${binding}" \
  serve \
  --copy-destination "${target}" ${lib.optionalString (!isNull workers) "--workers ${toString workers}"}
''
