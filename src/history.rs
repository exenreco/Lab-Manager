use crate::model::Server;

/// Session-local inventory history. A failed save must leave both stacks intact.
#[derive(Default)]
pub struct InventoryHistory {
    past: Vec<Vec<Server>>,
    future: Vec<Vec<Server>>,
}

impl InventoryHistory {
    pub fn record(&mut self, previous: Vec<Server>) {
        if self.past.len() == 50 {
            self.past.remove(0);
        }
        self.past.push(previous);
        self.future.clear();
    }
    pub fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    pub fn restore(
        &mut self,
        current: &[Server],
        redo: bool,
        save: impl FnOnce(&[Server]) -> Result<(), String>,
    ) -> Result<Option<Vec<Server>>, String> {
        let source = if redo {
            &mut self.future
        } else {
            &mut self.past
        };
        let Some(target) = source.last() else {
            return Ok(None);
        };
        save(target)?;
        let target = source
            .pop()
            .expect("the saved history entry is still present");
        let destination = if redo {
            &mut self.past
        } else {
            &mut self.future
        };
        destination.push(current.to_vec());
        Ok(Some(target))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn inventory(ip: &str) -> Vec<Server> {
        serde_json::from_value(serde_json::json!([{
            "name": ip, "ip": ip, "web_port": 80, "ssh_port": 22,
            "https": false, "username": "root"
        }]))
        .unwrap()
    }
    #[test]
    fn undo_redo_save_failure_and_new_branch() {
        let a = inventory("192.168.1.1");
        let b = inventory("192.168.1.2");
        let mut history = InventoryHistory::default();
        history.record(a.clone());
        assert!(
            history
                .restore(&b, false, |_| Err("disk full".into()))
                .is_err()
        );
        assert!(history.can_undo());
        assert!(!history.can_redo());
        assert_eq!(
            history
                .restore(&b, false, |target| {
                    assert_eq!(target, &a);
                    Ok(())
                })
                .unwrap(),
            Some(a.clone())
        );
        assert_eq!(
            history.restore(&a, true, |_| Ok(())).unwrap(),
            Some(b.clone())
        );
        history.restore(&b, false, |_| Ok(())).unwrap();
        history.record(a);
        assert!(!history.can_redo());
        assert!(!InventoryHistory::default().can_undo());
    }
}
