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
    use heapless::Vec;

    #[test]
    fn test_node_encode_decode() {
        let node = Node::new([1, 2, 3, 4, 5, 6]);
        let mut out = Vec::<u8, MESSAGE_SIZE>::new();
        node.encode(&mut out).expect("Encoding failed");
        assert_eq!(out.len(), 6);
        assert_eq!(&out[..], &[1, 2, 3, 4, 5, 6]);

        let mut cursor = Cursor::new(&out);
        let decoded = Node::decode(&mut cursor).expect("Decoding failed");
        assert_eq!(decoded, node);
    }

    #[test]
    fn test_message_type_encode_decode() {
        let msg_type = MessageType::Application;
        let mut out = Vec::<u8, MESSAGE_SIZE>::new();
        msg_type.encode(&mut out).expect("Encoding failed");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], 0x01);

        let mut cursor = Cursor::new(&out);
        let decoded = MessageType::decode(&mut cursor).expect("Decoding failed");
        assert_eq!(decoded, MessageType::Application);
    }

    #[test]
    fn test_send_message_serialize() {
        let data: Vec<u8, MESSAGE_SIZE> = Vec::from_slice(&[0x10, 0x20, 0x30]).unwrap();
        let final_destination = Node::new([10, 20, 30, 40, 50, 60]);
        let msg_type = MessageType::Application;
        let send_msg = SendMessage::new(data.clone(), final_destination.clone(), msg_type);

        let serialized = send_msg.serialize().expect("Serialize failed");

        // Serialized layout: 1 byte message_type + 6 bytes Node + data
        assert_eq!(serialized.len(), 1 + 6 + data.len());
        assert_eq!(serialized[0], 0x01); // MessageType::Application
        assert_eq!(&serialized[1..7], &final_destination.mac);
        assert_eq!(&serialized[7..], &data[..]);
    }

    #[test]
    fn test_receive_message_new_and_is_final_destination() {
        let mut payload = Vec::<u8, MESSAGE_SIZE>::new();
        payload.push(0x01).unwrap(); // MessageType::Application
        payload.extend_from_slice(&[10, 20, 30, 40, 50, 60]).unwrap(); // final_destination
        payload.extend_from_slice(&[0x55, 0x66]).unwrap(); // data

        let destination = Node::new([10, 20, 30, 40, 50, 60]);
        let source = Node::new([1, 2, 3, 4, 5, 6]);

        let recv_msg =
            ReceiveMessage::new(payload.clone(), destination.clone(), source).expect("ReceiveMessage new failed");

        assert_eq!(recv_msg.message_type, MessageType::Application);
        assert_eq!(recv_msg.final_destination, destination);
        assert_eq!(recv_msg.data.len(), 2);
        assert_eq!(recv_msg.data[0], 0x55);
        assert_eq!(recv_msg.data[1], 0x66);

        assert!(recv_msg.is_final_destination());

        let other_destination = Node::new([99, 99, 99, 99, 99, 99]);
        let recv_msg2 =
            ReceiveMessage::new(payload.clone(), other_destination.clone(), source).expect("ReceiveMessage new failed");

        assert!(!recv_msg2.is_final_destination());
    }

    #[test]
    fn test_into_send_message() {
        let mut payload = Vec::<u8, MESSAGE_SIZE>::new();
        payload.push(0x01).unwrap(); // MessageType::Application
        payload.extend_from_slice(&[1, 2, 3, 4, 5, 6]).unwrap(); // final_destination
        payload.extend_from_slice(&[0x99, 0xAA]).unwrap(); // data

        let destination = Node::new([1, 2, 3, 4, 5, 6]);
        let source = Node::new([7, 8, 9, 10, 11, 12]);

        let recv_msg =
            ReceiveMessage::new(payload, destination.clone(), source).expect("ReceiveMessage new failed");

        let send_msg: SendMessage = recv_msg.into();

        assert_eq!(send_msg.message_type, MessageType::Application);
        assert_eq!(send_msg.final_destination, destination);
        assert_eq!(send_msg.data.len(), 2);
        assert_eq!(send_msg.data[0], 0x99);
        assert_eq!(send_msg.data[1], 0xAA);
    }
}
