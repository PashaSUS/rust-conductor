//! Fuzz target: WorkflowDef JSON deserialization
//! Run: cargo +nightly fuzz run fuzz_workflow_def
//!
//! NOTE: Requires the crate to have a lib.rs exporting models.
//! Currently, fuzz-style tests run via proptest in engine::tests.

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    // Attempt JSON deserialization — must never panic
    let _ = serde_json::from_slice::<serde_json::Value>(data);
    // If valid JSON, try stronger typed deserialization
    if let Ok(v) = serde_json::from_slice::<serde_json::Value>(data) {
        if v.is_object() {
            // Would use: let _ = serde_json::from_value::<rust_conductor::WorkflowDef>(v);
            // when lib.rs is configured
            let _ = v;
        }
    }
});
