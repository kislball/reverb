use std::{collections::HashMap, error::Error};

use serde::{Deserialize, Serialize};

use crate::schema::{DataAction, DbValue};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct ContractContext {
    pub action: DataAction,
    pub namespace: String,
    pub contract_space: String,
    pub signed_by: Vec<u8>,
    pub contract_params: HashMap<String, DbValue>,
}

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
    fn execute(&mut self, ctx: ContractContext) -> Result<Vec<DataAction>, ContractError>;
}

pub trait ContractCompiler {
    fn create_contract(&self, bytecode: &[u8]) -> Result<Box<dyn Contract>, ContractError>;
}
