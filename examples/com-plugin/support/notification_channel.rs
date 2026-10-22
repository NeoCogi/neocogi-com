//! Host-owned domain notification queue; plugin contracts have no channel dependency.
use neocogi_com::{ComPtr, NcStatus, NcValueRef, NC_E_CLOSED};
use neocogi_idl_example_contracts::{ExtentNotification, IExtentObserver, IExtentObserverAbi};

#[neocogi_com::com_object(neocogi_idl_example_contracts::IExtentObserver)]
/// Retains the sending endpoint, never the producer or application state.
struct Observer {
    /// Owned facts survive source destruction.
    sender: std::sync::mpsc::Sender<ExtentNotification>,
}
impl IExtentObserver for Observer {
    fn changed(&mut self, notification: NcValueRef<'_, ExtentNotification>) -> Result<(), NcStatus> {
        self.sender.send(notification.to_owned()?).map_err(|_| NC_E_CLOSED)
    }
}
/// Creates an ordinary host queue behind the domain's typed observer ABI.
pub fn channel() -> (ComPtr<IExtentObserverAbi>, std::sync::mpsc::Receiver<ExtentNotification>) {
    let (sender, receiver) = std::sync::mpsc::channel();
    (Observer::from_value(Observer { sender }).query().unwrap(), receiver)
}
