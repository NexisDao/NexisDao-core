// Suppress warning here is because it is mistakenly treat the code as dead code when running unit tests.
#![allow(dead_code)]

use super::*;
use ckb_testtool::context::{Context,random_hash};
use ckb_tool::ckb_types::{
    bytes::Bytes, packed::*, prelude::*, 
};
use std::error::Error;

pub fn hex_to_bytes(input: &str) -> Result<Bytes, Box<dyn Error>> {
    let hex = input.trim_start_matches("0x");
    if hex == "" {
        Ok(Bytes::default())
    } else {
        Ok(Bytes::from(hex::decode(hex)?))
    }
}

pub fn hex_to_byte32(input: &str) -> Result<Byte32, Box<dyn Error>> {
    let hex = input.trim_start_matches("0x");
    let data = hex::decode(hex)?
        .into_iter()
        .map(Byte::new)
        .collect::<Vec<_>>();
    let mut inner = [Byte::new(0); 32];
    inner.copy_from_slice(&data);

    Ok(Byte32::new_builder().set(inner).build())
}

pub fn hex_try_to_byte32(input: &str) -> Result<Byte32, Box<dyn Error>> {
    if input.len() == 0{
        return Ok(random_hash());
    }
    let hex = input.trim_start_matches("0x");
    let data = hex::decode(hex)?
        .into_iter()
        .map(Byte::new)
        .collect::<Vec<_>>();
    let mut inner = [Byte::new(0); 32];
    inner.copy_from_slice(&data);

    Ok(Byte32::new_builder().set(inner).build())
}

pub fn hex_to_u64(input: &str) -> Result<u64, Box<dyn Error>> {
    let hex = input.trim_start_matches("0x");
    if hex == "" {
        Ok(0u64)
    } else {
        Ok(u64::from_str_radix(hex, 16)?)
    }
}

pub fn u32_to_uint32(val:u32)->Uint32{
    let buf = val.to_le_bytes();
    match Uint32::from_slice(&buf[..]){
        Ok(v)=>v,
        _=>Uint32::default()
    }
}

pub fn u64_to_uint64(val:u64)->Uint64{
    let buf = val.to_le_bytes();
    match Uint64::from_slice(&buf[..]){
        Ok(v)=>v,
        _=>Uint64::default()
    }
}

pub fn deploy_contract(context: &mut Context, binary_name: &str) -> OutPoint {
    let contract_bin: Bytes = Loader::default().load_binary(binary_name);
    context.deploy_cell(contract_bin)
}

pub fn deploy_builtin_contract(context: &mut Context, binary_name: &str) -> OutPoint {
    let contract_bin: Bytes = Loader::with_deployed_scripts().load_binary(binary_name);
    context.deploy_cell(contract_bin)
}

pub fn mock_script(context: &mut Context, out_point: OutPoint, args: Bytes) -> (Script, CellDep) {
    let script = context
        .build_script(&out_point, args)
        .expect("Build script failed, can not find cell of script.");
    let cell_dep = CellDep::new_builder().out_point(out_point).build();

    (script, cell_dep)
}

pub fn mock_cell(
    context: &mut Context,
    capacity: u64,
    lock_script: Script,
    type_script: Option<Script>,
    bytes: Option<Bytes>,
) -> OutPoint {
    let data;
    if bytes.is_some() {
        data = bytes.unwrap();
    } else {
        data = Bytes::new();
    }

    context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity.pack())
            .lock(lock_script)
            .type_(ScriptOpt::new_builder().set(type_script).build())
            .build(),
        data,
    )
}

pub fn mock_cell_with_outpoint(
    context: &mut Context,
    out_point: OutPoint,
    capacity: u64,
    lock_script: Script,
    type_script: Option<Script>,
    bytes: Option<Bytes>,
) {
    let data;
    if bytes.is_some() {
        data = bytes.unwrap();
    } else {
        data = Bytes::new();
    }

    context.create_cell_with_out_point(
        out_point.clone(),
        CellOutput::new_builder()
            .capacity(capacity.pack())
            .lock(lock_script)
            .type_(ScriptOpt::new_builder().set(type_script).build())
            .build(),
        data,
    )
}

pub fn mock_input(out_point: OutPoint, since: Option<u64>) -> CellInput {
    let mut builder = CellInput::new_builder().previous_output(out_point);

    if let Some(data) = since {
        builder = builder.since(data.pack());
    }

    builder.build()
}

pub fn mock_output(capacity: u64, lock_script: Script, type_script: Option<Script>) -> CellOutput {
    CellOutput::new_builder()
        .capacity(capacity.pack())
        .lock(lock_script)
        .type_(ScriptOpt::new_builder().set(type_script).build())
        .build()
}

pub fn mock_dep(out_point: OutPoint) -> CellDep {
    let builder = CellDep::new_builder().out_point(out_point);
    builder.build()
}
