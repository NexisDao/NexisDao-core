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
    high_level::{load_cell_type_hash, load_script},
};

fn check_type_exist(source: Source, args: &Bytes) -> Result<bool, Error> {
    for i in 0.. {
        match load_cell_type_hash(i, source) {
            Ok(data) => {
                if let Some(d) = data {
                    if args[..] == d[..] {
                        return Ok(true);
                    }
                }
            }
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => {
                debug!("error index data, index:{}, error:{:?}", i, err);
                return Err(err.into());
            }
        };
    }
    Ok(false)
}

pub fn main() -> Result<(), Error> {
    // remove below examples and write your code here

    let script = load_script()?;

    let args: Bytes = script.args().unpack();
    debug!("type_lock: args is {:x?}", args);

    // return an error if args is invalid
    if args.is_empty() {
        return Err(Error::ErrArgs);
    }
    let mode;
    let type_hash;
    if args.len() == 32 {
        mode = 0;
        type_hash = args;
    } else {
        mode = args[0];
        type_hash = args.slice(1..);
    }
    debug!("script,mode {}, type_hash is {:x?}", mode, type_hash);

    match mode {
        1 => match check_type_exist(Source::Input, &type_hash) {
            Ok(exist) => {
                if exist {
                    return Ok(());
                }
                return Err(Error::ErrNotExist);
            }
            Err(err) => return Err(err.into()),
        },
        2 => match check_type_exist(Source::Output, &type_hash) {
            Ok(exist) => {
                if exist {
                    return Ok(());
                }
                return Err(Error::ErrNotExist);
            }
            Err(err) => return Err(err.into()),
        },
        _ => {
            match check_type_exist(Source::Input, &type_hash) {
                Ok(exist) => {
                    if exist {
                        return Ok(());
                    }
                }
                Err(err) => return Err(err.into()),
            }

            match check_type_exist(Source::Output, &type_hash) {
                Ok(exist) => {
                    if exist {
                        return Ok(());
                    }
                    return Err(Error::ErrNotExist);
                }
                Err(err) => return Err(err.into()),
            }
        }
    }
}
