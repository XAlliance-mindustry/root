{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { nixpkgs, naersk, ... }:
    let
      systems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = callback:
        nixpkgs.lib.genAttrs systems (system:
          callback system nixpkgs.legacyPackages.${system}
        );

    in rec {
      packages = forAllSystems (system: pkgs:
        let 
          naersk' = pkgs.callPackage naersk {};

        in rec {
          default = naersk'.buildPackage {
            src = ./.;
            copyLibs = true;
          };

          core = pkgs.mindustry-server.overrideAttrs (prev:{
            postFixup = ''
              wrapProgram $out/bin/mindustry-server \
                --set LD_LIBRARY_PATH ${default}/lib
            '';

            patches = prev.patches ++ [ ./patches/core.patch ];
          });
        });

      hydraJobs = {
        default = forAllSystems (system: pkgs:
          packages.${system}.default
        );

        core = forAllSystems (system: pkgs:
          packages.${system}.core
        );
      };

      devShells = forAllSystems (system: pkgs: {
        default = pkgs.mkShell {
          nativeBuildInputs = with pkgs; [ rustc cargo ];
        };
      });
    };
}