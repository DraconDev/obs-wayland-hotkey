{ config, lib, pkgs, ... }:

let
  cfg = config.services.obs-hotkey;
in
{
  options.services.obs-hotkey = {
    enable = lib.mkEnableOption "OBS Hotkey Controller";
    user = lib.mkOption {
      type = lib.types.str;
      default = "dracon";
      description = "User to run obs-hotkey as";
    };
    configFile = lib.mkOption {
      type = lib.types.path;
      default = "/home/${cfg.user}/.config/obs-hotkey/hotkeys.json";
      description = "Path to the hotkeys config file";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ pkgs.obs-hotkey ];

    users.users.${cfg.user}.extraGroups = [ "input" ];

    systemd.user.services.obs-hotkey = {
      description = "OBS Hotkey Controller";
      after = [ "graphical-session.target" ];
      wantedBy = [ "graphical-session.target" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${pkgs.obs-hotkey}/bin/obs-hotkey --config ${cfg.configFile}";
        Restart = "on-failure";
        RestartSec = "10s";
      };
    };
  };
}
