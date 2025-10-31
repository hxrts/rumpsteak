{
  description = "Rumpsteak Aura - Session types for asynchronous communication";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
        ];

        buildInputs = with pkgs; [
          openssl
        ] ++ lib.optionals stdenv.isDarwin [
          libiconv
        ];

      in
      {
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs buildInputs;
          
          shellHook = ''
            echo "Rumpsteak Aura development environment"
            echo "Rust version: $(rustc --version)"
            echo "Cargo version: $(cargo --version)"
          '';
        };

        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "rumpsteak-aura";
          version = "0.1.1-aura";
          
          src = ./.;
          
          cargoLock = {
            lockFile = ./Cargo.lock;
          };
          
          inherit nativeBuildInputs buildInputs;
          
          meta = with pkgs.lib; {
            description = "Session types for asynchronous communication between multiple parties - Aura fork for threshold cryptography choreographies";
            homepage = "https://github.com/aura-project/rumpsteak-aura";
            license = with licenses; [ mit asl20 ];
            maintainers = [ ];
          };
        };
      });
}