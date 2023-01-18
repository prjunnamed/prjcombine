use prjcombine_entity::EntityId;
use prjcombine_xilinx_geom::{DeviceNaming, GeomDb, Grid};
use std::error::Error;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "xgprint", about = "Dump Xilinx geom file.")]
struct Opt {
    file: String,
    #[structopt(short, long)]
    intdb: bool,
    #[structopt(short, long)]
    devices: bool,
    #[structopt(short, long)]
    grids: bool,
    #[structopt(short, long)]
    pkgs: bool,
    #[structopt(short, long)]
    namings: bool,
}

mod xc4k {
    use prjcombine_entity::EntityId;
    use prjcombine_xc4k::grid::Grid;

    pub fn print_grid(grid: &Grid) {
        println!("\tKIND: {k:?}", k = grid.kind);
        println!("\tDIMS: {c}×{r}", c = grid.columns, r = grid.rows);
        println!("\tIS BUFF LARGE: {v}", v = grid.is_buff_large);
        println!("\tCFG PINS:");
        for (k, v) in &grid.cfg_io {
            println!(
                "\t\t{k:?}: X{x}Y{y}B{b}",
                x = v.col.to_idx(),
                y = v.row.to_idx(),
                b = v.iob.to_idx()
            );
        }
    }
}

mod xc5200 {
    use prjcombine_xc5200::grid::Grid;

    pub fn print_grid(grid: &Grid) {
        println!("\tKIND: Xc5200");
        println!("\tDIMS: {c}×{r}", c = grid.columns, r = grid.rows);
    }
}

mod virtex {
    use prjcombine_entity::EntityId;
    use prjcombine_virtex::grid::Grid;

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
                "\t\t{k:?}: X{x}Y{y}B{b}",
                x = v.col.to_idx(),
                y = v.row.to_idx(),
                b = v.iob.to_idx()
            );
        }
        println!("\tVREF:");
        for v in &grid.vref {
            println!(
                "\t\tX{x}Y{y}B{b}",
                x = v.col.to_idx(),
                y = v.row.to_idx(),
                b = v.iob.to_idx()
            );
        }
    }
}

mod virtex2 {
    use prjcombine_entity::EntityId;
    use prjcombine_virtex2::grid::{ColumnIoKind, ColumnKind, Grid, RowIoKind};

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
                "\t\t{k:?}: X{x}Y{y}B{b}",
                x = v.col.to_idx(),
                y = v.row.to_idx(),
                b = v.iob.to_idx()
            );
        }
        println!("\tVREF:");
        for v in &grid.vref {
            println!(
                "\t\tX{x}Y{y}B{b}",
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
}

mod spartan6 {
    use prjcombine_entity::EntityId;
    use prjcombine_spartan6::grid::{ColumnIoKind, ColumnKind, Grid, Gts};

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
                "\t\t{k:?}: X{x}Y{y}B{b}",
                x = v.col.to_idx(),
                y = v.row.to_idx(),
                b = v.iob.to_idx()
            );
        }
        println!("\tVREF:");
        for v in &grid.vref {
            println!(
                "\t\tX{x}Y{y}B{b}",
                x = v.col.to_idx(),
                y = v.row.to_idx(),
                b = v.iob.to_idx()
            );
        }
        if grid.has_encrypt {
            println!("\tHAS ENCRYPT");
        }
    }
}

mod virtex4 {
    use prjcombine_entity::EntityId;
    use prjcombine_virtex4::grid::{ColumnKind, Grid, GridKind, Pcie2Kind};

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
            let (col_r, row_t) = match grid.kind {
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
}

mod ultrascale {
    use prjcombine_entity::EntityId;
    use prjcombine_ultrascale::grid::{
        BramKind, CleLKind, CleMKind, ColumnKindLeft, ColumnKindRight, DspKind, Grid,
    };

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
                    | ColumnKindLeft::Hard(_)
                    | ColumnKindLeft::DfeC
                    | ColumnKindLeft::DfeDF
                    | ColumnKindLeft::DfeE
            ) {
                print!(
                    "\t\tX{cl}.R-X{c}.L: ",
                    cl = (col - 1).to_idx(),
                    c = col.to_idx()
                );
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
                ColumnKindLeft::Hard(_) => print!("HARD"),
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
            if let ColumnKindLeft::Hard(idx) = cd.l {
                let hc = &grid.cols_hard[idx];
                for (reg, kind) in &hc.regs {
                    println!("\t\t\tY{y}: {kind:?}", y = grid.row_reg_bot(reg).to_idx());
                }
            }
            if matches!(
                cd.r,
                ColumnKindRight::Uram
                    | ColumnKindRight::Hard(_)
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
                ColumnKindRight::Hard(_) => print!("HARD"),
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
        }
        println!("\tREGS: {r}", r = grid.regs);
    }
}

mod versal {
    use prjcombine_entity::EntityId;
    use prjcombine_versal::grid::{ColumnKind, Grid};

    pub fn print_grid(grid: &Grid) {
        println!("\tKIND: Versal");
        println!("\tPS: {v:?}", v = grid.ps);
        println!("\tCPM: {v:?}", v = grid.cpm);
        println!("\tHNICX: {v:?}", v = grid.has_hnicx);
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
                print!(
                    "\t\tX{cl}.R-X{c}.L: ",
                    cl = (col - 1).to_idx(),
                    c = col.to_idx()
                );
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
}

fn main() -> Result<(), Box<dyn Error>> {
    let opt = Opt::from_args();
    let geom = GeomDb::from_file(opt.file)?;
    if opt.intdb {
        for intdb in geom.ints.values() {
            intdb.print(&mut std::io::stdout())?;
        }
    }
    if opt.grids || opt.devices {
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
            if opt.grids {
                match grid {
                    Grid::Xc4k(g) => xc4k::print_grid(g),
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
    if opt.pkgs || opt.devices {
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
            if opt.pkgs {
                // XXX pretty
                println!("{bond:#?}");
            }
        }
    }
    if opt.devices {
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
    if opt.devices || opt.namings {
        for (dnid, dn) in geom.dev_namings {
            print!("NAMING {dnid}:", dnid = dnid.to_idx());
            for dev in &geom.devices {
                if dev.naming == dnid {
                    print!(" {dev}", dev = dev.name);
                }
            }
            println!();
            if opt.namings {
                // XXX pretty
                println!("{dn:#?}");
            }
        }
    }
    Ok(())
}
