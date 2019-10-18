{ pkgs ? import <nixpkgs> {} }:

let
  inherit (pkgs) rustPlatform;
in

{
  termplay = rustPlatform.buildRustPackage rec {
    name = "termplay";
    src = <src>;
    buildInputs = [];
    nativeBuildInputs = [];
    cargoSha256 = "0000000000000000000000000000000000000000000000000000000000000000";
  };
}
