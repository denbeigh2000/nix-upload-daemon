{ pkgs, lib, config }@inputs:

let
  cfg = config.services.nix-upload-daemon;

  inherit (lib) mkOption types;

  script = pkgs.callPackage ./wrapper.nix cfg;
in
{
  imports = [ ./common.nix ];

  options = with types; {
    group = mkOption {
      description = "Group to run daemon as";
      type = str;
      default = cfg.user;
      example = "upload-daemon";
    };
  };

  config = {
    users.users.${cfg.user}.group = cfg.group;
    users.groups.${cfg.group} = {};

    systemd.services.nix-upload-daemon = {
      wantedBy = [ "multi-user.target" ];
      path = [ pkgs.nix ];
      inherit script;
      serviceConfig = {
        Restart = "always";
        RuntimeDirectory = "upload-daemon";
        User = cfg.user;
        Group = cfg.group;
      };
    };
  };
}
