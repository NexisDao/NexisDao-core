// Import from `core` instead of from `std` since we are in no-std mode
use ckb_std::error::SysError;
use core::result::Result;

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use crate::error::Error;
use ckb_std::ckb_constants::Source;
use ckb_std::{
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{load_cell_data, load_input_out_point, load_script},
};

fn get_id_from_data(so: Source) -> Result<u32, Error> {
    let mut value: u32 = 0;
    let mut buf = [0u8; 4];
    for i in 0.. {
        let data = match load_cell_data(i, so) {
            Ok(data) => data,
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => {
                debug!("source:{:?}, index:{}, error:{:?}", so, i, err);
                return Err(err.into());
            }
        };
        if i > 0 {
            return Err(Error::ErrMoreOneCell);
        }
        buf.copy_from_slice(&data);
        value = u32::from_le_bytes(buf);
        debug!("source:{:?}, data:{:?}, value:{}", so, data, value);
    }
    Ok(value)
}

pub fn main() -> Result<(), Error> {
    // remove below examples and write your code here

    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    debug!("script args is {:x?}", args);

    // return an error if args is invalid
    if args.is_empty() {
        return Err(Error::MyError);
    }

    let in_id = match get_id_from_data(Source::GroupInput) {
        Ok(id) => id,
        Err(err) => return Err(err.into()),
    };

    let out_id = match get_id_from_data(Source::GroupOutput) {
        Ok(id) => id,
        Err(err) => return Err(err.into()),
    };

    if out_id == 0 {
        return Err(Error::ErrOutputData);
    }
    debug!("in:{:?}, out:{:?}", in_id, out_id);
    if in_id + 1 != out_id {
        return Err(Error::ErrIndex);
    }
    if in_id > 0 {
        return Ok(());
    }
    if out_id != 1 {
        debug!("error data, input:{:?}, output:{:?}", in_id, out_id);
        return Err(Error::ErrOutputData);
    }

    let data = match load_input_out_point(0, Source::Input) {
        Ok(op) => op.as_bytes(),
        Err(err) => {
            debug!("fail to load_input_out_point.{:?}", err);
            return Err(err.into());
        }
    };
    if args[..] == data[..] {
        return Ok(());
    }

    debug!("error args:{:x?}, outpoint:{:x?}", args, data);

    return Err(Error::ErrArgs);
}
