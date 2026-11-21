use crate::AttachmentInfo::AttachmentInfo;
use serde::{Deserialize, Serialize};

/// Identifies one user message waiting for the active chat turn to finish.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PendingQueueMessageItem {
    pub id: i64,
    pub text: String,
    /// Attachments captured from the composer when the draft was queued.
    /// Defaults to empty so entries serialized before this field existed still decode.
    #[serde(default)]
    pub attachments: Vec<AttachmentInfo>,
}

#[cfg(test)]
mod tests {
    use super::PendingQueueMessageItem;
    use crate::AttachmentInfo::AttachmentInfo;

    #[test]
    fn legacy_queue_entry_without_attachments_still_decodes() {
        let item: PendingQueueMessageItem =
            serde_json::from_str(r#"{"id":7,"text":"queued text"}"#).unwrap();
        assert_eq!(item.id, 7);
        assert_eq!(item.text, "queued text");
        assert!(item.attachments.is_empty());
    }

    #[test]
    fn queue_entry_round_trips_attachments() {
        let item = PendingQueueMessageItem {
            id: 3,
            text: "look at this".into(),
            attachments: vec![AttachmentInfo::new(
                "/tmp/image.png".into(),
                "image.png".into(),
                "image/png".into(),
                12,
            )],
        };
        let restored: PendingQueueMessageItem =
            serde_json::from_value(serde_json::to_value(&item).unwrap()).unwrap();
        assert_eq!(restored, item);
    }
}
