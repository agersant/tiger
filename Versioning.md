# How to increment Tiger format version

1. Create new module file as `src/sheet/compat/versionN.rs` (copy-paste the previous version as a starting point)
2. In your new module, update the `previous_version` import
3. Declare said module in `src/sheet/compat.rs`
4. Update the `pub use self::compat::version1::*;` line in src/sheet.rs
5. Update the `Version` enum, `CURRENT_VERSION` constant and `read_sheet` function in `src/sheet/compat.rs`
