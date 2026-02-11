{ config, lib, pkgs, ... }:

let
  obs-hotkey = pkgs.buildGoModule {
    pname = "obs-hotkey";
    version = "1.0.0";
    src = ./..;
    vendorHash = null; # Uses vendor directory
  };
in
{
  options.services.obs-hotkey = {
    enable = lib.mkEnableOption "OBS Wayland Hotkey Controller";
  };

  config = lib.mkIf config.services.obs-hotkey.enable {
    # Install binary system-wide
    environment.systemPackages = [ obs-hotkey ];

    # Create systemd service
    systemd.user.services.obs-hotkey = {
      description = "OBS Hotkey Controller (Wayland)";
      after = [ "graphical-session.target" ];
      wantedBy = [ "graphical-session.target" ];
      
      serviceConfig = {
        Type = "simple";
        ExecStart = "${obs-hotkey}/bin/obs-wayland-hotkey";
        Restart = "on-failure";
        RestartSec = "10s";
      };
    };

    # Allow running without password (needed for /dev/input access)
    security.sudo.extraRules = [
      {
        users = [ "dracon" ];
        commands = [
          {
            command = "${obs-hotkey}/bin/obs-wayland-hotkey";
            options = [ "NOPASSWD" ];
          }
        ];
      }
    ];
  };
}
