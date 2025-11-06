/// Attempts to find and build a single item from an input using a list of strategies.
///
/// This function iterates through the provided `strategies` in order. Each strategy
/// is a tuple containing a `finder`, a `builder`, and an `on_multiple_handler`.
///
/// For each strategy:
/// 1.  The `finder` is called with the `input`.
/// 2.  The `finder` can return:
///  * `Ok(vec![])`: (0 items) This strategy found nothing. The function continues
///    to the next strategy.
///  * `Ok(vec![item])`: (1 item) A single item was found. The `builder` is
///    called with this item, and the resulting `T` is returned as `Ok(T)`.
///    No further strategies are tried.
///  * `Ok(vec![...])`: (2+ items) Multiple items were found. The
///    `on_multiple_handler` is called with the `Vec<Item>` of items to
///    generate an error, which is returned as `Err(E)`. No further strategies
///    are tried.
///  * `Err(FE)`: (Finder Error) An error occurred during the find operation.
///    This error is converted into `E` and returned immediately as `Err(E)`.
///    No further strategies are tried.
///
/// If all strategies are exhausted and all return `Ok(vec![])`, the provided
/// `not_found_error` is returned.
///
/// # Type Parameters
///
/// * `'a`: The lifetime of the references in the `strategies` iterator.
/// * `T`: The output type to be built.
/// * `E`: The error type for this function.
/// * `I`: The input type to be searched.
/// * `Item`: The intermediate item type returned by the `finder` closure.
/// * `FE`: The specific error type for the finder, which must be convertible into `E`.
/// * `F`: The finder closure: `Fn(&I) -> Result<Vec<Item>, FE>`.
/// * `B`: The builder closure: `Fn(Item) -> T`.
/// * `M`: The handler for multiple matches: `Fn(Vec<Item>) -> E`.
///
/// # Arguments
///
/// * `input`: A reference to the input data to be processed.
/// * `strategies`: An iterable (such as a slice or `Vec`) of tuples, where
///   each tuple contains a `(finder, builder, on_multiple_handler)`.
/// * `not_found_error`: The error value to return if all strategies are
///   exhausted without finding a match. This value is created eagerly.
pub(crate) fn find_first_match<'a, T, E, I, Item, FE, F, B, M>(
    input: &I,
    strategies: impl IntoIterator<Item = &'a (F, B, M)>,
    not_found_error: E,
) -> Result<T, E>
where
    I: ?Sized,
    F: Fn(&I) -> Result<Vec<Item>, FE> + 'a,
    B: Fn(Item) -> T + 'a,
    M: Fn(Vec<Item>) -> E + 'a,
    FE: Into<E>,
{
    for (finder, builder, on_multiple) in strategies {
        let items = finder(input).map_err(Into::into)?;

        match items.len() {
            0 => {}
            1 => {
                return Ok(builder(
                    items.into_iter().next().expect("item should exist"),
                ));
            }
            _ => {
                return Err(on_multiple(items));
            }
        }
    }

    Err(not_found_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ITEM_FOO: &str = "item_foo";
    const ITEM_BAR: &str = "item_bar";
    const FINDER_ERR_MSG: &str = "database_connection_failed";

    #[derive(Debug, PartialEq)]
    enum TestError {
        NotFound,
        SpecificNotFound,
        TooMany(Vec<String>),
        FinderFailed(String),
    }
    struct MyFindError(String);

    impl From<MyFindError> for TestError {
        fn from(fe: MyFindError) -> Self {
            TestError::FinderFailed(fe.0)
        }
    }

    type TestItem = String;
    type TestOutput = String;

    type TestFinder = fn(&str) -> Result<Vec<TestItem>, MyFindError>;
    type TestBuilder = fn(TestItem) -> TestOutput;
    type TestMultiHandler = fn(Vec<TestItem>) -> TestError;

    #[allow(clippy::needless_pass_by_value)]
    fn builder_uppercase(item: TestItem) -> TestOutput {
        item.to_uppercase()
    }

    #[allow(clippy::needless_pass_by_value)]
    fn builder_prefix(item: TestItem) -> TestOutput {
        format!("prefix:{item}")
    }

    fn handle_multiple(items: Vec<TestItem>) -> TestError {
        TestError::TooMany(items)
    }

    #[allow(clippy::unnecessary_wraps)]
    fn finder_empty(_i: &str) -> Result<Vec<TestItem>, MyFindError> {
        Ok(vec![])
    }

    #[allow(clippy::unnecessary_wraps)]
    fn finder_foo(_i: &str) -> Result<Vec<TestItem>, MyFindError> {
        Ok(vec![ITEM_FOO.to_string()])
    }

    #[allow(clippy::unnecessary_wraps)]
    fn finder_bar(_i: &str) -> Result<Vec<TestItem>, MyFindError> {
        Ok(vec![ITEM_BAR.to_string()])
    }

    #[allow(clippy::unnecessary_wraps)]
    fn finder_many(_i: &str) -> Result<Vec<TestItem>, MyFindError> {
        Ok(vec![ITEM_FOO.to_string(), ITEM_BAR.to_string()])
    }

    fn finder_error(_i: &str) -> Result<Vec<TestItem>, MyFindError> {
        Err(MyFindError(FINDER_ERR_MSG.to_string()))
    }

    #[test]
    fn test_success_on_first_strategy() {
        let input = "find_me";
        let strategies: &[(TestFinder, TestBuilder, _)] = &[
            (finder_foo, builder_uppercase, handle_multiple),
            (finder_empty, builder_prefix, handle_multiple),
        ];

        let result = find_first_match(input, strategies, TestError::NotFound);
        assert_eq!(result, Ok(ITEM_FOO.to_uppercase()));
    }

    #[test]
    fn test_success_on_second_strategy() {
        let input = "find_me_later";
        let strategies: &[(TestFinder, TestBuilder, _)] = &[
            (finder_empty, builder_uppercase, handle_multiple),
            (finder_bar, builder_prefix, handle_multiple),
        ];

        let result = find_first_match(input, strategies, TestError::NotFound);
        assert_eq!(result, Ok(format!("prefix:{ITEM_BAR}")));
    }

    #[test]
    fn test_failure_all_strategies_find_nothing() {
        let input = "find_nothing";
        let strategies: &[(_, TestBuilder, _)] = &[
            (finder_empty, builder_uppercase, handle_multiple),
            (finder_empty, builder_prefix, handle_multiple),
        ];

        let result = find_first_match(input, strategies, TestError::SpecificNotFound);
        assert_eq!(result, Err(TestError::SpecificNotFound));
    }

    #[test]
    fn test_failure_on_multiple_items() {
        let input = "find_too_many";
        let strategies: &[(TestFinder, TestBuilder, _)] = &[
            (finder_many, builder_uppercase, handle_multiple),
            (finder_bar, builder_prefix, handle_multiple),
        ];

        let result = find_first_match(input, strategies, TestError::NotFound);
        let expected_err = TestError::TooMany(vec![ITEM_FOO.to_string(), ITEM_BAR.to_string()]);
        assert_eq!(result, Err(expected_err));
    }

    #[test]
    fn test_failure_on_finder_error() {
        let input = "cause_an_error";
        let strategies: &[(TestFinder, TestBuilder, _)] = &[
            (finder_error, builder_uppercase, handle_multiple),
            (finder_bar, builder_prefix, handle_multiple),
        ];

        let result = find_first_match(input, strategies, TestError::NotFound);
        let expected_err = TestError::FinderFailed(FINDER_ERR_MSG.to_string());
        assert_eq!(result, Err(expected_err));
    }

    #[test]
    fn test_empty_strategies_list() {
        let input = "anything";
        let strategies: &[(TestFinder, TestBuilder, TestMultiHandler)] = &[];
        let result = find_first_match(input, strategies, TestError::NotFound);
        assert_eq!(result, Err(TestError::NotFound));
    }
}
