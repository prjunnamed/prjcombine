pub mod error;
pub mod xilinx;
pub mod stringpool;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;
