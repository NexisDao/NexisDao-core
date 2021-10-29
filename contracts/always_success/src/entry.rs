// Import from `core` instead of from `std` since we are in no-std mode
use core::result::Result;
use crate::error::Error;

pub fn main() -> Result<(), Error> {
    Ok(())
}

