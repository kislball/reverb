use crate::accept::AcceptContractCompiler;
use crate::wasmtime::WasmtimeContractCompiler;
#[cfg(feature = "runtime")]
use rvb_common::contract::ContractCompiler;

pub mod accept;
#[cfg(feature = "runtime")]
pub mod wasmtime;

#[derive(Debug, Clone, Copy)]
pub enum ContractCompilerType {
    #[cfg(feature = "runtime")]
    Wasmtime,
    Accept,
}

#[must_use]
pub fn resolve_contract_runtime(feature: ContractCompilerType) -> Box<dyn ContractCompiler> {
    match feature {
        ContractCompilerType::Accept => Box::new(AcceptContractCompiler),
        #[cfg(feature = "runtime")]
        ContractCompilerType::Wasmtime => Box::new(WasmtimeContractCompiler),
    }
}
