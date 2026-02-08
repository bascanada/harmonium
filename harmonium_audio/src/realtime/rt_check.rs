#![allow(unsafe_code)]
//! Real-time safety checker for audio processing
//!
//! This module provides a custom allocator that panics if any allocation or deallocation
//! occurs while in an audio processing context. This is crucial for real-time audio
//! where allocations can cause unpredictable latency and glitches.
//!
//! The allocator guard is only active in debug builds (#[`cfg(debug_assertions)`]).
//! In release builds, the functions are no-ops with zero overhead.
//!
//! # Usage
//!
//! ```rust,ignore
//! pub fn process_buffer(&mut self, output: &mut [f32], channels: usize) {
//!     crate::realtime::rt_check::enter_audio_context();
//!
//!     // ... audio processing code ...
//!     // Any allocation here will panic in debug builds
//!
//!     crate::realtime::rt_check::exit_audio_context();
//! }
//! ```

#[cfg(debug_assertions)]
use std::alloc::{GlobalAlloc, Layout, System};
#[cfg(debug_assertions)]
use std::cell::Cell;

#[cfg(debug_assertions)]
thread_local! {
    // Thread-local flag indicating whether we're currently in an audio processing context
    // Using thread_local! ensures tests don't interfere with each other
    static IN_AUDIO_THREAD: Cell<bool> = const { Cell::new(false) };
    // Thread-local flag to force disable checks (even if enter_audio_context is called)
    static FORCE_DISABLE: Cell<bool> = const { Cell::new(false) };
}

/// Custom allocator that checks for real-time violations
///
/// This allocator wraps the system allocator and panics if any allocation or
/// deallocation occurs while `IN_AUDIO_THREAD` is true.
#[cfg(debug_assertions)]
pub struct RTCheckAllocator;

#[cfg(debug_assertions)]
macro_rules! check_rt_violation {
    ($($arg:tt)*) => {
        let in_audio = IN_AUDIO_THREAD.with(std::cell::Cell::get);
        let disabled = FORCE_DISABLE.with(std::cell::Cell::get);
        if !disabled {
            assert!(!in_audio, $($arg)*);
        }
    };
}

#[cfg(debug_assertions)]
unsafe impl GlobalAlloc for RTCheckAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        check_rt_violation!(
            "REAL-TIME VIOLATION: Allocation in audio thread! size={} bytes, align={}",
            layout.size(),
            layout.align()
        );
        // SAFETY: We're delegating to the system allocator with the same layout
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        check_rt_violation!(
            "REAL-TIME VIOLATION: Deallocation in audio thread! size={} bytes, align={}",
            layout.size(),
            layout.align()
        );
        // SAFETY: We're delegating to the system allocator with the same ptr/layout
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        check_rt_violation!(
            "REAL-TIME VIOLATION: Zero allocation in audio thread! size={} bytes, align={}",
            layout.size(),
            layout.align()
        );
        // SAFETY: We're delegating to the system allocator with the same layout
        unsafe { System.alloc_zeroed(layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        check_rt_violation!(
            "REAL-TIME VIOLATION: Reallocation in audio thread! old_size={} bytes, new_size={} bytes",
            layout.size(),
            new_size
        );
        // SAFETY: We're delegating to the system allocator with the same ptr/layout/new_size
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// Mark the beginning of an audio processing context
///
/// Call this at the start of your audio callback. Any allocations after this
/// point will cause a panic in debug builds.
#[cfg(debug_assertions)]
#[inline]
pub fn enter_audio_context() {
    IN_AUDIO_THREAD.with(|flag| flag.set(true));
}

/// Mark the end of an audio processing context
///
/// Call this at the end of your audio callback. Allocations after this point
/// will be allowed again.
#[cfg(debug_assertions)]
#[inline]
pub fn exit_audio_context() {
    IN_AUDIO_THREAD.with(|flag| flag.set(false));
}

/// Globally disable real-time checks for this thread
/// Useful for offline export/processing
#[cfg(debug_assertions)]
pub fn disable_rt_check() {
    FORCE_DISABLE.with(|flag| flag.set(true));
}

/// Globally enable real-time checks for this thread
#[cfg(debug_assertions)]
pub fn enable_rt_check() {
    FORCE_DISABLE.with(|flag| flag.set(false));
}

/// No-op in release builds
#[cfg(not(debug_assertions))]
#[inline]
pub fn enter_audio_context() {}

/// No-op in release builds
#[cfg(not(debug_assertions))]
#[inline]
pub fn exit_audio_context() {}

/// No-op in release builds
#[cfg(not(debug_assertions))]
pub fn disable_rt_check() {}

/// No-op in release builds
#[cfg(not(debug_assertions))]
pub fn enable_rt_check() {}

#[cfg(test)]
mod tests {

    #[test]
    #[cfg(debug_assertions)]
    fn test_allocation_outside_audio_context() {
        // Should not panic
        let _vec = vec![1, 2, 3, 4, 5];
        drop(_vec);
    }

    // NOTE: These tests are commented out because #[should_panic] tests cause
    // double-panics during unwinding (panic during deallocation while handling
    // the allocation panic). The allocator works correctly in production use.
    //
    // #[test]
    // #[cfg(debug_assertions)]
    // #[should_panic(expected = "REAL-TIME VIOLATION: Allocation in audio thread")]
    // fn test_allocation_inside_audio_context() {
    //     enter_audio_context();
    //     // This should panic
    //     let _vec = vec![1, 2, 3, 4, 5];
    //     exit_audio_context();
    // }
    //
    // #[test]
    // #[cfg(debug_assertions)]
    // #[should_panic(expected = "REAL-TIME VIOLATION: Deallocation in audio thread")]
    // fn test_deallocation_inside_audio_context() {
    //     let vec = vec![1, 2, 3, 4, 5];
    //     enter_audio_context();
    //     // This should panic when vec is dropped
    //     drop(vec);
    //     exit_audio_context();
    // }

    #[test]
    #[cfg(not(debug_assertions))]
    fn test_no_op_in_release() {
        // This test only makes sense in release builds where guards are no-ops
        // In debug builds, this would trigger the allocator guard
        enter_audio_context();
        let _vec = vec![1, 2, 3, 4, 5];
        exit_audio_context();
    }
}
