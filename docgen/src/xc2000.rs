
use crate::DocgenContext;
use crate::interconnect::gen_intdb;

pub fn gen_xc2000(ctx: &mut DocgenContext) {
    for kind in [
        "xc2000",
        "xc3000",
        "xc3000a",
        "xc4000",
        "xc4000a",
        "xc4000h",
        "xc4000e",
        "xc4000ex",
        "xc4000xla",
        "xc4000xv",
        "spartanxl",
        "xc5200",
    ] {
        let db = prjcombine_xc2000::db::Database::from_file(
            ctx.ctx.root.join(format!("../databases/{kind}.zstd")),
        )
        .unwrap();
        gen_intdb(ctx, kind, &db.int);
    }
}
