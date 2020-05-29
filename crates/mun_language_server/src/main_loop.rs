use crate::protocol::{Connection, Message};
use crate::Result;
use futures::StreamExt;

enum Event {
    Msg(Message),
}

/// Runs the main loop of the language server. This will receive requests and handle them.
pub async fn main_loop(mut connection: Connection) -> Result<()> {
    loop {
        // Determine what to do next. This selects from different channels, the first message to
        // arrive is returned. If an error occurs on one of the channel the main loop is shutdown
        // with an error.
        let event = futures::select! {
            msg = connection.receiver.next() => match msg {
                Some(msg) => Event::Msg(msg),
                None => return Err(anyhow::anyhow!("client exited without shutdown")),
            }
        };

        // Handle the event
        match handle_event(event, &mut connection).await? {
            LoopState::Continue => {}
            LoopState::Shutdown => {
                break;
            }
        }
    }

    Ok(())
}

/// A `LoopState` enumerator determines the state of the main loop
enum LoopState {
    Continue,
    Shutdown,
}

/// Handles an incoming event. Returns a `LoopState` state which determines whether processing
/// should continue.
async fn handle_event(event: Event, connection: &mut Connection) -> Result<LoopState> {
    if let Event::Msg(Message::Request(req)) = event {
        if connection.handle_shutdown(&req).await? {
            return Ok(LoopState::Shutdown);
        };
    }

    Ok(LoopState::Continue)
}
