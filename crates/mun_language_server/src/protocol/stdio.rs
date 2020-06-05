use super::Message;
use async_std::io::BufReader;
use futures::{channel::mpsc, SinkExt, StreamExt};

/// Constructs a communication channel over stdin (input) and stdout (output)
pub(crate) fn stdio_transport() -> (mpsc::Sender<Message>, mpsc::Receiver<Message>) {
    let (writer_sender, mut writer_receiver) = mpsc::channel::<Message>(0);
    let (mut reader_sender, reader_receiver) = mpsc::channel::<Message>(0);

    // Receive messages over the channel and forward them to stdout
    async_std::task::spawn(async move {
        let mut stdout = async_std::io::stdout();
        while let Some(msg) = writer_receiver.next().await {
            msg.write(&mut stdout).await.unwrap();
        }
    });

    // Receive data over stdin and forward to the application
    async_std::task::spawn(async move {
        let mut stdin = BufReader::new(async_std::io::stdin());
        while let Some(msg) = Message::read(&mut stdin).await.unwrap() {
            let is_exit = match &msg {
                Message::Notification(n) => n.is_exit(),
                _ => false,
            };

            reader_sender.send(msg).await.unwrap();

            if is_exit {
                break;
            }
        }
    });

    (writer_sender, reader_receiver)
}
