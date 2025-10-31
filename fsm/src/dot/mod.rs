//! DOT format support for FSMs.
//!
//! This module provides both export to and parsing from the DOT graph format.
//! DOT files can be visualized using Graphviz tools.

mod parse;

#[cfg(feature = "parsing")]
pub use self::parse::{parse, parse_with_refinements, ParseErrors};

use super::Fsm;
use std::fmt::{self, Display, Formatter};

/// Wrapper for exporting an FSM in DOT format.
///
/// DOT is a graph description language used by Graphviz for visualization.
///
/// # Example
///
/// ```rust
/// use rumpsteak_fsm::{Fsm, Dot};
///
/// let fsm = Fsm::new("Client");
/// let dot = Dot::new(&fsm);
/// println!("{}", dot);
/// ```
pub struct Dot<'a, R, N, E>(&'a Fsm<R, N, E>);

impl<'a, R, N, E> Dot<'a, R, N, E> {
    /// Creates a new DOT exporter for the given FSM.
    pub fn new(fsm: &'a Fsm<R, N, E>) -> Self {
        Self(fsm)
    }
}

impl<'a, R: Display, N: Display, E: Display> Display for Dot<'a, R, N, E> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "digraph \"{}\" {{", self.0.role())?;
        let (states, transitions) = self.0.size();

        if states > 0 {
            writeln!(f)?;
        }

        for i in self.0.states() {
            writeln!(f, "    {};", i.index())?;
        }

        if transitions > 0 {
            writeln!(f)?;
        }

        for (from, to, transition) in self.0.transitions() {
            let (from, to) = (from.index(), to.index());
            writeln!(f, "    {} -> {} [label=\"{}\"];", from, to, transition)?;
        }

        write!(f, "}}")
    }
}
