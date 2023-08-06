let
  nixos = import ./nixos.nix;
  darwin = import ./nixos.nix;
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
