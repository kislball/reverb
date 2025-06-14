use rvb_common::{contract::ContractContext, schema::DataAction};

pub use rvb_common::contract;
pub use rvb_common::crypto;
pub use rvb_common::schema;
pub use serde::{Deserialize, Serialize};

#[link(wasm_import_module = "rvb_host")]
unsafe extern "C" {
    unsafe fn get_context_length() -> u64;
    unsafe fn write_context(ptr: u64) -> u64;
}

#[must_use]
pub fn get_context() -> ContractContext {
    // SAFETY: always safe since it just returns a number with no side effects
    let len = unsafe { get_context_length() };
    let buf = vec![0u8; len as usize];
    // SAFETY: safe, as the WASM host does not write over len
    let res = unsafe { write_context(buf.as_ptr() as u64) };

    assert!((res == 0), "Failed to write context, error code {res}");

    rmp_serde::from_slice(&buf).expect("Invalid payload")
}

pub fn run_contract(f: impl Fn(ContractContext) -> Result<Vec<DataAction>, u64>) -> (u64, u64) {
    let ctx = get_context();
    let res = f(ctx);

    match res {
        Err(i) => (0, i),
        Ok(acc) => match rmp_serde::to_vec(&acc) {
            Err(_) => (0, 10),
            Ok(v) => {
                let v = v.leak();
                (v.len() as u64, v.as_ptr() as u64)
            }
        },
    }
}

#[macro_export]
macro_rules! contract {
    (|$i:ident| $b:block) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn rvb_contract() -> u64 {
            let (len, begin) = $crate::run_contract(|$i: $crate::contract::ContractContext| $b);
            ((begin as u64) << 32) | (len as u64)
        }
    };
}
