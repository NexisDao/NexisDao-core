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
    high_level::{load_cell_data, load_cell_type_hash, load_script},
};

fn get_id_from_data(so: Source, mul: bool) -> Result<u32, Error> {
    let mut value: u32 = 0;
    let mut buf = [0u8; 4];
    let mut find: bool = false;
    for i in 0.. {
        let data = match load_cell_data(i, so) {
            Ok(data) => data,
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => {
                debug!("source:{:?}, index:{}, error:{:?}", so, i, err);
                return Err(err.into());
            }
        };
        buf.copy_from_slice(&data[..4]);
        let lv = u32::from_le_bytes(buf);
        if lv == 0 {
            return Err(Error::ErrZero);
        }
        if find {
            if !mul {
                return Err(Error::ErrMoreOneCell);
            }
            if lv != value {
                return Err(Error::ErrDifferenceData);
            }
        }
        find = true;
        value = lv;
        debug!("source:{:?}, data:{:?}, value:{}", so, data, value);
    }
    Ok(value)
}

fn get_id_from_index_script(args: Bytes) -> Result<u32, Error> {
    let mut value: u32 = 0;
    let mut buf = [0u8; 4];
    let mut find: bool = false;
    for i in 0.. {
        let type_hash = match load_cell_type_hash(i, Source::Input) {
            Ok(data) => {
                if let Some(d) = data {
                    d
                } else {
                    [0u8; 32]
                }
            }
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => {
                debug!("error index data, index:{}, error:{:?}", i, err);
                return Err(err.into());
            }
        };
        if args[..] != type_hash[..] {
            debug!("index:{}, hash:{:x?}", i, type_hash);
            continue;
        }
        let data = match load_cell_data(i, Source::Input) {
            Ok(data) => data,
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => {
                debug!("error index data, index:{}, error:{:?}", i, err);
                return Err(err.into());
            }
        };
        if find {
            return Err(Error::ErrMoreOneCell);
        }
        find = true;
        buf.copy_from_slice(&data[..4]);
        value = u32::from_le_bytes(buf);
        debug!("index:{}, data:{:?}, value:{}", i, data, value);
    }
    Ok(value)
}

pub fn main() -> Result<(), Error> {
    let in_id = match get_id_from_data(Source::GroupInput, false) {
        Ok(id) => id,
        Err(err) => return Err(err.into()),
    };

    let out_id = match get_id_from_data(Source::GroupOutput, in_id == 0) {
        Ok(id) => id,
        Err(err) => return Err(err.into()),
    };

    if out_id == 0 {
        return Err(Error::ErrOutputData);
    }
    debug!("in:{:?}, out:{:?}", in_id, out_id);

    if in_id > 0 && in_id == out_id {
        return Ok(());
    }
    if in_id > 0 {
        return Err(Error::ErrIndex);
    }
    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    debug!("script args is {:x?}", args);

    // return an error if args is invalid
    if args.is_empty() {
        return Err(Error::MyError);
    }

    let index_id = match get_id_from_index_script(args) {
        Ok(id) => id,
        Err(err) => return Err(err.into()),
    };

    if index_id == out_id {
        return Ok(());
    }

    debug!("error index:{:?}, hope:{:?}", index_id, out_id);

    return Err(Error::ErrArgs);
}
