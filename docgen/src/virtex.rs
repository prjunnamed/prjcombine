use crate::{DocgenContext, interconnect::gen_intdb};

pub fn gen_virtex(ctx: &mut DocgenContext) {
    let db =
        prjcombine_virtex::db::Database::from_file(ctx.ctx.root.join("../databases/virtex.zstd"))
            .unwrap();
    gen_intdb(ctx, "virtex", &db.int);
}
