# Choreographic Projection Patterns

The projection algorithm transforms global choreographic protocols into local session types for each participating role. The algorithm has been enhanced to support three additional patterns beyond the basic send/receive operations.

## Supported Patterns

### 1. Basic Send/Receive

**Global Protocol:**
```rust
Protocol::Send {
    from: alice,
    to: bob,
    message: Data,
    continuation: End,
}
```

**Alice's Projection:**
```rust
LocalType::Send {
    to: bob,
    message: Data,
    continuation: End,
}
```

**Bob's Projection:**
```rust
LocalType::Receive {
    from: alice,
    message: Data,
    continuation: End,
}
```

**Charlie's Projection (uninvolved):**
```rust
LocalType::End
```

---

### 2. Communicated Choice

**Global Protocol:**
```rust
Protocol::Choice {
    role: alice,
    branches: vec![
        Branch {
            label: "yes",
            protocol: Send { from: alice, to: bob, ... },
        },
        Branch {
            label: "no",
            protocol: Send { from: alice, to: bob, ... },
        },
    ],
}
```

**Alice's Projection (chooser):**
```rust
LocalType::Select {
    to: bob,
    branches: vec![
        ("yes", ...),
        ("no", ...),
    ],
}
```

**Bob's Projection (receiver):**
```rust
LocalType::Branch {
    from: alice,
    branches: vec![
        ("yes", ...),
        ("no", ...),
    ],
}
```

---

### 3. Local Choice (No Communication)

Supports choice branches that don't start with `Send`, allowing for local decisions.

**Global Protocol:**
```rust
Protocol::Choice {
    role: alice,
    branches: vec![
        Branch {
            label: "option1",
            protocol: End,  // No Send!
        },
        Branch {
            label: "option2",
            protocol: End,
        },
    ],
}
```

**Alice's Projection:**
```rust
LocalType::LocalChoice {
    branches: vec![
        ("option1", End),
        ("option2", End),
    ],
}
```

**Key Difference:** `LocalChoice` vs `Select`:
- **Select**: Communicated choice (sends selection to another role)
- **LocalChoice**: Local decision (no communication)

---

### 4. Loop with Conditions

Loop conditions are now preserved in the projected local types.

**Global Protocol:**
```rust
Protocol::Loop {
    condition: Some(Condition::Count(5)),
    body: Send { from: alice, to: bob, ... },
}
```

**Alice's Projection:**
```rust
LocalType::Loop {
    condition: Some(Condition::Count(5)),
    body: Send { to: bob, ... },
}
```

**Supported Conditions:**
- `Condition::Count(n)` - Fixed iteration count
- `Condition::RoleDecides(role)` - Loop while a role decides
- `Condition::Custom(expr)` - Custom boolean expression
- `None` - Infinite loop (must be terminated externally)

---

### 5. Parallel Composition

Parallel merging with conflict detection.

#### Compatible Parallel (No Conflict)

**Global Protocol:**
```rust
Protocol::Parallel {
    protocols: vec![
        Send { from: alice, to: bob, ... },
        Send { from: alice, to: charlie, ... },  // Different recipient
    ],
}
```

**Alice's Projection:**
```rust
// Merged sequentially (order non-deterministic at runtime)
LocalType::Send {
    to: bob,
    continuation: Send {
        to: charlie,
        continuation: End,
    },
}
```

**Result:** **Success** - Different recipients, no conflict

#### Conflicting Parallel (Error)

**Global Protocol:**
```rust
Protocol::Parallel {
    protocols: vec![
        Send { from: alice, to: bob, ... },
        Send { from: alice, to: bob, ... },  // Same recipient!
    ],
}
```

**Alice's Projection:**
```rust
Err(ProjectionError::InconsistentParallel)
```

**Result:** **Error** - Cannot send to same recipient in parallel

**Conflict Detection Rules:**
- Multiple sends to the same role → **Conflict**
- Multiple receives from the same role → **Conflict**
- Multiple selects to the same role → **Conflict**
- Multiple branches from the same role → **Conflict**
- Operations on different roles → **OK**

---

## Projection Rules Summary

### Chooser's View

| Branch Pattern | Projection |
|----------------|------------|
| All branches start with `Send` | `Select` (communicated) |
| Branches don't start with `Send` | `LocalChoice` (local) |

### Receiver's View

| Participation | Projection |
|---------------|------------|
| Receives the choice | `Branch` |
| Not involved | Merge continuations |

### Parallel Composition

| Role Participation | Projection |
|--------------------|------------|
| Appears in 0 branches | `End` |
| Appears in 1 branch | Use that projection |
| Appears in 2+ branches | Merge if compatible, error if conflict |

---

## Implementation Notes

### LocalType Variants

The enhanced projection algorithm uses these `LocalType` variants:

```rust
pub enum LocalType {
    Send { to, message, continuation },
    Receive { from, message, continuation },
    Select { to, branches },
    Branch { from, branches },
    LocalChoice { branches },
    Loop { condition, body },
    Rec { label, body },
    Var(label),
    End,
}
```

### Code Generation

The `generate_type_expr` function in `codegen.rs` handles all variants including the new `LocalChoice` and `Loop` types.
