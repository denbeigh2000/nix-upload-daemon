{ pkgs, lib, config, ... }@inputs:

let
  cfg = config.services.nix-upload-daemon;

  inherit (lib) mkOption types;

  script = pkgs.callPackage ./wrapper.nix cfg;
in
{
  imports = [ ./common.nix ];

  options.services.nix-upload-daemon = with types; {
    group = mkOption {
      description = "Group to run daemon as";
      type = str;
      default = cfg.username;
      example = "upload-daemon";
    };
  };

  config = {
    users.users.${cfg.username} = {
      isSystemUser = true;
      group = cfg.group;
    };
    users.groups.${cfg.group} = {};

    systemd.services.nix-upload-daemon = {
      wantedBy = [ "multi-user.target" ];
      path = [ pkgs.nix pkgs.openssh ];
      inherit script;
      serviceConfig = {
        Restart = "always";
        RuntimeDirectory = "upload-daemon";
        User = cfg.username;
        Group = cfg.group;
      };
    };
  };
}
