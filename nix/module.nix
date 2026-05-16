{ config, lib, pkgs, self, ... }:

let
  cfg = config.services.obs-hotkey;
in
{
  options.services.obs-hotkey = {
    enable = lib.mkEnableOption "OBS Hotkey Controller";
    user = lib.mkOption {
      type = lib.types.str;
      description = "User to run obs-hotkey as";
    };
    configFile = lib.mkOption {
      type = lib.types.path;
      default = "/home/${cfg.user}/.config/obs-hotkey/hotkeys.json";
      description = "Path to the hotkeys config file";
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ self.packages.${pkgs.system}.default ];

    users.users.${cfg.user}.extraGroups = [ "input" ];

    systemd.user.services.obs-hotkey = {
      description = "OBS Hotkey Controller";
      after = [ "graphical-session.target" ];
      wantedBy = [ "graphical-session.target" ];

      serviceConfig = {
        Type = "simple";
        ExecStart = "${self.packages.${pkgs.system}.default}/bin/obs-hotkey --config ${cfg.configFile}";
        Restart = "on-failure";
        RestartSec = "10s";
      };
    };
  };
}