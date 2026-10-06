# Installing Workspace Skills

Workspace is an ordinary publisher in an SKM registry. Its skills own governance,
standard versions, adoption, and migration. SKM owns the
[registry and installation formats](registry-format.md) and applies the same
rules to every publisher. Generic local toolkit configuration is documented in
[toolkits.md](toolkits.md).

Install the adoption skill and its exact dependencies:

```sh
skm add workspace/wk-adopt --registry default
skm check
```

Invoke `wk-adopt` in your agent and state the desired assessment, adoption,
upgrade, or repair. The skill validates governance compatibility and performs
authorized repository work.

A publisher may also define bundles, installed through the ordinary bundle path:

```sh
skm add workspace/all-workspace-skills --registry default --kind bundle --dry-run
skm add workspace/all-workspace-skills --registry default --kind bundle --yes
```

The bundle name has no built-in completeness meaning in SKM; its manifest
chooses its members. Existing `workspace` configuration and lock data are
preserved as opaque metadata. SKM no longer has Workspace-specific init flags,
standard-source validation/hashing, or repository identity checks. Installed
skills manage that domain data. Existing `.skm/workspace-plan.yaml` artifacts
from older releases remain available for review; SKM does not update them.
