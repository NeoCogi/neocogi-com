//! Installation of a concrete COM output into an ordinary Rust channel.

use std::sync::mpsc::Sender;
use neocogi_com::NcStatus;

/// Installs or replaces an output's destination and maps each decoded event into a message.
/// The generated receiver owns the mapper and sender; the application owns the receiver.
/// Translation runs immediately, including after consumer destruction. A disconnected
/// channel discards its translated value successfully, so closing a consumer cannot
/// turn a producer's already committed operation into an error.
/// Neither channel endpoint nor the application message crosses the COM boundary.
pub fn map<Event: 'static, Message: 'static>(
    install: impl FnOnce(Box<dyn FnMut(Event) -> Result<(), NcStatus>>) -> Result<(), NcStatus>,
    sender: Sender<Message>,
    mut translate: impl FnMut(Event) -> Message + 'static,
) -> Result<(), NcStatus> {
    install(Box::new(move |event| {
        // Mapping only constructs owned data. Receiver destruction is normal teardown;
        // send returns that undeliverable value and dropping it releases its resources.
        let _ = sender.send(translate(event));
        Ok(())
    }))
}
