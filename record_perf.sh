RUSTFLAGS='-g' cargo build --release
RUSTFLAGS='-g' perf record --call-graph=dwarf cargo run --release
perf report