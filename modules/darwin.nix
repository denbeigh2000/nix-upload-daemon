{ pkgs, lib, config, ... }@inputs:

let
  inherit (lib) mkIf;
  cfg = config.services.nix-upload-daemon;

  command = pkgs.callPackage ./wrapper.nix cfg;
in
{
  imports = [ ./common.nix ];

  config = mkIf cfg.enable {
    users.users.${cfg.username} = {
      # Unconditionally run in daemon group on darwin
      gid = 1;
      isHidden = true;
    };

    users.knownUsers = [ cfg.username ];

    launchd.daemons.upload-daemon = {
      command = "${command}/bin/nix-upload-daemon-wrapped";
      serviceConfig = {
        UserName = cfg.username;
        GroupName = "daemon";
        KeepAlive = true;
        StandardOutPath = "/tmp/org.nixos.nix-upload-daemon.log";
        StandardErrorPath = "/tmp/org.nixos.nix-upload-daemon.err";
      };
    };
  };
}
