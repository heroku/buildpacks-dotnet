/// Asserts that an expression matches a given pattern.
///
/// This is a local polyfill for `assert_matches!` to ensure 100% region coverage.
/// Unlike the crate or standard macros, this implementation is stable and avoids
/// generating unreachable panic branches that `llvm-cov` flags as uncovered regions.
///
/// Additionally, for expressions with a guard, this macro improves test error output
/// by explicitly printing the pattern, guard condition, and actual value on failure.
#[cfg(test)]
#[macro_export]
macro_rules! assert_matches {
    // With a guard (e.g. `Ok(x) if x > 10``)
    ($expression:expr, $pattern:pat if $guard:expr $(,)?) => {
        assert!(
            matches!(&$expression, $pattern if $guard),
            "Expected match pattern: {} where {}, but got {:?}",
            stringify!($pattern),
            stringify!($guard),
            $expression
        );
    };

    // Without a guard (injects `if true` to force branch coverage)
    ($expression:expr, $pattern:pat $(,)?) => {
        assert!(
            matches!(&$expression, $pattern if true),
            "Expected match pattern: {}, but got {:?}",
            stringify!($pattern),
            $expression
        );
    };
}
