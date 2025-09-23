use tokio::sync::watch;

pub type ShutdownSender = watch::Sender<bool>;
pub type ShutdownReceiver = watch::Receiver<bool>;

pub fn channel() -> (ShutdownSender, ShutdownReceiver) {
    watch::channel(false)
}

pub fn trigger(sender: &ShutdownSender) {
    let _ = sender.send(true);
}

pub async fn wait(mut receiver: ShutdownReceiver) {
    if *receiver.borrow() {
        return;
    }

    while receiver.changed().await.is_ok() {
        if *receiver.borrow() {
            break;
        }
    }
}
