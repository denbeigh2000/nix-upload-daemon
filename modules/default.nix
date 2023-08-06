let
  nixos = import ./nixos.nix;
  darwin = import ./darwin.nix;
in
{
  nixosModules = {
    default = nixos;
    nix-upload-daemon = nixos;
  };

  darwinModules = {
    default = darwin;
    nix-upload-daemon = darwin;
  };
}
