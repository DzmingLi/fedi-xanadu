{
  description = "NightBoat: federated knowledge sharing platform on AT Protocol";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, crane, rust-overlay }:
    let
      nixosModule = { config, lib, pkgs, ... }:
        let
          cfg = config.services.nightboat;
          pkg = self.packages.${pkgs.stdenv.hostPlatform.system}.default;
        in {
          options.services.nightboat = {
            enable = lib.mkEnableOption "NightBoat knowledge sharing platform";
            port = lib.mkOption {
              type = lib.types.port;
              default = 3847;
              description = "Port for the backend server";
            };
            dataDir = lib.mkOption {
              type = lib.types.str;
              default = "/var/lib/nightboat";
              description = "State directory for Pijul store and other data";
            };
            instanceName = lib.mkOption {
              type = lib.types.str;
              default = "NightBoat";
              description = "Display name for this instance";
            };
            corsOrigins = lib.mkOption {
              type = lib.types.listOf lib.types.str;
              default = [];
              description = "Allowed CORS origins. Empty = same-origin only.";
            };
            publicUrl = lib.mkOption {
              type = lib.types.str;
              default = "";
              description = "Public URL of this instance (for OAuth client_id and callback). e.g. https://nightboat.dzming.li";
            };
            adminSecretFile = lib.mkOption {
              type = lib.types.nullOr lib.types.path;
              default = null;
              description = "Path to a file containing the admin API secret. Enables admin endpoints (user management, publish-as).";
            };
            database = {
              name = lib.mkOption {
                type = lib.types.str;
                default = "nightboat";
                description = "PostgreSQL database name";
              };
              user = lib.mkOption {
                type = lib.types.str;
                default = "nightboat";
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
                default = "/var/backup/nightboat";
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
            systemd.services.nightboat = {
              description = "NightBoat knowledge sharing platform";
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
              } // lib.optionalAttrs (cfg.publicUrl != "") {
                FX_PUBLIC_URL = cfg.publicUrl;
              } // lib.optionalAttrs (cfg.corsOrigins != []) {
                FX_CORS_ORIGINS = lib.concatStringsSep "," cfg.corsOrigins;
              };
              serviceConfig = {
                ExecStart = "${pkg}/bin/nightboat";
                WorkingDirectory = "${pkg}/share/nightboat";
                User = cfg.database.user;
                Group = cfg.database.user;
                StateDirectory = "nightboat";
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
            systemd.services.nightboat-backup = lib.mkIf cfg.backup.enable {
              description = "NightBoat database and Pijul store backup";
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

            systemd.timers.nightboat-backup = lib.mkIf cfg.backup.enable {
              description = "Daily NightBoat backup";
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
          cfg = config.programs.nbt;
          pkg = self.packages.${pkgs.stdenv.hostPlatform.system}.nightboat-cli;
        in {
          options.programs.nbt = {
            enable = lib.mkEnableOption "NightBoat CLI (nbt)";
            server = lib.mkOption {
              type = lib.types.str;
              default = "https://nightboat.dzming.li";
              description = "NightBoat server URL";
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
                pkgs.writeShellScriptBin "nbt" ''
                  _secret="$(cat ${cfg.adminSecretFile})"
                  export NBT_ADMIN_SECRET="''${_secret#FX_ADMIN_SECRET=}"
                  exec ${pkg}/bin/nbt "$@"
                ''
              else pkg)
            ];

            # Auto-login on activation if handle + passwordFile are set
            home.activation.nbt-login = lib.mkIf (cfg.handle != null && cfg.passwordFile != null) (
              lib.hm.dag.entryAfter [ "writeBoundary" ] ''
                if [ -f "${cfg.passwordFile}" ]; then
                  ${pkg}/bin/nbt --server "${cfg.server}" login "${cfg.handle}" "$(cat "${cfg.passwordFile}")" 2>/dev/null || true
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
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchain;

        # Filter out non-Rust directories for build cache efficiency
        src = pkgs.lib.cleanSourceWith {
          src = pkgs.lib.cleanSource ./.;
          filter = path: _type:
            let rel = pkgs.lib.removePrefix (toString ./. + "/") path;
            in !(builtins.any (prefix: pkgs.lib.hasPrefix prefix rel) [
              "frontend/" "docs/" "scripts/" "crates/frontend/"
            ]);
        };

        # Vendor cargo deps (including git sources)
        cargoVendorDir = craneLib.vendorCargoDeps {
          inherit src;
          outputHashes = {
            "git+https://github.com/DzmingLi/atproto-auth.git#2a7ea6e41f68a34eebd0cbdf6c90462cc62cf4a2" = "sha256-1fu23HYuUDQLXZBqFnsuCEcojoWeqmtTfDgPpbFa0Zg=";
            "git+https://github.com/DzmingLi/typst-render.git#3017aebba6f4ac96e935a0ff203b8522699c7899" = "sha256-BA5UoUtPvAQh0BBD9bf5Yudy/245bpTlvIIm/91JYJ8=";
            "git+https://github.com/DzmingLi/pijul-knot.git#d86a9504b001b09f38db2063bc64cc1764f79baf" = "sha256-kV301kyoDvBZmlmRFFSNZZolNHOy6VX+ypbbXXv5YsM=";
          };
        };

        commonArgs = {
          inherit src cargoVendorDir;
          pname = "nightboat";
          version = "0.1.0";
          nativeBuildInputs = with pkgs; [ pkg-config ];
          buildInputs = with pkgs; [ openssl postgresql ];
          SQLX_OFFLINE = "true";
          doCheck = false;
        };

        # Deps layer — only rebuilds when Cargo.lock changes
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # Server binary (reuses cached deps)
        nightboat-bin = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          cargoExtraArgs = "--package fx-server";
          postInstall = "rm -f $out/bin/nbt 2>/dev/null || true";
        });

        # CLI binary (reuses same cached deps)
        nbt_cli = craneLib.buildPackage (commonArgs // {
          inherit cargoArtifacts;
          pname = "nightboat-cli";
          cargoExtraArgs = "--package nightboat-cli";
          postInstall = "rm -f $out/bin/nightboat 2>/dev/null || true";
        });

        frontendDist = pkgs.runCommand "nightboat-frontend-dist" {} ''
          cp -r ${./frontend/dist} $out
        '';

        migrationsDrv = pkgs.runCommand "nightboat-migrations" {} ''
          cp -r ${./migrations_pg} $out
        '';
      in
      {
        packages."nightboat-cli" = nbt_cli;

        packages.default = pkgs.runCommand "nightboat" {
          nativeBuildInputs = [ pkgs.makeWrapper ];
        } ''
          mkdir -p $out/bin $out/share/nightboat/frontend
          cp ${nightboat-bin}/bin/nightboat $out/bin/nightboat
          chmod +x $out/bin/nightboat
          wrapProgram $out/bin/nightboat \
            --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.pandoc ]}
          cp -r ${frontendDist} $out/share/nightboat/frontend/dist
          cp -r ${migrationsDrv} $out/share/nightboat/migrations_pg
        '';

        devShells.default = craneLib.devShell {
          packages = with pkgs; [
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
