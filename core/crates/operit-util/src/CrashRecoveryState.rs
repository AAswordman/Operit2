#[derive(Debug, Clone, Eq, PartialEq)]
pub enum CrashRecoveryState {
    Idle,
    Recovering,
    Recovered,
}
