use log::debug;
use rvb_common::{
    contract::{Contract, ContractCompiler, ContractContext, ContractError},
    schema::DataAction,
};
use wasmtime::{Caller, Config, Engine, Linker, Module, Store};

pub struct WasmtimeContractCompiler;

impl ContractCompiler for WasmtimeContractCompiler {
    fn create_contract(&self, bytecode: &[u8]) -> Result<Box<dyn Contract>, ContractError> {
        let engine = Engine::new(&Config::default())
            .map_err(|x| ContractError::CompilationError(x.to_string()))?;
        let module = Module::new(&engine, bytecode)
            .map_err(|x| ContractError::CompilationError(x.to_string()))?;

        Ok(Box::new(WasmtimeContract { module, engine }))
    }
}

pub struct WasmtimeContract {
    module: Module,
    engine: Engine,
}

pub const ALLOC_ERROR_CODE: u8 = 1;

impl WasmtimeContract {
    fn register_functions(&self, linker: &mut Linker<Vec<u8>>) -> Result<(), ContractError> {
        linker
            .func_wrap(
                "rvb_host",
                "get_context_length",
                |caller: Caller<'_, Vec<u8>>| -> u64 { caller.data().len() as u64 },
            )
            .map_err(|x| ContractError::RuntimeError(x.to_string().into()))?;

        linker
            .func_wrap(
                "rvb_host",
                "write_context",
                |mut caller: Caller<'_, Vec<u8>>, ptr: u64| -> u64 {
                    let memory = match caller.get_export("memory") {
                        Some(wasmtime::Extern::Memory(mem)) => mem,
                        _ => return 1, // ALLOC_ERROR_CODE
                    };

                    let buf = caller.data().clone();
                    if let Err(e) = memory.write(&mut caller, ptr as usize, &buf) {
                        debug!("Failed to write to memory {e}");
                        1
                    } else {
                        debug!("wrote to memory");
                        0
                    }
                },
            )
            .map_err(|x| ContractError::RuntimeError(x.to_string().into()))?;

        Ok(())
    }
}

impl Contract for WasmtimeContract {
    fn execute(&mut self, ctx: ContractContext) -> Result<Vec<DataAction>, ContractError> {
        let fmt_ctx =
            rmp_serde::to_vec(&ctx).map_err(|x| ContractError::RuntimeError(Box::new(x)))?;

        let mut linker = Linker::new(&self.engine);

        self.register_functions(&mut linker)?;

        let mut store = Store::new(&self.engine, fmt_ctx);
        let instance = linker.instantiate(&mut store, &self.module).map_err(|x| {
            debug!("Instantiate error {x}");
            ContractError::CompilationError(x.to_string())
        })?;

        let memory = instance.get_memory(&mut store, "memory").unwrap();
        memory.grow(&mut store, 1024).map_err(|x| {
            debug!("Failed to grow WASM memory {x}");
            ContractError::CompilationError(x.to_string())
        })?;

        let f = instance
            .get_typed_func::<(), u64>(&mut store, "rvb_contract")
            .map_err(|x| {
                debug!("Function getter error {x}");
                ContractError::CompilationError(x.to_string())
            })?;

        let res = f.call(&mut store, ()).map_err(|e| {
            debug!("Error calling contract function: {e:?}");
            ContractError::ContractNotImplemented
        })?;
        let res = (res as u32, (res >> 32) as u32);

        if res.0 == 0 {
            return Err(ContractError::ContractFailed(res.1 as usize));
        }

        let mut buffer = vec![0u8; res.0 as usize];
        memory
            .read(&mut store, res.1 as usize, &mut buffer)
            .map_err(|e| {
                debug!("Error reading memory: {e:?}");
                ContractError::ContractNotImplemented
            })?;

        rmp_serde::from_slice(&buffer).map_err(|e| {
            debug!("Error deserializing contract response: {e:?}");
            ContractError::InvalidResponse
        })
    }
}

#[cfg(test)]
mod tests;
