use std::any::Any;
use std::sync::Arc;

use tokio::sync::broadcast;

pub trait Event: Send + Sync + Any + 'static {}

impl<T> Event for T where T: Send + Sync + Any + 'static {}

pub type BusEvent = Arc<dyn Any + Send + Sync>;

#[derive(Clone)]
pub struct Bus {
    sender: broadcast::Sender<BusEvent>,
}

impl Bus {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<BusEvent> {
        self.sender.subscribe()
    }

    pub fn publish<E>(&self, event: E) -> Result<usize, broadcast::error::SendError<BusEvent>>
    where
        E: Event + 'static,
    {
        self.sender.send(Arc::new(event))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[derive(Debug)]
    struct TestEvent {
        value: i32,
    }

    #[tokio::test]
    async fn publish_and_receive_event() {
        let bus = Bus::new(8);
        let mut rx = bus.subscribe();

        let _ = bus.publish(TestEvent { value: 42 });

        let received = timeout(Duration::from_millis(100), rx.recv())
            .await
            .expect("timeout")
            .expect("recv");
        let event = received
            .as_ref()
            .downcast_ref::<TestEvent>()
            .expect("type");
        assert_eq!(event.value, 42);
    }

    #[tokio::test]
    async fn multiple_subscribers_receive_event() {
        let bus = Bus::new(8);
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let _ = bus.publish(TestEvent { value: 7 });

        let event1 = rx1.recv().await.expect("recv1");
        let event2 = rx2.recv().await.expect("recv2");

        let event1 = event1
            .as_ref()
            .downcast_ref::<TestEvent>()
            .expect("type1");
        let event2 = event2
            .as_ref()
            .downcast_ref::<TestEvent>()
            .expect("type2");
        assert_eq!(event1.value, 7);
        assert_eq!(event2.value, 7);
    }
}
