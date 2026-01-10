/// Real-time audio safety utilities
///
/// This module provides utilities for ensuring real-time safety in audio processing code.
/// The debug allocator guard can catch allocations and deallocations in the audio thread,
/// which are prohibited in real-time audio contexts.

pub mod rt_check;
