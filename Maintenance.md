# How to make a release

1. git tag -a VERSION_NAME -m SOME_USELESS COMMENT
2. git push origin VERSION_NAME

# How to increment Tiger format version

1. Create a new module file under `src/sheet/compat/versionN.rs` (copy-paste the previous version as a starting point)
2. In your new module, update the THIS_VERSION constant and the `as previous_version` import
3. Declare your new module in `src/sheet/compat.rs`
4. Also in `src/sheet/compat.rs`, update the `Version` enum and the `CURRENT_VERSION` constant
5. Update the `pub use self::compat::versionN::*;` line in src/sheet.rs
6. Update the sheet structures and From<> implementations in your new module as needed
