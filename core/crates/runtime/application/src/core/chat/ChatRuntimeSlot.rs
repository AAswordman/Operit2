/// Identifies an independently addressable chat runtime owned by the holder.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum ChatRuntimeSlot {
    /// Primary in-app chat runtime that follows global chat selection.
    MAIN,
    /// Floating chat runtime that keeps local state while mirroring selected chats.
    FLOATING,
    /// Ad-hoc runtime keyed by an external identifier.
    DETACHED(String),
}
