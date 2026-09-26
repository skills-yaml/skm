# Memory Changelog

## 2026-09-26 - Record Registry release notification decision

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded in `workspace/agents/memory/decisions.md` that production releases
notify the Registry to review its documentation.

## 2026-09-26 - Record Workspace documentation separation test integration

- Type: fact
- Source: command
- Confidence: high
- Review: before production release
- Supersedes: none

Content:

Recorded PR #60's confirmed `development` merge in
`workspace/agents/memory/facts.md` and moved the separate-workspace-docs
specification to `test` with a matching catalog rationale.

## 2026-09-26 - Record command reorganization test integration

- Type: fact
- Source: pull request and CI
- Confidence: high
- Review: before production release
- Supersedes: none

Content:

Recorded PR #63's confirmed `development` merge and passing Validate run in
`workspace/agents/memory/facts.md`, and moved the command reorganization
specification to `test` with a matching catalog rationale.

## 2026-09-26 - Record bridge release prerequisite

- Type: fact
- Source: code and release qualification contract
- Confidence: high
- Review: after bridge release
- Supersedes: none

Content:

Recorded in `workspace/agents/memory/facts.md` that the older self-updater
cannot verify a binary without top-level `version`, so the breaking command
change requires a bridge release before production promotion.

## 2026-09-26 - Record implemented CLI command grouping

- Type: decision
- Source: user and implementation
- Confidence: high
- Review: after development integration
- Supersedes: 2026-09-26 - Record CLI command compatibility decision

Content:

Recorded the final cache, skill version, self-update, and installation-check
command grouping in `workspace/agents/memory/decisions.md`, including removed
old names and the explicit cache refresh boundary.

## 2026-09-26 - Record CLI command compatibility decision

- Type: decision
- Source: user
- Confidence: high
- Review: when command redesign is specified
- Supersedes: none

Content:

Recorded in `workspace/agents/memory/decisions.md` that the proposed CLI
command reorganization may break old command names without compatibility
aliases. The new command contract is not yet approved.

## 2026-09-25 - Record Workspace documentation separation decision

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded in `workspace/agents/memory/decisions.md` that SKM documentation stays
independent of Workspace, with Workspace toolkit material in
`workspace/docs/workspace-toolkit.md`.


## 2026-09-25 - Record SKM 0.8.0 production release

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: 2026-09-25 - Record add confirmation development integration; 2026-09-25 - Record two-column search development integration

Content:

Recorded the qualified SKM 0.8.0 production publication and verified release
identity in `workspace/agents/memory/facts.md`. Moved the add-confirmation and
two-column search specifications to `done` with confirmed release evidence.

## 2026-09-25 - Record add confirmation development integration

- Type: fact
- Source: command
- Confidence: high
- Review: before production release
- Supersedes: none

Content:

Recorded PR #56's confirmed `development` merge in
`workspace/agents/memory/facts.md` and moved the add-confirmation
specification to `test` with a matching catalog rationale.

## 2026-09-25 - Record planned add confirmation behavior

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded the approved default-No confirmation for planned single-skill and
bundle adds in `workspace/agents/memory/decisions.md`, including `--yes` for
non-interactive use and read-only bundle previews.

## 2026-09-25 - Record two-column search development integration

- Type: fact
- Source: command
- Confidence: high
- Review: before production release
- Supersedes: none

Content:

Recorded PR #54's confirmed `development` merge in
`workspace/agents/memory/facts.md` and moved the two-column search
specification to `test` with a matching catalog rationale.

## 2026-09-25 - Record two-column search display preference

- Type: preference
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded the approved `skm search` text layout in
`workspace/agents/memory/preferences.md`: name and kind in the first column,
skill version beside its name, and description plus registry in the second
column, without add or preview instructions.

## 2026-09-24 - Record SKM 0.7.0 production release

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: 2026-09-24 - Record unified registry workflow integration; 2026-09-24 - Record registry bundle development integration

Content:

Recorded the qualified and verified SKM 0.7.0 production publication, its
unified skill and bundle workflow, and the live published Workspace bundle
qualification in `workspace/agents/memory/facts.md`. Moved the three released
specifications to `done` with the confirmed release evidence.

## 2026-09-24 - Record unified registry workflow integration

- Type: fact
- Source: command
- Confidence: high
- Review: before production release
- Supersedes: none

Content:

Recorded PR #46's confirmed `development` merge and passing Validate check in
`workspace/agents/memory/facts.md`, and moved the unified registry workflow
specification to `test` with the merge commit evidence.

## 2026-09-24 - Record unified registry discovery and addition

- Type: decision
- Source: user
- Confidence: high
- Review: after development integration
- Supersedes: none

Content:

Recorded the requested unified `skm search` and `skm add` workflow, explicit
kind selection for collisions, and compatibility treatment of the existing
bundle command in `workspace/agents/memory/decisions.md`.

## 2026-09-24 - Record registry bundle development integration

- Type: fact
- Source: command
- Confidence: high
- Review: before production release
- Supersedes: none

Content:

Recorded PR #44's confirmed `development` merge and passing PR validation in
`workspace/agents/memory/facts.md`, and moved the bundle specification to
`test` with the merge commit evidence.

## 2026-09-24 - Record registry bundle installation decision

- Type: decision
- Source: user
- Confidence: high
- Review: after development integration
- Supersedes: none

Content:

Recorded in `workspace/agents/memory/decisions.md` that SKM expands published
registry bundles into exact project skill pins with one previewed,
rollback-protected application, rather than installing an instructionless
dependency metapackage.

## 2026-09-24 - Record skill-led Workspace management

- Type: decision
- Source: user
- Confidence: high
- Review: after development integration
- Supersedes: none

Content:

Recorded the removal of Workspace-specific adoption commands, the published
skill replacement, and the preserved toolkit integrity contract in
`workspace/agents/memory/decisions.md`.

## 2026-09-24 - Record skill discovery production release

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded PR #42's qualified production release, four-platform publication,
checksum, manifest, binary identity, and search smoke in
`workspace/agents/memory/facts.md`. The three released specifications moved to
`done` with confirmed release evidence.

## 2026-09-23 - Record labeled search and namespace collections

- Type: fact
- Source: user and implementation
- Confidence: high
- Review: after development integration
- Supersedes: 2026-09-23 - Record skill target and search discovery behavior (search presentation only)

Content:

Recorded labeled skill details, dependency discovery, and the distinction
between browseable namespace collections and explicit published bundles in
`workspace/agents/memory/facts.md`.

## 2026-09-19 - Record managed release update contract

- Type: fact
- Source: spec
- Confidence: high
- Review: required before the first production release
- Supersedes: tag-only self-update discovery

Content:

Recorded the SKM release manifest, verified self-update, transactional
publication, and held-production qualification contract in
`workspace/agents/memory/facts.md`.

## 2026-06-17 - Initialize Agent Memory

- Type: fact
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Initialized `agents/memory/` for skm during additive adoption of `workspace-docs@1.0.0`.

## 2026-06-19 - Record diagnostic log stdout/stderr alignment

- Type: fact
- Source: review
- Confidence: high
- Review: none
- Supersedes: none

Content:

Aligned diagnostic logs and validation check outputs to stderr, keeping stdout clean for listing pipes. Updated Decisions memory.

## 2026-06-19 - Align Specifications with workspace-docs Standard

- Type: fact
- Source: review
- Confidence: high
- Review: none
- Supersedes: none

Content:

Moved implemented specifications (Auto-Update Notification, Config Management, Global Env Auto Config, Local Dev Mode, Registry Management, Skill Version Management) to `docs/specs/done/` and unimplemented specifications (Skill Removal, Cleanup Commands) to `docs/specs/backlog/` to adhere to the `workspace-docs@1.0.0` specification state directory standard.

## 2026-06-19 - Implement Skill Removal Feature

- Type: fact
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Implemented `skm remove` command to safely remove skill entries from `./skills.yaml` and delete symlinks from agent directories, backed by unit tests. Moved `skill-removal.md` spec to `docs/specs/done/`.

## 2026-06-19 - Implement Cleanup and Maintenance Commands (skm clean)

- Type: fact
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Implemented `skm clean` subcommands (`symlinks`, `cache`, `reset`) to find and remove broken or orphaned symlinks, manage cache size/retention, show cache statistics, and perform full/selective workspace resets with backup, backed by unit tests. Moved `cleanup-commands.md` spec to `docs/specs/done/`.

## 2026-06-19 - Implement Local Development Mode (skm dev)

- Type: fact
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Implemented `skm dev` commands (`link`, `unlink`, `list`, `show`, `mode`) to enable linking local directories as development skills, unlinking them, listing them, showing info, and toggling dev mode, backed by unit tests. Moved `local-dev-mode.md` spec to `docs/specs/done/`.

## 2026-06-19 - Implement Skill Version Management

- Type: fact
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Implemented skill versioning commands (`versions`, `use`, `update-skill`) to list registry versions semantically, switch config/links to a specific version, and update skills to the latest version, backed by integration tests. Moved `skill-version-management.md` spec to `docs/specs/done/`.

## 2026-08-19 - Record Workspace Docs 5 Toolkit Compatibility

- Type: fact
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded the SKM 0.2.1 compatibility decision for Workspace Docs 4.x and 5.x
toolkit manifests in `workspace/agents/memory/decisions.md`.

## 2026-08-19 - Adopt workspace-docs@5.0.0

- Type: fact
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded the `workspace-docs@5.0.0` adoption decision, branch mapping, and
canonical workspace paths in `workspace/agents/memory/decisions.md` and
`workspace/agents/memory/facts.md`.

## 2026-08-29 - Bring skill removal back into safety conformance

- Type: fact
- Source: spec
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded in `workspace/agents/memory/facts.md` that removal now preflights all
configured targets before confirmation or mutation, keeps cancellation and
dry-run non-mutating, and reports partial unlink failures after attempting all
targets. Review also added symlinked namespace-parent rejection.

## 2026-09-05 - Bring cleanup behavior back into safety conformance

- Type: fact
- Source: spec
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded in `workspace/agents/memory/facts.md` that orphan cleanup uses
authoritative skill and development configuration, old-version cleanup
preserves aliases and manifest pins, and reset preserves real content and
agent skills-root symlinks.

## 2026-09-09 - Record init wizard persistence and compatibility

- Type: fact
- Source: spec
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded the init wizard's existing-file editing, preview, safe-save behavior,
script compatibility, and Linux terminal test entrypoint in
`workspace/agents/memory/facts.md`.

## 2026-09-09 - Record SKM 0.3.0 production publication

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded the SKM 0.3.0 production merge, successful platform publication, and
artifact verification in `workspace/agents/memory/facts.md`. The init wizard
specification now records its confirmed release in `done`.

## 2026-09-10 - Record Registry Dependency Resolution Contract

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded in `workspace/agents/memory/decisions.md` that SKM resolves exact
same-registry skill dependency closures before writes and uses
`.agents/skills/` for Codex while retaining only bounded legacy-lock migration
support for `.codex/skills/`.

## 2026-09-10 - Record SKM 0.4.0 production publication

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded the SKM 0.4.0 production merge, successful CI and platform release,
verified Linux checksum and binary version in `workspace/agents/memory/facts.md`.
The registry dependency specification now records its confirmed release in
`done`.

## 2026-09-15 - Record standalone registry skill search

- Type: fact
- Source: spec
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded the SKM 0.5.0 `skm search` lookup, deterministic output controls,
temporary remote discovery, and dependency-aware unambiguous `--add` contract
in `workspace/agents/memory/facts.md`.

## 2026-09-16 - Record SKM 0.5.0 production publication

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded the SKM 0.5.0 production merge, successful CI and four-platform
release, and verified Linux checksum, binary version, and search help in
`workspace/agents/memory/facts.md`. The registry search specification now
records its confirmed release in `done`.

## 2026-09-17 - Record Read-Only Search Command Boundary

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: 2026-09-15 - Record standalone registry skill search (installation behavior only)

Content:

Recorded in `workspace/agents/memory/decisions.md` that `skm search` is always
read-only and delegates installation to the dedicated `skm add` command.

## 2026-09-19 - Record sequential init interaction

- Type: fact
- Source: user
- Confidence: high
- Review: none
- Supersedes: 2026-09-09 - Record init wizard persistence and compatibility

Content:

Recorded the decision to replace the full-screen init TUI with sequential
prompts in `workspace/agents/memory/decisions.md`, and recorded on-demand
registry discovery and numbered selection in
`workspace/agents/memory/facts.md`.

## 2026-09-20 - Record SKM 0.6.0 production publication

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded the SKM 0.6.0 exact-commit production promotion, required-reviewer
release hold, successful four-platform self-update qualification, publication,
and checksum, binary identity, tag, and manifest verification in
`workspace/agents/memory/facts.md`. The sequential init, trusted updater, and
dedicated search-add specifications now record their confirmed release in
`done`.

## 2026-09-22 - Record explicit deduplicated agent skill targets

- Type: decision
- Source: spec
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded in `workspace/agents/memory/decisions.md` that agent skill paths stay
compiled into SKM and shared filesystem targets retain all claimants while
being processed once. Recorded the sixteen supported agents and corrected
Codex and Hermes target behavior in `workspace/agents/memory/facts.md`.

## 2026-09-22 - Record Workspace Docs 6 toolkit compatibility

- Type: fact
- Source: spec
- Confidence: high
- Review: before the first production release containing this change
- Supersedes: 2026-08-19 - Support Workspace Docs 5 Toolkits (supported-major set only)

Content:

Recorded in `workspace/agents/memory/facts.md` that the explicit toolkit
compatibility allowlist now accepts Workspace Docs 4.x, 5.x, and 6.x while
remaining fail-closed for missing, malformed, and future-major declarations.

## 2026-09-22 - Record development release of backlog implementations

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded the PR #38 `development` integration, passing CI, four-platform
development release, and verified release manifest, Linux checksum, and binary
identity in `workspace/agents/memory/facts.md`. Moved both integrated specs to
`test` with the exact merge and workflow evidence.

## 2026-09-22 - Record help update notice decision

- Type: decision
- Source: user
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded in `workspace/agents/memory/decisions.md` that interactive help
requests show the available-update notice before Clap prints help. The
implementation and local validation are tracked in the development spec.

## 2026-09-23 - Record SKM production promotion

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded PR #40's exact-commit production release, successful CI, complete
four-platform publication, manifest and Linux checksum verification, and
interactive help notice smoke in `workspace/agents/memory/facts.md`. Moved the
three released specifications to `done` with the confirmed release evidence.

## 2026-09-23 - Record skill target and search discovery behavior

- Type: fact
- Source: user and implementation
- Confidence: high
- Review: after development integration
- Supersedes: none

Content:

Recorded the skills-only effective-target guard, direct-child agent skill
links, final-name collision rule, description discovery, and explicit
schema-2 registry bundle listing in `workspace/agents/memory/facts.md`.
The current Workspace registry publishes no bundles.

## 2026-09-23 - Record development integration of skill discovery fixes

- Type: fact
- Source: command
- Confidence: high
- Review: none
- Supersedes: none

Content:

Recorded the exact `development` commit, passing CI, complete prerelease
publication, and local install/check smoke in `workspace/agents/memory/facts.md`.
Moved both implementation specs to `test` with the integration evidence.

## 2026-09-24 - Identify The First Bundle-Capable SKM Version

- Type: decision
- Source: implementation
- Confidence: high
- Review: after production release
- Supersedes: none

Content:

Recorded in `workspace/agents/memory/decisions.md` that SKM 0.7.0 is the
first version targeted for `skm bundle add`; production 0.6.0 lacks the
command. The version reservation is a candidate, not release evidence.

## 2026-09-24 - Record SKM 0.7.0 development qualification

- Type: fact
- Source: command
- Confidence: high
- Review: after Registry bundle publication
- Supersedes: none

Content:

Recorded the passing four-platform development release, checksum, manifest,
released-binary bundle preview, and `skm check` evidence in
`workspace/agents/memory/facts.md`. Registry publication and production release
remain pending.
