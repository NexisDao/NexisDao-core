// Import from `core` instead of from `std` since we are in no-std mode
use ckb_std::error::SysError;
use core::{result::Result, u64};

// Import CKB syscalls and structures
// https://nervosnetwork.github.io/ckb-std/riscv64imac-unknown-none-elf/doc/ckb_std/index.html
use crate::error::Error;
use alloc::vec::Vec;
use ckb_std::{ckb_constants::Source, high_level::load_cell_type_hash};
use ckb_std::{
    ckb_types::{bytes::Bytes, prelude::*},
    debug,
    high_level::{load_cell_data, load_cell_lock_hash, load_input_out_point, load_script},
};
use config::{config::ID, def_get_cell_time, def_get_config_data, require};
def_get_config_data!();
def_get_cell_time!();

// u128 is 16 bytes
const UDT_LEN: usize = 16;
const SCRIPT_LEN: usize = 32;
const OWNER_LEN: usize = 32;
const UDT_ID_LEN: usize = 4;
const INTEREST_LEN: usize = 4;
const TIME_LEN: usize = 8;

// id: udt.hash, rate, interest
fn get_udt_and_rate(conf: &Bytes, id: u32) -> Result<([u8; 32], f32, f32), Error> {
    require!(id % 2 == 0);
    let udt_data = get_config_data(conf.to_vec(), id)?;
    let mut udt_hash_buf = [0u8; 32];
    let rate_end = SCRIPT_LEN + INTEREST_LEN;
    udt_hash_buf.copy_from_slice(&udt_data[..SCRIPT_LEN]);
    let udt_rate_buf = &udt_data[SCRIPT_LEN..rate_end];
    let udt_interest_buf = &udt_data[rate_end..];
    let mut buf_id = [0u8; INTEREST_LEN];
    buf_id.copy_from_slice(udt_rate_buf);
    let rate = f32::from_le_bytes(buf_id);
    require!(rate < 1.0);
    buf_id.copy_from_slice(udt_interest_buf);
    let interest = f32::from_le_bytes(buf_id);

    Ok((udt_hash_buf, interest, rate))
}

// id: deposit, withdrawal
fn get_incentive_param(conf: &Bytes) -> Result<(f32, f32), Error> {
    let conf_data = get_config_data(conf.to_vec(), ID::IncentiveParam as u32)?;
    let deposit_incentive_buf = &conf_data[..INTEREST_LEN];
    let withdrawal_incentive_buf = &conf_data[INTEREST_LEN..];
    let mut buf_id = [0u8; INTEREST_LEN];
    buf_id.copy_from_slice(deposit_incentive_buf);
    let deposit_incentive = f32::from_le_bytes(buf_id);
    buf_id.copy_from_slice(withdrawal_incentive_buf);
    let withdrawal_incentive = f32::from_le_bytes(buf_id);

    Ok((deposit_incentive, withdrawal_incentive))
}

fn get_udt_price(conf: &Bytes, id: u32) -> Result<(f32, u64), Error> {
    let mut buf_id = [0u8; UDT_ID_LEN];
    let mut buf_time = [0u8; TIME_LEN];
    let udt_price_buf = get_config_data(conf.to_vec(), id)?;
    require!(udt_price_buf.len() >= UDT_ID_LEN + TIME_LEN);
    buf_id.copy_from_slice(&udt_price_buf[..UDT_ID_LEN]);
    buf_time.copy_from_slice(&udt_price_buf[UDT_ID_LEN..]);
    let price = f32::from_le_bytes(buf_id);
    let time = u64::from_le_bytes(buf_time);
    Ok((price, time))
}
/*
parse data to UDT.amount, TAI.amount, UDT.hash, owner.hash, time
    1.TAI.amount, u128, 16bit
    2.UDT.amount, u128, 16bit
    3.UDT config id, u32, 4bit
    4.owner(input[0].lock_hash): 32bit
    5.interest: 4bit
    6.time u64, 8bit
*/
fn parse_data(data: Vec<u8>) -> Result<(u128, u128, u32, Vec<u8>, f32, u64), Error> {
    require!(data.len() >= (UDT_LEN + UDT_LEN + UDT_ID_LEN + OWNER_LEN + INTEREST_LEN));

    let mut start: usize = 0;
    let mut end = UDT_LEN;
    let tai_amount_buf = &data[start..end];
    start = end;
    end = start + UDT_LEN;
    let udt_amount_buf = &data[start..end];
    start = end;
    end = start + UDT_ID_LEN;
    let udt_id_buf = &data[start..end];
    start = end;
    end = start + OWNER_LEN;
    let owner_buf = &data[start..end];
    start = end;
    end = start + INTEREST_LEN;
    let interest_buf = &data[start..end];
    start = end;
    let time_buf = &data[start..];

    let mut buf_id = [0u8; UDT_ID_LEN];
    buf_id.copy_from_slice(udt_id_buf);
    let id = u32::from_le_bytes(buf_id);

    let mut buf = [0u8; UDT_LEN];
    buf.copy_from_slice(udt_amount_buf);
    let udt_amount = u128::from_le_bytes(buf);

    buf.copy_from_slice(tai_amount_buf);
    let tai_amount = u128::from_le_bytes(buf);
    let owner_hash = owner_buf.to_vec();

    let mut buf = [0u8; INTEREST_LEN];
    buf.copy_from_slice(interest_buf);
    let interest = f32::from_le_bytes(buf);

    let mut buf = [0u8; TIME_LEN];
    buf.copy_from_slice(time_buf);
    let time = u64::from_le_bytes(buf);

    Ok((udt_amount, tai_amount, id, owner_hash, interest, time))
}

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

/*
deposit(New CDP):
    1. In output, the number is only 1
    2. Read the lock hash from the configuration,
    3. Check output[0].lock
    4. Check output[1].lock=output[0].lock
    5. Check output[0].type and amount
    6. Read UDT information from the configuration,
        a. price, rate
    7. Check TAI amount
*/
fn deposit(args: &Bytes, data: Vec<u8>) -> Result<(), Error> {
    // In output, the number is only 1
    load_cell_data(1, Source::GroupOutput).expect_err("more than one CPD(output)");
    load_cell_data(0, Source::GroupInput).expect_err("exist CDP(input)");

    let lock = get_config_data(args.to_vec(), ID::UDTLock as u32)?;
    let self_lock = load_cell_lock_hash(0, Source::GroupOutput)?;
    require!(&lock[..] == &self_lock[..]);
    let udt_lock = load_cell_lock_hash(0, Source::Output)?;
    require!(&lock[..] == &udt_lock[..]);

    let (udt_amount, tai_amount, id, _owner, user_interest, time1) = parse_data(data)?;

    let (udt_hash, rate, interest) = get_udt_and_rate(args, id)?;
    let (incentive, _) = get_incentive_param(args)?;
    require!(user_interest >= interest * incentive);
    let (price, time2) = get_udt_price(args, id + 1)?;

    require!(time1 == 0 || time1 == time2);

    let udt_type = load_cell_type_hash(0, Source::Output)?;
    require!(Some(udt_hash) == udt_type);

    let udt_data = load_cell_data(0, Source::Output)?;
    let mut buf = [0u8; UDT_LEN];
    buf.copy_from_slice(&udt_data[..UDT_LEN]);
    let v = u128::from_le_bytes(buf);
    require!(v == udt_amount);
    require!(udt_amount > 100);

    let limit = udt_amount as f64 * price as f64 * rate as f64;
    debug!(
        "price:{:?},rate:{:?},limit:{:?},hope:{:?}",
        price, rate, limit, tai_amount as f64
    );
    require!(limit >= tai_amount as f64);

    // output.TAI.amount - input.TAI.amount = CDP.TAI
    let tai_hash = get_config_data(args.to_vec(), ID::TAIType as u32)?;
    let input_amount = get_udt_amount(Source::Input, tai_hash.clone())?;
    let output_amount = get_udt_amount(Source::Output, tai_hash.clone())?;
    require!(input_amount < output_amount);
    require!(output_amount - input_amount == tai_amount);

    Ok(())
}

/*
withdrawal:
  1. In Input, the number is only 1
  2. Check input[0].lock==input[1].lock==UDTLock, the number is only 2
  3. Check input[0].type is UDT;input[0].UDT.amount=CPD.udt.amount
  4. Check output[0].lock=CPD.owner，output[0].type=UDT;output[0].amount=CPD.udt.amount
  5. Check input.TAI.amount-output.TAI.amount=CDP.TAI
  6. Check output[1].lock=CommunityLock, TAI.amount(fee) is enough
*/
fn withdrawal(args: &Bytes) -> Result<(), Error> {
    // 1. In Input, the number is only 1
    load_cell_data(1, Source::GroupInput).expect_err("more than one CPD(input)");
    load_cell_data(0, Source::GroupOutput).expect_err("exist CDP(output)");

    // 2. Check input[0].lock==input[1].lock==UDTLock, the number is only 2
    let lock = get_config_data(args.to_vec(), ID::UDTLock as u32)?;
    for i in 0.. {
        let lock_hash = match load_cell_lock_hash(i, Source::Input) {
            Ok(data) => data,
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => {
                debug!("error index data, index:{}, error:{:?}", i, err);
                return Err(err.into());
            }
        };
        if i == 0 || i == 1 {
            require!(lock[..] == lock_hash[..]);
        } else {
            require!(lock[..] != lock_hash[..]);
        }
    }
    let my_lock = load_cell_lock_hash(0, Source::GroupInput)?;
    require!(&my_lock[..] == &lock[..]);

    // same tx
    let outpoint1 = load_input_out_point(0, Source::Input).unwrap();
    let outpoint2 = load_input_out_point(1, Source::Input).unwrap();
    let tx1 = outpoint1.tx_hash().unpack();
    let tx2 = outpoint2.tx_hash().unpack();
    require!(tx1 == tx2);

    // 3. Check input[0]为UDT;input[0].UDT.amount=CPD.udt.amount
    let data = load_cell_data(0, Source::GroupInput)?;
    let (udt_amount, tai_amount, id, owner, user_interest, time1) = parse_data(data)?;
    let (udt_hash, _rate, _) = get_udt_and_rate(args, id)?;
    let udt_type = load_cell_type_hash(0, Source::Input)?;
    require!(udt_type == Some(udt_hash));
    let (_, incentive) = get_incentive_param(args)?;
    let interest = user_interest * incentive;

    let mut buf = [0u8; UDT_LEN];
    let udt_data = load_cell_data(0, Source::Input)?;
    buf.copy_from_slice(&udt_data[..UDT_LEN]);
    let v = u128::from_le_bytes(buf);
    require!(v == udt_amount);

    // 4. Check output[0].lock=CPD.owner，output[0].type=UDT;output[0].amount=CPD.udt.amount
    // if input[2].lock != owner, check output[0](lock/type/amount)
    let input_lock = load_cell_lock_hash(2, Source::Input)?;
    if &input_lock[..] != &owner[..] {
        let udt_lock = load_cell_lock_hash(0, Source::Output)?;
        require!(&udt_lock[..] == &owner[..]);

        let udt_type = load_cell_type_hash(0, Source::Output)?;
        require!(udt_type == Some(udt_hash));

        let udt_data = load_cell_data(0, Source::Output)?;
        buf.copy_from_slice(&udt_data[..UDT_LEN]);
        let v = u128::from_le_bytes(buf);
        require!(v == udt_amount);
    }

    // 5. Check input.TAI.amount-output.TAI.amount=CDP.TAI
    let tai_hash = get_config_data(args.to_vec(), ID::TAIType as u32)?;
    let input_amount = get_udt_amount(Source::Input, tai_hash.clone())?;
    let output_amount = get_udt_amount(Source::Output, tai_hash.clone())?;
    require!(input_amount > output_amount);
    require!(input_amount - output_amount == tai_amount);

    // 6. Check output[1].lock=CommunityLock, TAI.amount(fee) is enough
    let type_hash = load_cell_type_hash(1, Source::Output)?.unwrap();
    require!(&type_hash[..] == &tai_hash[..]);

    let community_lock = get_config_data(args.to_vec(), ID::CommunityLock as u32)?;
    let tai_lock = load_cell_lock_hash(1, Source::Output)?;
    require!(&community_lock[..] == &tai_lock[..]);

    let mut input_fee: u128 = 0;
    for i in 0.. {
        match load_cell_lock_hash(i, Source::Input) {
            Ok(lock) => {
                if &lock[..] != &community_lock[..]{
                    continue;
                }
                let t = load_cell_type_hash(i,Source::Input)?.unwrap();
                require!(&t[..] == &tai_hash[..]);
                let udt_data = load_cell_data(i, Source::Input)?;
                buf.copy_from_slice(&udt_data[..UDT_LEN]);
                input_fee = input_fee + u128::from_le_bytes(buf);
            },
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => {
                debug!("error index data, index:{}, error:{:?}", i, err);
                return Err(err.into());
            }
        };
    }

    // fee
    let udt_data = load_cell_data(1, Source::Output)?;
    buf.copy_from_slice(&udt_data[..UDT_LEN]);
    let fee = u128::from_le_bytes(buf) - input_fee;
    require!(fee > 0);
    let start: u64;
    if time1 != 0 {
        start = time1;
    } else {
        start = get_cell_time(Source::Input, 1)?;
    }
    // get time
    let (_, end) = get_udt_price(args, id + 1)?;
    debug!("block time,start{:?}, end:{}, fee:{}", start, end, fee);

    let time: u64;
    if end > start {
        time = (end - start) / 1000 / 3600 + 24;
    } else {
        time = 24;
    }
    let mut limit = tai_amount as f64 * interest as f64 * time as f64;
    limit = limit / 365.0 / 24.0;
    debug!(
        "tai_amount:{:?}, time:{:?}, interest:{}, limit:{}, fee:{}",
        tai_amount, time, interest, limit, fee
    );
    require!(fee as f64 > limit);

    Ok(())
}

/*
liquidation：
  1. In Input, the number is only 1
  2. Check input[0].lock==input[1].lock==UDTLock, the number is only 2
  3. Check input[0].type is UDT;input[0].UDT.amount=CPD.udt.amount
  4. Check output[0].lock=output[1].lock=debt_lock
  5. output[1].data=CPD.data, output[1].type = debt
  6. output[0].amount>=CPD.udt.amount*0.95, output[0].type = UDT
  7. UDT*price*rate < TAI
*/
fn liquidation(args: &Bytes) -> Result<(), Error> {
    // 1. In Input, the number is only 1
    load_cell_data(1, Source::GroupInput).expect_err("more than one CPD(input)");
    load_cell_data(0, Source::GroupOutput).expect_err("exist CDP(output)");

    // 2. Check input[0].lock==input[1].lock==UDTLock, the number is only 2
    let lock = get_config_data(args.to_vec(), ID::UDTLock as u32)?;
    for i in 0.. {
        let lock_hash = match load_cell_lock_hash(i, Source::Input) {
            Ok(data) => data,
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => {
                debug!("error index data, index:{}, error:{:?}", i, err);
                return Err(err.into());
            }
        };
        if i == 0 || i == 1 {
            require!(lock[..] == lock_hash[..]);
        } else {
            require!(lock[..] != lock_hash[..]);
        }
    }
    let my_lock = load_cell_lock_hash(0, Source::GroupInput)?;
    require!(&my_lock[..] == &lock[..]);

    // same tx
    let outpoint1 = load_input_out_point(0, Source::Input).unwrap();
    let outpoint2 = load_input_out_point(1, Source::Input).unwrap();
    let tx1 = outpoint1.tx_hash().unpack();
    let tx2 = outpoint2.tx_hash().unpack();
    require!(tx1 == tx2);

    // 3. Check input[0].type is UDT;input[0].UDT.amount=CPD.udt.amount
    let cdp_data = load_cell_data(0, Source::GroupInput)?;
    let (udt_amount, tai_amount, id, owner, interest, time1) = parse_data(cdp_data.clone())?;
    let (udt_hash, rate, _) = get_udt_and_rate(args, id)?;
    let udt_type = load_cell_type_hash(0, Source::Input)?;
    require!(udt_type == Some(udt_hash));

    let mut buf = [0u8; UDT_LEN];
    let udt_data = load_cell_data(0, Source::Input)?;
    buf.copy_from_slice(&udt_data[..UDT_LEN]);
    let v = u128::from_le_bytes(buf);
    require!(v == udt_amount);

    // 4. Check output[0].lock=output[1].lock=DebtLock
    let lock = get_config_data(args.to_vec(), ID::DebtLock as u32)?;
    for i in 0.. {
        let lock_hash = match load_cell_lock_hash(i, Source::Output) {
            Ok(data) => data,
            Err(SysError::IndexOutOfBound) => break,
            Err(err) => {
                debug!("error index data, index:{}, error:{:?}", i, err);
                return Err(err.into());
            }
        };
        if i == 0 || i == 1 {
            require!(lock[..] == lock_hash[..]);
        } else {
            require!(lock[..] != lock_hash[..]);
        }
    }

    // 5. output[1].data=CPD.data, output[1].type = debt
    let debt_data = load_cell_data(1, Source::Output)?;
    let (udt_amount2, tai_amount2, id2, owner2, _, _) = parse_data(debt_data.clone())?;
    require!(udt_amount2 == udt_amount * 95 / 100);

    require!(tai_amount2 == tai_amount);
    require!(id2 == id);
    require!(owner2 == owner);
    let debt_hash = get_config_data(args.to_vec(), ID::DebtType as u32)?;
    let debt_type = load_cell_type_hash(1, Source::Output)?.unwrap();
    require!(&debt_type[..] == &debt_hash[..]);

    // 6. output[0].amount>=CPD.udt.amount*0.95, output[0].type = UDT
    let udt_type = load_cell_type_hash(0, Source::Output)?;
    require!(udt_type == Some(udt_hash));

    let udt_data = load_cell_data(0, Source::Output)?;
    buf.copy_from_slice(&udt_data[..UDT_LEN]);
    let v = u128::from_le_bytes(buf);
    require!(v == udt_amount2);

    // 7. The value of UDT < TAI
    let (price, end) = get_udt_price(args, id + 1)?;
    let limit = udt_amount as f64 * price as f64 * rate as f64;
    let start: u64;
    if time1 != 0 {
        start = time1;
    } else {
        start = get_cell_time(Source::Input, 1)?;
    }

    let mut time: u64 = 24;
    if end > start {
        time += (end - start) / 1000 / 3600;
    }
    let mut fee = tai_amount as f64 * interest as f64 * time as f64;
    fee = fee / 365.0 / 24.0;

    require!(limit <= fee + tai_amount as f64);

    let tai_hash = get_config_data(args.to_vec(), ID::TAIType as u32)?;
    let output_amount = get_udt_amount(Source::Output, tai_hash.clone())?;
    require!(output_amount == 0);

    Ok(())
}

/*
args: config script hash
data:
    1.TAI.amount, u128, 16bit
    2.UDT.amount, u128, 16bit
    3.UDT config id, u32, 4bit
    4.owner(input[0].lock_hash)
*/
pub fn main() -> Result<(), Error> {
    let script = load_script()?;
    let args: Bytes = script.args().unpack();
    debug!("script args is {:x?}", args);

    // return an error if args is invalid
    require!(!args.is_empty());

    //deposit
    match load_cell_data(0, Source::GroupOutput) {
        Ok(data) => {
            return deposit(&args, data);
        }
        _ => {
            let debt_lock = get_config_data(args.to_vec(), ID::DebtLock as u32)?;
            let tai_lock = load_cell_lock_hash(0, Source::Output)?;
            // withdrawal: output[0].lock = user.lock
            if &debt_lock[..] != &tai_lock[..] {
                return withdrawal(&args);
            }
            // liquidation: output[0].lock = debt_lock
            return liquidation(&args);
        }
    }
}
