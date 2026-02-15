use tokio::sync::watch;

#[derive(Clone)]
pub struct CancellationToken {
    receiver: watch::Receiver<bool>,
}

impl CancellationToken {
    pub fn new() -> (watch::Sender<bool>, Self) {
        let (tx, rx) = watch::channel(false);
        (tx, Self { receiver: rx })
    }

    pub fn is_cancelled(&self) -> bool {
        *self.receiver.borrow()
    }

    pub async fn cancelled(&mut self) {
        while !*self.receiver.borrow() {
            if self.receiver.changed().await.is_err() {
                break;
            }
        }
    }

    pub fn receiver(&self) -> watch::Receiver<bool> {
        self.receiver.clone()
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        let (_, rx) = watch::channel(false);
        Self { receiver: rx }
    }
}
