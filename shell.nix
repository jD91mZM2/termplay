with import <nixpkgs> {};
pkgs.termplay.overrideAttrs(old: {
  buildInputs = lib.remove pkgs.rustc old.buildInputs;
})
