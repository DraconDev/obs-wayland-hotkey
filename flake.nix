{
  description = "OBS Wayland Hotkey Controller";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      pkgsFor = system: import nixpkgs { inherit system; };
    in
    {
      nixosModules.default = import ./nix/module.nix self;

      packages = forAllSystems (system:
        let pkgs = pkgsFor system; in
        {
          default = pkgs.buildGoModule {
            pname = "obs-hotkey";
            version = "1.0.0";
            src = ./.;
            vendorHash = null;
            ldflags = [ "-s" "-w" ];
            postInstall = ''
              mv $out/bin/obs-wayland-hotkey $out/bin/obs-hotkey
              cat > $out/bin/obs-hotkey-install-service << 'WRAPPER'
              #!/bin/sh
              exec @out@/bin/obs-hotkey --install-service
              WRAPPER
              chmod +x $out/bin/obs-hotkey-install-service
              substituteInPlace $out/bin/obs-hotkey-install-service --replace '@out@' $out
            '';
          };
        }
      );

      devShells = forAllSystems (system:
        let pkgs = pkgsFor system; in
        {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [ go ];
          };
        }
      );

      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/obs-hotkey";
        };
        "install-service" = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/obs-hotkey-install-service";
        };
      });
    };
}
