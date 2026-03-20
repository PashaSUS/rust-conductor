//! Fuzz target: TaskDef JSON deserialization
//! Run: cargo +nightly fuzz run fuzz_task_def

#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = serde_json::from_slice::<serde_json::Value>(data);
});
