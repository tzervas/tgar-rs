# Release (tgar-rs) — O11 pilot

Implements fleet-ops design: **dispatch → cz bump branch → PR to main → tag after merge**.

## Local parity

```bash
# preview only
cz bump --dry-run
# or
gh workflow run release.yml -f bump=auto -f dry_run=true
```

## Mutating release

```bash
gh workflow run release.yml -f bump=patch -f dry_run=false
# Review PR release/vX.Y.Z → main; merge when pedantic checks green.
# finalize-release job tags + creates GitHub Release on merge.
```

## Policy

- `major_version_zero = true` in `.cz.toml` — do not ship 1.x without human label + flag flip.
- Registry publish (crates.io) is **out of scope** for this pilot.
- Prefer dry_run until you intend a real version commit.
