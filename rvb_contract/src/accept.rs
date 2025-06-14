use rvb_common::{
    contract::{Contract, ContractCompiler, ContractContext, ContractError},
    schema::DataAction,
};

pub struct AcceptContractCompiler;

impl ContractCompiler for AcceptContractCompiler {
    fn create_contract(&self, _bytecode: &[u8]) -> Result<Box<dyn Contract>, ContractError> {
        Ok(Box::new(AcceptContract))
    }
}

pub struct AcceptContract;

impl Contract for AcceptContract {
    fn execute(&mut self, ctx: ContractContext) -> Result<Vec<DataAction>, ContractError> {
        Ok(vec![ctx.action])
    }
}
