#![forbid(unsafe_code)]
#![warn(missing_docs)]
//! Scripting: Lua 5.4 sandbox with budget enforcement, event IR
//! (graph nodes/edges compiled from visual graphs and Lua), and
//! serializable `Wait` continuations that survive save/load.
