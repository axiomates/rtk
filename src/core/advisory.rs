//! Controls RTK-authored advisory output.

/// Axiomate embeds this branch as a quiet service. RTK-authored hints and
/// parser diagnostics stay silent unless explicitly requested for debugging.
pub fn enabled() -> bool {
    std::env::var("RTK_ADVISORY").as_deref() == Ok("1")
        || std::env::var("RTK_VERBOSE_WARNINGS").as_deref() == Ok("1")
}

pub fn eprintln(args: std::fmt::Arguments<'_>) {
    if enabled() {
        eprintln!("{}", args);
    }
}

#[macro_export]
macro_rules! advisory_eprintln {
    ($($arg:tt)*) => {
        $crate::core::advisory::eprintln(format_args!($($arg)*))
    };
}
