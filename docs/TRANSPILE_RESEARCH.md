# Transpile research pointer (P23)

**tgar-rs** does not vendor the Mycelium transpiler. This note links **process and honesty patterns** used when porting Python (`tg-agent-relay`) to Rust, informed by a frozen snapshot of `mycelium-transpile` in the py2rust repo.

## Primary assessment

Deep architecture and reuse map (gap JSON, VR-5/G2, batch IR, what to adopt vs skip):

- **[py2rust `research/ASSESSMENT_mycelium_transpile.md`](https://github.com/tzervas/py2rust/blob/main/research/ASSESSMENT_mycelium_transpile.md)**

Snapshot provenance (read-only, not live Mycelium product):

- **[py2rust `research/mycelium-transpile-snapshot/PROVENANCE.md`](https://github.com/tzervas/py2rust/tree/main/research/mycelium-transpile-snapshot)**

## How this relates to tgar-rs

| Use | Skip |
|-----|------|
| Gap lists / “blocked construct” inventory per upstream `.py` module | Rust→Mycelium emission, `myc check` vet |
| Dual golden tests and strangler phases ([STRANGLER.md](STRANGLER.md), [PORTING.md](PORTING.md)) | Adding transpiler crates to this workspace |
| Optional **py2rust** assist on pure modules ([README.md](../README.md)) | Treating py2rust output as production-ready without `cargo test` + relay parity |

Plan context: `/root/work/plans/fractal/P23_TRANSPILE_SCAVENGE.md` (workspace maintainer copy).