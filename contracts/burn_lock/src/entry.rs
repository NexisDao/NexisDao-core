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
    high_level::{load_cell_data,load_cell_lock_hash, load_cell_type_hash, load_script},
};

// u128 is 16 bytes
const UDT_LEN: usize = 16;
const SCRIPT_LEN: usize = 32;

pub fn main() -> Result<(), Error> {
    // remove below examples and write your code here

    let script = load_script()?;

    let args: Bytes = script.args().unpack();
    debug!("type_lock: args is {:x?}", args);

    // return an error if args is invalid
    if args.is_empty() {
        return Err(Error::ErrArgs);
    }
    let mut udt_hash = [0u8; SCRIPT_LEN];
    udt_hash.copy_from_slice(&args.to_vec()[..]);

    let mut input_amount:u128=0;
    let mut output_amount:u128=0; 
    for i in 0.. {
        match load_cell_type_hash(i, Source::GroupInput) {
            Ok(data) => {
                assert_eq!(data,Some(udt_hash));
                let data = load_cell_data(i,Source::GroupInput)?;
                let mut buf = [0u8; UDT_LEN];
                buf.copy_from_slice(&data[..UDT_LEN]);
                input_amount += u128::from_le_bytes(buf);
            }
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => {
                debug!("error index type, index:{}, error:{:?}", i, err);
                return Err(err.into());
            }
        };
    }
    let lock = load_cell_lock_hash(0,Source::GroupInput)?;
    for i in 0.. {
        match load_cell_lock_hash(i, Source::Output) {
            Ok(data) => {
                if data != lock{
                    continue;
                }
                let typ = load_cell_type_hash(i,Source::Output)?;
                assert_eq!(typ,Some(udt_hash));
                let data = load_cell_data(i,Source::Output)?;
                let mut buf = [0u8; UDT_LEN];
                buf.copy_from_slice(&data[..UDT_LEN]);
                output_amount += u128::from_le_bytes(buf);
            }
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => {
                debug!("error index type, index:{}, error:{:?}", i, err);
                return Err(err.into());
            }
        };
    }
    if input_amount >= output_amount {
        return Err(Error::ErrAmount);
    }
    return Ok(());
}
