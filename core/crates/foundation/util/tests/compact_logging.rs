use std::cell::Cell;

#[test]
fn product_logging_policy_controls_argument_evaluation() {
    let evaluated = Cell::new(0);
    let result = operit_util::app_log!(i,
        { evaluated.set(evaluated.get() + 1); "CompilePolicyTest" },
        { evaluated.set(evaluated.get() + 1); "argument evaluation" },
    );
    assert_eq!(result, 0);
    assert_eq!(evaluated.get(), if cfg!(feature = "no-app-logging") { 0 } else { 2 });
}
