{ writeShellApplication
, lib
, binding
, target
, workers
, package
, nix
, openssh
, ...
}:

let
  inherit (builtins) isNull toString;
  inherit (lib) optionalString;
in

writeShellApplication {
  name = "nix-upload-daemon-wrapped";
  runtimeInputs = [ package nix openssh ];
  text = ''
    nix-upload-daemon \
      --bind "${binding}" \
      serve \
      --copy-destination "${target}" ${lib.optionalString (!isNull workers) "--workers ${toString workers}"}
  '';
}
