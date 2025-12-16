use core::fmt;

#[derive(Debug)]
pub enum TreeError {
    AddressNotFound,
    MemoryFull,
}

impl fmt::Display for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TreeError::AddressNotFound => write!(f, "Failed to find Address"),
            TreeError::MemoryFull=> write!(f, "To much Nodes in the Arena"),
        }
    }
}

#[derive(Debug)]
pub enum ArenaError {
    NodeNotFound,
    InvalidIndex,
}

impl fmt::Display for ArenaError{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArenaError::NodeNotFound => write!(f, "Failed to find Node"),
            ArenaError::InvalidIndex=> write!(f, "Index is not used"),
        }
    }
}


#[derive(Debug)]
pub enum MeshError {
    NodeNotFound,
    InvalidIndex,
}

impl fmt::Display for MeshError{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MeshError::NodeNotFound => write!(f, "Failed to find Node"),
            MeshError::InvalidIndex=> write!(f, "Index is not used"),
        }
    }
}
