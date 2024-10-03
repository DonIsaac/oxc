> I'm at a conference I'll flesh this out later

# Goals

- Aquire type information for use in other tools (particularly linter and
  minifier)
- All supported functionality should match TS exactly
    - When we do get a type from a node/whatever, it _must_ be what typescript
      would return. If we can't match it, abort with `any`.

# Scope

The following are out of scope
- Exact 1:1 typescript port
    - Trying to do everything TS does is a fool's errand. Lots of scenarios are
      really hard to support. In those cases, we should just return `any`.
- LSP
- A lot of hard type inference stuff, e.g. reverse mapping types, etc.

# Design
- Type-related flags (e.g. TypeFlags, ObjectFlags) are in `oxc_syntax`
- Trying to split checker.ts into separate files. Having everything in one file
  is pathological, and doesn't help w performance in Rust
- We try to port each function exactly/with the same name. If not possible, or
  if theres a slight change, it gets noted.

## Structure
- State and data structures, particularly those near the top of
  `createTypeChecker`, are in `subsystems`.
- Data in base `Type` interface is stored in `TypeTable`, a SoA structure. It's
  stored in an `Rc<RefCell<>>` within `TypeBuilder` to allow a shared mutable
  reference.

