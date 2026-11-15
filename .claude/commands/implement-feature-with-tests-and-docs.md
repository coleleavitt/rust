---
name: implement-feature-with-tests-and-docs
description: Workflow command scaffold for implement-feature-with-tests-and-docs in rust.
allowed_tools: ["Bash", "Read", "Write", "Grep", "Glob"]
---

# /implement-feature-with-tests-and-docs

Use this workflow when working on **implement-feature-with-tests-and-docs** in `rust`.

## Goal

Implements a new feature or significant change, accompanied by updates to documentation and the addition of new or updated tests.

## Common Files

- `compiler/*/src/**/*.rs`
- `src/tools/**/*.rs`
- `src/doc/**/*.md`
- `tests/**/*.rs`
- `tests/**/*.stderr`
- `tests/**/*.js`

## Suggested Sequence

1. Understand the current state and failure mode before editing.
2. Make the smallest coherent change that satisfies the workflow goal.
3. Run the most relevant verification for touched files.
4. Summarize what changed and what still needs review.

## Typical Commit Signals

- Implement or modify feature in source files (e.g., src/, compiler/)
- Update or add documentation files (e.g., src/doc/, *.md)
- Add or update test cases (e.g., tests/ui/, tests/rustdoc*, tests/ui/lint/, etc.)

## Notes

- Treat this as a scaffold, not a hard-coded script.
- Update the command if the workflow evolves materially.