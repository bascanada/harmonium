//! LLM Agent Integration
//!
//! Provides Claude API integration for LLM-assisted algorithm tuning.

mod claude;

pub use claude::{AgentError, ClaudeAgent, ParameterChange, TuningSuggestion};
