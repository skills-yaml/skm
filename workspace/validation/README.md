# Validation Modules

Use Taskfile commands only. Every command passes only on zero exit status; missing
tools, invalid records, and failed/interrupted runs fail. The complete `task check`
and `task test` aggregates are mandatory after independent review on a frozen
candidate and on actual combined integration revisions. Final evidence reuse is
disabled; changes after review renew affected review and verification.

| Module / command | Inputs | Dependencies and consumers | Resources and pass condition |
| --- | --- | --- | --- |
| `task workspace:check` / `task workspace:test` | Workspace validator/tests; target standard, generated AGENTS context, contained standard aliases, catalog, spec state/blocked metadata, memory, repository files for privacy | Governs workflow and spec state; any spec or governance pin change requires workspace check. SKM manifest metadata does not select the governance version. Tests exercise valid/invalid fixtures. | Structure validation reads governance declarations; repository privacy scanning remains generic. Tests use isolated temporary directories. All selected checks/tests must pass. |
| `task versions:check` / `task versions:test` | Version validator/tests; every non-legacy spec, release ledger, Cargo.toml/Cargo.lock | Consumes spec states including blocked Previous State; ledger and state moves require both workspace and versions gates. | Read-only repository check; isolated fixture tests. Version arithmetic, membership, timing, and source must agree. |
| `task coordination:check` / `task coordination:test` | Coordination validator/tests, peer record tree, peer-commands.json | Consumers are peer assignment/integration; record or validation-config edits require these gates and workspace privacy. | Read-only repository check; isolated fixture tests. Complete aggregate peer commands and safe records required. |
| `task qualify:release-update:unix` / `task qualify:release-update:windows` | Immutable bootstrap artifacts, exact commits/version, qualification scripts/workflow, current development manifest, held production run | All four native runners must pass ordinary consecutive-pair qualification. Breaking CLI migration additionally supplies legacy run/commit inputs and `SKM_REQUIRE_LEGACY_BOOTSTRAP=1`; current artifacts are rejected in that mode, then the actual older `version`/`update` binary is upgraded. | Temporary installations; checksum/source identity, terminal notice, actual replacement, no-op byte stability, and Windows cleanup must pass. Main/development/tag and pending protected production must remain the exact candidate. |
| `task installers:check` | Installer, release publisher, and qualification scripts | Consumed by installation/publication; selected on script changes and always by task check. | Shell syntax and required release identifiers/checksum support must pass. |
| `task test:toolkit` | Toolkit parser/validator, config, linker, adapters, lock ownership, transaction code/tests, Cargo manifests/lock and Taskfile/toolchain | Selected for toolkit installation or dependency changes; verifies installation independently of governance metadata. Consumed by install/check; included in task test. | Isolated temporary fixtures; serialize with all Cargo tasks sharing target storage. All toolkit tests pass, including failure-before-write, integrity, ownership, rollback, and idempotence cases. |
| `task test:bundle` | Bundle parser/validator, config manager, registry resolver/linker, transaction code/tests, Cargo manifests/lock and Taskfile/toolchain | Selected for bundle installation or manifest changes; consumed by bundle add and subsequent install/check. Included in task test. | Isolated temporary fixtures; serialize with all Cargo tasks sharing target storage. All bundle tests pass, including publisher-neutral subset installation and retained schema, adapter, provenance-format, dependency, and collision failures. |
| `task test:registry-format` | Search/discovery, config serialization, ordinary installer, init/help, wizard code/tests, Cargo manifests/lock and Taskfile/toolchain | Selected for publisher-neutral discovery, metadata round trips, config/lock schema, or init/prompt changes; consumed by search/config/install/init and included in task test. | Isolated temporary fixtures; serialize all Cargo tasks. Discovery, opaque metadata, real installation/lock regeneration, generic init, and wizard preservation/rejection tests must pass. |
| `task check` | All above inputs, Rust sources/tests, Cargo manifests/lock, build.rs, Taskfile/toolchain | Authoritative full formatting, Clippy, compilation, and governance coverage. Unknown selector/dependency scope falls back here. | Cargo shares target storage; serialize with other Cargo tasks. Every component passes, warnings denied. |
| `task test` | Rust unit/integration sources and Python validator fixtures, Taskfile/toolchain | Covers runtime and every changed governance module; unknown test scope falls back here. | Rust tests run with one test thread; aggregates serialize Cargo and fixture suites. Every test passes. |

Selectors are explicit Taskfile module names, not inferred path-based receipts.
Validator fixtures include representative failures for blocked fields, state/
catalog mismatch, aliases/pins, memory obligations, version application, and branch
records. Running a focused module does not establish full final coverage.

For each coherent change or review fix, map changed behavior and direct/transitive
consumers, including interfaces, errors, security, timing, side effects,
configuration, dependencies, and generated outputs. First run the smallest
meaningful checks/tests for the changed component. After they pass, run affected
consumer checks in dependency order, continuing through the transitive chain.
Failure blocks dependent stages; missing coverage or tools is an explicit gap.
Deduplicate overlapping checks, group dependency cycles as one stage, and assess
both old and new consumers for deleted/renamed behavior. Unknown impact requires
a documented safe aggregate fallback. Preserve required contract/integration/
end-to-end coverage; a flat selector list is not execution order.

For changes limited to governance guidance and records, inspect the changed
guidance and run
`task workspace:check` and `task workspace:test` first. After both pass,
run affected version and coordination checks/tests as consumer modules.
Those independent consumer modules are read-only, and their fixture suites use
isolated temporary directories, so they may run concurrently within that stage.
No editing occurs during candidate checks;
full Cargo aggregates run sequentially. `peer-commands.json` conservatively
requires both complete aggregates for serialized integration; the detached peer
runtime remains unchanged and its Windows locking support remains unqualified.
