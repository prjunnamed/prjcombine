use std::path::PathBuf;

use clap::Parser;
use prjcombine_re_fpga_hammer::bitdata::CollectorData;
use prjcombine_re_xilinx_geom::Chip;
use prjcombine_types::db::DumpFlags;

mod spartan6;
mod ultrascale;
mod virtex;
mod virtex2;
mod virtex4;
mod xc2000;

#[derive(Debug, Parser)]
struct Args {
    db: PathBuf,
    txt: PathBuf,
    #[arg(long)]
    xact: Option<PathBuf>,
    #[arg(long)]
    geom: Option<PathBuf>,
    bitdb: Vec<PathBuf>,
}

fn main() {
    let args = Args::parse();
    if args.xact.is_none() && args.geom.is_none() {
        panic!("no geometry given");
    }
    let xact = args
        .xact
        .map(|f| prjcombine_re_xilinx_xact_geom::GeomDb::from_file(&f).unwrap());
    let geom = args
        .geom
        .map(|f| prjcombine_re_xilinx_geom::GeomDb::from_file(&f).unwrap());
    let mut bitdb = CollectorData::default();
    for fname in args.bitdb {
        let cur = CollectorData::from_file(fname).unwrap();
        bitdb.merge(cur);
    }
    if let Some(geom) = geom {
        let chip = geom.chips.first().unwrap();
        match chip {
            Chip::Xc2000(_) => {
                let db = xc2000::finish(xact, Some(geom), bitdb);
                db.to_file(&args.db).unwrap();
                db.dump(
                    &mut std::fs::File::create(&args.txt).unwrap(),
                    DumpFlags::all(),
                )
                .unwrap();
            }
            Chip::Virtex(_) => {
                let db = virtex::finish(geom, bitdb.bsdata);
                db.to_file(&args.db).unwrap();
                db.dump(
                    &mut std::fs::File::create(&args.txt).unwrap(),
                    DumpFlags::all(),
                )
                .unwrap();
            }
            Chip::Virtex2(_) => {
                let db = virtex2::finish(geom, bitdb.bsdata);
                db.to_file(&args.db).unwrap();
                db.dump(
                    &mut std::fs::File::create(&args.txt).unwrap(),
                    DumpFlags::all(),
                )
                .unwrap();
            }
            Chip::Spartan6(_) => {
                let db = spartan6::finish(geom, bitdb.bsdata);
                db.to_file(&args.db).unwrap();
                db.dump(
                    &mut std::fs::File::create(&args.txt).unwrap(),
                    DumpFlags::all(),
                )
                .unwrap();
            }
            Chip::Virtex4(_) => {
                let db = virtex4::finish(geom, bitdb.bsdata);
                db.to_file(&args.db).unwrap();
                db.dump(
                    &mut std::fs::File::create(&args.txt).unwrap(),
                    DumpFlags::all(),
                )
                .unwrap();
            }
            Chip::Ultrascale(_) => {
                let db = ultrascale::finish(geom, bitdb.bsdata);
                db.to_file(&args.db).unwrap();
                db.dump(
                    &mut std::fs::File::create(&args.txt).unwrap(),
                    DumpFlags::all(),
                )
                .unwrap();
            }
            Chip::Versal(_) => todo!(),
        }
    } else {
        let db = xc2000::finish(xact, None, bitdb);
        db.to_file(&args.db).unwrap();
        db.dump(
            &mut std::fs::File::create(&args.txt).unwrap(),
            DumpFlags::all(),
        )
        .unwrap();
    }
}
