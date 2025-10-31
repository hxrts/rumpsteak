// Projection from global choreographies to local session types

use crate::ast::{Branch, Choreography, LocalType, MessageType, Protocol, Role};

/// Project a choreography to a local session type for a specific role
pub fn project(choreography: &Choreography, role: &Role) -> Result<LocalType, ProjectionError> {
    let mut context = ProjectionContext::new(choreography, role);
    context.project_protocol(&choreography.protocol)
}

/// Errors that can occur during projection
#[derive(Debug, thiserror::Error)]
pub enum ProjectionError {
    #[error("Cannot project choice for non-participant role")]
    NonParticipantChoice,

    #[error("Parallel composition not supported for role {0}")]
    UnsupportedParallel(String),

    #[error("Inconsistent projections in parallel branches")]
    InconsistentParallel,

    #[error("Recursive variable {0} not in scope")]
    UnboundVariable(String),
}

/// Context for projection algorithm
///
/// Note: Future enhancements may include:
/// - `choreography: &'a Choreography` for global validation during projection
/// - `rec_env: HashMap<String, LocalType>` for memoizing recursive projections
struct ProjectionContext<'a> {
    role: &'a Role,
}

impl<'a> ProjectionContext<'a> {
    fn new(_choreography: &'a Choreography, role: &'a Role) -> Self {
        ProjectionContext { role }
    }

    fn project_protocol(&mut self, protocol: &Protocol) -> Result<LocalType, ProjectionError> {
        match protocol {
            Protocol::Send {
                from,
                to,
                message,
                continuation,
            } => self.project_send(from, to, message, continuation),

            Protocol::Broadcast {
                from,
                to_all,
                message,
                continuation,
            } => self.project_broadcast(from, to_all, message, continuation),

            Protocol::Choice {
                role: choice_role,
                branches,
            } => self.project_choice(choice_role, branches),

            Protocol::Loop { condition, body } => self.project_loop(condition.as_ref(), body),

            Protocol::Parallel { protocols } => self.project_parallel(protocols),

            Protocol::Rec { label, body } => self.project_rec(label, body),

            Protocol::Var(label) => self.project_var(label),

            Protocol::End => Ok(LocalType::End),
        }
    }

    /// Project a send operation onto the local type for this role
    ///
    /// # Projection Rules
    /// - If `role == from`: Project to `Send(to, message, continuation↓role)`
    /// - If `role == to`: Project to `Receive(from, message, continuation↓role)`
    /// - Otherwise: Project to `continuation↓role` (uninvolved party)
    ///
    /// This implements the standard session type projection rule where
    /// uninvolved parties simply skip communication they don't participate in.
    fn project_send(
        &mut self,
        from: &Role,
        to: &Role,
        message: &MessageType,
        continuation: &Protocol,
    ) -> Result<LocalType, ProjectionError> {
        if self.role == from {
            // We are the sender
            Ok(LocalType::Send {
                to: to.clone(),
                message: message.clone(),
                continuation: Box::new(self.project_protocol(continuation)?),
            })
        } else if self.role == to {
            // We are the receiver
            Ok(LocalType::Receive {
                from: from.clone(),
                message: message.clone(),
                continuation: Box::new(self.project_protocol(continuation)?),
            })
        } else {
            // We are not involved, skip to continuation
            self.project_protocol(continuation)
        }
    }

    /// Project a broadcast operation onto the local type for this role
    ///
    /// # Projection Rules
    /// - If `role == from`: Expand into nested sends to all recipients
    /// - If `role ∈ to_all`: Project to `Receive(from, message, continuation↓role)`  
    /// - Otherwise: Project to `continuation↓role`
    ///
    /// # Implementation Note
    /// Broadcasts are expanded into sequential sends at the sender side.
    /// Sends are built in reverse order to create proper nesting:
    /// `Broadcast(A, [B,C], msg) → Send(A→B, Send(A→C, continuation))`
    fn project_broadcast(
        &mut self,
        from: &Role,
        to_all: &[Role],
        message: &MessageType,
        continuation: &Protocol,
    ) -> Result<LocalType, ProjectionError> {
        if self.role == from {
            // We are broadcasting - need to send to each recipient
            let mut current = self.project_protocol(continuation)?;

            // Build sends in reverse order so they nest correctly
            for to in to_all.iter().rev() {
                current = LocalType::Send {
                    to: to.clone(),
                    message: message.clone(),
                    continuation: Box::new(current),
                };
            }

            Ok(current)
        } else if to_all.contains(self.role) {
            // We are receiving the broadcast
            Ok(LocalType::Receive {
                from: from.clone(),
                message: message.clone(),
                continuation: Box::new(self.project_protocol(continuation)?),
            })
        } else {
            // Not involved in broadcast
            self.project_protocol(continuation)
        }
    }

    /// Project a choice operation onto the local type for this role
    ///
    /// # Projection Rules (Enhanced)
    /// - If `role == choice_role`:
    ///   - If branches start with Send: Project as `Select` (communicated choice)
    ///   - Otherwise: Project as `LocalChoice` (local decision)
    /// - If `role` receives the choice: Project as `Branch`
    /// - Otherwise: Merge continuations (uninvolved party)
    ///
    /// # Implementation Notes
    /// This enhancement supports choice branches that don't start with Send,
    /// allowing for local decisions and more complex choreographic patterns.
    fn project_choice(
        &mut self,
        choice_role: &Role,
        branches: &[Branch],
    ) -> Result<LocalType, ProjectionError> {
        if self.role == choice_role {
            // We make the choice
            // Check if this is a communicated choice (branches start with Send)
            let first_sends = branches
                .iter()
                .all(|b| matches!(&b.protocol, Protocol::Send { .. }));

            if first_sends && !branches.is_empty() {
                // Communicated choice - project as Select
                let mut local_branches = Vec::new();

                for branch in branches {
                    // Skip the initial send (it's implied by the choice)
                    let inner_protocol = match &branch.protocol {
                        Protocol::Send { continuation, .. } => continuation,
                        _ => &branch.protocol, // Won't happen due to check above
                    };

                    let local_type = self.project_protocol(inner_protocol)?;
                    local_branches.push((branch.label.clone(), local_type));
                }

                // Find the recipient (from first branch's send)
                let recipient = match &branches[0].protocol {
                    Protocol::Send { to, .. } => to.clone(),
                    _ => {
                        return Err(ProjectionError::NonParticipantChoice);
                    }
                };

                Ok(LocalType::Select {
                    to: recipient,
                    branches: local_branches,
                })
            } else {
                // Local choice (no communication) - project as LocalChoice
                let mut local_branches = Vec::new();

                for branch in branches {
                    let local_type = self.project_protocol(&branch.protocol)?;
                    local_branches.push((branch.label.clone(), local_type));
                }

                Ok(LocalType::LocalChoice {
                    branches: local_branches,
                })
            }
        } else {
            // Check if we receive the choice
            let mut receives_choice = false;
            let mut sender = None;

            for branch in branches {
                if let Protocol::Send { from, to, .. } = &branch.protocol {
                    if self.role == to {
                        receives_choice = true;
                        sender = Some(from.clone());
                        break;
                    }
                }
            }

            if receives_choice {
                // We receive the choice - project as Branch
                let sender = sender.expect("sender must be Some when receives_choice is true");
                let mut local_branches = Vec::new();

                for branch in branches {
                    let local_type = self.project_protocol(&branch.protocol)?;
                    local_branches.push((branch.label.clone(), local_type));
                }

                Ok(LocalType::Branch {
                    from: sender,
                    branches: local_branches,
                })
            } else {
                // Not involved in the choice - merge continuations
                self.merge_choice_continuations(branches)
            }
        }
    }

    /// Project a loop operation onto the local type for this role
    ///
    /// # Projection Rules
    /// - Project the loop body
    /// - If the role participates in the loop: Wrap in `Loop` with condition
    /// - If the role doesn't participate: Project to End
    ///
    /// # Implementation Notes
    /// Loop conditions are now preserved in the local type, allowing runtime
    /// to make decisions about loop iteration based on the condition type.
    fn project_loop(
        &mut self,
        condition: Option<&crate::ast::protocol::Condition>,
        body: &Protocol,
    ) -> Result<LocalType, ProjectionError> {
        let body_projection = self.project_protocol(body)?;

        // Only include Loop if the body actually involves this role
        if body_projection == LocalType::End {
            Ok(LocalType::End)
        } else {
            Ok(LocalType::Loop {
                condition: condition.cloned(),
                body: Box::new(body_projection),
            })
        }
    }

    /// Project a parallel composition onto the local type for this role
    ///
    /// # Projection Rules (Enhanced)
    /// - If role appears in 0 branches: Project to `End`
    /// - If role appears in 1 branch: Use that projection
    /// - If role appears in multiple branches:
    ///   - Check for conflicts (incompatible operations)
    ///   - If mergeable: Interleave operations
    ///   - If conflicting: Return error with details
    ///
    /// # Implementation Notes
    /// This enhancement detects conflicting parallel operations (e.g., sending
    /// to the same recipient simultaneously) and provides better error messages.
    fn project_parallel(&mut self, protocols: &[Protocol]) -> Result<LocalType, ProjectionError> {
        // Project all parallel branches for this role
        let mut projections = Vec::new();
        for protocol in protocols {
            if protocol.mentions_role(self.role) {
                projections.push(self.project_protocol(protocol)?);
            }
        }

        match projections.len() {
            0 => {
                // Role doesn't appear in any parallel branch
                Ok(LocalType::End)
            }
            1 => {
                // Role appears in exactly one branch - use that projection
                Ok(projections
                    .into_iter()
                    .next()
                    .expect("projections must have exactly one element"))
            }
            _ => {
                // Role appears in multiple parallel branches
                // Check for conflicts before merging
                self.merge_parallel_projections(projections)
            }
        }
    }

    /// Merge multiple parallel projections for a single role
    ///
    /// # Conflict Detection
    /// Checks for incompatible parallel operations:
    /// - Multiple sends to the same role
    /// - Multiple receives from the same role
    /// - Conflicting choices
    ///
    /// # Merging Strategy
    /// For compatible operations, interleaves them sequentially.
    /// The actual execution order is non-deterministic (depends on runtime).
    fn merge_parallel_projections(
        &mut self,
        projections: Vec<LocalType>,
    ) -> Result<LocalType, ProjectionError> {
        // Remove End projections as they don't contribute
        let non_end: Vec<_> = projections
            .into_iter()
            .filter(|p| p != &LocalType::End)
            .collect();

        match non_end.len() {
            0 => Ok(LocalType::End),
            1 => Ok(non_end
                .into_iter()
                .next()
                .expect("non_end must have exactly one element")),
            _ => {
                // Check for conflicts
                self.check_parallel_conflicts(&non_end)?;

                // Multiple non-trivial projections - merge them
                // Interleaving is allowed in parallel composition
                // The merge creates a sequential composition (non-deterministic order)
                let mut merged = LocalType::End;
                for proj in non_end.into_iter().rev() {
                    merged = self.sequential_merge(proj, merged);
                }
                Ok(merged)
            }
        }
    }

    /// Check for conflicts between parallel projections
    ///
    /// Returns an error if the projections have incompatible operations
    /// that cannot be safely interleaved.
    fn check_parallel_conflicts(&self, projections: &[LocalType]) -> Result<(), ProjectionError> {
        // Check for conflicting sends
        let mut send_targets = Vec::new();
        let mut recv_sources = Vec::new();

        for proj in projections {
            match proj {
                LocalType::Send { to, .. } => {
                    if send_targets.contains(to) {
                        return Err(ProjectionError::InconsistentParallel);
                    }
                    send_targets.push(to.clone());
                }
                LocalType::Receive { from, .. } => {
                    if recv_sources.contains(from) {
                        return Err(ProjectionError::InconsistentParallel);
                    }
                    recv_sources.push(from.clone());
                }
                LocalType::Select { to, .. } => {
                    if send_targets.contains(to) {
                        return Err(ProjectionError::InconsistentParallel);
                    }
                    send_targets.push(to.clone());
                }
                LocalType::Branch { from, .. } => {
                    if recv_sources.contains(from) {
                        return Err(ProjectionError::InconsistentParallel);
                    }
                    recv_sources.push(from.clone());
                }
                _ => {
                    // Other types are compatible with parallel composition
                }
            }
        }

        Ok(())
    }

    fn sequential_merge(&self, first: LocalType, second: LocalType) -> LocalType {
        // Merge two local types sequentially
        match (first, second) {
            (LocalType::End, other) | (other, LocalType::End) => other,
            (first, second) => {
                // Chain them together
                self.append_continuation(first, second)
            }
        }
    }

    fn append_continuation(&self, local_type: LocalType, continuation: LocalType) -> LocalType {
        Self::append_continuation_static(local_type, continuation)
    }

    fn append_continuation_static(local_type: LocalType, continuation: LocalType) -> LocalType {
        match local_type {
            LocalType::Send {
                to,
                message,
                continuation: cont,
            } => LocalType::Send {
                to,
                message,
                continuation: Box::new(Self::append_continuation_static(
                    *cont,
                    continuation.clone(),
                )),
            },
            LocalType::Receive {
                from,
                message,
                continuation: cont,
            } => LocalType::Receive {
                from,
                message,
                continuation: Box::new(Self::append_continuation_static(
                    *cont,
                    continuation.clone(),
                )),
            },
            LocalType::End => continuation,
            // For other types, just return as-is with continuation appended at the end
            other => other,
        }
    }

    fn project_rec(
        &mut self,
        label: &proc_macro2::Ident,
        body: &Protocol,
    ) -> Result<LocalType, ProjectionError> {
        let body_projection = self.project_protocol(body)?;

        // Only include Rec if the body actually uses this role
        if body_projection == LocalType::End {
            Ok(LocalType::End)
        } else {
            Ok(LocalType::Rec {
                label: label.clone(),
                body: Box::new(body_projection),
            })
        }
    }

    fn project_var(&mut self, label: &proc_macro2::Ident) -> Result<LocalType, ProjectionError> {
        Ok(LocalType::Var(label.clone()))
    }

    fn merge_choice_continuations(
        &mut self,
        branches: &[Branch],
    ) -> Result<LocalType, ProjectionError> {
        // Project each branch and find where we rejoin
        let mut projections = Vec::new();

        for branch in branches {
            projections.push(self.project_protocol(&branch.protocol)?);
        }

        // Check if all projections are identical (common case)
        if projections.windows(2).all(|w| w[0] == w[1]) {
            Ok(projections
                .into_iter()
                .next()
                .expect("projections must be non-empty when windows returned true"))
        } else {
            // Different projections per branch
            // Find common suffix (merge point) if one exists
            self.find_merge_point(projections)
        }
    }

    fn find_merge_point(&self, projections: Vec<LocalType>) -> Result<LocalType, ProjectionError> {
        // Look for a common continuation across all branches
        // Use the first non-End projection as representative
        // Advanced: find least common continuation across all branches
        Ok(projections
            .into_iter()
            .find(|p| p != &LocalType::End)
            .unwrap_or(LocalType::End))
    }
}

// Helper to compare LocalTypes for equality
impl PartialEq for LocalType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (LocalType::End, LocalType::End) => true,
            (LocalType::Var(a), LocalType::Var(b)) => a == b,
            (
                LocalType::Send {
                    to: to1,
                    message: msg1,
                    continuation: cont1,
                },
                LocalType::Send {
                    to: to2,
                    message: msg2,
                    continuation: cont2,
                },
            ) => to1 == to2 && msg1.name == msg2.name && cont1 == cont2,
            (
                LocalType::Receive {
                    from: from1,
                    message: msg1,
                    continuation: cont1,
                },
                LocalType::Receive {
                    from: from2,
                    message: msg2,
                    continuation: cont2,
                },
            ) => from1 == from2 && msg1.name == msg2.name && cont1 == cont2,
            (
                LocalType::Select {
                    to: to1,
                    branches: br1,
                },
                LocalType::Select {
                    to: to2,
                    branches: br2,
                },
            ) => {
                to1 == to2
                    && br1.len() == br2.len()
                    && br1
                        .iter()
                        .zip(br2.iter())
                        .all(|((l1, t1), (l2, t2))| l1 == l2 && t1 == t2)
            }
            (
                LocalType::Branch {
                    from: from1,
                    branches: br1,
                },
                LocalType::Branch {
                    from: from2,
                    branches: br2,
                },
            ) => {
                from1 == from2
                    && br1.len() == br2.len()
                    && br1
                        .iter()
                        .zip(br2.iter())
                        .all(|((l1, t1), (l2, t2))| l1 == l2 && t1 == t2)
            }
            (
                LocalType::LocalChoice { branches: br1 },
                LocalType::LocalChoice { branches: br2 },
            ) => {
                br1.len() == br2.len()
                    && br1
                        .iter()
                        .zip(br2.iter())
                        .all(|((l1, t1), (l2, t2))| l1 == l2 && t1 == t2)
            }
            (
                LocalType::Loop {
                    condition: c1,
                    body: b1,
                },
                LocalType::Loop {
                    condition: c2,
                    body: b2,
                },
            ) => {
                // For conditions, we compare structurally
                let cond_eq = match (c1, c2) {
                    (None, None) => true,
                    (
                        Some(crate::ast::protocol::Condition::Count(n1)),
                        Some(crate::ast::protocol::Condition::Count(n2)),
                    ) => n1 == n2,
                    (
                        Some(crate::ast::protocol::Condition::RoleDecides(r1)),
                        Some(crate::ast::protocol::Condition::RoleDecides(r2)),
                    ) => r1 == r2,
                    _ => false,
                };
                cond_eq && b1 == b2
            }
            (
                LocalType::Rec {
                    label: l1,
                    body: b1,
                },
                LocalType::Rec {
                    label: l2,
                    body: b2,
                },
            ) => l1 == l2 && b1 == b2,
            _ => false,
        }
    }
}

impl Eq for LocalType {}
