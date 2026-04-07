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
          pkg = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
        in {
          options.services.fedi-xanadu = {
            enable = lib.mkEnableOption "Fedi-Xanadu knowledge sharing platform";
            port = lib.mkOption {
              type = lib.types.port;
              default = 3847;
              description = "Port for the backend server";
            };
            dataDir = lib.mkOption {
              type = lib.types.str;
              default = "/var/lib/fedi-xanadu";
              description = "State directory for Pijul store and other data";
            };
            instanceName = lib.mkOption {
              type = lib.types.str;
              default = "Fedi-Xanadu";
              description = "Display name for this instance";
            };
            corsOrigins = lib.mkOption {
              type = lib.types.listOf lib.types.str;
              default = [];
              description = "Allowed CORS origins. Empty = same-origin only.";
            };
            adminSecretFile = lib.mkOption {
              type = lib.types.nullOr lib.types.path;
              default = null;
              description = "Path to a file containing the admin API secret. Enables admin endpoints (user management, publish-as).";
            };
            database = {
              name = lib.mkOption {
                type = lib.types.str;
                default = "fedi_xanadu";
                description = "PostgreSQL database name";
              };
              user = lib.mkOption {
                type = lib.types.str;
                default = "fedi_xanadu";
                description = "PostgreSQL user name";
              };
            };
            backup = {
              enable = lib.mkOption {
                type = lib.types.bool;
                default = true;
                description = "Enable daily database and Pijul store backups";
              };
              dir = lib.mkOption {
                type = lib.types.str;
                default = "/var/backup/fedi-xanadu";
                description = "Backup directory";
              };
              retention = lib.mkOption {
                type = lib.types.int;
                default = 7;
                description = "Number of daily backups to retain";
              };
            };
          };

          config = lib.mkIf cfg.enable {
            # PostgreSQL database
            services.postgresql = {
              enable = true;
              ensureDatabases = [ cfg.database.name ];
              ensureUsers = [{
                name = cfg.database.user;
                ensureDBOwnership = true;
              }];
            };

            # Systemd service
            systemd.services.fedi-xanadu = {
              description = "Fedi-Xanadu knowledge sharing platform";
              after = [ "network.target" "postgresql.service" ];
              requires = [ "postgresql.service" ];
              wantedBy = [ "multi-user.target" ];
              environment = {
                FX_HOST = "127.0.0.1";
                FX_PORT = toString cfg.port;
                FX_DATABASE_URL = "postgres:///${cfg.database.name}?host=/run/postgresql";
                FX_PIJUL_STORE_PATH = "${cfg.dataDir}/pijul-store";
                FX_INSTANCE_NAME = cfg.instanceName;
                FX_ENV = "production";
                RUST_LOG = "info";
              } // lib.optionalAttrs (cfg.corsOrigins != []) {
                FX_CORS_ORIGINS = lib.concatStringsSep "," cfg.corsOrigins;
              };
              serviceConfig = {
                ExecStart = "${pkg}/bin/fedi-xanadu";
                WorkingDirectory = "${pkg}/share/fedi-xanadu";
                User = cfg.database.user;
                Group = cfg.database.user;
                StateDirectory = "fedi-xanadu";
                Restart = "on-failure";
                RestartSec = 5;

                # Hardening
                NoNewPrivileges = true;
                ProtectSystem = "strict";
                ProtectHome = true;
                PrivateTmp = true;
                PrivateDevices = true;
                ProtectKernelTunables = true;
                ProtectKernelModules = true;
                ProtectControlGroups = true;
                RestrictSUIDSGID = true;
                ReadWritePaths = [ cfg.dataDir ];
              } // lib.optionalAttrs (cfg.adminSecretFile != null) {
                EnvironmentFile = cfg.adminSecretFile;
              };
            };

            # Create system user
            users.users.${cfg.database.user} = {
              isSystemUser = true;
              group = cfg.database.user;
              home = cfg.dataDir;
            };
            users.groups.${cfg.database.user} = {};

            # Ensure data directory
            systemd.tmpfiles.rules = [
              "d ${cfg.dataDir} 0750 ${cfg.database.user} ${cfg.database.user} -"
              "d ${cfg.dataDir}/pijul-store 0750 ${cfg.database.user} ${cfg.database.user} -"
            ] ++ lib.optionals cfg.backup.enable [
              "d ${cfg.backup.dir} 0750 ${cfg.database.user} ${cfg.database.user} -"
            ];

            # Daily backup timer
            systemd.services.fedi-xanadu-backup = lib.mkIf cfg.backup.enable {
              description = "Fedi-Xanadu database and Pijul store backup";
              serviceConfig = {
                Type = "oneshot";
                User = cfg.database.user;
                Group = cfg.database.user;
              };
              script = ''
                set -euo pipefail
                TIMESTAMP=$(date +%Y%m%d-%H%M%S)
                BACKUP_DIR="${cfg.backup.dir}"

                # PostgreSQL dump
                ${config.services.postgresql.package}/bin/pg_dump \
                  ${cfg.database.name} \
                  | ${pkgs.zstd}/bin/zstd -9 \
                  > "$BACKUP_DIR/db-$TIMESTAMP.sql.zst"

                # Pijul store snapshot (rsync incremental)
                ${pkgs.rsync}/bin/rsync -a --delete \
                  "${cfg.dataDir}/pijul-store/" \
                  "$BACKUP_DIR/pijul-store/"

                # Prune old DB dumps, keep last N
                ls -1t "$BACKUP_DIR"/db-*.sql.zst 2>/dev/null \
                  | tail -n +${toString (cfg.backup.retention + 1)} \
                  | xargs -r rm -f

                echo "Backup completed: db-$TIMESTAMP.sql.zst"
              '';
            };

            systemd.timers.fedi-xanadu-backup = lib.mkIf cfg.backup.enable {
              description = "Daily Fedi-Xanadu backup";
              wantedBy = [ "timers.target" ];
              timerConfig = {
                OnCalendar = "daily";
                Persistent = true;
                RandomizedDelaySec = "1h";
              };
            };
          };
        };
    in
    let
      homeManagerModule = { config, lib, pkgs, ... }:
        let
          cfg = config.programs.fx;
          pkg = self.packages.${pkgs.stdenv.hostPlatform.system}.fx-cli;
        in {
          options.programs.fx = {
            enable = lib.mkEnableOption "fx CLI for Fedi-Xanadu";
            server = lib.mkOption {
              type = lib.types.str;
              default = "https://fedi-xanadu.dzming.li";
              description = "Fedi-Xanadu server URL";
            };
            handle = lib.mkOption {
              type = lib.types.nullOr lib.types.str;
              default = null;
              example = "user.bsky.social";
              description = "AT Protocol handle for auto-login";
            };
            passwordFile = lib.mkOption {
              type = lib.types.nullOr lib.types.path;
              default = null;
              description = ''
                Path to a file containing only the app password.
                Should be readable only by the user (mode 0400).
                Both handle and passwordFile must be set for auto-login.
              '';
            };
            adminSecretFile = lib.mkOption {
              type = lib.types.nullOr lib.types.path;
              default = null;
              description = "Path to a file containing the admin secret for fx admin commands.";
            };
          };

          config = lib.mkIf cfg.enable {
            home.packages = [
              (if cfg.adminSecretFile != null then
                pkgs.writeShellScriptBin "fx" ''
                  _secret="$(cat ${cfg.adminSecretFile})"
                  export FX_ADMIN_SECRET="''${_secret#FX_ADMIN_SECRET=}"
                  exec ${pkg}/bin/fx "$@"
                ''
              else pkg)
            ];

            # Auto-login on activation if handle + passwordFile are set
            home.activation.fx-login = lib.mkIf (cfg.handle != null && cfg.passwordFile != null) (
              lib.hm.dag.entryAfter [ "writeBoundary" ] ''
                if [ -f "${cfg.passwordFile}" ]; then
                  ${pkg}/bin/fx --server "${cfg.server}" login "${cfg.handle}" "$(cat "${cfg.passwordFile}")" 2>/dev/null || true
                fi
              ''
            );
          };
        };
    in
    {
      nixosModules.default = nixosModule;
      homeManagerModules.default = homeManagerModule;
    } //
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };
        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Strip non-Rust directories from the source tree.
        # Cargo workspace resolution needs all Cargo.toml files, but not src/ of
        # crates that aren't being compiled — so we can filter those for cache isolation.
        stripNonRust = src: pkgs.lib.cleanSourceWith {
          inherit src;
          filter = path: _type:
            let rel = pkgs.lib.removePrefix (toString ./. + "/") path;
            in !(builtins.any (prefix: pkgs.lib.hasPrefix prefix rel) [
              "frontend/" "docs/" "scripts/" "crates/frontend/"
            ]);
        };

        # Full Rust source (used as base for per-package filters below).
        rustSrc = stripNonRust (pkgs.lib.cleanSource ./.);

        # fx-cli only needs its own crate + shared lib crates.
        # Exclude server-only crate src/ so server changes don't bust the CLI cache.
        fxCliSrc = pkgs.lib.cleanSourceWith {
          src = rustSrc;
          filter = path: _type:
            let rel = pkgs.lib.removePrefix (toString ./. + "/") path;
            in !(builtins.any (prefix: pkgs.lib.hasPrefix prefix rel) [
              "crates/fx-server/src"
              "crates/fx-pijul/src"
              "crates/fx-atproto/src"
              "crates/fx-render/src"
              "crates/fx-search/src"
            ]);
        };

        # Server excludes the CLI src/ so CLI-only changes don't bust the server cache.
        serverSrc = pkgs.lib.cleanSourceWith {
          src = rustSrc;
          filter = path: _type:
            let rel = pkgs.lib.removePrefix (toString ./. + "/") path;
            in !(pkgs.lib.hasPrefix "crates/fx-cli/src" rel);
        };

        # Pre-built frontend assets as a standalone derivation (just a file copy).
        # Only rebuilds when frontend/dist changes; Rust derivation is unaffected.
        frontendDist = pkgs.runCommand "fedi-xanadu-frontend-dist" {} ''
          cp -r ${./frontend/dist} $out
        '';

        # Migrations as a standalone derivation.
        migrationsDrv = pkgs.runCommand "fedi-xanadu-migrations" {} ''
          cp -r ${./migrations_pg} $out
        '';

        # Rust binary only — no frontendDist dependency so frontend changes
        # don't invalidate this derivation and trigger a full Rust recompile.
        rustBinary = pkgs.rustPlatform.buildRustPackage {
          pname = "fedi-xanadu-bin";
          version = "0.1.0";
          src = serverSrc;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = with pkgs; [ openssl postgresql ];
          doCheck = false;
          env.SQLX_OFFLINE = "true";
          cargoBuildFlags = [ "--package" "fx-server" ];

          postInstall = ''
            rm -f $out/bin/fx-cli 2>/dev/null || true
          '';
        };
      in
      {
        packages.fx-cli = pkgs.rustPlatform.buildRustPackage {
          pname = "fx-cli";
          version = "0.1.0";
          src = fxCliSrc;
          cargoLock.lockFile = ./Cargo.lock;
          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = with pkgs; [ openssl postgresql ];
          doCheck = false;
          env.SQLX_OFFLINE = "true";
          cargoBuildFlags = [ "--package" "fx-cli" ];

          postInstall = ''
            rm -f $out/bin/fedi-xanadu
          '';
        };

        # Final package: assemble binary + frontend + migrations.
        # This runCommand is fast — no Rust compilation here.
        # Frontend changes only rebuild frontendDist + this tiny assembly step.
        packages.default = pkgs.runCommand "fedi-xanadu" {
          nativeBuildInputs = [ pkgs.makeWrapper ];
        } ''
          mkdir -p $out/bin $out/share/fedi-xanadu/frontend
          cp ${rustBinary}/bin/fedi-xanadu $out/bin/fedi-xanadu
          chmod +x $out/bin/fedi-xanadu
          wrapProgram $out/bin/fedi-xanadu \
            --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.pandoc ]}
          cp -r ${frontendDist} $out/share/fedi-xanadu/frontend/dist
          cp -r ${migrationsDrv} $out/share/fedi-xanadu/migrations_pg
        '';

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustToolchain
            pkg-config
            openssl
            postgresql
            sqlx-cli
            nodejs_22
            nodePackages.npm
            pandoc
          ];
          SQLX_OFFLINE = "true";
        };
      }
    );
}
