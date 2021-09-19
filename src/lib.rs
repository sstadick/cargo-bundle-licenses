#![forbid(unsafe_code)]
#![allow(
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate
)]
pub mod bundle;
pub mod discovery;
pub mod finalized_license;
pub mod format;
pub mod found_license;
pub mod license;
pub mod package_loader;
