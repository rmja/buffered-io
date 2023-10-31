mod read;
mod write;

pub use read::BufferedRead;
pub use write::BufferedWrite;

/// Unable to bypass the current buffered reader or writer because there are buffered bytes.
#[derive(Debug)]
pub struct BypassError;
