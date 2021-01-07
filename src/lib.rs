pub mod error;
pub mod xilinx;
pub mod stringpool;
pub mod toolreader;
pub mod toolchain;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;
