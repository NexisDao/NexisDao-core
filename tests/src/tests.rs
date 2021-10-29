//! Here is the tests for always_success contract AND template_parser
use crate::template_parser::TemplateParser;
use ckb_testtool::context::Context;
use ckb_tool::{ckb_error::assert_error_eq, ckb_script::ScriptError};
use paste::paste;
use walkdir::WalkDir;

const MAX_CYCLES: u64 = 1000_000_000;

fn run_test(id: u32, name: &str) {
    let mut context = Context::default();
    let mut parser = TemplateParser::from_file(&mut context, name.to_string()).expect(name);
    println!(
        "==========start test {} {} file:{:?}",
        id, parser.data.name, name
    );

    // parse transaction template
    parser.parse();

    // build transaction
    let tx = parser.build_tx();

    let hope = parser.data.hope_result.clone();
    // run in vm
    let cycles = context.verify_tx(&tx, MAX_CYCLES);

    match hope.error_type.as_str() {
        "" => match cycles {
            Ok(c) => println!("...test {} finish: {:?} cycles, file:{:?}", id, c, name),
            Err(e) => panic!("id:{} name:{},error:{}", id, name, e),
        },
        "input" => {
            let err = cycles.unwrap_err();
            assert_error_eq!(
                err,
                ScriptError::ValidationFailure(hope.error_number)
                    .input_type_script(hope.cell_index),
                "{}",
                name
            );
        }
        "output" => {
            let err = cycles.unwrap_err();
            assert_error_eq!(
                err,
                ScriptError::ValidationFailure(hope.error_number)
                    .output_type_script(hope.cell_index),
                "{}",
                name
            );
        }
        "lock" => {
            let err = cycles.unwrap_err();
            assert_error_eq!(
                err,
                ScriptError::ValidationFailure(hope.error_number)
                    .input_lock_script(hope.cell_index),
                "{}",
                name
            );
        }
        _ => {
            panic!("unknow error type");
        }
    }
    println!("==========finish test {} file:{:?}", id, name);
}

#[test]
fn test_all() {
    let mut i: u32 = 0;
    for entry in WalkDir::new("./templates/")
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.path().is_dir() {
            continue;
        }
        if entry.path().extension().unwrap() != "json" {
            continue;
        }
        i = i + 1;
        run_test(i, entry.path().to_str().expect("error path"));
    }
}

macro_rules! new_test {
    ($id:expr,$name:expr) => {
        paste! {
            #[test]
            fn [<test_ $id>](){
                run_test($id, $name);
            }
        }
    };
}

new_test!(1, "./templates/always_success.json");
