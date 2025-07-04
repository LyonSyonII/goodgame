{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    fenix.url = "github:nix-community/fenix/monthly";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = { self, nixpkgs, flake-utils, ... }@inputs: flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ inputs.fenix.overlays.default ];
      };
      lib = pkgs.lib;
      components = [
          "rustc"
          "cargo"
          "clippy"
          "rustfmt"
          "rust-analyzer"
          "rust-src"
          "llvm-tools-preview"
          # Nightly
          "rustc-codegen-cranelift-preview"
          "miri"
      ];
      target = "x86_64-unknown-linux-gnu";
      nightly = pkgs.fenix.complete.withComponents components;
      # stable = pkgs.fenix.stable.withComponents ( nixpkgs.lib.sublist 0 (builtins.length components - 3) components );
    in {
      devShells.default = pkgs.mkShell rec {
        nativeBuildInputs = with pkgs; [
          # stable
          nightly
          fenix.targets.${target}.latest.rust-std
        ];

        buildInputs = with pkgs; [
          pkg-config
        ];

        RUST_SRC_PATH = "${pkgs.fenix.complete.rust-src}/lib/rustlib/src/rust/library";
        RUSTC_WRAPPER = "${pkgs.sccache}/bin/sccache";
        CARGO_BUILD_TARGET = target;
        "CARGO_TARGET_${builtins.replaceStrings ["-"] ["_"] (lib.strings.toUpper target)}_LINKER" = "${pkgs.clang}/bin/clang";
        RUSTFLAGS = ''-Clink-arg=-fuse-ld=${pkgs.mold}/bin/mold'';
        # RUSTFLAGS = ''-Clink-arg=-fuse-ld=${pkgs.lld}/bin/ld.lld''; # Uncomment if 'mold' does not link correctly

        LD_LIBRARY_PATH = lib.makeLibraryPath nativeBuildInputs;
      };

      apps.init = {
        type = "app";
        program = "${(pkgs.writeShellScript "shell-init" "${nightly}/bin/cargo init")}";
      };
    });
}
