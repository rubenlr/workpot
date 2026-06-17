---
name: code-quality-rust
description: Correctness rules, Rust patterns, comments, avoiding over-engineering
origin: https://raw.githubusercontent.com/tursodatabase/turso/refs/heads/main/docs/agent-guides/code-quality.md
---

# Code Quality Guide

## Core Principle

Visibility. User needs to know what happens. All actions need to log debug, all visual interactions need feedback.

## Correctness Rules

1. **No workarounds or quick hacks.** Handle all errors, check invariants
2. **Assert often.** Never silently fail or swallow edge cases
3. **Crash on invalid state** if it risks data integrity. Don't continue in undefined state
4. **Consider edge cases.** On long enough timeline, all possible bugs will happen

## Rust Patterns

- Make illegal states unrepresentable
- Exhaustive pattern matching
- Prefer enums over strings/sentinels

### Avoid `unwrap()`

Never use bare `unwrap()`. Instead:

**For truly unreachable states** (invariants that should never be violated):

```rust
// Good: documents the invariant
let value = option.expect("value must be set in Init phase");
```

**For recoverable errors** (states that could happen at runtime):

```rust
// Good: proper error handling
let Some(value) = option else {
    return Err(LimboError::InvalidArgument("value not provided".into()));
};
```

The choice depends on whether the None/Err case represents:

- A bug in the code (use `expect` with descriptive message)
- A valid runtime condition (use `let ... else` or `match`)

## If-Statements

Wrong:

```rust
if condition {
    // happy path
} else {
    // "shouldn't happen" - silently ignored
}
```

Right:

```rust
// If only one branch should ever be hit:
assert!(condition, "invariant violated: ...");
// OR
return Err(LimboError::InternalError("unexpected state".into()));
// OR
unreachable!("impossible state: ...");
```

Use if-statements only when both branches are expected paths.

## Comments

**Do:**

- Document WHY, not what
- Document functions, structs, enums, variants
- Focus on why something is necessary
- Preserve explanatory comments when refactoring

**Don't:**

- Comments that repeat code
- References to AI conversations ("This test should trigger the bug")
- Temporal markers ("added", "existing code", "Phase 1")

## Avoid Over-Engineering

- Only changes directly requested or clearly necessary
- Don't add features beyond what's asked
- Don't add docstrings/comments to unchanged code
- Don't add error handling for impossible scenarios
- Don't create abstractions for one-time operations
- Three similar lines > premature abstraction

## Cleanup

- Delete unused code completely
- No backwards-compat hacks (renamed `_vars`, re-exports, `// removed` comments)
