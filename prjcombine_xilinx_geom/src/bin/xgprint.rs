use clap::Parser;
use prjcombine_xilinx_geom::{Bond, DeviceNaming, GeomDb, Grid};
use std::{error::Error, path::PathBuf};
use unnamed_entity::EntityId;

#[derive(Debug, Parser)]
#[command(name = "xgprint", about = "Dump Xilinx geom file.")]
struct Args {
    file: PathBuf,
    #[arg(short, long)]
    intdb: bool,
    #[arg(short, long)]
    devices: bool,
    #[arg(short, long)]
    grids: bool,
    #[arg(short, long)]
    pkgs: bool,
    #[arg(short, long)]
    namings: bool,
}

pub fn pad_sort_key(name: &str) -> (usize, &str, u32) {
    let pos = name.find(|x: char| x.is_ascii_digit()).unwrap();
    (pos, &name[..pos], name[pos..].parse().unwrap())
}

mod xc4000 {
    use itertools::Itertools;
    use prjcombine_xc4000::{
        bond::{Bond, BondPin, CfgPin},
        grid::Grid,
    };

    use crate::pad_sort_key;

    pub fn print_grid(grid: &Grid) {
        println!("\tKIND: {k:?}", k = grid.kind);
        println!("\tDIMS: {c}×{r}", c = grid.columns, r = grid.rows);
        println!("\tIS BUFF LARGE: {v}", v = grid.is_buff_large);
        println!("\tCFG PINS:");
        for (k, v) in &grid.cfg_io {
            println!(
                "\t\t{k:?}: IOB_X{x}Y{y}B{b}",
                x = v.col,
                y = v.row,
                b = v.iob
            );
        }
    }

    pub fn print_bond(bond: &Bond) {
        println!("\tPINS:");
        for (pin, pad) in bond.pins.iter().sorted_by_key(|(k, _)| pad_sort_key(k)) {
            print!("\t\t{pin:4}: ");
            match pad {
                BondPin::Io(io) => print!("IOB_X{x}Y{y}B{b}", x = io.col, y = io.row, b = io.iob),
                BondPin::Nc => print!("NC"),
                BondPin::Gnd => print!("GND"),
                BondPin::VccInt => print!("VCCINT"),
                BondPin::VccO => print!("VCCO"),
                BondPin::Cfg(CfgPin::Cclk) => print!("CCLK"),
                BondPin::Cfg(CfgPin::Done) => print!("DONE"),
                BondPin::Cfg(CfgPin::ProgB) => print!("PROG_B"),
                BondPin::Cfg(CfgPin::M0) => print!("M0"),
                BondPin::Cfg(CfgPin::M1) => print!("M1"),
                BondPin::Cfg(CfgPin::M2) => print!("M2"),
                BondPin::Cfg(CfgPin::Tdo) => print!("TDO"),
                BondPin::Cfg(CfgPin::PwrdwnB) => print!("PWRDWN_B"),
            }
            println!();
        }
    }
}

mod xc5200 {
    use itertools::Itertools;
    use prjcombine_xc5200::{
        bond::{Bond, BondPin, CfgPin},
        grid::Grid,
    };

    use crate::pad_sort_key;

    pub fn print_grid(grid: &Grid) {
        println!("\tKIND: Xc5200");
        println!("\tDIMS: {c}×{r}", c = grid.columns, r = grid.rows);
        println!("\tCFG PINS:");
        for (k, v) in &grid.cfg_io {
            println!(
                "\t\t{k:?}: IOB_X{x}Y{y}B{b}",
                x = v.col,
                y = v.row,
                b = v.iob
            );
        }
    }

    pub fn print_bond(bond: &Bond) {
        println!("\tPINS:");
        for (pin, pad) in bond.pins.iter().sorted_by_key(|(k, _)| pad_sort_key(k)) {
            print!("\t\t{pin:4}: ");
            match pad {
                BondPin::Io(io) => print!("IOB_X{x}Y{y}B{b}", x = io.col, y = io.row, b = io.iob),
                BondPin::Nc => print!("NC"),
                BondPin::Gnd => print!("GND"),
                BondPin::Vcc => print!("VCC"),
                BondPin::Cfg(CfgPin::Cclk) => print!("CCLK"),
                BondPin::Cfg(CfgPin::Done) => print!("DONE"),
                BondPin::Cfg(CfgPin::ProgB) => print!("PROG_B"),
            }
            println!();
        }
    }
}

mod virtex {
    use itertools::Itertools;
    use prjcombine_virtex::{
        bond::{Bond, BondPin, CfgPin},
        grid::Grid,
    };
    use unnamed_entity::EntityId;

    use crate::pad_sort_key;

    pub fn print_grid(grid: &Grid) {
        println!("\tKIND: {k:?}", k = grid.kind);
        println!("\tDIMS: {c}×{r}", c = grid.columns, r = grid.rows);
        println!("\tCOLS:");
        let mut clkv_idx = 0;
        for col in grid.columns() {
            if col == grid.cols_clkv[clkv_idx].0 {
                println!("\t\t--- clock column");
            }
            if col == grid.cols_clkv[clkv_idx].2 {
                println!("\t\t--- clock break");
                clkv_idx += 1;
            }
            println!(
                "\t\tX{c}: {kind}",
                c = col.to_idx(),
                kind = if grid.cols_bram.contains(&col) {
                    "BRAM"
                } else if col == grid.col_lio() {
                    "LIO"
                } else if col == grid.col_rio() {
                    "RIO"
                } else {
                    "CLB"
                }
            );
        }
        println!("\tCFG PINS:");
        for (k, v) in &grid.cfg_io {
            println!(
                "\t\t{k:?}: IOB_X{x}Y{y}B{b}",
                x = v.col.to_idx(),
                y = v.row.to_idx(),
                b = v.iob.to_idx()
            );
        }
    }

    pub fn print_bond(bond: &Bond) {
        println!("\tBANKS:");
        for (k, v) in &bond.io_banks {
            println!("\t\t{k}: {v}");
        }
        println!("\tPINS:");
        for (pin, pad) in bond.pins.iter().sorted_by_key(|(k, _)| pad_sort_key(k)) {
            print!("\t\t{pin:4}: ");
            match pad {
                BondPin::Io(io) => print!("IOB_X{x}Y{y}B{b}", x = io.col, y = io.row, b = io.iob),
                BondPin::Clk(idx) => print!("CLK{idx}"),
                BondPin::Nc => print!("NC"),
                BondPin::Gnd => print!("GND"),
                BondPin::VccInt => print!("VCCINT"),
                BondPin::VccAux => print!("VCCAUX"),
                BondPin::VccO(bank) => print!("VCCO{bank}"),
                BondPin::Cfg(CfgPin::Cclk) => print!("CCLK"),
                BondPin::Cfg(CfgPin::Done) => print!("DONE"),
                BondPin::Cfg(CfgPin::M0) => print!("M0"),
                BondPin::Cfg(CfgPin::M1) => print!("M1"),
                BondPin::Cfg(CfgPin::M2) => print!("M2"),
                BondPin::Cfg(CfgPin::ProgB) => print!("PROG_B"),
                BondPin::Cfg(CfgPin::Tck) => print!("TCK"),
                BondPin::Cfg(CfgPin::Tms) => print!("TMS"),
                BondPin::Cfg(CfgPin::Tdi) => print!("TDI"),
                BondPin::Cfg(CfgPin::Tdo) => print!("TDO"),
                BondPin::Dxn => print!("DXN"),
                BondPin::Dxp => print!("DXP"),
            }
            println!();
        }
        println!("\tVREF:");
        for v in &bond.vref {
            println!(
                "\t\tIOB_X{x}Y{y}B{b}",
                x = v.col.to_idx(),
                y = v.row.to_idx(),
                b = v.iob.to_idx()
            );
        }
        println!("\tDIFFP:");
        for v in &bond.diffp {
            println!(
                "\t\tIOB_X{x}Y{y}B{b}",
                x = v.col.to_idx(),
                y = v.row.to_idx(),
                b = v.iob.to_idx()
            );
        }
        println!("\tDIFFN:");
        for v in &bond.diffn {
            println!(
                "\t\tIOB_X{x}Y{y}B{b}",
                x = v.col.to_idx(),
                y = v.row.to_idx(),
                b = v.iob.to_idx()
            );
        }
    }
}

mod virtex2 {
    use itertools::Itertools;
    use prjcombine_virtex2::{
        bond::{Bond, BondPin, CfgPin, GtPin},
        grid::{ColumnIoKind, ColumnKind, Grid, RowIoKind},
    };
    use unnamed_entity::EntityId;

    use crate::pad_sort_key;

    pub fn print_grid(grid: &Grid) {
        println!("\tKIND: {k:?}", k = grid.kind);
        println!("\tCOLS:");
        for (col, cd) in &grid.columns {
            if let Some((cl, cr)) = grid.cols_clkv {
                if col == cl {
                    println!("\t\t--- clock left spine");
                }
                if col == cr {
                    println!("\t\t--- clock right spine");
                }
            }
            if col == grid.col_clk {
                println!("\t\t--- clock spine");
            }
            print!("\t\tX{c}: ", c = col.to_idx());
            match cd.kind {
                ColumnKind::Io => print!("IO"),
                ColumnKind::Clb => print!("CLB   "),
                ColumnKind::Bram => print!("BRAM  "),
                ColumnKind::BramCont(i) => print!("BRAM.{i}"),
                ColumnKind::Dsp => print!("DSP   "),
            }
            match cd.io {
                ColumnIoKind::None => (),
                ColumnIoKind::Single => print!(" IO: 1"),
                ColumnIoKind::Double(i) => print!(" IO: 2.{i}"),
                ColumnIoKind::Triple(i) => print!(" IO: 3.{i}"),
                ColumnIoKind::Quad(i) => print!(" IO: 4.{i}"),
                ColumnIoKind::SingleLeft => print!(" IO: 1L"),
                ColumnIoKind::SingleRight => print!(" IO: 1R"),
                ColumnIoKind::SingleLeftAlt => print!(" IO: 1LA"),
                ColumnIoKind::SingleRightAlt => print!(" IO: 1RA"),
                ColumnIoKind::DoubleLeft(i) => print!(" IO: 2L.{i}"),
                ColumnIoKind::DoubleRight(i) => print!(" IO: 2R.{i}"),
            }
            if let Some(&(bb, bt)) = grid.cols_gt.get(&col) {
                print!(" GT: BOT {bb} TOP {bt}");
            }
            println!();
        }
        let mut clkv_idx = 0;
        println!("\tROWS:");
        for (row, rd) in &grid.rows {
            if row == grid.rows_hclk[clkv_idx].0 {
                println!("\t\t--- clock row");
            }
            if row == grid.rows_hclk[clkv_idx].2 {
                println!("\t\t--- clock break");
                clkv_idx += 1;
            }
            if Some(row) == grid.row_pci {
                println!("\t\t--- PCI row");
            }
            if row == grid.row_mid() {
                println!("\t\t--- spine row");
            }
            print!("\t\tY{r}: ", r = row.to_idx());
            match rd {
                RowIoKind::None => (),
                RowIoKind::Single => print!(" IO: 1"),
                RowIoKind::Double(i) => print!(" IO: 2.{i}"),
                RowIoKind::Triple(i) => print!(" IO: 3.{i}"),
                RowIoKind::Quad(i) => print!(" IO: 4.{i}"),
                RowIoKind::DoubleBot(i) => print!(" IO: 2B.{i}"),
                RowIoKind::DoubleTop(i) => print!(" IO: 2T.{i}"),
            }
            if let Some((rb, rt)) = grid.rows_ram {
                if row == rb {
                    print!(" BRAM BOT TERM");
                }
                if row == rt {
                    print!(" BRAM TOP TERM");
                }
            }
            println!();
        }
        for &(col, row) in &grid.holes_ppc {
            println!(
                "\tPPC: X{xl}:X{xr} Y{yb}:Y{yt}",
                xl = col.to_idx(),
                xr = col.to_idx() + 10,
                yb = row.to_idx(),
                yt = row.to_idx() + 16
            );
        }
        if let Some(dcms) = grid.dcms {
            println!("\tDCMS: {dcms:?}");
        }
        if grid.has_ll {
            println!("\tHAS LL SPLITTERS");
        }
        println!("\tHAS_SMALL_INT: {v:?}", v = grid.has_small_int);
        println!("\tCFG PINS:");
        for (k, v) in &grid.cfg_io {
            println!(
                "\t\t{k:?}: IOB_X{x}Y{y}B{b}",
                x = v.col.to_idx(),
                y = v.row.to_idx(),
                b = v.iob.to_idx()
            );
        }
        if !grid.dci_io.is_empty() {
            println!("\tDCI:");
            for k in 0..8 {
                println!("\t\t{k}:");
                if let Some(&(vp, vn)) = grid.dci_io.get(&k) {
                    println!(
                        "\t\t\tVP: X{x}Y{y}B{b}",
                        x = vp.col.to_idx(),
                        y = vp.row.to_idx(),
                        b = vp.iob.to_idx()
                    );
                    println!(
                        "\t\t\tVN: X{x}Y{y}B{b}",
                        x = vn.col.to_idx(),
                        y = vn.row.to_idx(),
                        b = vn.iob.to_idx()
                    );
                }
                if let Some(&(vp, vn)) = grid.dci_io_alt.get(&k) {
                    println!(
                        "\t\t\tALT VP: X{x}Y{y}B{b}",
                        x = vp.col.to_idx(),
                        y = vp.row.to_idx(),
                        b = vp.iob.to_idx()
                    );
                    println!(
                        "\t\t\tALT VN: X{x}Y{y}B{b}",
                        x = vn.col.to_idx(),
                        y = vn.row.to_idx(),
                        b = vn.iob.to_idx()
                    );
                }
            }
        }
    }

    pub fn print_bond(bond: &Bond) {
        println!("\tBANKS:");
        for (k, v) in &bond.io_banks {
            println!("\t\t{k}: {v}");
        }
        println!("\tPINS:");
        for (pin, pad) in bond.pins.iter().sorted_by_key(|(k, _)| pad_sort_key(k)) {
            print!("\t\t{pin:4}: ");
            match pad {
                BondPin::Io(io) => print!("IOB_X{x}Y{y}B{b}", x = io.col, y = io.row, b = io.iob),
                BondPin::Gt(bank, gtpin) => {
                    print!("GT{bank}.");
                    match gtpin {
                        GtPin::RxP => print!("RXP"),
                        GtPin::RxN => print!("RXN"),
                        GtPin::TxP => print!("TXP"),
                        GtPin::TxN => print!("TXN"),
                        GtPin::GndA => print!("GNDA"),
                        GtPin::VtRx => print!("VTRX"),
                        GtPin::VtTx => print!("VTTX"),
                        GtPin::AVccAuxRx => print!("AVCCAUXRX"),
                        GtPin::AVccAuxTx => print!("AVCCAUXTX"),
                    }
                }
                BondPin::Nc => print!("NC"),
                BondPin::Gnd => print!("GND"),
                BondPin::VccInt => print!("VCCINT"),
                BondPin::VccAux => print!("VCCAUX"),
                BondPin::VccO(bank) => print!("VCCO{bank}"),
                BondPin::VccBatt => print!("VCC_BATT"),
                BondPin::Cfg(CfgPin::Cclk) => print!("CCLK"),
                BondPin::Cfg(CfgPin::Done) => print!("DONE"),
                BondPin::Cfg(CfgPin::M0) => print!("M0"),
                BondPin::Cfg(CfgPin::M1) => print!("M1"),
                BondPin::Cfg(CfgPin::M2) => print!("M2"),
                BondPin::Cfg(CfgPin::ProgB) => print!("PROG_B"),
                BondPin::Cfg(CfgPin::Tck) => print!("TCK"),
                BondPin::Cfg(CfgPin::Tms) => print!("TMS"),
                BondPin::Cfg(CfgPin::Tdi) => print!("TDI"),
                BondPin::Cfg(CfgPin::Tdo) => print!("TDO"),
                BondPin::Cfg(CfgPin::PwrdwnB) => print!("PWRDWN_B"),
                BondPin::Cfg(CfgPin::HswapEn) => print!("HSWAP_EN"),
                BondPin::Cfg(CfgPin::Suspend) => print!("SUSPEND"),
                BondPin::Dxn => print!("DXN"),
                BondPin::Dxp => print!("DXP"),
                BondPin::Rsvd => print!("RSVD"),
            }
            println!();
        }
        println!("\tVREF:");
        for v in &bond.vref {
            println!(
                "\t\tIOB_X{x}Y{y}B{b}",
                x = v.col.to_idx(),
                y = v.row.to_idx(),
                b = v.iob.to_idx()
            );
        }
    }
}

mod spartan6 {
    use itertools::Itertools;
    use prjcombine_spartan6::{
        bond::{Bond, BondPin, CfgPin, GtPin},
        grid::{ColumnIoKind, ColumnKind, Grid, Gts},
    };
    use unnamed_entity::EntityId;

    use crate::pad_sort_key;

    pub fn print_grid(grid: &Grid) {
        println!("\tKIND: Spartan6");
        println!("\tCOLS:");
        for (col, cd) in &grid.columns {
            print!("\t\tX{c}: ", c = col.to_idx());
            match cd.kind {
                ColumnKind::Io => print!("IO"),
                ColumnKind::CleXL => print!("CLEXL"),
                ColumnKind::CleXM => print!("CLEXM"),
                ColumnKind::CleClk => print!("CLEXL+CLK"),
                ColumnKind::Bram => print!("BRAM"),
                ColumnKind::Dsp => print!("DSP"),
                ColumnKind::DspPlus => print!("DSP*"),
            }
            match cd.bio {
                ColumnIoKind::None => (),
                ColumnIoKind::Inner => print!(" BIO: I-"),
                ColumnIoKind::Outer => print!(" BIO: -O"),
                ColumnIoKind::Both => print!(" BIO: IO"),
            }
            match cd.tio {
                ColumnIoKind::None => (),
                ColumnIoKind::Inner => print!(" TIO: I-"),
                ColumnIoKind::Outer => print!(" TIO: -O"),
                ColumnIoKind::Both => print!(" TIO: IO"),
            }
            if let Some((cl, cr)) = grid.cols_clk_fold {
                if col == cl || col == cr {
                    print!(" FOLD");
                }
            }
            if col == grid.cols_reg_buf.0 || col == grid.cols_reg_buf.1 {
                print!(" REGBUF");
            }
            if let Gts::Single(cl) | Gts::Double(cl, _) | Gts::Quad(cl, _) = grid.gts {
                if col == cl {
                    print!(" LGT");
                }
            }
            if let Gts::Double(_, cr) | Gts::Quad(_, cr) = grid.gts {
                if col == cr {
                    print!(" RGT");
                }
            }
            println!();
        }
        println!("\tROWS:");
        for (row, rd) in &grid.rows {
            if row.to_idx() != 0 && row.to_idx() % 16 == 0 {
                println!("\t\t--- clock break");
            }
            if row.to_idx() % 16 == 8 {
                println!("\t\t--- clock row");
            }
            if row == grid.row_clk() {
                println!("\t\t--- spine row");
            }
            if let Some((rl, rr)) = grid.rows_bank_split {
                if row == rl {
                    println!("\t\t--- left bank split");
                }
                if row == rr {
                    println!("\t\t--- right bank split");
                }
            }
            if Some(row) == grid.row_mcb_split {
                println!("\t\t--- MCB split");
            }
            print!("\t\tY{r}: ", r = row.to_idx());
            if rd.lio {
                print!(" LIO");
            }
            if rd.rio {
                print!(" RIO");
            }
            if row == grid.rows_midbuf.0 || row == grid.rows_midbuf.1 {
                print!(" MIDBUF");
            }
            if row == grid.rows_hclkbuf.0 || row == grid.rows_hclkbuf.1 {
                print!(" HCLKBUF");
            }
            for (i, mcb) in grid.mcbs.iter().enumerate() {
                if row == mcb.row_mcb {
                    print!(" MCB{i}.MCB");
                }
                for (j, &r) in mcb.row_mui.iter().enumerate() {
                    if row == r {
                        print!(" MCB{i}.MUI{j}");
                    }
                }
                for (j, &r) in mcb.iop_dq.iter().enumerate() {
                    if row == r {
                        print!(" MCB{i}.DQ({jj0},{jj1})", jj0 = j * 2, jj1 = j * 2 + 1);
                    }
                }
                for (j, &r) in mcb.iop_dqs.iter().enumerate() {
                    if row == r {
                        print!(" MCB{i}.DQS{j}");
                    }
                }
                if row == mcb.iop_clk {
                    print!(" MCB{i}.CLK");
                }
                let mut pins: [Option<&'static str>; 2] = [None, None];
                for (pin, io) in [
                    ("DM0", mcb.io_dm[0]),
                    ("DM1", mcb.io_dm[1]),
                    ("A0", mcb.io_addr[0]),
                    ("A1", mcb.io_addr[1]),
                    ("A2", mcb.io_addr[2]),
                    ("A3", mcb.io_addr[3]),
                    ("A4", mcb.io_addr[4]),
                    ("A5", mcb.io_addr[5]),
                    ("A6", mcb.io_addr[6]),
                    ("A7", mcb.io_addr[7]),
                    ("A8", mcb.io_addr[8]),
                    ("A9", mcb.io_addr[9]),
                    ("A10", mcb.io_addr[10]),
                    ("A11", mcb.io_addr[11]),
                    ("A12", mcb.io_addr[12]),
                    ("A13", mcb.io_addr[13]),
                    ("A14", mcb.io_addr[14]),
                    ("BA0", mcb.io_ba[0]),
                    ("BA1", mcb.io_ba[1]),
                    ("BA2", mcb.io_ba[2]),
                    ("RAS", mcb.io_ras),
                    ("CAS", mcb.io_cas),
                    ("WE", mcb.io_we),
                    ("ODT", mcb.io_odt),
                    ("CKE", mcb.io_cke),
                    ("RST", mcb.io_reset),
                ] {
                    if row == io.row {
                        pins[io.iob.to_idx()] = Some(pin);
                    }
                }
                if pins.iter().any(|x| x.is_some()) {
                    print!(
                        " MCB{i}.({p0},{p1})",
                        p0 = pins[0].unwrap(),
                        p1 = pins[1].unwrap()
                    );
                }
            }
            println!();
        }
        match grid.gts {
            Gts::None => (),
            Gts::Single(..) => println!("\tGTS: SINGLE"),
            Gts::Double(..) => println!("\tGTS: DOUBLE"),
            Gts::Quad(..) => println!("\tGTS: QUAD"),
        }
        println!("\tCFG PINS:");
        for (k, v) in &grid.cfg_io {
            println!(
                "\t\t{k:?}: IOB_X{x}Y{y}B{b}",
                x = v.col.to_idx(),
                y = v.row.to_idx(),
                b = v.iob.to_idx()
            );
        }
        if grid.has_encrypt {
            println!("\tHAS ENCRYPT");
        }
    }

    pub fn print_bond(bond: &Bond) {
        println!("\tBANKS:");
        for (k, v) in &bond.io_banks {
            println!("\t\t{k}: {v}");
        }
        println!("\tPINS:");
        for (pin, pad) in bond.pins.iter().sorted_by_key(|(k, _)| pad_sort_key(k)) {
            print!("\t\t{pin:4}: ");
            match pad {
                BondPin::Io(io) => print!("IOB_X{x}Y{y}B{b}", x = io.col, y = io.row, b = io.iob),
                BondPin::Gt(bank, gtpin) => {
                    print!("GT{bank}.");
                    match gtpin {
                        GtPin::RxP(idx) => print!("RXP{idx}"),
                        GtPin::RxN(idx) => print!("RXN{idx}"),
                        GtPin::TxP(idx) => print!("TXP{idx}"),
                        GtPin::TxN(idx) => print!("TXN{idx}"),
                        GtPin::VtRx => print!("VTRX"),
                        GtPin::VtTx => print!("VTTX"),
                        GtPin::ClkP(idx) => print!("CLKP{idx}"),
                        GtPin::ClkN(idx) => print!("CLKN{idx}"),
                        GtPin::AVcc => print!("AVCC"),
                        GtPin::AVccPll(idx) => print!("AVCCPLL{idx}"),
                        GtPin::RRef => print!("RREF"),
                        GtPin::AVttRCal => print!("AVTTRCAL"),
                    }
                }
                BondPin::Nc => print!("NC"),
                BondPin::Gnd => print!("GND"),
                BondPin::VccInt => print!("VCCINT"),
                BondPin::VccAux => print!("VCCAUX"),
                BondPin::VccO(bank) => print!("VCCO{bank}"),
                BondPin::VccBatt => print!("VCC_BATT"),
                BondPin::Cfg(CfgPin::Done) => print!("DONE"),
                BondPin::Cfg(CfgPin::ProgB) => print!("PROG_B"),
                BondPin::Cfg(CfgPin::Tck) => print!("TCK"),
                BondPin::Cfg(CfgPin::Tms) => print!("TMS"),
                BondPin::Cfg(CfgPin::Tdi) => print!("TDI"),
                BondPin::Cfg(CfgPin::Tdo) => print!("TDO"),
                BondPin::Cfg(CfgPin::Suspend) => print!("SUSPEND"),
                BondPin::Cfg(CfgPin::CmpCsB) => print!("CMPCS_B"),
                BondPin::Vfs => print!("VFS"),
                BondPin::RFuse => print!("RFUSE"),
            }
            println!();
        }
        println!("\tVREF:");
        for v in &bond.vref {
            println!(
                "\t\tIOB_X{x}Y{y}B{b}",
                x = v.col.to_idx(),
                y = v.row.to_idx(),
                b = v.iob.to_idx()
            );
        }
    }
}

mod virtex4 {
    use itertools::Itertools;
    use prjcombine_int::grid::{ColId, RowId};
    use prjcombine_virtex4::{
        bond::{Bond, BondPin, CfgPin, GtPin, GtRegion, GtRegionPin, GtzPin, PsPin, SysMonPin},
        grid::{ColumnKind, Grid, GridKind, Pcie2Kind},
    };
    use unnamed_entity::EntityId;

    use crate::pad_sort_key;

    pub fn print_grid(grid: &Grid) {
        println!("\tKIND: {v:?}", v = grid.kind);
        if grid.has_ps {
            println!("\tHAS PS");
        }
        if grid.has_slr {
            println!("\tHAS SLR");
        }
        if grid.has_no_tbuturn {
            println!("\tHAS NO TB UTURN");
        }
        println!("\tCOLS:");
        for (col, &cd) in &grid.columns {
            if grid.cols_vbrk.contains(&col) {
                println!("\t\t--- break");
            }
            print!("\t\tX{c}: ", c = col.to_idx());
            match cd {
                ColumnKind::Io => print!("IO"),
                ColumnKind::ClbLL => print!("CLBLL"),
                ColumnKind::ClbLM => print!("CLBLM"),
                ColumnKind::Bram => print!("BRAM"),
                ColumnKind::Dsp => print!("DSP"),
                ColumnKind::Gt => print!("GT"),
                ColumnKind::Cmt => print!("CMT"),
                ColumnKind::Clk => print!("CLK"),
                ColumnKind::Cfg => print!("CFG"),
            }
            if grid.cols_mgt_buf.contains(&col) {
                print!(" MGT_BUF");
            }
            if let Some((cl, cr)) = grid.cols_qbuf {
                if col == cl || col == cr {
                    print!(" QBUF");
                }
            }
            println!();
            if let Some(ref hard) = grid.col_hard {
                if hard.col == col {
                    for &row in &hard.rows_pcie {
                        println!("\t\t\tY{y}: PCIE", y = row.to_idx());
                    }
                    for &row in &hard.rows_emac {
                        println!("\t\t\tY{y}: EMAC", y = row.to_idx());
                    }
                }
            }
            for ioc in &grid.cols_io {
                if ioc.col == col {
                    for (reg, kind) in &ioc.regs {
                        if let Some(kind) = kind {
                            println!("\t\t\tY{y}: {kind:?}", y = grid.row_reg_bot(reg).to_idx());
                        }
                    }
                }
            }
            for gtc in &grid.cols_gt {
                if gtc.col == col {
                    for (reg, kind) in &gtc.regs {
                        if let Some(kind) = kind {
                            println!("\t\t\tY{y}: {kind:?}", y = grid.row_reg_bot(reg).to_idx());
                        }
                    }
                }
            }
            if cd == ColumnKind::Cfg {
                for &(row, kind) in &grid.rows_cfg {
                    println!("\t\t\tY{y}: {kind:?}", y = row.to_idx());
                }
            }
        }
        println!("\tREGS: {r}", r = grid.regs);
        println!("\tCFG REG: {v:?}", v = grid.reg_cfg.to_idx());
        println!("\tCLK REG: {v:?}", v = grid.reg_clk.to_idx());
        for &(col, row) in &grid.holes_ppc {
            let (col_r, row_t): (ColId, RowId) = match grid.kind {
                GridKind::Virtex4 => (col + 9, row + 24),
                GridKind::Virtex5 => (col + 14, row + 40),
                _ => unreachable!(),
            };
            println!(
                "\tPPC: X{xl}:X{xr} Y{yb}:Y{yt}",
                xl = col.to_idx(),
                xr = col_r.to_idx(),
                yb = row.to_idx(),
                yt = row_t.to_idx(),
            );
        }
        for pcie in &grid.holes_pcie2 {
            println!(
                "\tPCIE2.{lr}: X{xl}:X{xr} Y{yb}:Y{yt}",
                lr = match pcie.kind {
                    Pcie2Kind::Left => 'L',
                    Pcie2Kind::Right => 'R',
                },
                xl = pcie.col.to_idx(),
                xr = pcie.col.to_idx() + 4,
                yb = pcie.row.to_idx(),
                yt = pcie.row.to_idx() + 25
            );
        }
        for &(col, row) in &grid.holes_pcie3 {
            println!(
                "\tPCIE3: X{xl}:X{xr} Y{yb}:Y{yt}",
                xl = col.to_idx(),
                xr = col.to_idx() + 6,
                yb = row.to_idx(),
                yt = row.to_idx() + 50
            );
        }
        println!("\tHAS BRAM_FX: {v:?}", v = grid.has_bram_fx);
    }

    pub fn print_bond(bond: &Bond) {
        println!("\tPINS:");
        for (pin, pad) in bond.pins.iter().sorted_by_key(|(k, _)| pad_sort_key(k)) {
            print!("\t\t{pin:4}: ");
            match pad {
                BondPin::Io(bank, idx) => print!("IOB_{bank}_{idx}"),
                BondPin::Gt(bank, gtpin) => {
                    print!("GT{bank}.");
                    match gtpin {
                        GtPin::RxP(idx) => print!("RXP{idx}"),
                        GtPin::RxN(idx) => print!("RXN{idx}"),
                        GtPin::TxP(idx) => print!("TXP{idx}"),
                        GtPin::TxN(idx) => print!("TXN{idx}"),
                        GtPin::ClkP(idx) => print!("CLKP{idx}"),
                        GtPin::ClkN(idx) => print!("CLKN{idx}"),
                        GtPin::GndA => print!("GNDA"),
                        GtPin::AVccAuxTx => print!("AVCCAUXTX"),
                        GtPin::AVccAuxRx(idx) => print!("AVCCAUXRX{idx}"),
                        GtPin::AVccAuxMgt => print!("AVCCAUXMGT"),
                        GtPin::RTerm => print!("RTERM"),
                        GtPin::MgtVRef => print!("MGTVREF"),
                        GtPin::VtRx(idx) => print!("VTRX{idx}"),
                        GtPin::VtTx(idx) => print!("VTTX{idx}"),
                        GtPin::AVcc => print!("AVCC"),
                        GtPin::AVccPll => print!("AVCCPLL"),
                        GtPin::RRef => print!("RREF"),
                        GtPin::AVttRCal => print!("AVTTRCAL"),
                        GtPin::RBias => print!("RBIAS"),
                    }
                }
                BondPin::Gtz(bank, gtpin) => {
                    print!("GTZ{bank}.");
                    match gtpin {
                        GtzPin::RxP(idx) => print!("RXP{idx}"),
                        GtzPin::RxN(idx) => print!("RXN{idx}"),
                        GtzPin::TxP(idx) => print!("TXP{idx}"),
                        GtzPin::TxN(idx) => print!("TXN{idx}"),
                        GtzPin::ClkP(idx) => print!("CLKP{idx}"),
                        GtzPin::ClkN(idx) => print!("CLKN{idx}"),
                        GtzPin::AGnd => print!("AGND"),
                        GtzPin::AVcc => print!("AVCC"),
                        GtzPin::VccH => print!("VCCH"),
                        GtzPin::VccL => print!("VCCL"),
                        GtzPin::ObsClkP => print!("OBSCLKP"),
                        GtzPin::ObsClkN => print!("OBSCLKN"),
                        GtzPin::ThermIn => print!("THERM_IN"),
                        GtzPin::ThermOut => print!("THERM_OUT"),
                        GtzPin::SenseAGnd => print!("SENSE_AGND"),
                        GtzPin::SenseGnd => print!("SENSE_GND"),
                        GtzPin::SenseGndL => print!("SENSE_GNDL"),
                        GtzPin::SenseAVcc => print!("SENSE_AVCC"),
                        GtzPin::SenseVcc => print!("SENSE_VCC"),
                        GtzPin::SenseVccL => print!("SENSE_VCCL"),
                        GtzPin::SenseVccH => print!("SENSE_VCCH"),
                    }
                }
                BondPin::GtRegion(region, gtpin) => {
                    print!("GTREG");
                    match region {
                        GtRegion::All => (),
                        GtRegion::S => print!("S"),
                        GtRegion::N => print!("N"),
                        GtRegion::L => print!("L"),
                        GtRegion::R => print!("R"),
                        GtRegion::LS => print!("LS"),
                        GtRegion::RS => print!("RS"),
                        GtRegion::LN => print!("LN"),
                        GtRegion::RN => print!("RN"),
                        GtRegion::H => print!("H"),
                        GtRegion::LH => print!("LH"),
                        GtRegion::RH => print!("RH"),
                        GtRegion::Num(n) => print!("{n}"),
                    }
                    print!(".");
                    match gtpin {
                        GtRegionPin::AVtt => print!("AVTT"),
                        GtRegionPin::AGnd => print!("AGND"),
                        GtRegionPin::AVcc => print!("AVCC"),
                        GtRegionPin::AVccRx => print!("AVCCRX"),
                        GtRegionPin::AVccPll => print!("AVCCPLL"),
                        GtRegionPin::AVttRxC => print!("AVTTRXC"),
                        GtRegionPin::VccAux => print!("VCCAUX"),
                    }
                }
                BondPin::Nc => print!("NC"),
                BondPin::Gnd => print!("GND"),
                BondPin::VccInt => print!("VCCINT"),
                BondPin::VccAux => print!("VCCAUX"),
                BondPin::VccAuxIo(idx) => print!("VCCAUX_IO{idx}"),
                BondPin::VccBram => print!("VCCBRAM"),
                BondPin::VccO(bank) => print!("VCCO{bank}"),
                BondPin::VccBatt => print!("VCC_BATT"),
                BondPin::Cfg(CfgPin::Cclk) => print!("CCLK"),
                BondPin::Cfg(CfgPin::Done) => print!("DONE"),
                BondPin::Cfg(CfgPin::M0) => print!("M0"),
                BondPin::Cfg(CfgPin::M1) => print!("M1"),
                BondPin::Cfg(CfgPin::M2) => print!("M2"),
                BondPin::Cfg(CfgPin::ProgB) => print!("PROG_B"),
                BondPin::Cfg(CfgPin::InitB) => print!("INIT_B"),
                BondPin::Cfg(CfgPin::RdWrB) => print!("RDWR_B"),
                BondPin::Cfg(CfgPin::CsiB) => print!("CSI_B"),
                BondPin::Cfg(CfgPin::Tck) => print!("TCK"),
                BondPin::Cfg(CfgPin::Tms) => print!("TMS"),
                BondPin::Cfg(CfgPin::Tdi) => print!("TDI"),
                BondPin::Cfg(CfgPin::Tdo) => print!("TDO"),
                BondPin::Cfg(CfgPin::PwrdwnB) => print!("PWRDWN_B"),
                BondPin::Cfg(CfgPin::HswapEn) => print!("HSWAP_EN"),
                BondPin::Cfg(CfgPin::Din) => print!("DIN"),
                BondPin::Cfg(CfgPin::Dout) => print!("DOUT"),
                BondPin::Cfg(CfgPin::CfgBvs) => print!("CFGBVS"),
                BondPin::Dxn => print!("DXN"),
                BondPin::Dxp => print!("DXP"),
                BondPin::Rsvd => print!("RSVD"),
                BondPin::RsvdGnd => print!("RSVDGND"),
                BondPin::Vfs => print!("VFS"),
                BondPin::SysMon(bank, pin) => {
                    print!("SYSMON{bank}.");
                    match pin {
                        SysMonPin::VP => print!("VP"),
                        SysMonPin::VN => print!("VN"),
                        SysMonPin::AVss => print!("AVSS"),
                        SysMonPin::AVdd => print!("AVDD"),
                        SysMonPin::VRefP => print!("VREFP"),
                        SysMonPin::VRefN => print!("VREFN"),
                    }
                }
                BondPin::VccPsInt => print!("VCC_PS_INT"),
                BondPin::VccPsAux => print!("VCC_PS_AUX"),
                BondPin::VccPsPll => print!("VCC_PS_PLL"),
                BondPin::PsVref(bank, idx) => print!("PS{bank}.VREF{idx}"),
                BondPin::PsIo(bank, pin) => {
                    print!("PS{bank}.");
                    match pin {
                        PsPin::Mio(i) => print!("MIO{i}"),
                        PsPin::Clk => print!("CLK"),
                        PsPin::PorB => print!("POR_B"),
                        PsPin::SrstB => print!("SRST_B"),
                        PsPin::DdrDq(i) => print!("DDR_DQ{i}"),
                        PsPin::DdrDm(i) => print!("DDR_DM{i}"),
                        PsPin::DdrDqsP(i) => print!("DDR_DQS_P{i}"),
                        PsPin::DdrDqsN(i) => print!("DDR_DQS_N{i}"),
                        PsPin::DdrA(i) => print!("DDR_A{i}"),
                        PsPin::DdrBa(i) => print!("DDR_BA{i}"),
                        PsPin::DdrVrP => print!("DDR_VRP"),
                        PsPin::DdrVrN => print!("DDR_VRN"),
                        PsPin::DdrCkP => print!("DDR_CKP"),
                        PsPin::DdrCkN => print!("DDR_CKN"),
                        PsPin::DdrCke => print!("DDR_CKE"),
                        PsPin::DdrOdt => print!("DDR_ODT"),
                        PsPin::DdrDrstB => print!("DDR_DRST_B"),
                        PsPin::DdrCsB => print!("DDR_CS_B"),
                        PsPin::DdrRasB => print!("DDR_RAS_B"),
                        PsPin::DdrCasB => print!("DDR_CAS_B"),
                        PsPin::DdrWeB => print!("DDR_WE_B"),
                    }
                }
            }
            println!();
        }
    }
}

mod ultrascale {
    use itertools::Itertools;
    use prjcombine_ultrascale::{
        bond::{
            Bond, BondPin, CfgPin, GtPin, GtRegion, GtRegionPin, HbmPin, PsPin, RfAdcPin, RfDacPin,
            SysMonPin,
        },
        grid::{
            BramKind, CleLKind, CleMKind, ColumnKindLeft, ColumnKindRight, DspKind, Grid, HardKind,
        },
    };
    use unnamed_entity::EntityId;

    use crate::pad_sort_key;

    pub fn print_grid(grid: &Grid) {
        println!("\tKIND: {v:?}", v = grid.kind);
        if let Some(ps) = grid.ps {
            print!("\tPS {v:?}", v = ps.intf_kind);
            if ps.has_vcu {
                print!(" VCU");
            }
            println!();
        }
        if grid.has_hbm {
            println!("\tHAS HBM");
        }
        if grid.is_dmc {
            println!("\tIS DMC");
        }
        if grid.is_alt_cfg {
            println!("\tIS ALT CFG");
        }
        println!("\tCOLS:");
        for (col, cd) in &grid.columns {
            if grid.cols_vbrk.contains(&col) {
                println!("\t\t--- break");
            }
            if grid.cols_fsr_gap.contains(&col) {
                println!("\t\t--- FSR gap");
            }
            if matches!(
                cd.l,
                ColumnKindLeft::Uram
                    | ColumnKindLeft::Hard(_, _)
                    | ColumnKindLeft::DfeC
                    | ColumnKindLeft::DfeDF
                    | ColumnKindLeft::DfeE
            ) {
                print!("\t\tX{cl}.R-X{c}.L: ", cl = col - 1, c = col);
            } else {
                print!("\t\tX{c}.L: ", c = col.to_idx());
            }
            match cd.l {
                ColumnKindLeft::Io(_) => print!("IO"),
                ColumnKindLeft::Gt(_) => print!("GT"),
                ColumnKindLeft::CleL => print!("CLEL"),
                ColumnKindLeft::CleM(CleMKind::Plain) => print!("CLEM"),
                ColumnKindLeft::CleM(CleMKind::ClkBuf) => print!("CLEM.CLK"),
                ColumnKindLeft::CleM(CleMKind::Laguna) => print!("CLEM.LAGUNA"),
                ColumnKindLeft::Bram(BramKind::Plain) => print!("BRAM"),
                ColumnKindLeft::Bram(BramKind::AuxClmp) => print!("BRAM.AUX_CLMP"),
                ColumnKindLeft::Bram(BramKind::BramClmp) => print!("BRAM.BRAM_CLMP"),
                ColumnKindLeft::Bram(BramKind::AuxClmpMaybe) => print!("BRAM.AUX_CLMP*"),
                ColumnKindLeft::Bram(BramKind::BramClmpMaybe) => print!("BRAM.BRAM_CLMP*"),
                ColumnKindLeft::Bram(BramKind::Td) => print!("BRAM.TD"),
                ColumnKindLeft::Uram => print!("URAM"),
                ColumnKindLeft::Hard(hk, _) => {
                    print!("HARD{}", if hk == HardKind::Clk { " CLK" } else { "" })
                }
                ColumnKindLeft::Sdfec => print!("SDFEC"),
                ColumnKindLeft::DfeC => print!("DFE_C"),
                ColumnKindLeft::DfeDF => print!("DFE_DF"),
                ColumnKindLeft::DfeE => print!("DFE_E"),
            }
            if cd.clk_l.iter().any(|x| x.is_some()) {
                print!(" CLK");
                for v in cd.clk_l {
                    if let Some(v) = v {
                        print!(" {v}");
                    } else {
                        print!(" -");
                    }
                }
            }
            if let Some(ps) = grid.ps {
                if ps.col == col {
                    print!(" PS");
                }
            }
            println!();
            if let ColumnKindLeft::Io(idx) | ColumnKindLeft::Gt(idx) = cd.l {
                let ioc = &grid.cols_io[idx];
                for (reg, kind) in &ioc.regs {
                    println!("\t\t\tY{y}: {kind:?}", y = grid.row_reg_bot(reg).to_idx());
                }
            }
            if let ColumnKindLeft::Hard(_, idx) = cd.l {
                let hc = &grid.cols_hard[idx];
                for (reg, kind) in &hc.regs {
                    println!("\t\t\tY{y}: {kind:?}", y = grid.row_reg_bot(reg).to_idx());
                }
            }
            if matches!(
                cd.r,
                ColumnKindRight::Uram
                    | ColumnKindRight::Hard(HardKind::Clk | HardKind::NonClk, _)
                    | ColumnKindRight::DfeC
                    | ColumnKindRight::DfeDF
                    | ColumnKindRight::DfeE
            ) {
                continue;
            }
            print!("\t\tX{c}.R: ", c = col.to_idx());
            match cd.r {
                ColumnKindRight::Io(_) => print!("IO"),
                ColumnKindRight::Gt(_) => print!("GT"),
                ColumnKindRight::CleL(CleLKind::Plain) => print!("CLEL"),
                ColumnKindRight::CleL(CleLKind::Dcg10) => print!("CLEL.DCG10"),
                ColumnKindRight::Dsp(DspKind::Plain) => print!("DSP"),
                ColumnKindRight::Dsp(DspKind::ClkBuf) => print!("DSP.CLK"),
                ColumnKindRight::Uram => print!("URAM"),
                ColumnKindRight::Hard(_, _) => print!("HARD TERM"),
                ColumnKindRight::DfeB => print!("DFE_B"),
                ColumnKindRight::DfeC => print!("DFE_C"),
                ColumnKindRight::DfeDF => print!("DFE_DF"),
                ColumnKindRight::DfeE => print!("DFE_E"),
            }
            if cd.clk_r.iter().any(|x| x.is_some()) {
                print!(" CLK");
                for v in cd.clk_r {
                    if let Some(v) = v {
                        print!(" {v}");
                    } else {
                        print!(" -");
                    }
                }
            }
            println!();
            if let ColumnKindRight::Io(idx) | ColumnKindRight::Gt(idx) = cd.r {
                let ioc = &grid.cols_io[idx];
                for (reg, kind) in &ioc.regs {
                    println!("\t\t\tY{y}: {kind:?}", y = grid.row_reg_bot(reg).to_idx());
                }
            }
            if let ColumnKindRight::Hard(__, idx) = cd.r {
                let hc = &grid.cols_hard[idx];
                for (reg, kind) in &hc.regs {
                    println!("\t\t\tY{y}: {kind:?}", y = grid.row_reg_bot(reg).to_idx());
                }
            }
        }
        println!("\tREGS: {r}", r = grid.regs);
    }

    pub fn print_bond(bond: &Bond) {
        println!("\tPINS:");
        for (pin, pad) in bond.pins.iter().sorted_by_key(|(k, _)| pad_sort_key(k)) {
            print!("\t\t{pin:4}: ");
            match pad {
                BondPin::Hpio(bank, idx) => print!("HPIOB_{bank}_{idx}"),
                BondPin::Hdio(bank, idx) => print!("HDIOB_{bank}_{idx}"),
                BondPin::IoVref(bank) => print!("IO_{bank}_VREF"),
                BondPin::Gt(bank, gtpin) => {
                    print!("GT{bank}.");
                    match gtpin {
                        GtPin::RxP(idx) => print!("RXP{idx}"),
                        GtPin::RxN(idx) => print!("RXN{idx}"),
                        GtPin::TxP(idx) => print!("TXP{idx}"),
                        GtPin::TxN(idx) => print!("TXN{idx}"),
                        GtPin::ClkP(idx) => print!("CLKP{idx}"),
                        GtPin::ClkN(idx) => print!("CLKN{idx}"),
                        GtPin::AVcc => print!("AVCC"),
                        GtPin::RRef => print!("RREF"),
                        GtPin::AVttRCal => print!("AVTTRCAL"),
                        GtPin::AVtt => print!("AVTT"),
                    }
                }
                BondPin::GtRegion(region, gtpin) => {
                    print!("GTREG");
                    match region {
                        GtRegion::All => (),
                        GtRegion::L => print!("L"),
                        GtRegion::R => print!("R"),
                        GtRegion::LS => print!("LS"),
                        GtRegion::RS => print!("RS"),
                        GtRegion::LN => print!("LN"),
                        GtRegion::RN => print!("RN"),
                        GtRegion::LLC => print!("LLC"),
                        GtRegion::RLC => print!("RLC"),
                        GtRegion::LC => print!("LC"),
                        GtRegion::RC => print!("RC"),
                        GtRegion::LUC => print!("LUC"),
                        GtRegion::RUC => print!("RUC"),
                    }
                    print!(".");
                    match gtpin {
                        GtRegionPin::AVtt => print!("AVTT"),
                        GtRegionPin::AVcc => print!("AVCC"),
                        GtRegionPin::VccAux => print!("VCCAUX"),
                        GtRegionPin::VccInt => print!("VCCINT"),
                    }
                }
                BondPin::Nc => print!("NC"),
                BondPin::Gnd => print!("GND"),
                BondPin::VccInt => print!("VCCINT"),
                BondPin::VccAux => print!("VCCAUX"),
                BondPin::VccBram => print!("VCCBRAM"),
                BondPin::VccO(bank) => print!("VCCO{bank}"),
                BondPin::VccBatt => print!("VCC_BATT"),
                BondPin::Cfg(CfgPin::Cclk) => print!("CCLK"),
                BondPin::Cfg(CfgPin::Done) => print!("DONE"),
                BondPin::Cfg(CfgPin::M0) => print!("M0"),
                BondPin::Cfg(CfgPin::M1) => print!("M1"),
                BondPin::Cfg(CfgPin::M2) => print!("M2"),
                BondPin::Cfg(CfgPin::ProgB) => print!("PROG_B"),
                BondPin::Cfg(CfgPin::InitB) => print!("INIT_B"),
                BondPin::Cfg(CfgPin::RdWrB) => print!("RDWR_B"),
                BondPin::Cfg(CfgPin::Tck) => print!("TCK"),
                BondPin::Cfg(CfgPin::Tms) => print!("TMS"),
                BondPin::Cfg(CfgPin::Tdi) => print!("TDI"),
                BondPin::Cfg(CfgPin::Tdo) => print!("TDO"),
                BondPin::Cfg(CfgPin::HswapEn) => print!("HSWAP_EN"),
                BondPin::Cfg(CfgPin::Data(idx)) => print!("DATA{idx}"),
                BondPin::Cfg(CfgPin::CfgBvs) => print!("CFGBVS"),
                BondPin::Cfg(CfgPin::PorOverride) => print!("POR_OVERRIDE"),
                BondPin::Dxn => print!("DXN"),
                BondPin::Dxp => print!("DXP"),
                BondPin::Rsvd => print!("RSVD"),
                BondPin::RsvdGnd => print!("RSVDGND"),
                BondPin::SysMon(bank, pin) => {
                    print!("SYSMON{bank}.");
                    match pin {
                        SysMonPin::VP => print!("VP"),
                        SysMonPin::VN => print!("VN"),
                    }
                }
                BondPin::VccPsAux => print!("VCC_PS_AUX"),
                BondPin::VccPsPll => print!("VCC_PS_PLL"),
                BondPin::IoPs(bank, pin) => {
                    print!("PS{bank}.");
                    match pin {
                        PsPin::Mio(i) => print!("MIO{i}"),
                        PsPin::Clk => print!("CLK"),
                        PsPin::PorB => print!("POR_B"),
                        PsPin::SrstB => print!("SRST_B"),
                        PsPin::DdrDq(i) => print!("DDR_DQ{i}"),
                        PsPin::DdrDm(i) => print!("DDR_DM{i}"),
                        PsPin::DdrDqsP(i) => print!("DDR_DQS_P{i}"),
                        PsPin::DdrDqsN(i) => print!("DDR_DQS_N{i}"),
                        PsPin::DdrA(i) => print!("DDR_A{i}"),
                        PsPin::DdrBa(i) => print!("DDR_BA{i}"),
                        PsPin::DdrCkP(idx) => print!("DDR_CKP{idx}"),
                        PsPin::DdrCkN(idx) => print!("DDR_CKN{idx}"),
                        PsPin::DdrCke(idx) => print!("DDR_CKE{idx}"),
                        PsPin::DdrOdt(idx) => print!("DDR_ODT{idx}"),
                        PsPin::DdrCsB(idx) => print!("DDR_CS_B{idx}"),
                        PsPin::DdrDrstB => print!("DDR_DRST_B"),
                        PsPin::DdrActN => print!("DDR_ACT_N"),
                        PsPin::DdrAlertN => print!("DDR_ALERT_N"),
                        PsPin::DdrBg(idx) => print!("DDR_BG{idx}"),
                        PsPin::DdrParity => print!("DDR_PARITY"),
                        PsPin::DdrZq => print!("DDR_ZQ"),
                        PsPin::ErrorOut => print!("ERROR_OUT"),
                        PsPin::ErrorStatus => print!("ERROR_STATUS"),
                        PsPin::Done => print!("DONE"),
                        PsPin::InitB => print!("INIT_B"),
                        PsPin::ProgB => print!("PROG_B"),
                        PsPin::JtagTck => print!("JTAG_TCK"),
                        PsPin::JtagTdi => print!("JTAG_TDI"),
                        PsPin::JtagTdo => print!("JTAG_TDO"),
                        PsPin::JtagTms => print!("JTAG_TMS"),
                        PsPin::Mode(i) => print!("MODE{i}"),
                        PsPin::PadI => print!("PAD_I"),
                        PsPin::PadO => print!("PAD_O"),
                    }
                }
                BondPin::SysMonVRefP => print!("SYSMON_VREFP"),
                BondPin::SysMonVRefN => print!("SYSMON_VREFN"),
                BondPin::SysMonGnd => print!("SYSMON_GND"),
                BondPin::SysMonVcc => print!("SYSMON_VCC"),
                BondPin::PsSysMonGnd => print!("PS_SYSMON_GND"),
                BondPin::PsSysMonVcc => print!("PS_SYSMON_VCC"),
                BondPin::VccAuxHpio => print!("VCCAUX_HPIO"),
                BondPin::VccAuxHdio => print!("VCCAUX_HDIO"),
                BondPin::VccAuxIo => print!("VCCAUX_IO"),
                BondPin::VccIntIo => print!("VCCINT_IO"),
                BondPin::VccPsIntLp => print!("VCC_PS_INT_LP"),
                BondPin::VccPsIntFp => print!("VCC_PS_INT_FP"),
                BondPin::VccPsIntFpDdr => print!("VCC_PS_INT_FP_DDR"),
                BondPin::VccPsBatt => print!("VCC_PS_BATT"),
                BondPin::VccPsDdrPll => print!("VCC_PS_DDR_PLL"),
                BondPin::VccIntVcu => print!("VCCINT_VCU"),
                BondPin::GndSense => print!("GND_SENSE"),
                BondPin::VccIntSense => print!("VCCINT_SENSE"),
                BondPin::VccIntAms => print!("VCCINT_AMS"),
                BondPin::VccSdfec => print!("VCC_SDFEC"),
                BondPin::RfDacGnd => print!("RFDAC_GND"),
                BondPin::RfDacSubGnd => print!("RFDAC_AGND"),
                BondPin::RfDacAVcc => print!("RFDAC_AVCC"),
                BondPin::RfDacAVccAux => print!("RFDAC_AVCCAUX"),
                BondPin::RfDacAVtt => print!("RFDAC_AVTT"),
                BondPin::RfAdcGnd => print!("RFADC_GND"),
                BondPin::RfAdcSubGnd => print!("RFADC_SUBGND"),
                BondPin::RfAdcAVcc => print!("RFADC_AVCC"),
                BondPin::RfAdcAVccAux => print!("RFADC_AVCCAUX"),
                BondPin::Hbm(bank, pin) => {
                    print!("HBM{bank}.");
                    match pin {
                        HbmPin::Vcc => print!("VCC"),
                        HbmPin::VccIo => print!("VCCIO"),
                        HbmPin::VccAux => print!("VCCAUX"),
                        HbmPin::Rsvd => print!("RSVD"),
                        HbmPin::RsvdGnd => print!("RSVD_GND"),
                    }
                }
                BondPin::RfDac(bank, pin) => {
                    print!("RFDAC{bank}.");
                    match pin {
                        RfDacPin::VOutP(idx) => print!("VOUT{idx}P"),
                        RfDacPin::VOutN(idx) => print!("VOUT{idx}N"),
                        RfDacPin::ClkP => print!("CLKP"),
                        RfDacPin::ClkN => print!("CLKN"),
                        RfDacPin::RExt => print!("REXT"),
                        RfDacPin::SysRefP => print!("SYSREFP"),
                        RfDacPin::SysRefN => print!("SYSREFN"),
                    }
                }
                BondPin::RfAdc(bank, pin) => {
                    print!("RFADC{bank}.");
                    match pin {
                        RfAdcPin::VInP(idx) => print!("VIN{idx}_P"),
                        RfAdcPin::VInN(idx) => print!("VIN{idx}_N"),
                        RfAdcPin::VInPairP(idx) => print!("VIN_PAIR{idx}_P"),
                        RfAdcPin::VInPairN(idx) => print!("VIN_PAIR{idx}_N"),
                        RfAdcPin::ClkP => print!("CLKP"),
                        RfAdcPin::ClkN => print!("CLKN"),
                        RfAdcPin::VCm(idx) => print!("VCM{idx}"),
                        RfAdcPin::RExt => print!("REXT"),
                        RfAdcPin::PllTestOutP => print!("PLL_TEST_OUT_P"),
                        RfAdcPin::PllTestOutN => print!("PLL_TEST_OUT_N"),
                    }
                }
            }
            println!();
        }
    }
}

mod versal {
    use prjcombine_versal::{
        bond::Bond,
        grid::{ColumnKind, Grid},
    };
    use unnamed_entity::EntityId;

    pub fn print_grid(grid: &Grid) {
        println!("\tKIND: Versal");
        println!("\tPS: {v:?}", v = grid.ps);
        println!("\tCPM: {v:?}", v = grid.cpm);
        println!("\tHNICX: {v:?}", v = grid.has_hnicx);
        println!("\tXRAM TOP: {v:?}", v = grid.has_xram_top);
        println!("\tTOP: {v:?}", v = grid.top);
        println!("\tBOTTOM: {v:?}", v = grid.bottom);
        println!("\tCOLS:");
        for (col, cd) in &grid.columns {
            if grid.cols_vbrk.contains(&col) {
                println!("\t\t--- break");
            }
            if grid.cols_cpipe.contains(&col) {
                println!("\t\t--- CPIPE");
            }
            if matches!(
                cd.l,
                ColumnKind::Cle
                    | ColumnKind::CleLaguna
                    | ColumnKind::Dsp
                    | ColumnKind::Hard
                    | ColumnKind::VNoc
                    | ColumnKind::VNoc2
            ) {
                print!("\t\tX{cl}.R-X{col}.L: ", cl = col - 1);
            } else {
                print!("\t\tX{c}.L: ", c = col.to_idx());
            }
            match cd.l {
                ColumnKind::None => print!("---"),
                ColumnKind::Cle => print!("CLE"),
                ColumnKind::CleLaguna => print!("CLE.LAGUNA"),
                ColumnKind::Dsp => print!("DSP"),
                ColumnKind::Bram => print!("BRAM"),
                ColumnKind::BramClkBuf => print!("BRAM.CLK"),
                ColumnKind::Uram => print!("URAM"),
                ColumnKind::Hard => print!("HARD"),
                ColumnKind::Gt => print!("GT"),
                ColumnKind::Cfrm => print!("CFRM"),
                ColumnKind::VNoc => print!("VNOC"),
                ColumnKind::VNoc2 => print!("VNOC2"),
            }
            if cd.has_bli_bot_l {
                print!(" BLI.BOT");
            }
            if cd.has_bli_top_l {
                print!(" BLI.TOP");
            }
            println!();
            for hc in &grid.cols_hard {
                if hc.col == col {
                    for (reg, kind) in &hc.regs {
                        println!("\t\t\tY{y}: {kind:?}", y = grid.row_reg_bot(reg).to_idx());
                    }
                }
            }
            if matches!(
                cd.r,
                ColumnKind::Cle
                    | ColumnKind::CleLaguna
                    | ColumnKind::Dsp
                    | ColumnKind::Hard
                    | ColumnKind::VNoc
                    | ColumnKind::VNoc2
            ) {
                continue;
            }
            print!("\t\tX{c}.R: ", c = col.to_idx());
            match cd.r {
                ColumnKind::None => print!("---"),
                ColumnKind::Cle => print!("CLE"),
                ColumnKind::CleLaguna => print!("CLE.LAGUNA"),
                ColumnKind::Dsp => print!("DSP"),
                ColumnKind::Bram => print!("BRAM"),
                ColumnKind::BramClkBuf => print!("BRAM.CLK"),
                ColumnKind::Uram => print!("URAM"),
                ColumnKind::Hard => print!("HARD"),
                ColumnKind::Gt => print!("GT"),
                ColumnKind::Cfrm => print!("CFRM"),
                ColumnKind::VNoc => print!("VNOC"),
                ColumnKind::VNoc2 => print!("VNOC2"),
            }
            if cd.has_bli_bot_r {
                print!(" BLI.BOT");
            }
            if cd.has_bli_top_r {
                print!(" BLI.TOP");
            }
            println!();
        }
        println!("\tGT LEFT:");
        for (reg, kind) in &grid.regs_gt_left {
            println!("\t\tY{y}: {kind:?}", y = grid.row_reg_bot(reg).to_idx());
        }
        if let Some(ref regs) = grid.regs_gt_right {
            println!("\tGT RIGHT:");
            for (reg, kind) in regs {
                println!("\t\tY{y}: {kind:?}", y = grid.row_reg_bot(reg).to_idx());
            }
        }
        println!("\tREGS: {r}", r = grid.regs);
    }

    pub fn print_bond(_bond: &Bond) {
        // well.
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let geom = GeomDb::from_file(args.file)?;
    if args.intdb {
        for intdb in geom.ints.values() {
            intdb.print(&mut std::io::stdout())?;
        }
    }
    if args.grids || args.devices {
        for (gid, grid) in &geom.grids {
            print!("GRID {gid}:", gid = gid.to_idx());
            for dev in &geom.devices {
                for (did, &die) in &dev.grids {
                    if die == gid {
                        if dev.grids.len() == 1 {
                            print!(" {dev}", dev = dev.name);
                        } else {
                            print!(" {dev}.{did}", dev = dev.name, did = did.to_idx());
                        }
                    }
                }
            }
            println!();
            if args.grids {
                match grid {
                    Grid::Xc4000(g) => xc4000::print_grid(g),
                    Grid::Xc5200(g) => xc5200::print_grid(g),
                    Grid::Virtex(g) => virtex::print_grid(g),
                    Grid::Virtex2(g) => virtex2::print_grid(g),
                    Grid::Spartan6(g) => spartan6::print_grid(g),
                    Grid::Virtex4(g) => virtex4::print_grid(g),
                    Grid::Ultrascale(g) => ultrascale::print_grid(g),
                    Grid::Versal(g) => versal::print_grid(g),
                }
            }
        }
    }
    if args.pkgs || args.devices {
        for (bid, bond) in &geom.bonds {
            print!("BOND {bid}:", bid = bid.to_idx());
            for dev in &geom.devices {
                for dbond in dev.bonds.values() {
                    if dbond.bond == bid {
                        print!(" {dev}-{pkg}", dev = dev.name, pkg = dbond.name);
                    }
                }
            }
            println!();
            if args.pkgs {
                match bond {
                    Bond::Xc4000(bond) => xc4000::print_bond(bond),
                    Bond::Xc5200(bond) => xc5200::print_bond(bond),
                    Bond::Virtex(bond) => virtex::print_bond(bond),
                    Bond::Virtex2(bond) => virtex2::print_bond(bond),
                    Bond::Spartan6(bond) => spartan6::print_bond(bond),
                    Bond::Virtex4(bond) => virtex4::print_bond(bond),
                    Bond::Ultrascale(bond) => ultrascale::print_bond(bond),
                    Bond::Versal(bond) => versal::print_bond(bond),
                }
            }
        }
    }
    if args.devices {
        for dev in &geom.devices {
            print!("DEVICE {n} GRIDS", n = dev.name);
            for (did, &gid) in &dev.grids {
                print!(" {g}", g = gid.to_idx());
                if did == dev.grid_master {
                    print!("*");
                }
            }
            if !dev.extras.is_empty() {
                print!(" EXTRAS");
                for xtra in &dev.extras {
                    print!(" {xtra:?}");
                }
            }
            println!();
            for disabled in &dev.disabled {
                println!("\tDISABLED {disabled:?}");
            }
            for bond in dev.bonds.values() {
                println!("\tBOND {n}: {i}", n = bond.name, i = bond.bond.to_idx());
            }
            for combo in &dev.combos {
                println!(
                    "\tPART {n}: {bn} {sn}",
                    n = combo.name,
                    bn = dev.bonds[combo.devbond_idx].name,
                    sn = dev.speeds[combo.speed_idx]
                );
            }
            if geom.dev_namings[dev.naming] != DeviceNaming::Dummy {
                println!("\tNAMING {n}", n = dev.naming.to_idx());
            }
        }
    }
    if args.devices || args.namings {
        for (dnid, dn) in geom.dev_namings {
            print!("NAMING {dnid}:", dnid = dnid.to_idx());
            for dev in &geom.devices {
                if dev.naming == dnid {
                    print!(" {dev}", dev = dev.name);
                }
            }
            println!();
            if args.namings {
                // XXX pretty
                println!("{dn:#?}");
            }
        }
    }
    Ok(())
}
