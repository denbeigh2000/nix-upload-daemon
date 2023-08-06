{ pkgs, lib, config, ... }@inputs:

let
  cfg = config.services.nix-upload-daemon;

  script = pkgs.callPackage ./wrapper.nix cfg;
in
{
  imports = [ ./common.nix ];

  config = {
    users.users.${cfg.username} = {
      # Unconditionally run in daemon group on darwin
      gid = 1;
      isHidden = true;
    };

    users.knownUsers = [ cfg.username ];

    launchd.daemons.upload-daemon = {
      inherit script;
      serviceConfig = {
        UserName = cfg.username;
        GroupName = "daemon";
        KeepAlive = true;
        StandardOutPath = "/tmp/org.nixos.nix-upload-daemon.log";
        StandardErrorPath = "/tmp/org.nixos.nix-upload-daemon.err";
        EnvironmentVariables = {
          PATH = "${pkgs.nix}/bin:${pkgs.openssh}/bin";
        };
      };
    };
  };
}
