use rvb_clib::contract;
use rvb_common::ContractAction;

contract! {
    |ctx| {
        Ok(vec![
            ctx.action.clone(),
            ContractAction {
                test: format!("{ctx:?}"),
            },
        ])
    }
}
