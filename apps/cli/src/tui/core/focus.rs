use super::app::{FocusArea, OperitTui};

impl OperitTui {
    pub(super) fn focus_next_area(&mut self) {
        self.clamp_pending_queue_selection();
        let queue_available = !self.pending_queue_messages.is_empty();
        self.focus = match self.focus {
            FocusArea::Chats => FocusArea::Input,
            FocusArea::ModelChooser => FocusArea::Input,
            FocusArea::Queue => {
                if self.show_chat_list {
                    FocusArea::Chats
                } else {
                    FocusArea::Input
                }
            }
            FocusArea::Input => {
                if queue_available {
                    FocusArea::Queue
                } else if self.show_chat_list {
                    FocusArea::Chats
                } else {
                    FocusArea::Input
                }
            }
        };
    }
}
