//! Harmonium Lab - Musical DNA Extraction, Benchmarking, and LLM-Assisted Tuning
//!
//! This crate provides tools for:
//! - Ingesting music scores (MusicXML/PDMX) and extracting Musical DNA
//! - Comparing generated music against reference corpora
//! - Building style profiles from music collections
//! - LLM-assisted algorithm tuning via Claude API
//!
//! ## Modules
//!
//! - `ingest` - MusicXML parsing and DNA extraction from scores
//! - `dna` - DNA comparison and similarity metrics
//! - `benchmark` - Style profile management and benchmarking
//! - `agent` - Claude API integration for tuning suggestions

pub mod agent;
pub mod benchmark;
pub mod dna;
pub mod ingest;

// Re-export key types
pub use benchmark::StyleProfile;
pub use dna::DNAComparator;
pub use ingest::MusicXMLIngester;
