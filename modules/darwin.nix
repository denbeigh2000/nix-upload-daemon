{ pkgs, lib, config }@inputs:

let
  cfg = config.services.nix-upload-daemon;

  script = pkgs.callPackage ./wrapper.nix cfg;
in
{
  imports = [ ./common.nix ];

  config = {
    # Unconditionally run in daemon group on darwin
    users.users.${cfg.user}.gid = 1;

    users.knownUsers = [ cfg.user ];

    launchd.daemons.upload-daemon = {
      inherit script;
      serviceConfig = {
        UserName = cfg.user;
        GroupName = "daemon";
        KeepAlive = true;
      };
    };
  };
}
