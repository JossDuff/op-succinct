cd proposer/succinct;
SP1_DUMP=1 RUST_LOG=info RUST_BACKTRACE=1 cargo run --bin minimal-server;
cp stdin.bin ../../;
cp program.bin ../../;
