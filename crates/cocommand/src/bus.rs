use tokio::sync::broadcast;

use crate::event::CoreEvent;

#[derive(Clone)]
pub struct Bus {
    sender: broadcast::Sender<CoreEvent>,
}

impl Bus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<CoreEvent> {
        self.sender.subscribe()
    }

    pub fn publish(
        &self,
        event: CoreEvent,
    ) -> Result<usize, broadcast::error::SendError<CoreEvent>> {
        self.sender.send(event)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::SessionPartUpdatedPayload;
    use crate::message::{MessagePart, PartBase, TextPart};
    use tokio::time::{timeout, Duration};

    fn test_event() -> CoreEvent {
        CoreEvent::SessionPartUpdated(SessionPartUpdatedPayload {
            request_id: "req-1".to_string(),
            session_id: "sess-1".to_string(),
            message_id: "msg-1".to_string(),
            part_id: "part-1".to_string(),
            part: MessagePart::Text(TextPart {
                base: PartBase::new("sess-1", "msg-1"),
                text: "hello".to_string(),
            }),
        })
    }

    #[tokio::test]
    async fn publish_and_receive_event() {
        let bus = Bus::new(8);
        let mut rx = bus.subscribe();

        let _ = bus.publish(test_event());

        let received = timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("timeout")
            .expect("recv");
        assert!(
            matches!(received, CoreEvent::SessionPartUpdated(ref e) if e.request_id == "req-1")
        );
    }

    #[tokio::test]
    async fn multiple_subscribers_receive_event() {
        let bus = Bus::new(8);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let _ = bus.publish(test_event());

        let event1 = rx1.recv().await.expect("recv1");
        let event2 = rx2.recv().await.expect("recv2");

        assert!(matches!(event1, CoreEvent::SessionPartUpdated(ref e) if e.request_id == "req-1"));
        assert!(matches!(event2, CoreEvent::SessionPartUpdated(ref e) if e.request_id == "req-1"));
    }
}
