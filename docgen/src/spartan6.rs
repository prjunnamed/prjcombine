use indexmap::IndexMap;
use prjcombine_spartan6::defs::devdata;

use crate::{
    DocgenContext,
    interconnect::{gen_devdata, gen_intdb},
};

pub fn gen_spartan6(ctx: &mut DocgenContext) {
    let db = prjcombine_spartan6::db::Database::from_file(
        ctx.ctx.root.join("../databases/spartan6.zstd"),
    )
    .unwrap();
    gen_intdb(ctx, "spartan6", &db.int);
    let mut devdata = IndexMap::new();
    for device in &db.devices {
        devdata.insert(device.name.as_str(), &device.data);
    }

    gen_devdata(
        ctx,
        "spartan6",
        &db.int,
        "pci-ce-delay",
        &devdata,
        &[devdata::PCILOGICSE_PCI_CE_DELAY],
    );
}
