/// The native UI consumes send results once. Browser state is read by multiple
/// observers, so retain one bounded receipt and let each view acknowledge its id.
#[derive(Default)]
pub struct SendReceipt {
    sequence: u64,
    send_id: Option<String>,
    value: Option<serde_json::Value>,
}

impl SendReceipt {
    pub fn begin(&mut self, send_id: Option<&str>) {
        self.send_id = send_id.map(str::to_owned);
        self.value = None;
    }

    pub fn capture(&mut self, result: Option<Result<(), String>>) {
        let Some(result) = result else { return };
        self.sequence += 1;
        self.value = Some(match result {
            Ok(()) => serde_json::json!({"id": self.sequence, "sendId": self.send_id, "ok": true}),
            Err(error) => {
                serde_json::json!({"id": self.sequence, "sendId": self.send_id, "ok": false, "error": error})
            }
        });
    }

    pub fn snapshot(&self) -> Option<&serde_json::Value> {
        self.value.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::SendReceipt;

    #[test]
    fn observers_share_receipts_without_consuming_or_repeating_ids() {
        let mut receipt = SendReceipt::default();
        assert!(receipt.snapshot().is_none());
        receipt.capture(Some(Ok(())));
        let first = receipt.snapshot().cloned().unwrap();
        receipt.capture(None);
        assert_eq!(receipt.snapshot(), Some(&first));
        assert_eq!(receipt.snapshot().unwrap()["id"], 1);
        receipt.begin(Some("browser-two-send"));
        assert!(receipt.snapshot().is_none());
        receipt.capture(Some(Err("Core unavailable".into())));
        assert_eq!(receipt.snapshot().unwrap()["id"], 2);
        assert_eq!(receipt.snapshot().unwrap()["sendId"], "browser-two-send");
        assert_eq!(receipt.snapshot().unwrap()["error"], "Core unavailable");
    }
}
