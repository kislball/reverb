use crate::accept::AcceptContractCompiler;
#[cfg(feature = "wasmtime")]
use crate::wasmtime::WasmtimeContractCompiler;
use rvb_core::contract::ContractCompiler;

pub mod accept;
#[cfg(feature = "wasmtime")]
pub mod wasmtime;

pub enum ContractCompilerType {
    #[cfg(feature = "wasmtime")]
    Wasmtime,
    Accept,
}

pub fn resolve_contract_runtime(feature: ContractCompilerType) -> Box<dyn ContractCompiler> {
    match feature {
        ContractCompilerType::Accept => Box::new(AcceptContractCompiler),
        #[cfg(feature = "wasmtime")]
        ContractCompilerType::Wasmtime => Box::new(WasmtimeContractCompiler),
    }
}
