use crate::logic::{
    error::{CodecError, MessageTypeError, ReceiveMessageError, SendMessageError},
    node::Node,
    wire::{Cursor, WireCodec},
};
use heapless::Vec;

pub const MESSAGE_SIZE: usize = 256;

pub type MessageData = Vec<u8, MESSAGE_SIZE>;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum MessageType {
    Application = 0x01,
}

impl WireCodec<MESSAGE_SIZE> for MessageType {
    const SIZE: usize = 1;

    fn encode(&self, out: &mut MessageData) -> Result<(), CodecError> {
        out.push(*self as u8)
            .map_err(|e| CodecError::BufferOverflowError(e))
    }

    fn decode(cursor: &mut Cursor<'_>) -> Result<Self, CodecError> {
        let b = cursor.take(1).map_err(|e| CodecError::CursorReadError(e))?[0];
        match b {
            0x01 => Ok(MessageType::Application),
            e => Err(CodecError::MessageTypeError(
                MessageTypeError::InvalidMessageType(e),
            )),
        }
    }
}

#[derive(Debug)]
pub struct SendMessage {
    message_type: MessageType,
    data: MessageData,
    pub final_destination: Node,
}

impl SendMessage {
    pub fn new(data: MessageData, final_destination: Node, message_type: MessageType) -> Self {
        return SendMessage {
            data,
            final_destination,
            message_type,
        };
    }

    pub fn serialize(&self) -> Result<MessageData, SendMessageError> {
        let mut out = MessageData::new();
        self.message_type
            .encode(&mut out)
            .map_err(|e| SendMessageError::MessageTypeEncodeError(e))?;
        self.final_destination
            .encode(&mut out)
            .map_err(|e| SendMessageError::FinalDestinationEncodeError(e))?;
        out.extend_from_slice(&self.data)
            .map_err(|e| SendMessageError::MessageTooLargeError(e))?;
        Ok(out)
    }
}

pub struct ReceiveMessage {
    pub message_type: MessageType,
    pub data: MessageData,
    final_destination: Node,
    destination: Node,
    _source: Node,
}

impl ReceiveMessage {
    pub fn new(
        payload: MessageData,
        destination: Node,
        source: Node,
    ) -> Result<Self, ReceiveMessageError> {
        let mut cursor = Cursor::new(&payload);
        let message_type = MessageType::decode(&mut cursor)
            .map_err(|e| ReceiveMessageError::MessageTypeDecodeError(e))?;
        let final_destination = Node::decode(&mut cursor)
            .map_err(|e| ReceiveMessageError::FinalDestinationDecodeError(e))?;
        let mut data = MessageData::new();
        data.extend_from_slice(cursor.remaining())
            .map_err(|e| ReceiveMessageError::BufferOverflowError(e))?;
        Ok(ReceiveMessage {
            destination,
            _source: source,
            final_destination,
            data,
            message_type,
        })
    }

    pub fn is_final_destination(&self) -> bool {
        self.final_destination == self.destination
    }
}

impl Into<SendMessage> for ReceiveMessage {
    fn into(self) -> SendMessage {
        SendMessage {
            message_type: self.message_type,
            final_destination: self.final_destination,
            data: self.data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::unwrap_print;
    use heapless::Vec;

    #[test]
    fn test_message_type_encode_decode() {
        let msg_type = MessageType::Application;
        let mut out = MessageData::new();
        unwrap_print!(msg_type.encode(&mut out));

        let mut cursor = Cursor::new(&out);
        let decoded = unwrap_print!(MessageType::decode(&mut cursor));
        assert_eq!(decoded, MessageType::Application);
    }

    #[test]
    fn test_send_message_to_receive_message() {
        let data: MessageData = Vec::from_slice(&[0x10, 0x20, 0x30]).unwrap();
        let final_destination = Node::new([10, 20, 30, 40, 50, 60]);
        let destination = Node::new([10, 20, 30, 40, 50, 60]);
        let source = Node::new([10, 20, 30, 40, 50, 60]);
        let msg_type = MessageType::Application;
        let send_msg = SendMessage::new(data.clone(), final_destination.clone(), msg_type);

        let serialized = unwrap_print!(send_msg.serialize());

        let receive_msg = unwrap_print!(ReceiveMessage::new(serialized, destination, source));

        assert_eq!(data, receive_msg.data);
        assert_eq!(final_destination, receive_msg.final_destination);
        assert_eq!(msg_type, receive_msg.message_type);
    }

    #[test]
    fn test_receive_message_into_send_message() {
        let data: MessageData = Vec::from_slice(&[0x10, 0x20, 0x30]).unwrap();
        let final_destination = Node::new([10, 20, 30, 40, 50, 60]);
        let destination = Node::new([10, 20, 30, 40, 50, 60]);
        let source = Node::new([10, 20, 30, 40, 50, 60]);
        let msg_type = MessageType::Application;
        let tmpl_msg = SendMessage::new(data.clone(), final_destination.clone(), msg_type);

        let serialized = unwrap_print!(tmpl_msg.serialize());
        let receive_msg = unwrap_print!(ReceiveMessage::new(serialized, destination, source));
        let send_msg: SendMessage = receive_msg.into();

        assert_eq!(data, send_msg.data);
        assert_eq!(final_destination, send_msg.final_destination);
        assert_eq!(msg_type, send_msg.message_type);
    }
}
