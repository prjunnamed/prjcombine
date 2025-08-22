use std::collections::{HashMap, HashSet};

use prjcombine_interconnect::{
    db::WireSlotId,
    dir::Dir,
    grid::{CellCoord, ColId, DieId, RowId, WireCoord},
};
use prjcombine_siliconblue::{
    chip::{ChipKind, SpecialIoKey, SpecialTileKey},
    expanded::ExpandedDevice,
};
use unnamed_entity::EntityId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GenericNet {
    Int(WireCoord),
    Cmux(CellCoord),
    Gbout(CellCoord, usize),
    Cout(CellCoord, usize),
    Ltin(CellCoord),
    Ltout(CellCoord, usize),
    GlobalPadIn(CellCoord),
    GlobalClkh,
    GlobalClkl,
    DummyHold(CellCoord),
    CascAddr(CellCoord, usize),
    Unknown,
}

pub fn xlat_wire(edev: &ExpandedDevice, x: u32, y: u32, name: &str) -> GenericNet {
    fn wire_sp4_h(n: &str, is_l: bool) -> String {
        let n: u32 = n.parse().unwrap();
        let mut seg = n / 12;
        let mut which = n % 12;
        which ^= seg & 1;
        if is_l {
            seg += 1;
        }
        format!("QUAD.H{which}.{seg}")
    }

    fn wire_sp4_io_h(n: &str, is_l: bool) -> String {
        let n: u32 = n.parse().unwrap();
        let mut seg = n / 4;
        let which = n % 4;
        if is_l {
            seg += 1;
        }
        format!("QUAD.H{which}.{seg}")
    }

    fn wire_sp4_v(n: &str, is_b: bool, is_r: bool) -> String {
        let n: u32 = n.parse().unwrap();
        let mut seg = 3 - n / 12;
        let mut which = n % 12;
        which ^= seg & 1;
        if is_b {
            seg += 1;
        }
        if is_r {
            format!("QUAD.V{which}.{seg}.W")
        } else {
            format!("QUAD.V{which}.{seg}")
        }
    }

    fn wire_sp4_io_v(n: &str, is_b: bool) -> String {
        let n: u32 = n.parse().unwrap();
        let mut seg = 3 - n / 4;
        let which = n % 4;
        if is_b {
            seg += 1;
        }
        format!("QUAD.V{which}.{seg}")
    }

    fn wire_sp12_h(n: &str, is_l: bool) -> String {
        let n: u32 = n.parse().unwrap();
        let mut seg = n / 2;
        let mut which = n % 2;
        which ^= seg & 1;
        if is_l {
            seg += 1;
        }
        format!("LONG.H{which}.{seg}")
    }

    fn wire_sp12_v(n: &str, is_b: bool) -> String {
        let n: u32 = n.parse().unwrap();
        let mut seg = 11 - n / 2;
        let mut which = n % 2;
        which ^= seg & 1;
        if is_b {
            seg += 1;
        }
        format!("LONG.V{which}.{seg}")
    }

    let mut cell = CellCoord::new(
        DieId::from_idx(0),
        ColId::from_idx(x as usize),
        RowId::from_idx(y as usize),
    );
    let wname: String;
    match name {
        "wire_io_cluster/io_0/gbout" | "gbout_0" => return GenericNet::Gbout(cell, 0),
        "wire_io_cluster/io_1/gbout" | "gbout_1" => return GenericNet::Gbout(cell, 1),
        "wire_io_cluster/io_0/D_IN_0" => wname = "OUT.LC0".into(),
        "wire_io_cluster/io_0/D_IN_1" => wname = "OUT.LC1".into(),
        "wire_io_cluster/io_1/D_IN_0" => wname = "OUT.LC2".into(),
        "wire_io_cluster/io_1/D_IN_1" => wname = "OUT.LC3".into(),
        "wire_io_cluster/io_0/D_OUT_0" | "dout_0" | "pad_out_0" => wname = "IMUX.IO0.DOUT0".into(),
        "wire_io_cluster/io_0/D_OUT_1" | "dout_1" => wname = "IMUX.IO0.DOUT1".into(),
        "wire_io_cluster/io_0/OUT_ENB" => wname = "IMUX.IO0.OE".into(),
        "wire_io_cluster/io_1/D_OUT_0" | "dout_2" | "pad_out_1" => wname = "IMUX.IO1.DOUT0".into(),
        "wire_io_cluster/io_1/D_OUT_1" | "dout_3" => wname = "IMUX.IO1.DOUT1".into(),
        "pad_in_0" => wname = "OUT.LC0".into(),
        "pad_in_1" => wname = "OUT.LC2".into(),
        "wire_io_cluster/io_1/OUT_ENB" => wname = "IMUX.IO1.OE".into(),
        "inclk" | "wire_io_cluster/io_0/inclk" | "wire_io_cluster/io_1/inclk" => {
            wname = "IMUX.IO.ICLK".into()
        }
        "outclk" | "wire_io_cluster/io_0/outclk" | "wire_io_cluster/io_1/outclk" => {
            wname = "IMUX.IO.OCLK".into()
        }
        "cen"
        | "wire_logic_cluster/ram/RCLKE"
        | "wire_logic_cluster/ram/WCLKE"
        | "wire_bram/ram/RCLKE"
        | "wire_bram/ram/WCLKE"
        | "wire_io_cluster/io_0/cen"
        | "wire_io_cluster/io_1/cen" => wname = "IMUX.CE".into(),
        "clk"
        | "wire_logic_cluster/ram/RCLK"
        | "wire_logic_cluster/ram/WCLK"
        | "wire_bram/ram/RCLK"
        | "wire_bram/ram/WCLK" => wname = "IMUX.CLK".into(),
        "s_r"
        | "wire_logic_cluster/ram/RE"
        | "wire_logic_cluster/ram/WE"
        | "wire_bram/ram/RE"
        | "wire_bram/ram/WE" => wname = "IMUX.RST".into(),
        "fabout" | "wire_gbuf/in" | "wire_gbuf/out" => wname = "IMUX.IO.EXTRA".into(),
        "padin" | "wire_pll/outglobal" => return GenericNet::GlobalPadIn(cell),
        "clklf" => {
            assert_eq!(edev.chip.kind, ChipKind::Ice40T01);
            return GenericNet::GlobalClkl;
        }
        "clkhf" => {
            assert_eq!(edev.chip.kind, ChipKind::Ice40T01);
            return GenericNet::GlobalClkh;
        }
        "wire_pll/outglobalb" => return GenericNet::GlobalPadIn(cell.delta(1, 0)),
        "hold" | "wire_io_cluster/io_0/hold" | "wire_io_cluster/io_1/hold" => {
            let wire = edev.egrid.db.get_wire("IMUX.IO.EXTRA");
            let edge = if cell.col == edev.chip.col_w() {
                Dir::W
            } else if cell.col == edev.chip.col_e() {
                Dir::E
            } else if cell.row == edev.chip.row_s() {
                Dir::S
            } else if cell.row == edev.chip.row_n() {
                Dir::N
            } else {
                unreachable!()
            };
            if let Some(special) = edev.chip.special_tiles.get(&SpecialTileKey::LatchIo(edge)) {
                return GenericNet::Int(special.cells.first().unwrap().wire(wire));
            } else {
                return GenericNet::DummyHold(cell);
            }
        }
        "ltoutIn" | "wire_bram/ram/NC_5" => {
            if cell.col == edev.chip.col_w()
                || cell.col == edev.chip.col_e()
                || (edev.chip.cols_bram.contains(&cell.col)
                    && !matches!(edev.chip.kind, ChipKind::Ice40P08 | ChipKind::Ice40P01))
            {
                return GenericNet::Ltin(cell);
            } else {
                return GenericNet::Ltout(cell.delta(0, -1), 7);
            }
        }
        "ltoutOut" | "wire_bram/ram/NC_6" => return GenericNet::Ltout(cell, 7),
        "wire_logic_cluster/carry_in_mux/cout"
        | "wire_mult/carry_in_mux/cout"
        | "wire_con_box/carry_in_mux/cout" => return GenericNet::Cmux(cell),
        "wire_pll/outcoreb" | "outcoreb" => {
            cell.col += 1;
            wname = "OUT.LC0".into();
        }
        "wire_pll/outcore" => {
            wname = "OUT.LC2".into();
        }
        _ => {
            if let Some(suf) = name.strip_prefix("wire_logic_cluster/lc_") {
                let (lc, suf) = suf.split_once('/').unwrap();
                let lc: usize = lc.parse().unwrap();
                match suf {
                    "ltout" => return GenericNet::Ltout(cell, lc),
                    "cout" => return GenericNet::Cout(cell, lc),
                    "out" => wname = format!("OUT.LC{lc}"),
                    "clk" => wname = "IMUX.CLK".into(),
                    "s_r" => wname = "IMUX.RST".into(),
                    "cen" => wname = "IMUX.CE".into(),
                    "in_0" => wname = format!("IMUX.LC{lc}.I0"),
                    "in_1" => wname = format!("IMUX.LC{lc}.I1"),
                    "in_2" => wname = format!("IMUX.LC{lc}.I2"),
                    "in_3" => wname = format!("IMUX.LC{lc}.I3"),
                    _ => return GenericNet::Unknown,
                }
            } else if let Some(suf) = name.strip_prefix("wire_mult/lc_") {
                let (lc, suf) = suf.split_once('/').unwrap();
                let lc: usize = lc.parse().unwrap();
                match suf {
                    "cout" => return GenericNet::Cout(cell, lc),
                    "out" => wname = format!("OUT.LC{lc}"),
                    "clk" => wname = "IMUX.CLK".into(),
                    "s_r" => wname = "IMUX.RST".into(),
                    "cen" => wname = "IMUX.CE".into(),
                    "in_0" => wname = format!("IMUX.LC{lc}.I0"),
                    "in_1" => wname = format!("IMUX.LC{lc}.I1"),
                    "in_2" => wname = format!("IMUX.LC{lc}.I2"),
                    "in_3" => wname = format!("IMUX.LC{lc}.I3"),
                    _ => return GenericNet::Unknown,
                }
            } else if let Some(suf) = name.strip_prefix("wire_con_box/lc_") {
                let (lc, suf) = suf.split_once('/').unwrap();
                let lc: usize = lc.parse().unwrap();
                match suf {
                    "cout" => return GenericNet::Cout(cell, lc),
                    "in_0" => wname = format!("IMUX.LC{lc}.I0"),
                    "in_1" => wname = format!("IMUX.LC{lc}.I1"),
                    "in_3" => wname = format!("IMUX.LC{lc}.I3"),
                    _ => return GenericNet::Unknown,
                }
            } else if let Some(suf) = name.strip_prefix("lc_trk_g") {
                let (a, b) = suf.split_once('_').unwrap();
                let a: usize = a.parse().unwrap();
                let b: usize = b.parse().unwrap();
                wname = format!("LOCAL.{a}.{b}");
            } else if let Some(suf) = name.strip_prefix("input_") {
                let (a, b) = suf.split_once('_').unwrap();
                let a: usize = a.parse().unwrap();
                let b: usize = b.parse().unwrap();
                wname = format!("IMUX.LC{b}.I{a}");
            } else if let Some(suf) = name.strip_prefix("input") {
                let (a, b) = suf.split_once('_').unwrap();
                let a: usize = a.parse().unwrap();
                let b: usize = b.parse().unwrap();
                if (edev.chip.kind.has_ioi_we()
                    && (cell.col == edev.chip.col_w() || cell.col == edev.chip.col_e()))
                    || cell.row == edev.chip.row_s()
                    || cell.row == edev.chip.row_n()
                {
                    wname = match (a, b) {
                        (0, 1) => "IMUX.IO0.DOUT0",
                        (0, 2) => "IMUX.IO0.DOUT1",
                        (1, 1) => "IMUX.IO1.DOUT0",
                        (1, 2) => "IMUX.IO1.DOUT1",
                        _ => unreachable!(),
                    }
                    .into();
                } else {
                    wname = format!("IMUX.LC{b}.I{a}");
                }
            } else if let Some(idx) = name.strip_prefix("IPinput_") {
                let idx: usize = idx.parse().unwrap();
                let lc = idx % 8;
                wname = match idx {
                    0..8 => format!("IMUX.LC{lc}.I3"),
                    8..16 => format!("IMUX.LC{lc}.I1"),
                    16..24 => format!("IMUX.LC{lc}.I0"),
                    _ => unreachable!(),
                };
            } else if let Some(idx) = name.strip_prefix("glb_netwk_") {
                let idx: usize = idx.parse().unwrap();
                wname = format!("GLOBAL.{idx}");
            } else if let Some(idx) = name.strip_prefix("fabout_") {
                let idx: usize = idx.parse().unwrap();
                let wire = edev.egrid.db.get_wire("IMUX.IO.EXTRA");
                let special = &edev.chip.special_tiles[&SpecialTileKey::GbFabric(idx)];
                return GenericNet::Int(special.cells.first().unwrap().wire(wire));
            } else if let Some(idx) = name.strip_prefix("padin_") {
                let idx: usize = idx.parse().unwrap();
                if let Some(special) = edev.chip.special_tiles.get(&SpecialTileKey::GbIo(idx)) {
                    let bel = edev.chip.get_io_loc(special.io[&SpecialIoKey::GbIn]);
                    return GenericNet::GlobalPadIn(bel.cell);
                } else {
                    return match idx {
                        4 => GenericNet::GlobalClkh,
                        5 => GenericNet::GlobalClkl,
                        _ => unreachable!(),
                    };
                }
            } else if let Some(idx) = name.strip_prefix("glb2local_") {
                let idx: usize = idx.parse().unwrap();
                let idx = idx + 4;
                wname = format!("LOCAL.0.{idx}");
            } else if let Some(idx) = name.strip_prefix("out_") {
                let idx: usize = idx.parse().unwrap();
                wname = format!("OUT.LC{idx}");
            } else if let Some(idx) = name.strip_prefix("slf_op_") {
                let idx: usize = idx.parse().unwrap();
                wname = format!("OUT.LC{idx}");
            } else if let Some(idx) = name.strip_prefix("wire_logic_cluster/ram/RDATA_") {
                let idx: usize = idx.parse().unwrap();
                let idx = idx % 8;
                wname = format!("OUT.LC{idx}");
            } else if let Some(idx) = name.strip_prefix("wire_bram/ram/RDATA_") {
                let mut idx: usize = idx.parse().unwrap();
                idx %= 8;
                if edev.chip.kind.has_ice40_bramv2() {
                    idx ^= 7;
                }
                wname = format!("OUT.LC{idx}");
            } else if let Some(idx) = name.strip_prefix("wire_bram/ram/WDATA_") {
                let idx: usize = idx.parse().unwrap();
                let idx = idx % 8;
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                wname = format!("IMUX.LC{xi}.I1");
            } else if let Some(idx) = name.strip_prefix("wire_logic_cluster/ram/WDATA_") {
                let idx: usize = idx.parse().unwrap();
                let idx = idx % 8;
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                wname = format!("IMUX.LC{xi}.I1");
            } else if let Some(idx) = name.strip_prefix("wire_bram/ram/MASK_") {
                let idx: usize = idx.parse().unwrap();
                let idx = idx % 8;
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                wname = format!("IMUX.LC{xi}.I3");
            } else if let Some(idx) = name.strip_prefix("wire_logic_cluster/ram/MASK_") {
                let idx: usize = idx.parse().unwrap();
                let idx = idx % 8;
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                wname = format!("IMUX.LC{xi}.I3");
            } else if let Some(idx) = name.strip_prefix("wire_bram/ram/RADDR_") {
                let idx: usize = idx.parse().unwrap();
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                let lc = xi % 8;
                let ii = if xi >= 8 { 2 } else { 0 };
                wname = format!("IMUX.LC{lc}.I{ii}");
            } else if let Some(idx) = name.strip_prefix("wire_logic_cluster/ram/RADDR_") {
                let idx: usize = idx.parse().unwrap();
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                let lc = xi % 8;
                let ii = if xi >= 8 { 2 } else { 0 };
                wname = format!("IMUX.LC{lc}.I{ii}");
            } else if let Some(idx) = name.strip_prefix("wire_bram/ram/WADDR_") {
                let idx: usize = idx.parse().unwrap();
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                let lc = xi % 8;
                let ii = if xi >= 8 { 2 } else { 0 };
                wname = format!("IMUX.LC{lc}.I{ii}");
            } else if let Some(idx) = name.strip_prefix("wire_logic_cluster/ram/WADDR_") {
                let idx: usize = idx.parse().unwrap();
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                let lc = xi % 8;
                let ii = if xi >= 8 { 2 } else { 0 };
                wname = format!("IMUX.LC{lc}.I{ii}");
            } else if let Some(idx) = name.strip_prefix("RADDR_") {
                let idx: usize = idx.parse().unwrap();
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                let lc = xi % 8;
                let ii = if xi >= 8 { 2 } else { 0 };
                wname = format!("IMUX.LC{lc}.I{ii}");
            } else if let Some(idx) = name.strip_prefix("WADDR_") {
                let idx: usize = idx.parse().unwrap();
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                let lc = xi % 8;
                let ii = if xi >= 8 { 2 } else { 0 };
                wname = format!("IMUX.LC{lc}.I{ii}");
            } else if let Some(idx) = name.strip_prefix("downADDR_") {
                let idx: usize = idx.parse().unwrap();
                return GenericNet::CascAddr(cell, idx);
            } else if let Some(idx) = name.strip_prefix("upADDR_") {
                let idx: usize = idx.parse().unwrap();
                return GenericNet::CascAddr(cell.delta(0, 2), idx);
            } else if let Some(idx) = name.strip_prefix("lft_op_") {
                let idx: usize = idx.parse().unwrap();
                wname = format!("OUT.LC{idx}.E");
            } else if let Some(idx) = name.strip_prefix("rgt_op_") {
                let idx: usize = idx.parse().unwrap();
                wname = format!("OUT.LC{idx}.W");
            } else if let Some(idx) = name.strip_prefix("bot_op_") {
                let idx: usize = idx.parse().unwrap();
                wname = format!("OUT.LC{idx}.N");
            } else if let Some(idx) = name.strip_prefix("top_op_") {
                let idx: usize = idx.parse().unwrap();
                wname = format!("OUT.LC{idx}.S");
            } else if let Some(idx) = name.strip_prefix("bnl_op_") {
                let idx: usize = idx.parse().unwrap();
                wname = format!("OUT.LC{idx}.EN");
            } else if let Some(idx) = name.strip_prefix("bnr_op_") {
                let idx: usize = idx.parse().unwrap();
                wname = format!("OUT.LC{idx}.WN");
            } else if let Some(idx) = name.strip_prefix("tnl_op_") {
                let idx: usize = idx.parse().unwrap();
                wname = format!("OUT.LC{idx}.ES");
            } else if let Some(idx) = name.strip_prefix("tnr_op_") {
                let idx: usize = idx.parse().unwrap();
                wname = format!("OUT.LC{idx}.WS");
            } else if let Some(idx) = name.strip_prefix("logic_op_lft_") {
                let idx: usize = idx.parse().unwrap();
                wname = format!("OUT.LC{idx}.E");
            } else if let Some(idx) = name.strip_prefix("logic_op_rgt_") {
                let idx: usize = idx.parse().unwrap();
                wname = format!("OUT.LC{idx}.W");
            } else if let Some(idx) = name.strip_prefix("logic_op_bot_") {
                let idx: usize = idx.parse().unwrap();
                wname = format!("OUT.LC{idx}.N");
            } else if let Some(idx) = name.strip_prefix("logic_op_top_") {
                let idx: usize = idx.parse().unwrap();
                wname = format!("OUT.LC{idx}.S");
            } else if let Some(idx) = name.strip_prefix("logic_op_bnl_") {
                let idx: usize = idx.parse().unwrap();
                wname = format!("OUT.LC{idx}.EN");
            } else if let Some(idx) = name.strip_prefix("logic_op_bnr_") {
                let idx: usize = idx.parse().unwrap();
                wname = format!("OUT.LC{idx}.WN");
            } else if let Some(idx) = name.strip_prefix("logic_op_tnl_") {
                let idx: usize = idx.parse().unwrap();
                wname = format!("OUT.LC{idx}.ES");
            } else if let Some(idx) = name.strip_prefix("logic_op_tnr_") {
                let idx: usize = idx.parse().unwrap();
                wname = format!("OUT.LC{idx}.WS");
            } else if let Some(lc) = name.strip_prefix("carry_") {
                let lc: usize = lc.parse().unwrap();
                if lc == 0 {
                    return GenericNet::Cmux(cell);
                } else {
                    return GenericNet::Cout(cell, lc - 1);
                }
            } else if let Some(lc) = name.strip_prefix("cascade_") {
                let lc: usize = lc.parse().unwrap();
                if lc == 0 {
                    if cell.col == edev.chip.col_w()
                        || cell.col == edev.chip.col_e()
                        || edev.chip.cols_bram.contains(&cell.col)
                    {
                        return GenericNet::Ltin(cell);
                    } else {
                        return GenericNet::Ltout(cell.delta(0, -1), 7);
                    }
                } else {
                    return GenericNet::Ltout(cell, lc - 1);
                }
            } else if let Some(n) = name.strip_prefix("sp4_h_r_") {
                wname = wire_sp4_h(n, false);
            } else if let Some(n) = name.strip_prefix("sp4_h_l_") {
                wname = wire_sp4_h(n, true);
            } else if let Some(n) = name.strip_prefix("span4_horz_r_") {
                wname = wire_sp4_io_h(n, false);
            } else if let Some(n) = name.strip_prefix("span4_horz_l_") {
                wname = wire_sp4_io_h(n, true);
            } else if let Some(n) = name.strip_prefix("span4_horz_") {
                if cell.col == edev.chip.col_w() {
                    wname = wire_sp4_h(n, false);
                } else if cell.col == edev.chip.col_e() {
                    wname = wire_sp4_h(n, true);
                } else {
                    return GenericNet::Unknown;
                }
            } else if let Some(n) = name.strip_prefix("sp4_v_b_") {
                wname = wire_sp4_v(n, true, false);
            } else if let Some(n) = name.strip_prefix("sp4_v_t_") {
                wname = wire_sp4_v(n, false, false);
            } else if let Some(n) = name.strip_prefix("sp4_r_v_b_") {
                wname = wire_sp4_v(n, true, true);
            } else if let Some(n) = name.strip_prefix("span4_vert_b_") {
                wname = wire_sp4_io_v(n, true);
            } else if let Some(n) = name.strip_prefix("span4_vert_t_") {
                wname = wire_sp4_io_v(n, false);
            } else if let Some(n) = name.strip_prefix("span4_vert_") {
                if cell.row == edev.chip.row_s() {
                    wname = wire_sp4_v(n, false, false);
                } else if cell.row == edev.chip.row_n() {
                    wname = wire_sp4_v(n, true, false);
                } else {
                    return GenericNet::Unknown;
                }
            } else if let Some(n) = name.strip_prefix("sp12_h_r_") {
                wname = wire_sp12_h(n, false);
            } else if let Some(n) = name.strip_prefix("sp12_h_l_") {
                wname = wire_sp12_h(n, true);
            } else if let Some(n) = name.strip_prefix("span12_horz_") {
                if cell.col == edev.chip.col_w() {
                    wname = wire_sp12_h(n, false);
                } else if cell.col == edev.chip.col_e() {
                    wname = wire_sp12_h(n, true);
                } else {
                    return GenericNet::Unknown;
                }
            } else if let Some(n) = name.strip_prefix("sp12_v_b_") {
                wname = wire_sp12_v(n, true);
            } else if let Some(n) = name.strip_prefix("sp12_v_t_") {
                wname = wire_sp12_v(n, false);
            } else if let Some(n) = name.strip_prefix("span12_vert_") {
                if cell.row == edev.chip.row_s() {
                    wname = wire_sp12_v(n, false);
                } else if cell.row == edev.chip.row_n() {
                    wname = wire_sp12_v(n, true);
                } else {
                    return GenericNet::Unknown;
                }
            } else {
                return GenericNet::Unknown;
            }
        }
    };
    let wire = edev.egrid.db.get_wire(&wname);
    let mut wire = edev.egrid.resolve_wire(cell.wire(wire)).unwrap();
    let wname = edev.egrid.db.wires.key(wire.slot);
    if let Some(suf) = wname.strip_prefix("OUT.LC")
        && !suf.contains('.')
    {
        let mut idx: u32 = suf.parse().unwrap();
        if (wire.cell.col == edev.chip.col_w() || wire.cell.col == edev.chip.col_e())
            && (wire.cell.row == edev.chip.row_s() || wire.cell.row == edev.chip.row_n())
        {
            wire.slot = edev.egrid.db.get_wire("OUT.LC0");
        } else if wire.cell.row == edev.chip.row_s()
            || wire.cell.row == edev.chip.row_n()
            || (wire.cell.col == edev.chip.col_w() && edev.chip.kind.has_ioi_we())
            || (wire.cell.col == edev.chip.col_e() && edev.chip.kind.has_ioi_we())
        {
            idx %= 4;
            wire.slot = edev.egrid.db.get_wire(&format!("OUT.LC{idx}"));
        }
    }
    GenericNet::Int(wire)
}

pub fn xlat_mux_in(
    edev: &ExpandedDevice,
    mut wa: WireCoord,
    wb: WireCoord,
    na: (u32, u32, &str),
    nb: (u32, u32, &str),
) -> (CellCoord, WireSlotId, WireSlotId) {
    let wna = edev.egrid.db.wires.key(wa.slot);
    let wnb = edev.egrid.db.wires.key(wb.slot);
    if wna.starts_with("GLOBAL") {
        return (wb.cell, wa.slot, wb.slot);
    }
    if wna.starts_with("OUT.LC") && wnb.starts_with("LOCAL") {
        let out_idx: usize = wna[6..].parse().unwrap();
        let local_idx: usize = wnb[8..].parse().unwrap();
        let is_lr = wa.cell.col == edev.chip.col_w() || wa.cell.col == edev.chip.col_e();
        let is_bt = wa.cell.row == edev.chip.row_s() || wa.cell.row == edev.chip.row_n();
        if is_lr && is_bt {
            // could be anything
        } else if (is_lr && edev.chip.kind.has_ioi_we()) || is_bt {
            assert_eq!(out_idx & 3, local_idx & 3);
        } else {
            assert_eq!(out_idx, local_idx);
        }
        wa.slot = edev.egrid.db.get_wire(&format!("OUT.LC{local_idx}"));
    }
    let wna = edev.egrid.db.wires.key(wa.slot);
    let mut locs_a: HashMap<_, HashSet<_>> = HashMap::new();
    for wire in edev.egrid.wire_tree(wa) {
        locs_a.entry(wire.cell).or_default().insert(wire.slot);
    }
    let mut locs_b: HashMap<_, HashSet<_>> = HashMap::new();
    for wire in edev.egrid.wire_tree(wb) {
        locs_b.entry(wire.cell).or_default().insert(wire.slot);
    }
    for locs in [&mut locs_a, &mut locs_b] {
        // kill corners
        locs.retain(|&cell, _| {
            !((cell.col == edev.chip.col_w() || cell.col == edev.chip.col_e())
                && (cell.row == edev.chip.row_s() || cell.row == edev.chip.row_n()))
        });
        for wires in locs.values_mut() {
            if wires.len() > 1 {
                wires.retain(|&wire| {
                    let wn = edev.egrid.db.wires.key(wire);
                    !(wn.starts_with("QUAD.V") && wn.ends_with(".W"))
                });
            }
            assert_eq!(wires.len(), 1);
        }
    }
    let mut locs_a: HashMap<_, _> = HashMap::from_iter(
        locs_a
            .into_iter()
            .map(|(loc, wires)| (loc, wires.into_iter().next().unwrap())),
    );
    let mut locs_b: HashMap<_, _> = HashMap::from_iter(
        locs_b
            .into_iter()
            .map(|(loc, wires)| (loc, wires.into_iter().next().unwrap())),
    );
    let mut locs: HashSet<_> = HashSet::from_iter(locs_a.keys().copied());
    locs.retain(|&loc| locs_b.contains_key(&loc));
    if locs.len() > 1 {
        if wna.starts_with("OUT.LC") {
            locs = HashSet::from_iter([wa.cell]);
        } else if wna.starts_with("LONG.H") && wnb.starts_with("QUAD.H") {
            locs_b.retain(|_, &mut wire| edev.egrid.db.wires.key(wire).ends_with(".1"));
            locs.retain(|&loc| locs_b.contains_key(&loc));
        } else if wna.starts_with("LONG.V") && wnb.starts_with("QUAD.V") {
            locs_b.retain(|_, &mut wire| edev.egrid.db.wires.key(wire).ends_with(".3"));
            locs.retain(|&loc| locs_b.contains_key(&loc));
        } else if wna.starts_with("QUAD.H") && wnb.starts_with("QUAD.H") {
            locs.retain(|&cell| cell.col == edev.chip.col_w() || cell.col == edev.chip.col_e());
        } else if wna.starts_with("QUAD.V") && wnb.starts_with("QUAD.V") {
            locs_a.retain(|_, &mut wire| {
                let wn = edev.egrid.db.wires.key(wire);
                !(wn.starts_with("QUAD.V") && wn.ends_with(".W"))
            });
            locs_b.retain(|_, &mut wire| {
                let wn = edev.egrid.db.wires.key(wire);
                !(wn.starts_with("QUAD.V") && wn.ends_with(".W"))
            });
            locs.retain(|&loc| locs_a.contains_key(&loc));
            locs.retain(|&loc| locs_b.contains_key(&loc));
            locs.retain(|&cell| cell.row == edev.chip.row_s() || cell.row == edev.chip.row_n());
        } else {
            locs_a.retain(|_, &mut wire| {
                let wn = edev.egrid.db.wires.key(wire);
                !(wn.starts_with("QUAD.V") && wn.ends_with(".W"))
            });
            locs_b.retain(|_, &mut wire| {
                let wn = edev.egrid.db.wires.key(wire);
                !(wn.starts_with("QUAD.V") && wn.ends_with(".W"))
            });
            locs.retain(|&loc| locs_a.contains_key(&loc));
            locs.retain(|&loc| locs_b.contains_key(&loc));
        }
        if locs.len() > 1 {
            let (ax, ay, aw) = na;
            let (bx, by, bw) = nb;
            println!("UHHHHHHHHHHH MANY POSSIBILITIES HERE {ax}:{ay}:{aw} vs {bx}:{by}:{bw}");
            println!("{wa:?} ({wna}):");
            for (&cell, &wire) in &locs_a {
                println!("  {wire}", wire = cell.wire(wire).to_string(edev.egrid.db));
            }
            println!("{wb:?} ({wnb}):");
            for (&cell, &wire) in &locs_b {
                println!("  {wire}", wire = cell.wire(wire).to_string(edev.egrid.db));
            }
            println!("common {locs:?}");
            panic!("welp");
        }
    }
    if locs.is_empty() {
        let (ax, ay, aw) = na;
        let (bx, by, bw) = nb;
        println!("NO SPEAKA ENGLISH {ax}:{ay}:{aw} vs {bx}:{by}:{bw}");
        println!("{wa:?} ({wna}):");
        for (&cell, &wire) in &locs_a {
            println!("  {wire}", wire = cell.wire(wire).to_string(edev.egrid.db));
        }
        println!("{wb:?} ({wnb}):");
        for (&cell, &wire) in &locs_b {
            println!("  {wire}", wire = cell.wire(wire).to_string(edev.egrid.db));
        }
        println!("common {locs:?}");
    }
    let cell = locs.iter().copied().next().unwrap();
    (cell, locs_a[&cell], locs_b[&cell])
}
