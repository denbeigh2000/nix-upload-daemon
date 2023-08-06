{ pkgs, lib, config }:

let
  cfg = config.services.nix-upload-daemon;

  inherit (pkgs) nix-upload-daemon writeShellScript;
  inherit (lib) mkEnableOption mkIf mkOption optionalString types;
  description = "a daemon that asynchronously copies paths to a remote store";
  key-flag = optionalString (cfg.keyPath != null) "--sign-key ${cfg.keyPath}";
  upload-hook = writeShellScript "post-build-hook" ''
    OUT_PATHS="$OUT_PATHS" ${nix-upload-daemon}/bin/nix-upload-daemon \
      --bind ${cfg.binding} \
      upload ${key-flag}
  '';
in

{
  options = with types; {
    enable = mkEnableOption description;
    target = mkOption {
      description = "URL of store to upload to";
      type = str;
    };
    binding = mkOption {
      description = "URL of socket to bind to (either tcp or unix)";
      type = str;
    };
    package = mkOption {
      description = "Package containing upload-daemon";
      type = package;
      default = pkgs.nix-upload-daemon;
    };
    post-build-hook = {
      enable = mkEnableOption "post-build-hook that uploads the built path to a remote store";
      secretKey = mkOption {
        type = path;
        description = "Path to the key with which to sign the paths";
      };
    };
    workers = mkOption {
      description = "Number of nix-copies to run at the same time, null means use the number of CPUs";
      type = nullOr int;
      default = null;
      example = 4;
    };
    username = mkOption {
      description = "User to run daemon as";
      type = str;
      default = "upload-daemon";
      example = "upload-daemon";
    };
    uid = mkOption {
      description = "UID for the created user";
      type = int;
      default = 712;
      example = 712;
    };
  };

  config = mkIf cfg.enable {
    users.users.${cfg.user} = {
      isSystemUser = true;
      inherit (cfg) uid;
      # group = cfg.group;
    };

    nix.extraOptions = optionalString cfg.post-build-hook.enable "post-build-hook = ${upload-paths}";

    users.groups.${cfg.group} = {};

    # systemd.services.upload
  };
}
