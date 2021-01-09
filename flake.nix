{
  description = "Play videos in your terminal";

  inputs = {
    utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nmattia/naersk";
    mozillapkgs = {
      url = "github:mozilla/nixpkgs-mozilla";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, utils, naersk, mozillapkgs }:
    utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages."${system}";

      mozilla = pkgs.callPackage (mozillapkgs + "/package-set.nix") {};
      rust = (mozilla.rustChannelOf {
        date = "2021-01-09";
        channel = "nightly";
        sha256 = "sha256-utyBii+c0ohEjvxvr0Cf8MB97du2Gsxozm+0Q+FhVNw=";
      }).rust;

      naersk-lib = naersk.lib."${system}".override {
        cargo = rust;
        rustc = rust;
      };

      nativeBuildInputs = with pkgs; [ pkgconfig makeWrapper ];
      buildInputs = with pkgs; [
        glib
        libsixel

        # gstreamer - Needed to compile
        gst_all_1.gstreamer
        gst_all_1.gst-plugins-base

        # gstreamer - Needed to play mp4s
        gst_all_1.gst-plugins-good
        gst_all_1.gst-plugins-bad
      ];
    in rec {
      # `nix build`
      packages.termplay = naersk-lib.buildPackage {
        pname = "termplay";
        root = ./.;

        inherit nativeBuildInputs buildInputs;

        cargoBuildOptions = prev: prev ++ [ "--features" "bin" ];

        overrideMain = prev: {
          installPhase = ''
            ${prev.installPhase}
            wrapProgram "$out/bin/termplay" --set GST_PLUGIN_SYSTEM_PATH_1_0 "$GST_PLUGIN_SYSTEM_PATH_1_0"
          '';
        };
      };
      defaultPackage = packages.termplay;

      # `nix run`
      apps.termplay = utils.lib.mkApp {
        drv = packages.termplay;
      };
      defaultApp = apps.termplay;

      # `nix develop`
      devShell = pkgs.mkShell {
        buildInputs = buildInputs;
        nativeBuildInputs = nativeBuildInputs ++ [ rust ];
      };
    });
}
