# NightBoat

## Build
- `nix develop --command cargo check`
- `nix develop --command cargo test`
- `nix develop --command bash -c "cd frontend && npm run build"`
- `SQLX_OFFLINE=true` for compile-time (no live DB needed)

## Database (PostgreSQL)
- Placeholders: `$1, $2, ...` — Timestamps: `NOW()` — Upsert: `ON CONFLICT ... DO UPDATE SET`

## No Parallel Agents for Cross-Codebase Migrations
Process files serially or batch all changes, then verify once at the end.

## Admin CLI (`fx admin`)
```bash
fx admin create-user <handle> <password> --display-name "Name"
fx admin publish --as <handle> -f file.md -t "Title" --tags cs --lang zh
```

## Commit Workflow (frontend changes)
`frontend/dist/` is tracked in git and served directly. After any frontend source change:
1. `nix develop --command bash -c "cd frontend && npm run build"`
2. `git add frontend/dist` (include build artifacts)
3. Commit source + dist together, then push

## Deploy
- Server: `ssh root@dzming.li`
- NixOS config: `/home/lee/nixos-config/hosts/hetzner-server/xanadu.nix`
- Admin secret: agenix, `EnvironmentFile` format (`FX_ADMIN_SECRET=xxx`)
