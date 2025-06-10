use rvb_common::{ContractAction, ContractContext};

#[link(wasm_import_module = "rvb_host")]
unsafe extern "C" {
    unsafe fn get_context_length() -> u32;
    unsafe fn write_context(ptr: u32) -> u32;
}

pub fn get_context() -> ContractContext {
    // SAFETY: always safe since it just returns a number with no side effects
    let len = unsafe { get_context_length() };
    let buf = vec![0u8; len as usize];
    // SAFETY: safe, as the WASM host does not write over len
    let res = unsafe { write_context(buf.as_ptr() as u32) };
    if res != 0 {
        panic!("Failed to write context, error code {res}");
    }

    rmp_serde::from_slice(&buf).expect("Invalid payload")
}

pub fn run_contract(f: impl Fn(ContractContext) -> Result<Vec<ContractAction>, i32>) -> (i32, i32) {
    let res = f(get_context());

    match res {
        Err(i) => (-1, i),
        Ok(acc) => match rmp_serde::to_vec(&acc) {
            Err(_) => (0, 10),
            Ok(v) => (v.len() as i32, v.as_ptr() as i32),
        },
    }
}

#[macro_export]
macro_rules! contract {
    (|$i:ident| $b:block) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn rvb_contract() -> i64 {
            let (len, begin) = rvb_clib::run_contract(|$i: rvb_common::ContractContext| $b);
            ((begin as u32 as i64) << 32) | (len as u32 as i64)
        }
    };
}
