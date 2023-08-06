{ naersk }:

final: prev:
let naersk' = prev.callPackage naersk { };
in
{
  nix-upload-daemon = import ./. { naersk = naersk'; };
}
