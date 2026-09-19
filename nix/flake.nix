{
  description = "A startup rust project with devshell";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.devshell.flakeModule ];

      perSystem =
        { pkgs, system, ... }:
        let
          rust-toolchain = (pkgs.rust-bin.fromRustupToolchainFile ../rust-toolchain.toml).override {
            extensions = [
              "rust-src"
              "rustc-dev"
              "llvm-tools-preview"
              "rust-analyzer"
            ];
          };

          verusfmt =
            let
              src = pkgs.fetchFromGitHub {
                owner = "verus-lang";
                repo = "verusfmt";
                rev = "v0.7.2";
                hash = "sha256-TE1Qyk5y8G/Kid6/BmUIMZ8Fr+y8GkLkRrzXGFHVe7I=";
              };
            in
            pkgs.rustPlatform.buildRustPackage {
              pname = "verusfmt";
              version = "0.7.2";
              inherit src;
              cargoHash = "sha256-QY8Sju3AzfGiSp6V2TsuUlT1EmW3rOdhp3EeU1XM3Bg=";
              doCheck = false;
              meta.mainProgram = "verusfmt";
            };
        in
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [
              (import inputs.rust-overlay)
            ];
          };

          devshells.default = {
            imports = [ "${inputs.devshell}/extra/language/c.nix" ];
            packages = [
              rust-toolchain
              verusfmt
            ]
            ++ (with pkgs; [
              gnumake
              z3
            ]);
            language.c.includes = [ pkgs.openssl ];
            language.c.libraries = with pkgs; [ openssl.dev ];
          };
          packages.verusfmt = verusfmt;
        };

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
    };
}
