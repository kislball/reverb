use env_logger::Env;
use std::collections::HashMap;

use super::*;
const TEST_DATA: &[u8] = include_bytes!("../test_contract.wasm");

#[test]
fn run_contract() {
    env_logger::init_from_env(Env::new().default_filter_or("rvb_contract=trace"));
    let mut contract = WasmtimeContractCompiler.create_contract(TEST_DATA).unwrap();
    let ctx = ContractContext {
        action: DataAction::Insert {
            incoming_data: rvb_common::schema::DbValue::Number(45),
            key: String::from("vadim"),
            params: HashMap::new(),
        },
        namespace: "test".into(),
        contract_space: "contract".into(),
        signed_by: vec![1, 2, 3],
        contract_params: HashMap::new(),
    };
    let actions = contract.execute(ctx.clone()).unwrap();
    let actions2 = contract.execute(ctx.clone()).unwrap();
    let actions3 = contract.execute(ctx.clone()).unwrap();

    assert_eq!(actions, actions2);
    assert_eq!(actions2, actions3);
    assert_eq!(
        actions,
        vec![
            ctx.action.clone(),
            DataAction::Insert {
                params: HashMap::new(),
                incoming_data: rvb_common::schema::DbValue::Boolean(false),
                key: String::from("vadim"),
            },
        ],
    );
}
