use prjcombine_hammer::Session;

use crate::{backend::XactBackend, collector::CollectorCtx};

mod clb;
mod int;
mod io;
mod misc;

pub fn add_fuzzers<'a>(session: &mut Session<'a, XactBackend<'a>>, backend: &'a XactBackend<'a>) {
    int::add_fuzzers(session, backend);
    clb::add_fuzzers(session, backend);
    io::add_fuzzers(session, backend);
    misc::add_fuzzers(session, backend);
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    int::collect_fuzzers(ctx);
    clb::collect_fuzzers(ctx);
    io::collect_fuzzers(ctx);
    misc::collect_fuzzers(ctx);
}
