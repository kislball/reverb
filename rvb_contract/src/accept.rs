use rvb_common::{ContractAction, ContractContext};
use rvb_core::contract::{Contract, ContractCompiler, ContractError};

pub struct AcceptContractCompiler;

impl ContractCompiler for AcceptContractCompiler {
    fn create_contract(&mut self, _bytecode: &[u8]) -> Result<Box<dyn Contract>, ContractError> {
        Ok(Box::new(AcceptContract))
    }
}

pub struct AcceptContract;

impl Contract for AcceptContract {
    fn execute(&mut self, ctx: ContractContext) -> Result<Vec<ContractAction>, ContractError> {
        Ok(vec![ctx.action])
    }
}
