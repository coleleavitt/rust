---
name: update-lint-attributes-or-remove-superfluous-attributes
description: Workflow command scaffold for update-lint-attributes-or-remove-superfluous-attributes in rust.
allowed_tools: ["Bash", "Read", "Write", "Grep", "Glob"]
---

# /update-lint-attributes-or-remove-superfluous-attributes

Use this workflow when working on **update-lint-attributes-or-remove-superfluous-attributes** in `rust`.

## Goal

Updates, removes, or cleans up lint attributes across multiple source files, often in response to codebase-wide linting or refactoring efforts.

## Common Files

- `compiler/rustc_*/src/**/*.rs`

## Suggested Sequence

1. Understand the current state and failure mode before editing.
2. Make the smallest coherent change that satisfies the workflow goal.
3. Run the most relevant verification for touched files.
4. Summarize what changed and what still needs review.

## Typical Commit Signals

- Edit multiple source files to update or remove lint attributes
- Optionally update related documentation or test files

## Notes

- Treat this as a scaffold, not a hard-coded script.
- Update the command if the workflow evolves materially.