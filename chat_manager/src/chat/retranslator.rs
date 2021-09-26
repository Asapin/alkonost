use tokio::sync::mpsc::{Receiver, Sender};

pub struct Retranslator<T, E>
where
    E: From<T>,
{
    rx_from_outside: Receiver<T>,
    tx_to_inside: Sender<E>,
    should_close: Box<dyn Fn(&E) -> bool + Send>,
}

impl<T, E> Retranslator<T, E>
where
    E: From<T>,
{
    pub fn init(
        rx_from_outside: Receiver<T>,
        tx_to_inside: Sender<E>,
        should_close: Box<dyn Fn(&E) -> bool + Send>,
    ) -> Self {
        Self {
            rx_from_outside,
            tx_to_inside,
            should_close,
        }
    }

    pub async fn run(&mut self) {
        loop {
            match self.rx_from_outside.recv().await {
                Some(message) => {
                    let converted_message: E = message.into();
                    let close = (self.should_close)(&converted_message);
                    match self.tx_to_inside.send(converted_message).await {
                        Ok(_r) => {}
                        Err(e) => {
                            panic!("Inner channel was closed before the retranslator: {}", &e);
                        }
                    }
                    if close {
                        return;
                    }
                }
                None => {
                    panic!("Retranslator should always be closed first")
                }
            }
        }
    }
}
