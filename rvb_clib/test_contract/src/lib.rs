use rvb_clib::{
    contract,
    schema::{DataAction, DbValue},
};
use std::collections::HashMap;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

contract! {
    |ctx| {
        let DataAction::Insert { key, incoming_data, .. } = ctx.action;

        Ok(vec![
            DataAction::Insert { key, incoming_data, params: HashMap::new() },
            DataAction::Insert {
                params:HashMap::new(),
                key: String::from("vadim"),
                incoming_data: DbValue::Boolean(false),
            },
        ])
    }
}
