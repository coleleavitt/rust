```markdown
# rust Development Patterns

> Auto-generated skill from repository analysis

## Overview

This skill teaches the core development patterns, coding conventions, and collaborative workflows used in the `rust` repository. It covers how to implement features, update configuration, manage lints, write and update tests, and perform codebase-wide refactoring. The guide is based on repository analysis and is designed to help contributors quickly align with established practices.

## Coding Conventions

- **File Naming:**  
  Source files use camelCase (e.g., `myModule.rs`, `parseExpr.rs`).

- **Import Style:**  
  Relative imports are preferred:
  ```rust
  mod utils;
  use crate::parser::parseExpr;
  ```

- **Export Style:**  
  Named exports are used:
  ```rust
  pub fn parse_expr() { ... }
  pub struct Parser { ... }
  ```

- **Commit Messages:**  
  - Prefixes: `feat`, `fix`
  - Freeform style, e.g.:  
    `feat: add new allocator config option`  
    `fix: correct lint attribute handling in parser`

## Workflows

### Implement Feature with Tests and Docs
**Trigger:** When adding a new compiler or tool feature, or making a significant change that needs to be tested and documented  
**Command:** `/feature-with-tests-docs`

1. Implement or modify the feature in relevant source files (e.g., `src/`, `compiler/`).
2. Update or add documentation files (e.g., `src/doc/`, `*.md`).
3. Add or update test cases (e.g., `tests/ui/`, `tests/rustdoc*`, `tests/ui/lint/`).

**Example:**
```rust
// src/myFeature.rs
pub fn new_feature() { /* ... */ }
```
```markdown
<!-- src/doc/myFeature.md -->
# My Feature
Documentation for the new feature.
```
```rust
// tests/ui/my_feature.rs
fn main() {
    // test cases for new_feature
}
```

---

### Update Lint Attributes or Remove Superfluous Attributes
**Trigger:** When updating, removing, or cleaning up lint attributes across the codebase  
**Command:** `/update-lints`

1. Edit multiple source files to update or remove lint attributes.
2. Optionally update related documentation or test files.

**Example:**
```rust
// Before
#![allow(dead_code)]
// After
#![warn(dead_code)]
```

---

### Add or Update Config Setting with Docs and CI
**Trigger:** When adding, renaming, or changing a configuration option and ensuring it is reflected everywhere  
**Command:** `/update-config`

1. Edit config source files (e.g., `src/bootstrap/src/core/config/`).
2. Update documentation files (e.g., `INSTALL.md`, `bootstrap.example.toml`).
3. Update CI or Docker files if needed.
4. Update or add related test files.

**Example:**
```rust
// src/bootstrap/src/core/config/allocator.rs
pub struct AllocatorConfig { /* ... */ }
```
```toml
# bootstrap.example.toml
allocator = "system"
```
```markdown
<!-- INSTALL.md -->
- `allocator`: Set the memory allocator (default: system)
```

---

### Add or Update Tests for Existing Feature or Bugfix
**Trigger:** When adding or improving test coverage for a feature or fixing a regression  
**Command:** `/add-test`

1. Add or update test files (e.g., `tests/ui/`, `tests/rustdoc*`, `tests/ui/lint/`).
2. Optionally update related source files for test harness or error output.

**Example:**
```rust
// tests/ui/bugfix_issue_123.rs
fn main() {
    // regression test for issue #123
}
```

---

### Refactor or Cleanup Across Multiple Modules
**Trigger:** When refactoring code for clarity, maintainability, or to meet style guidelines  
**Command:** `/refactor`

1. Edit multiple source files to refactor code (move functions, reorder arguments, clean imports).
2. Update related test files if necessary.

**Example:**
```rust
// Move function from one module to another
// Before: src/oldModule.rs
pub fn helper() { ... }
// After: src/newModule.rs
pub fn helper() { ... }
```

## Testing Patterns

- **Framework:** Unknown (custom or built-in Rust test harness likely)
- **File Pattern:** Test files are Rust source files, named with `.rs` and often placed in `tests/ui/`, `tests/rustdoc*`, or `tests/ui/lint/`.
- **Example Test File:**
  ```rust
  // tests/ui/sample_test.rs
  #[test]
  fn test_feature() {
      assert_eq!(my_feature(), expected_value);
  }
  ```
- **Other Test Artifacts:**  
  - `.stderr` files for expected error output  
  - `.js` files for JS-related tests

## Commands

| Command                    | Purpose                                                        |
|----------------------------|----------------------------------------------------------------|
| /feature-with-tests-docs    | Implement a new feature with tests and documentation           |
| /update-lints               | Update or clean up lint attributes across the codebase         |
| /update-config              | Add or update a configuration setting, docs, and CI            |
| /add-test                   | Add or update tests for an existing feature or bugfix          |
| /refactor                   | Refactor or clean up code across multiple modules              |
```
