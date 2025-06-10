use rvb_common::{ContractAction, ContractContext};
use rvb_core::contract::{Contract, ContractCompiler, ContractError};
use wasmtime::{Caller, Config, Engine, Linker, Module, Store};

pub struct WasmtimeContractCompiler;

impl ContractCompiler for WasmtimeContractCompiler {
    fn create_contract(&mut self, bytecode: &[u8]) -> Result<Box<dyn Contract>, ContractError> {
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
                |caller: Caller<'_, Vec<u8>>| -> u32 { caller.data().len() as u32 },
            )
            .map_err(|x| ContractError::RuntimeError(x.to_string().into()))?;

        linker
            .func_wrap(
                "rvb_host",
                "write_context",
                |mut caller: Caller<'_, Vec<u8>>, ptr: u32| -> u32 {
                    let memory = match caller.get_export("memory") {
                        Some(wasmtime::Extern::Memory(mem)) => mem,
                        _ => return 1, // ALLOC_ERROR_CODE
                    };
                    let buf = caller.data().clone();
                    if memory.write(&mut caller, ptr as usize, &buf).is_ok() {
                        0
                    } else {
                        1 // ALLOC_ERROR_CODE
                    }
                },
            )
            .map_err(|x| ContractError::RuntimeError(x.to_string().into()))?;

        Ok(())
    }
}

impl Contract for WasmtimeContract {
    fn execute(&mut self, ctx: ContractContext) -> Result<Vec<ContractAction>, ContractError> {
        let fmt_ctx =
            rmp_serde::to_vec(&ctx).map_err(|x| ContractError::RuntimeError(Box::new(x)))?;

        let mut linker = Linker::new(&self.engine);

        self.register_functions(&mut linker)?;

        let mut store = Store::new(&self.engine, fmt_ctx);
        let instance = linker
            .instantiate(&mut store, &self.module)
            .map_err(|x| ContractError::CompilationError(x.to_string()))?;

        let memory = instance.get_memory(&mut store, "memory").unwrap();

        let f = instance
            .get_typed_func::<(), i64>(&mut store, "rvb_contract")
            .map_err(|x| ContractError::CompilationError(x.to_string()))?;
        f.call(&mut store, ())
            .map_err(|_| ContractError::ContractNotImplemented)?;

        let res = f
            .call(&mut store, ())
            .map_err(|_| ContractError::ContractNotImplemented)?;
        let res = (res as u32 as i32, (res >> 32) as u32 as i32);

        if res.0 == 0 {
            return Err(ContractError::ContractFailed(res.1 as usize));
        }

        let mut buffer = vec![0u8; res.0.try_into().unwrap()];
        memory
            .read(&mut store, res.1.try_into().unwrap(), &mut buffer)
            .map_err(|_| ContractError::ContractNotImplemented)?;

        rmp_serde::from_slice(&buffer).map_err(|_| ContractError::InvalidResponse)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    const TEST_DATA: &'static [u8] = include_bytes!("./test_contract.wasm");

    #[test]
    fn run_contract() {
        let mut contract = WasmtimeContractCompiler.create_contract(TEST_DATA).unwrap();
        let ctx = ContractContext {
            action: ContractAction {
                test: "test data".into(),
            },
            space: "test".into(),
            signed_by: vec![1, 2, 3],
            contract_params: HashMap::new(),
        };
        let actions = contract.execute(ctx.clone()).unwrap();

        assert_eq!(
            actions,
            vec![
                ctx.action.clone(),
                ContractAction {
                    test: format!("{ctx:?}"),
                },
            ],
        );
    }
}
