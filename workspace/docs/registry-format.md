# SKM Registry and Installation Formats

SKM owns these formats. Publishers provide skill content, package names, versions,
and domain metadata. The same rules apply to every registry and namespace;
package names and bundle names carry no special behavior.

## Registry Layout and Skill Metadata

```text
skills/<namespace>/manifest.yaml
skills/<namespace>/<package>/v<major>.<minor>.<patch>/SKILL.md
```

Skill names and registry names must pass SKM's path-safe identifier validation.
`SKILL.md` contains YAML frontmatter. `name` identifies the skill;
`metadata.skm-version` supplies its published version, and
`metadata.skm-dependencies` may declare comma-separated exact dependencies such
as `company/helper@1.2.0`. SKM resolves the full dependency closure and rejects
cycles, conflicting pins, unsafe paths, and invalid versions.

## Namespace Manifests

Schema 1 publishes a package collection for discovery. Schema 2 also publishes
installable bundles:

```yaml
schema_version: 2
namespace: company
minimum_skm_version: 0.9.0
skm_adapter_compatibility: 2.x
packages:
  author: 1.2.0
  reviewer: 2.0.0
bundles:
  writing:
    packages: [author]
```

The namespace must match its registry directory. Package IDs are safe identifiers
and versions are exact semantic versions. Bundles have safe IDs and nonempty,
unique member lists containing only published package IDs. A bundle may contain
any subset of the namespace's packages. Installation uses schema 2; schema 1
remains discoverable as a collection. Unknown root-level publisher fields are
ignored; declared installation fields retain typed parsing, and bundle member
structures reject unknown fields.

Optional installation metadata applies identically to every publisher:

| Field | Installation rule |
| --- | --- |
| `minimum_skm_version` | Exact version; the running SKM must meet it. |
| `skm_adapter_compatibility` | If declared, must select the supported `2.x` contract. |
| `toolkit_version` | If declared, must be an exact release version. |
| `source_repository` | If declared, a nonempty descriptive source string. No repository identity allowlist. |
| `source_revision` | If declared, a 40-character lowercase hexadecimal Git commit. |

These optional fields are not required because of a namespace's identity.
Publisher provenance claims do not authorize a source or establish trust;
installation resolves packages from the user's selected registry and validates
actual source paths and links. Search describes published packages and bundles;
installation enforces the optional runtime compatibility requirements.

## Local Toolkits and Lockfiles

Local toolkits use SKM manifest schema 1 with an ID, exact selected version,
minimum SKM version, skills, profiles, and bundles. Root publisher metadata is
ignored. Nested installation entries remain strict; dependencies, paths,
frontmatter, adapters, and managed-output ownership are checked. See
[toolkit configuration](toolkits.md).

SKM writes schema-1 installation locks containing the resolved installation
data. Skills-only locks have `kind: skills`; toolkit locks contain the selected
toolkit, adapter/profile data, source integrity, and owned outputs. Existing
unknown root-level lock values survive regeneration as opaque metadata, while
SKM regenerates its installation fields. New locks do not copy unrelated project
metadata. Root publisher configuration also survives configuration round trips
without interpretation; it grants no installation capability or source trust.

Domain rules, governance versions, and standard-source validation belong to
installed skills. SKM validates its installation format and filesystem effects.
