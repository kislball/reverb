compile_test_contract:
	cd rvb_clib/test_contract && cargo build --release --target wasm32-unknown-unknown
	cp ./target/wasm32-unknown-unknown/release/test_contract.wasm ./rvb_contract/src