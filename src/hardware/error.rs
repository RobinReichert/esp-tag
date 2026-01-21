use core::fmt;

pub enum DisplayError {
    DrawError,
    FlushError,
    InitError,
    ClearError,
}

impl fmt::Display for DisplayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self { 
            Self::DrawError => write!(f, "failed to draw on display\n"),
            Self::FlushError => write!(f, "failed to flush to display\n"),
            Self::InitError => write!(f, "failed to init display\n"),
            Self::ClearError => write!(f, "failed to clear display\n"),
        }
    }
}
