# need to use a single thread for tests due to the gtk tests
cargo test -- --test-threads=1 $*
