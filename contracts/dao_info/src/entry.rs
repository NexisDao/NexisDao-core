use crate::error::Error;
use alloc::vec::Vec;
use ckb_std::error::SysError;
use ckb_std::{
    ckb_constants::Source,
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{
        load_cell_capacity, load_cell_data, load_cell_lock_hash, load_cell_type_hash,
        load_input_out_point, load_script,
    },
};

use config::{config::ID, def_get_config_data, require};
use core::result::Result;
def_get_config_data!();
const SCRIPT_LEN: usize = 32;
const UDT_LEN: usize = 16;
const DAO_DATA_LEN: usize = 8;

fn get_udt_amount(so: Source, udt: Vec<u8>) -> Result<u128, Error> {
    let mut value: u128 = 0;
    let mut buf = [0u8; UDT_LEN];
    debug!("get_udt_amount, udt:{:x?}", udt);
    for i in 0.. {
        let type_hash = match load_cell_type_hash(i, so) {
            Ok(data) => {
                if let Some(d) = data {
                    d
                } else {
                    [0u8; 32]
                }
            }
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => {
                debug!("error index type, index:{}, error:{:?}", i, err);
                return Err(err.into());
            }
        };
        if &udt[..] != &type_hash[..] {
            debug!("index:{}, hash:{:x?}", i, type_hash);
            continue;
        }
        let data = match load_cell_data(i, so) {
            Ok(data) => data,
            Err(err) => {
                debug!("error index data, index:{}, error:{:?}", i, err);
                return Err(err.into());
            }
        };
        debug!("index:{}, data:{:x?}", i, data);
        buf.copy_from_slice(&data[..UDT_LEN]);
        value += u128::from_le_bytes(buf);
        debug!("index:{}, data:{:?}, value:{}", i, data, value);
    }
    Ok(value)
}

fn deposit(args: &Bytes, data: Vec<u8>) -> Result<(), Error> {
    // In output, only 1
    load_cell_data(1, Source::GroupOutput).expect_err("more than one DAO Info(output)");
    load_cell_data(0, Source::GroupInput).expect_err("exist DAO Info(input)");

    let dao_type = get_config_data(args.to_vec(), ID::NervosDAOType as u32)?;
    require!(dao_type.len() == SCRIPT_LEN);
    let mut cfg_script = [0u8; SCRIPT_LEN];
    cfg_script.copy_from_slice(&dao_type[..]);
    let dao_udt = get_config_data(args.to_vec(), ID::DAOUDT as u32)?;

    let input_amount = get_udt_amount(Source::Input, dao_udt.clone())?;
    let output_amount = get_udt_amount(Source::Output, dao_udt.clone())?;
    require!(output_amount > input_amount);

    let cap = load_cell_capacity(0, Source::Output)?;
    require!(output_amount-input_amount == cap as u128);
    
    let mut buf = [0u8; UDT_LEN];
    buf.copy_from_slice(&data[..UDT_LEN]);
    let value = u128::from_le_bytes(buf);
    require!(output_amount-input_amount == value);

    let output_type = load_cell_type_hash(0, Source::Output)?;
    require!(Some(cfg_script) == output_type);

    //the data of Nervos DAO = u64(0)
    let output_data = load_cell_data(0, Source::Output)?;
    let mut buf1 = [0u8; DAO_DATA_LEN];
    buf1.copy_from_slice(&output_data[..DAO_DATA_LEN]);
    let dao_data = u64::from_le_bytes(buf1);
    require!(dao_data==0);

    let cfg_lock = get_config_data(args.to_vec(), ID::NervosDAOLock as u32)?;
    let lock = load_cell_lock_hash(0, Source::Output)?;
    require!(&lock[..] == &cfg_lock[..]);

    Ok(())
}

// Withdraw Phase 1, burn dCKB
fn withdrawal(args: &Bytes) -> Result<(), Error> {
    // 1. In input, only 1
    load_cell_data(1, Source::GroupInput).expect_err("more than one DAO Info(input)");
    load_cell_data(0, Source::GroupOutput).expect_err("exist DAO Info(output)");

    let data = load_cell_data(0, Source::GroupInput)?;

    let dao_type = get_config_data(args.to_vec(), ID::NervosDAOType as u32)?;
    require!(dao_type.len() == SCRIPT_LEN);
    let mut cfg_script = [0u8; SCRIPT_LEN];
    cfg_script.copy_from_slice(&dao_type[..]);
    let dao_udt = get_config_data(args.to_vec(), ID::DAOUDT as u32)?;

    let input_amount = get_udt_amount(Source::Input, dao_udt.clone())?;
    let output_amount = get_udt_amount(Source::Output, dao_udt.clone())?;
    require!(input_amount > output_amount);

    let cap = load_cell_capacity(0, Source::Input)?;
    require!(input_amount - output_amount == cap as u128);

    let mut buf = [0u8; UDT_LEN];
    buf.copy_from_slice(&data[..UDT_LEN]);
    let value = u128::from_le_bytes(buf);
    require!(input_amount - output_amount == value);
    
    //Nervos DAO Withdraw Phase 1
    let input_type = load_cell_type_hash(0, Source::Input)?;
    require!(Some(cfg_script) == input_type);
    let output_type = load_cell_type_hash(0, Source::Output)?;
    require!(Some(cfg_script) == output_type);

    // same tx
    let outpoint1 = load_input_out_point(0, Source::Input).unwrap();
    let outpoint2 = load_input_out_point(0, Source::GroupInput).unwrap();
    let tx1 = outpoint1.tx_hash().unpack();
    let tx2 = outpoint2.tx_hash().unpack();
    require!(tx1 == tx2);

    Ok(())
}

pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    debug!("script args is {:x?}", args);

    // return an error if args is invalid
    require!(!args.is_empty());

    match load_cell_data(0, Source::GroupOutput) {
        Ok(data) => {
            return deposit(&args, data);
        }
        _ => {
            return withdrawal(&args);
        }
    }
}
