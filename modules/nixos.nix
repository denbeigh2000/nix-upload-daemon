{ pkgs, lib, config, ... }@inputs:

let
  cfg = config.services.nix-upload-daemon;

  inherit (lib) mkIf mkOption types;

  command = pkgs.callPackage ./wrapper.nix cfg;
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

  config = mkIf cfg.enable {
    users.users.${cfg.username} = {
      isSystemUser = true;
      group = cfg.group;
    };
    users.groups.${cfg.group} = { };

    systemd.services.nix-upload-daemon = {
      wantedBy = [ "multi-user.target" ];
      script = "${command}/bin/nix-upload-daemon-wrapped";
      serviceConfig = {
        Restart = "always";
        RuntimeDirectory = "upload-daemon";
        User = cfg.username;
        Group = cfg.group;
      };
    };
  };
}
