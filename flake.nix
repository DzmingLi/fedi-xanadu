{
  description = "Fedi-Xanadu: federated knowledge sharing platform on AT Protocol";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    let
      nixosModule = { config, lib, pkgs, ... }:
        let
          cfg = config.services.fedi-xanadu;
          pkg = self.packages.${pkgs.system}.default;
        in {
          options.services.fedi-xanadu = {
            enable = lib.mkEnableOption "Fedi-Xanadu knowledge sharing platform";
            domain = lib.mkOption {
              type = lib.types.str;
              description = "Domain name for the service";
            };
            port = lib.mkOption {
              type = lib.types.port;
              default = 3847;
            };
            dataDir = lib.mkOption {
              type = lib.types.str;
              default = "/var/lib/fedi-xanadu";
            };
          };

          config = lib.mkIf cfg.enable {
            systemd.services.fedi-xanadu = {
              description = "Fedi-Xanadu";
              after = [ "network.target" ];
              wantedBy = [ "multi-user.target" ];
              environment = {
                FX_HOST = "127.0.0.1";
                FX_PORT = toString cfg.port;
                FX_DATABASE_URL = "sqlite://${cfg.dataDir}/fedi-xanadu.db?mode=rwc";
                FX_PIJUL_STORE_PATH = "${cfg.dataDir}/pijul-store";
              };
              serviceConfig = {
                ExecStart = "${pkg}/bin/fedi-xanadu";
                WorkingDirectory = "${pkg}/share/fedi-xanadu";
                DynamicUser = true;
                StateDirectory = "fedi-xanadu";
                Restart = "on-failure";
                RestartSec = 5;
              };
            };

            services.caddy.virtualHosts."${cfg.domain}".extraConfig = ''
              reverse_proxy 127.0.0.1:${toString cfg.port}
            '';
          };
        };
    in
    {
      nixosModules.default = nixosModule;
    } //
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "fedi-xanadu";
          version = "0.1.0";
          src = pkgs.lib.cleanSource ./.;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = with pkgs; [ openssl sqlite ];
          doCheck = false;
          env.SQLX_OFFLINE = "true";

          postInstall = ''
            mkdir -p $out/share/fedi-xanadu/frontend
            cp -r $src/frontend/dist $out/share/fedi-xanadu/frontend/dist
            cp -r $src/migrations $out/share/fedi-xanadu/migrations
            if [ -d "$src/static" ]; then
              cp -r $src/static $out/share/fedi-xanadu/static
            fi
          '';
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
            openssl
            sqlite
            sqlx-cli
            nodejs_22
            nodePackages.npm
          ];
          SQLX_OFFLINE = "true";
          DATABASE_URL = "sqlite://data/fedi-xanadu.db?mode=rwc";
        };
      }
    );
}
