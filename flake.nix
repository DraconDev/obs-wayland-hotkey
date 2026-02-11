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
      # NixOS Module
      nixosModules.default = import ./nix/module.nix;

      # Packages
      packages = forAllSystems (system: 
        let pkgs = pkgsFor system; in
        {
          default = pkgs.buildGoModule {
            pname = "obs-hotkey-go";
            version = "1.0.0";
            src = ./.;
            vendorHash = null;
          };
        }
      );

      # Development shell
      devShells = forAllSystems (system:
        let pkgs = pkgsFor system; in
        {
          default = pkgs.mkShell {
            buildInputs = with pkgs; [ go ];
          };
        }
      );

      # Apps for running directly
      apps = forAllSystems (system: {
        default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/obs-hotkey-go";
        };
      });
    };
}
