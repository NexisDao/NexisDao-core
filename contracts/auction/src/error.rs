use ckb_std::error::SysError;

// /// Error
// #[repr(u32)]
// pub enum Error {
//     IndexOutOfBound = 1,
//     ItemMissing,
//     LengthNotEnough,
//     Encoding,
//     // Add customized errors here...
//     ErrArgs,
//     ErrUDTAmount,
// }

pub struct Error {
    pub err: u32,
}

impl From<SysError> for Error {
    fn from(err: SysError) -> Self {
        use SysError::*;
        match err {
            IndexOutOfBound => Error{err:1},
            ItemMissing => Error{err:2},
            LengthNotEnough(_) => Error{err:3},
            Encoding => Error{err:4},
            Unknown(err_code) => panic!("unexpected sys error {}", err_code),
        }
    }
}

impl From<u32> for Error {
    fn from(err: u32) -> Self {Error{err}}
}
