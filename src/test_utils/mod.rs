#[macro_export]
macro_rules! check {
    ($name:expr, $cond:expr, $passed:expr, $failed:expr) => {
        if $cond {
            $crate::kinfo!("[PASS] {}", $name);
            $passed += 1;
        } else {
            $crate::kerror!("[FAIL] {}", $name);
            $failed += 1;
        }
    };
}
