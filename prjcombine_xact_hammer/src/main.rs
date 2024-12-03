use std::{
    collections::HashMap,
    error::Error,
    path::{Path, PathBuf},
};

use backend::XactBackend;
use clap::Parser;
use collector::CollectorCtx;
use itertools::Itertools;
use prjcombine_collector::Collector;
use prjcombine_hammer::Session;
use prjcombine_types::tiledb::TileDb;
use prjcombine_xact_geom::{Device, GeomDb};
use prjcombine_xc2000::grid::GridKind;

mod backend;
mod collector;
mod fbuild;
mod fgen;
mod lca;
mod xc2000;
mod xc3000;
mod xc4000;
mod xc5200;

#[derive(Debug, Parser)]
#[command(
    name = "xact_hammer",
    about = "Swing the Massive Hammer on XACT parts."
)]
struct Args {
    xact: PathBuf,
    geomdb: PathBuf,
    tiledb: PathBuf,
    parts: Vec<String>,
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
    #[arg(long)]
    no_dup: bool,
}

#[derive(Copy, Clone, Debug)]
struct RunOpts {
    debug: u8,
    no_dup: bool,
}

fn run(xact_path: &Path, db: &GeomDb, part: &Device, tiledb: &mut TileDb, opts: &RunOpts) {
    println!("part {name}", name = part.name);
    let edev = db.expand_grid(part);
    let endev = db.name(part, &edev);

    let backend = XactBackend {
        debug: opts.debug,
        xact_path,
        device: part,
        bs_geom: &edev.bs_geom,
        egrid: &edev.egrid,
        ngrid: &endev.ngrid,
        edev: &edev,
    };
    let mut hammer = Session::new(&backend);
    hammer.debug = opts.debug;
    if opts.no_dup {
        hammer.dup_factor = 1;
    }
    match edev.grid.kind {
        GridKind::Xc2000 => xc2000::add_fuzzers(&mut hammer, &backend),
        GridKind::Xc3000 | GridKind::Xc3000A => xc3000::add_fuzzers(&mut hammer, &backend),
        GridKind::Xc4000
        | GridKind::Xc4000A
        | GridKind::Xc4000H
        | GridKind::Xc4000E
        | GridKind::Xc4000Ex
        | GridKind::Xc4000Xla
        | GridKind::Xc4000Xv
        | GridKind::SpartanXl => xc4000::add_fuzzers(&mut hammer, &backend),
        GridKind::Xc5200 => xc5200::add_fuzzers(&mut hammer, &backend),
    }
    let mut state = hammer.run().unwrap();
    let mut ctx = CollectorCtx {
        device: part,
        edev: &edev,
        collector: Collector {
            state: &mut state,
            tiledb,
        },
    };
    match edev.grid.kind {
        GridKind::Xc2000 => xc2000::collect_fuzzers(&mut ctx),
        GridKind::Xc3000 | GridKind::Xc3000A => xc3000::collect_fuzzers(&mut ctx),
        GridKind::Xc4000
        | GridKind::Xc4000A
        | GridKind::Xc4000H
        | GridKind::Xc4000E
        | GridKind::Xc4000Ex
        | GridKind::Xc4000Xla
        | GridKind::Xc4000Xv
        | GridKind::SpartanXl => xc4000::collect_fuzzers(&mut ctx),
        GridKind::Xc5200 => xc5200::collect_fuzzers(&mut ctx),
    }
    for (feat, data) in state.features.iter().sorted_by_key(|&(k, _)| k) {
        println!(
            "{} {} {} {}: {:?}",
            feat.tile, feat.bel, feat.attr, feat.val, data.diffs
        );
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let db = GeomDb::from_file(args.geomdb)?;
    let mut tiledb = TileDb::new();
    let opts = RunOpts {
        no_dup: args.no_dup,
        debug: args.debug,
    };
    let parts_dict: HashMap<_, _> = db
        .devices
        .iter()
        .map(|part| (&part.name[..], part))
        .collect();
    if args.parts.is_empty() {
        for part in &db.devices {
            run(&args.xact, &db, part, &mut tiledb, &opts);
        }
    } else {
        for pname in args.parts {
            let part = parts_dict[&&pname[..]];
            run(&args.xact, &db, part, &mut tiledb, &opts);
        }
    }

    tiledb.to_file(&args.tiledb)?;
    Ok(())
}
