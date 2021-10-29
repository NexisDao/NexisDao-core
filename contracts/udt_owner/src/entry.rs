use crate::error::Error;
use alloc::vec::Vec;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{load_cell_lock_hash, load_cell_type_hash, load_script},
};

use config::{def_get_config_data, require};
use core::result::Result;
def_get_config_data!();

const SCRIPT_LEN: usize = 32;

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    debug!("script args is {:x?}", args);

    // return an error if args is invalid
    require!(!args.is_empty());

    let mut owner = [0u8; SCRIPT_LEN];
    if args.len() == 32 {
        owner.copy_from_slice(&args[..SCRIPT_LEN]);
    } else {
        let config = &args[..SCRIPT_LEN];
        let mut buf = [0u8; 4];
        buf.copy_from_slice(&args[SCRIPT_LEN..]);
        let id = u32::from_le_bytes(buf);
        let app_type = get_config_data(config.to_vec(), id)?;
        owner.copy_from_slice(&app_type[..SCRIPT_LEN]);
    }

    load_cell_lock_hash(1, Source::GroupInput).expect_err("more than one lock");
    for i in 0.. {
        match load_cell_type_hash(i, Source::Input) {
            Ok(data) => {
                if Some(owner) == data {
                    return Ok(());
                }
            }
            Err(_) => {
                break;
            }
        };
    }
    for i in 0.. {
        match load_cell_type_hash(i, Source::Output) {
            Ok(data) => {
                if Some(owner) == data {
                    return Ok(());
                }
            }
            Err(err) => {
                return Err(err.into());
            }
        };
    }
    require!(false);
    Ok(())
}
