use crate::logic::{
    error::{CodecError, MessageTypeError, ReceiveMessageError, SendMessageError},
    node::Node,
    wire::{Cursor, WireCodec},
};
use heapless::Vec;

pub const MESSAGE_SIZE: usize = 256;

pub type MessageData = Vec<u8, MESSAGE_SIZE>;

pub const BROADCAST_NODE: Node = Node::new([0xFF; 6]);

#[derive(Clone, Debug)]
pub enum MessageContent {
    Application(MessageData),
    Discovery,
    Invitation,
    RequestNews,
    SendNew((Node, i32)),
    FinSendNew,
    UpsertEdge((Option<Node>, Option<Node>)),
    RequestInitTopology(Node),
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MessageType {
    Application = 0x01,
    Discovery = 0x02,
    Invitation = 0x03,
    RequestNews = 0x04,
    SendNew = 0x05,
    FinSendNew = 0x06,
    UpsertEdge = 0x07,
    RequestInitTopology = 0x08,
}

impl From<&MessageContent> for MessageType {
    fn from(content: &MessageContent) -> Self {
        match content {
            MessageContent::Application(_) => MessageType::Application,
            MessageContent::Discovery => MessageType::Discovery,
            MessageContent::Invitation => MessageType::Invitation,
            MessageContent::RequestNews => MessageType::RequestNews,
            MessageContent::SendNew(_) => MessageType::SendNew,
            MessageContent::FinSendNew => MessageType::FinSendNew,
            MessageContent::UpsertEdge(_) => MessageType::UpsertEdge,
            MessageContent::RequestInitTopology(_) => MessageType::RequestInitTopology,
        }
    }
}

impl TryFrom<u8> for MessageType {
    type Error = MessageTypeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(MessageType::Application),
            0x02 => Ok(MessageType::Discovery),
            0x03 => Ok(MessageType::Invitation),
            0x04 => Ok(MessageType::RequestNews),
            0x05 => Ok(MessageType::SendNew),
            0x06 => Ok(MessageType::FinSendNew),
            0x07 => Ok(MessageType::UpsertEdge),
            0x08 => Ok(MessageType::RequestInitTopology),
            v => Err(MessageTypeError::InvalidMessageType(v)),
        }
    }
}

impl WireCodec<MESSAGE_SIZE> for MessageContent {
    fn encode(&self, out: &mut MessageData) -> Result<(), CodecError> {
        out.push(MessageType::from(self) as u8)
            .map_err(|e| CodecError::BufferOverflowError(e))?;
        match self {
            Self::Application(d) => {
                out.push(d.len() as u8)
                    .map_err(|e| CodecError::BufferOverflowError(e))?;
                out.extend_from_slice(d)
                    .map_err(|e| CodecError::BufferCapacityError(e))?;
            }
            Self::Discovery => {}
            Self::Invitation => {}
            Self::RequestNews => {}
            Self::SendNew((n, rssi)) => {
                n.encode(out).map_err(|_| CodecError::CodecError)?;
                out.extend_from_slice(&rssi.to_le_bytes())
                    .map_err(|e| CodecError::BufferCapacityError(e))?;
            }
            Self::FinSendNew => {}
            Self::UpsertEdge((n, p)) => {
                n.encode(out).map_err(|_| CodecError::CodecError)?;
                p.encode(out).map_err(|_| CodecError::CodecError)?;
            }
            Self::RequestInitTopology(n) => {
                n.encode(out).map_err(|_| CodecError::CodecError)?;
            }
        }
        Ok(())
    }

    fn decode(cursor: &mut Cursor<'_>) -> Result<Self, CodecError> {
        let type_byte = cursor.take(1).map_err(|e| CodecError::CursorReadError(e))?[0];
        let msg_type =
            MessageType::try_from(type_byte).map_err(|e| CodecError::MessageTypeError(e))?;
        match msg_type {
            MessageType::Application => {
                let len_byte = cursor.take(1).map_err(|e| CodecError::CursorReadError(e))?[0];
                let mut d = MessageData::new();
                d.extend_from_slice(
                    cursor
                        .take(len_byte as usize)
                        .map_err(|e| CodecError::CursorReadError(e))?,
                )
                .map_err(|e| CodecError::BufferCapacityError(e))?;
                Ok(MessageContent::Application(d))
            }
            MessageType::Discovery => Ok(MessageContent::Discovery),
            MessageType::Invitation => Ok(MessageContent::Invitation),
            MessageType::RequestNews => Ok(MessageContent::RequestNews),
            MessageType::SendNew => {
                let n = Node::decode(cursor).map_err(|_| CodecError::CodecError)?;
                let rssi_bytes = cursor.take(4).map_err(|e| CodecError::CursorReadError(e))?;
                let rssi =
                    i32::from_le_bytes(rssi_bytes.try_into().map_err(|_| CodecError::CodecError)?);
                Ok(MessageContent::SendNew((n, rssi)))
            }
            MessageType::FinSendNew => Ok(MessageContent::FinSendNew),
            MessageType::UpsertEdge => {
                let n = Option::<Node>::decode(cursor).map_err(|_| CodecError::CodecError)?;
                let p = Option::<Node>::decode(cursor).map_err(|_| CodecError::CodecError)?;
                Ok(MessageContent::UpsertEdge((n, p)))
            }
            MessageType::RequestInitTopology => {
                let n = Node::decode(cursor).map_err(|_| CodecError::CodecError)?;
                Ok(MessageContent::RequestInitTopology(n))
            }
        }
    }
}

#[derive(Debug)]
pub struct SendMessage {
    data: MessageContent,
    pub final_destination: Node,
    pub final_source: Option<Node>,
}

impl SendMessage {
    pub fn new(final_destination: Node, data: MessageContent, final_source: Option<Node>) -> Self {
        return SendMessage {
            data,
            final_destination,
            final_source,
        };
    }

    pub fn serialize(&self) -> Result<MessageData, SendMessageError> {
        let mut out = MessageData::new();
        self.data
            .encode(&mut out)
            .map_err(|e| SendMessageError::MessageTypeEncodeError(e))?;
        self.final_destination
            .encode(&mut out)
            .map_err(|e| SendMessageError::FinalDestinationEncodeError(e))?;
        self.final_source
            .encode(&mut out)
            .map_err(|e| SendMessageError::FinalSourceEncodeError(e))?;
        Ok(out)
    }
}

#[derive(Debug)]
pub struct ReceiveMessage {
    pub data: MessageContent,
    pub final_destination: Node,
    pub destination: Node,
    pub source: Node,
    pub final_source: Node,
    pub rssi: i32,
}

impl ReceiveMessage {
    pub fn new(
        payload: MessageData,
        destination: Node,
        source: Node,
        rssi: i32,
    ) -> Result<Self, ReceiveMessageError> {
        let mut cursor = Cursor::new(&payload);
        let data = MessageContent::decode(&mut cursor)
            .map_err(|e| ReceiveMessageError::MessageTypeDecodeError(e))?;
        let final_destination = Node::decode(&mut cursor)
            .map_err(|e| ReceiveMessageError::FinalDestinationDecodeError(e))?;
        let final_source = match Option::<Node>::decode(&mut cursor)
            .map_err(|e| ReceiveMessageError::FinalSourceDecodeError(e))?
        {
            Some(val) => val,
            None => source,
        };
        Ok(ReceiveMessage {
            data,
            destination,
            source: source,
            final_destination,
            final_source,
            rssi,
        })
    }

    pub fn is_final_destination(&self) -> bool {
        self.final_destination == self.destination
    }

    pub fn is_organization(&self) -> bool {
        match MessageType::from(&self.data) {
            MessageType::Discovery => true,
            MessageType::SendNew => true,
            MessageType::Invitation => true,
            MessageType::FinSendNew => true,
            MessageType::RequestNews => true,
            MessageType::UpsertEdge => true,
            MessageType::RequestInitTopology => true,
            _ => false,
        }
    }
}

impl Into<SendMessage> for ReceiveMessage {
    fn into(self) -> SendMessage {
        SendMessage {
            final_destination: self.final_destination,
            final_source: Some(self.final_source),
            data: self.data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unwrap_print;

    #[test]
    fn test_message_type_encode_decode() {
        let msg_content = MessageContent::Application(MessageData::new());
        let mut out = MessageData::new();
        unwrap_print!(msg_content.encode(&mut out));

        let mut cursor = Cursor::new(&out);
        let decoded = unwrap_print!(MessageContent::decode(&mut cursor));
        assert_eq!(MessageType::from(&decoded), MessageType::Application);
    }

    #[test]
    fn test_send_message_to_receive_message() {
        let final_destination = Node::new([10, 20, 30, 40, 50, 60]);
        let destination = Node::new([10, 20, 30, 40, 50, 60]);
        let source = Node::new([10, 20, 30, 40, 50, 60]);
        let data = MessageContent::Application(MessageData::new());
        let send_msg = SendMessage::new(final_destination.clone(), data.clone(), None);

        let serialized = unwrap_print!(send_msg.serialize());

        let receive_msg = unwrap_print!(ReceiveMessage::new(serialized, destination, source, 0));

        assert_eq!(
            MessageType::from(&data),
            MessageType::from(&receive_msg.data)
        );
        assert_eq!(final_destination, receive_msg.final_destination);
    }

    #[test]
    fn test_receive_message_into_send_message() {
        let final_destination = Node::new([10, 20, 30, 40, 50, 60]);
        let destination = Node::new([10, 20, 30, 40, 50, 60]);
        let source = Node::new([10, 20, 30, 40, 50, 60]);
        let data = MessageContent::Application(MessageData::new());
        let tmpl_msg = SendMessage::new(final_destination.clone(), data.clone(), None);

        let serialized = unwrap_print!(tmpl_msg.serialize());
        let receive_msg = unwrap_print!(ReceiveMessage::new(serialized, destination, source, 0));
        let send_msg: SendMessage = receive_msg.into();

        assert_eq!(MessageType::from(&data), MessageType::from(&send_msg.data));
        assert_eq!(final_destination, send_msg.final_destination);
    }
}
