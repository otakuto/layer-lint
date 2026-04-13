# layer-lint

A lint tool that enforces layered architecture rules on crate dependencies in a Rust workspace.

It reads the dependency graph from `cargo metadata` and checks it against rules defined in a YAML config file.

## Install

```bash
cargo install --path .
```

## Usage

```bash
# Run from the workspace root
layer-lint --config .layer-lint.yaml check
```

If `--config` is omitted, it defaults to `.layer-lint.yaml`.

## Configuration

Define layers and rules in `.layer-lint.yaml`.

### Layers

Group crates into named layers. Layers are split into `internal` (workspace members) and `external` (third-party dependencies).

```yaml
layers:
  internal:
    domain:
      - layer: id
      - layer: entity
    id:
      - regex: "^app-id-(.+)$"
    entity:
      - regex: "^app-entity-(.+)$"
  external:
    db:
      - crate: diesel
      - crate: diesel-derive-enum
    graphql:
      - crate: async-graphql
```

Layer members can be specified in three ways:

| Kind | Example | Description |
|------|---------|-------------|
| `crate` | `- crate: app-server` | Exact match |
| `regex` | `- regex: "^app-id-(.+)$"` | Regex match |
| `layer` | `- layer: id` | Reference another layer |

Use `exclude` to exclude specific crates:

```yaml
usecase:
  - regex: "^app-usecase-(.+)$"
  - exclude:
    - crate: app-usecase-fixtures
```

### Rules

Define allowed/denied dependencies for each crate. Rules are split into `internal` (dependencies on workspace members) and `external` (dependencies on third-party crates).

```yaml
rules:
  # Apply rules to a layer
  - layer: entity
    internal:
      allow:
        - layer: id
        - layer: value-object
    external:
      allow:
        - crate: serde
        - crate: chrono

  # Apply rules to a specific crate
  - crate: app-server
    internal:
      allow:
        - layer: domain
        - layer: usecase
    external:
      allow:
        - layer: graphql

  # Apply rules by regex
  - regex: "^app-cli-(.+)$"
    internal:
      allow:
        - layer: domain
        - layer: service
```

### Policies

| Policy | Description |
|--------|-------------|
| `allow` | Allow the dependency. When the first policy is `allow`, unmatched dependencies are implicitly denied (default deny). |
| `deny` | Explicitly deny the dependency. |
| `ignore` | Skip the dependency check (for temporary exceptions). |

When multiple rules match, the **last rule wins** (last-match-wins).

### Full Example

```yaml
layers:
  internal:
    internal-all:
      - regex: "^(.+)$"
    internal-default-deny:
      - layer: internal-all
    domain:
      - layer: id
      - layer: entity
    id:
      - regex: "^app-id-(.+)$"
    entity:
      - regex: "^app-entity-(.+)$"
  external:
    external-default-deny:
      - layer: db
    db:
      - crate: diesel

rules:
  # Apply default deny to all crates
  - layer: internal-all
    internal:
      deny:
        - layer: internal-default-deny
    external:
      deny:
        - layer: external-default-deny

  # Rules for the entity layer
  - layer: entity
    internal:
      allow:
        - layer: id
    external:
      allow:
        - crate: serde

  # Temporary exception
  - crate: app-entity-user
    external:
      ignore:
        - crate: diesel
```

## Error Types

| Code | Description |
|------|-------------|
| `deny-policy` | Dependency denied by rule |
| `unused-ignore` | Unnecessary ignore entry |
| `unused-allow` | Allow entry that matches no actual dependency |
| `uncovered-crate` | Crate not covered by any rule |
| `undefined-layer` | Reference to an undefined layer |
| `layer-cycle` | Circular reference in layer definitions |

## License

MIT
