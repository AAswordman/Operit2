use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttachmentInfo {
    pub filePath: String,
    /// The CoreNode holding this file; absent for inline content and legacy attachments.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nodeId: Option<String>,
    pub fileName: String,
    pub mimeType: String,
    pub fileSize: i64,
    pub content: String,
}

impl AttachmentInfo {
    pub fn new(filePath: String, fileName: String, mimeType: String, fileSize: i64) -> Self {
        Self {
            filePath,
            nodeId: None,
            fileName,
            mimeType,
            fileSize,
            content: String::new(),
        }
    }
    /// File tools use the same VFS path even when device storage roots differ.
    /// Keep the original host path for local previews and direct media processing.
    pub fn fileToolPath(&self) -> String {
        let normalized = self.filePath.replace('\\', "/");
        if let Some((_, relative)) = normalized.rsplit_once("/data/chat_attachments/") {
            if !relative.is_empty()
                && relative
                    .split('/')
                    .all(|part| !part.is_empty() && part != "." && part != "..")
            {
                return format!("/app/data/data/chat_attachments/{relative}");
            }
        }
        self.filePath.clone()
    }

    /// Never read a known remote attachment through the current node's file host.
    /// Missing origin information retains legacy behavior rather than inventing an owner.
    pub fn isLocalToNode(&self, currentNodeId: Option<&str>) -> bool {
        match self.nodeId.as_deref() {
            Some(nodeId) => Some(nodeId) == currentNodeId,
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::AttachmentInfo;

    #[test]
    fn legacy_attachment_does_not_invent_a_source_node() {
        let attachment: AttachmentInfo = serde_json::from_str(
            r#"{"filePath":"/old/file.pdf","fileName":"file.pdf","mimeType":"application/pdf","fileSize":3,"content":""}"#,
        ).unwrap();
        assert_eq!(attachment.nodeId, None);
        assert!(serde_json::to_value(attachment)
            .unwrap()
            .get("nodeId")
            .is_none());
    }

    #[test]
    fn source_node_survives_serialization_and_rejects_other_nodes() {
        let mut attachment = AttachmentInfo::new(
            "/file.pdf".into(),
            "file.pdf".into(),
            "application/pdf".into(),
            3,
        );
        attachment.nodeId = Some("core-b".into());
        let restored: AttachmentInfo =
            serde_json::from_value(serde_json::to_value(&attachment).unwrap()).unwrap();
        assert_eq!(restored, attachment);
        assert!(attachment.isLocalToNode(Some("core-b")));
        assert!(!attachment.isLocalToNode(Some("core-a")));
        assert!(!attachment.isLocalToNode(None));
    }

    #[test]
    fn persistent_file_tool_paths_are_portable_without_changing_host_paths() {
        for path in [
            "/device-b/runtime/data/chat_attachments/chat-a/attachment-a/file.pdf",
            r"C:\runtime\data\chat_attachments\chat-a\attachment-a\file.pdf",
        ] {
            let attachment =
                AttachmentInfo::new(path.into(), "file.pdf".into(), "application/pdf".into(), 3);
            assert_eq!(
                attachment.fileToolPath(),
                "/app/data/data/chat_attachments/chat-a/attachment-a/file.pdf"
            );
            assert_eq!(attachment.filePath, path);
        }
        let attachment = AttachmentInfo::new(
            "/runtime/data/chat_attachments/chat-a/../file.pdf".into(),
            "file.pdf".into(),
            "application/pdf".into(),
            3,
        );
        assert_eq!(attachment.fileToolPath(), attachment.filePath);
    }
}
