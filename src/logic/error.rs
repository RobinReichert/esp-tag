use crate::logic::{
    arena::SlotId,
    link::SendData,
    message::{MessageData, ReceiveMessage},
    node::Node,
};
use core::fmt;
use heapless::CapacityError;

#[derive(Debug)]
pub enum AsyncError {
    SpawnError,
}

impl fmt::Display for AsyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SpawnError => write!(f, "Failed to spawn task"),
        }
    }
}

#[derive(Debug)]
pub enum LinkError {
    QueueFullError(),
    QueueEmptyError(),
    AlreadyInitialized,
    SpawnError,
    MockError,
}

impl fmt::Display for LinkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::QueueFullError() => write!(
                f,
                "Failed to push new message since message queue was full:\n"
            ),
            Self::QueueEmptyError() => write!(
                f,
                "Failed to receive new message since message queue was empty:\n"
            ),
            Self::AlreadyInitialized => write!(f, "Link has already been initialized"),
            Self::SpawnError => write!(f, "Failed to spawn task"),
            Self::MockError => write!(f, "Nothing failed this is just a test"),
        }
    }
}

#[derive(Debug)]
pub enum MeshError {
    SerializationError(SendMessageError),
    TreeError(TreeError),
    LinkError(LinkError),
    ReceiveMessageError(ReceiveMessageError),
    OrganizeQueueSendError(),
    OrganizeQueueRecvError(),
    ReceiveQueueSendError(),
    SpawnError,
}

impl fmt::Display for MeshError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SerializationError(e) => write!(f, "Failed to serialize data:\n{}", e),
            Self::TreeError(e) => write!(f, "Failed to get next hop:\n{}", e),
            Self::LinkError(e) => write!(f, "Link produced an error:\n{}", e),
            Self::ReceiveMessageError(e) => write!(f, "Failed to create ReceiveMessage:\n{}", e),
            Self::OrganizeQueueSendError() => {
                write!(f, "Failed to send receive message to channel:\n")
            }
            Self::OrganizeQueueRecvError() => {
                write!(f, "Failed to receive message from channel:\n")
            }
            Self::ReceiveQueueSendError() => {
                write!(f, "Failed to send receive message to channel:\n")
            }
            Self::SpawnError => write!(f, "Failed to spawn task"),
        }
    }
}

#[derive(Debug)]
pub enum TreeError {
    LeafAllocationError,
    NodeNotFoundError,
    LeafNotFoundError(ArenaError),
    RootIsDestinationError,
    UninitializedError,
}

impl fmt::Display for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LeafAllocationError => write!(f, "Failed to allocate Leaf"),
            Self::NodeNotFoundError => write!(f, "Could not find Node"),
            Self::LeafNotFoundError(e) => write!(f, "Could not find Leaf:\n{}", e),
            Self::RootIsDestinationError => write!(f, "The root of this tree is the destination"),
            Self::UninitializedError => write!(f, "Tree is uninitialized"),
        }
    }
}

#[derive(Debug)]
pub enum ArenaError {
    SlotEmptyError(SlotId),
    InvalidIndexError(SlotId),
}

impl fmt::Display for ArenaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArenaError::SlotEmptyError(e) => write!(f, "Failed to find Node: {}", e),
            ArenaError::InvalidIndexError(e) => write!(f, "Index is not used: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum CursorError {
    BufferUnderflowError,
}

impl fmt::Display for CursorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferUnderflowError => write!(f, "Buffer underflow"),
        }
    }
}

#[derive(Debug)]
pub enum CodecError {
    MessageTypeError(MessageTypeError),
    CursorReadError(CursorError),
    BufferCapacityError(CapacityError),
    BufferOverflowError(u8),
    InvalidOptionFlagError(u8),
    CodecError,
}

impl fmt::Display for CodecError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MessageTypeError(e) => write!(f, "Error with MessageType:\n{}", e),
            Self::CursorReadError(e) => write!(f, "Failed to read from Cursor:\n{}", e),
            Self::BufferCapacityError(e) => {
                write!(f, "Buffer capacity exceeded while extending data.:\n{}", e)
            }
            Self::BufferOverflowError(e) => {
                write!(f, "Buffer is full; cannot push more bytes:\n{}", e)
            }
            Self::InvalidOptionFlagError(e) => write!(f, "Flag {} is not supported for option", e),
            Self::CodecError => write!(f, "Failed to encode component:\n"),
        }
    }
}

#[derive(Debug)]
pub enum MessageTypeError {
    InvalidMessageType(u8),
}

impl fmt::Display for MessageTypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMessageType(e) => write!(f, "Failed to parse message type from: {}", e),
        }
    }
}

#[derive(Debug)]
pub enum SendMessageError {
    MessageTypeEncodeError(CodecError),
    FinalDestinationEncodeError(CodecError),
    FinalSourceEncodeError(CodecError),
    MessageTooLargeError(CapacityError),
}

impl fmt::Display for SendMessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MessageTypeEncodeError(e) => write!(f, "Failed to encode MessageType:\n{}", e),
            Self::FinalDestinationEncodeError(e) => {
                write!(f, "Failed to encode final destination:\n{}", e)
            }
            Self::FinalSourceEncodeError(e) => {
                write!(f, "Failed to encode final source:\n{}", e)
            }
            Self::MessageTooLargeError(e) => {
                write!(f, "Message size exceeds buffer capacity:\n{}", e)
            }
        }
    }
}

#[derive(Debug)]
pub enum ReceiveMessageError {
    MessageTypeDecodeError(CodecError),
    FinalDestinationDecodeError(CodecError),
    FinalSourceDecodeError(CodecError),
    BufferOverflowError(CapacityError),
}

impl fmt::Display for ReceiveMessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MessageTypeDecodeError(e) => write!(f, "Failed to decode message type:\n{}", e),
            Self::FinalDestinationDecodeError(e) => {
                write!(f, "Failed to decode final destination:\n{}", e)
            }
            Self::FinalSourceDecodeError(e) => {
                write!(f, "Failed to decode final source:\n{}", e)
            }
            Self::BufferOverflowError(e) => {
                write!(f, "Failed to extend data from cursor buffer:\n{}", e)
            }
        }
    }
}
