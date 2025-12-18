use core::fmt;
use embassy_sync::channel::{TryReceiveError, TrySendError};
use esp_radio::esp_now::EspNowError;
use heapless::CapacityError;
use crate::logic::{message::SendMessage, error::{TreeError, SendMessageError}};

#[derive(Debug)]
pub enum MeshError {
    SliceConversionError(CapacityError),
    SendQueueError(TrySendError<SendMessage>),
    ReceiveQueueError(TryReceiveError),
    TreeSetupError(TreeError),
    SendMessageError(EspNowError),
    SerializeMessageError(SendMessageError),
    RouteError(TreeError),
}

impl fmt::Display for MeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SliceConversionError(e) => write!(f, "Failed to convert slice:\n{}", e),
            Self::SendQueueError(e) => write!(f, "Failed to send into queue:\n{:?}", e),
            Self::ReceiveQueueError(e) => write!(f, "Failed to receieve from queue:\n{:?}", e),
            Self::TreeSetupError(e) => write!(f, "Failed to create route Tree:\n{}", e),
            Self::SendMessageError(e) => write!(f, "Failed to send message:\n{}", e),
            Self::SerializeMessageError(e) => write!(f, "Failed to serialize message:\n{}", e),
            Self::RouteError(e) => write!(f, "Failed to get next hop:\n{}", e),
        }
    }
}
