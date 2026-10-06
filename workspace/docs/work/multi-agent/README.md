# Multi-Agent Work Records

Concurrent mutating agents write task indexes and individual WIP records here
through the repository's `coordinate-multi-agent-development` skill. The atomic
board, lock, managed worktrees, and recovery archives stay under the local Git
common directory and are never committed.

Existing peer record history is preserved under the 7.0.0 contract. Existing
linked worktrees remain untouched; their existence alone does not establish an
active coordinated assignment.
