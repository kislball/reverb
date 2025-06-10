use rvb_common::{ContractAction, ContractContext};
use std::error::Error;

#[derive(Debug, thiserror::Error)]
pub enum ContractError {
    #[error("Runtime error {0}")]
    RuntimeError(Box<dyn Error>),
    #[error("Compilation error {0}")]
    CompilationError(String),
    #[error("Contract not implemented")]
    ContractNotImplemented,
    #[error("Invalid response")]
    InvalidResponse,
    #[error("Contract failed. Code: {0}")]
    ContractFailed(usize),
}

pub trait Contract {
    fn execute(&mut self, ctx: ContractContext) -> Result<Vec<ContractAction>, ContractError>;
}

pub trait ContractCompiler {
    fn create_contract(&mut self, bytecode: &[u8]) -> Result<Box<dyn Contract>, ContractError>;
}
