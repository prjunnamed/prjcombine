use std::collections::{HashMap, HashSet};

use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    db::WireSlotId,
    dir::Dir,
    grid::{CellCoord, ColId, DieId, RowId, WireCoord},
};
use prjcombine_siliconblue::{
    chip::{ChipKind, SpecialIoKey, SpecialTileKey},
    defs,
    expanded::ExpandedDevice,
};

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
    fn wire_sp4_h(n: &str, is_l: bool) -> WireSlotId {
        let n: usize = n.parse().unwrap();
        let mut seg = n / 12;
        let mut which = n % 12;
        which ^= seg & 1;
        if is_l {
            seg += 1;
        }
        match seg {
            0 => defs::wires::QUAD_H0[which],
            1 => defs::wires::QUAD_H1[which],
            2 => defs::wires::QUAD_H2[which],
            3 => defs::wires::QUAD_H3[which],
            4 => defs::wires::QUAD_H4[which],
            _ => unreachable!(),
        }
    }

    fn wire_sp4_io_h(n: &str, is_l: bool) -> WireSlotId {
        let n: usize = n.parse().unwrap();
        let mut seg = n / 4;
        let which = n % 4;
        if is_l {
            seg += 1;
        }
        match seg {
            0 => defs::wires::QUAD_H0[which],
            1 => defs::wires::QUAD_H1[which],
            2 => defs::wires::QUAD_H2[which],
            3 => defs::wires::QUAD_H3[which],
            4 => defs::wires::QUAD_H4[which],
            _ => unreachable!(),
        }
    }

    fn wire_sp4_v(n: &str, is_b: bool, is_r: bool) -> WireSlotId {
        let n: usize = n.parse().unwrap();
        let mut seg = 3 - n / 12;
        let mut which = n % 12;
        which ^= seg & 1;
        if is_b {
            seg += 1;
        }
        if is_r {
            match seg {
                1 => defs::wires::QUAD_V1_W[which],
                2 => defs::wires::QUAD_V2_W[which],
                3 => defs::wires::QUAD_V3_W[which],
                4 => defs::wires::QUAD_V4_W[which],
                _ => unreachable!(),
            }
        } else {
            match seg {
                0 => defs::wires::QUAD_V0[which],
                1 => defs::wires::QUAD_V1[which],
                2 => defs::wires::QUAD_V2[which],
                3 => defs::wires::QUAD_V3[which],
                4 => defs::wires::QUAD_V4[which],
                _ => unreachable!(),
            }
        }
    }

    fn wire_sp4_io_v(n: &str, is_b: bool) -> WireSlotId {
        let n: usize = n.parse().unwrap();
        let mut seg = 3 - n / 4;
        let which = n % 4;
        if is_b {
            seg += 1;
        }
        match seg {
            0 => defs::wires::QUAD_V0[which],
            1 => defs::wires::QUAD_V1[which],
            2 => defs::wires::QUAD_V2[which],
            3 => defs::wires::QUAD_V3[which],
            4 => defs::wires::QUAD_V4[which],
            _ => unreachable!(),
        }
    }

    fn wire_sp12_h(n: &str, is_l: bool) -> WireSlotId {
        let n: usize = n.parse().unwrap();
        let mut seg = n / 2;
        let mut which = n % 2;
        which ^= seg & 1;
        if is_l {
            seg += 1;
        }
        match seg {
            0 => defs::wires::LONG_H0[which],
            1 => defs::wires::LONG_H1[which],
            2 => defs::wires::LONG_H2[which],
            3 => defs::wires::LONG_H3[which],
            4 => defs::wires::LONG_H4[which],
            5 => defs::wires::LONG_H5[which],
            6 => defs::wires::LONG_H6[which],
            7 => defs::wires::LONG_H7[which],
            8 => defs::wires::LONG_H8[which],
            9 => defs::wires::LONG_H9[which],
            10 => defs::wires::LONG_H10[which],
            11 => defs::wires::LONG_H11[which],
            12 => defs::wires::LONG_H12[which],
            _ => unreachable!(),
        }
    }

    fn wire_sp12_v(n: &str, is_b: bool) -> WireSlotId {
        let n: usize = n.parse().unwrap();
        let mut seg = 11 - n / 2;
        let mut which = n % 2;
        which ^= seg & 1;
        if is_b {
            seg += 1;
        }
        match seg {
            0 => defs::wires::LONG_V0[which],
            1 => defs::wires::LONG_V1[which],
            2 => defs::wires::LONG_V2[which],
            3 => defs::wires::LONG_V3[which],
            4 => defs::wires::LONG_V4[which],
            5 => defs::wires::LONG_V5[which],
            6 => defs::wires::LONG_V6[which],
            7 => defs::wires::LONG_V7[which],
            8 => defs::wires::LONG_V8[which],
            9 => defs::wires::LONG_V9[which],
            10 => defs::wires::LONG_V10[which],
            11 => defs::wires::LONG_V11[which],
            12 => defs::wires::LONG_V12[which],
            _ => unreachable!(),
        }
    }

    let mut cell = CellCoord::new(
        DieId::from_idx(0),
        ColId::from_idx(x as usize),
        RowId::from_idx(y as usize),
    );
    let wire: WireSlotId;
    match name {
        "wire_io_cluster/io_0/gbout" | "gbout_0" => return GenericNet::Gbout(cell, 0),
        "wire_io_cluster/io_1/gbout" | "gbout_1" => return GenericNet::Gbout(cell, 1),
        "wire_io_cluster/io_0/D_IN_0" => wire = defs::wires::OUT_LC[0],
        "wire_io_cluster/io_0/D_IN_1" => wire = defs::wires::OUT_LC[1],
        "wire_io_cluster/io_1/D_IN_0" => wire = defs::wires::OUT_LC[2],
        "wire_io_cluster/io_1/D_IN_1" => wire = defs::wires::OUT_LC[3],
        "wire_io_cluster/io_0/D_OUT_0" | "dout_0" | "pad_out_0" => {
            wire = defs::wires::IMUX_IO_DOUT0[0]
        }
        "wire_io_cluster/io_0/D_OUT_1" | "dout_1" => wire = defs::wires::IMUX_IO_DOUT1[0],
        "wire_io_cluster/io_0/OUT_ENB" => wire = defs::wires::IMUX_IO_OE[0],
        "wire_io_cluster/io_1/D_OUT_0" | "dout_2" | "pad_out_1" => {
            wire = defs::wires::IMUX_IO_DOUT0[1]
        }
        "wire_io_cluster/io_1/D_OUT_1" | "dout_3" => wire = defs::wires::IMUX_IO_DOUT1[1],
        "wire_io_cluster/io_1/OUT_ENB" => wire = defs::wires::IMUX_IO_OE[1],
        "pad_in_0" => wire = defs::wires::OUT_LC[0],
        "pad_in_1" => wire = defs::wires::OUT_LC[2],
        "inclk" | "wire_io_cluster/io_0/inclk" | "wire_io_cluster/io_1/inclk" => {
            wire = defs::wires::IMUX_IO_ICLK
        }
        "outclk" | "wire_io_cluster/io_0/outclk" | "wire_io_cluster/io_1/outclk" => {
            wire = defs::wires::IMUX_IO_OCLK
        }
        "cen"
        | "wire_logic_cluster/ram/RCLKE"
        | "wire_logic_cluster/ram/WCLKE"
        | "wire_bram/ram/RCLKE"
        | "wire_bram/ram/WCLKE"
        | "wire_io_cluster/io_0/cen"
        | "wire_io_cluster/io_1/cen" => wire = defs::wires::IMUX_CE,
        "clk"
        | "wire_logic_cluster/ram/RCLK"
        | "wire_logic_cluster/ram/WCLK"
        | "wire_bram/ram/RCLK"
        | "wire_bram/ram/WCLK" => wire = defs::wires::IMUX_CLK,
        "s_r"
        | "wire_logic_cluster/ram/RE"
        | "wire_logic_cluster/ram/WE"
        | "wire_bram/ram/RE"
        | "wire_bram/ram/WE" => wire = defs::wires::IMUX_RST,
        "fabout" | "wire_gbuf/in" | "wire_gbuf/out" => wire = defs::wires::IMUX_IO_EXTRA,
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
            let wire = defs::wires::IMUX_IO_EXTRA;
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
            wire = defs::wires::OUT_LC[0];
        }
        "wire_pll/outcore" => {
            wire = defs::wires::OUT_LC[2];
        }
        _ => {
            if let Some(suf) = name.strip_prefix("wire_logic_cluster/lc_") {
                let (lc, suf) = suf.split_once('/').unwrap();
                let lc: usize = lc.parse().unwrap();
                match suf {
                    "ltout" => return GenericNet::Ltout(cell, lc),
                    "cout" => return GenericNet::Cout(cell, lc),
                    "out" => wire = defs::wires::OUT_LC[lc],
                    "clk" => wire = defs::wires::IMUX_CLK,
                    "s_r" => wire = defs::wires::IMUX_RST,
                    "cen" => wire = defs::wires::IMUX_CE,
                    "in_0" => wire = defs::wires::IMUX_LC_I0[lc],
                    "in_1" => wire = defs::wires::IMUX_LC_I1[lc],
                    "in_2" => wire = defs::wires::IMUX_LC_I2[lc],
                    "in_3" => wire = defs::wires::IMUX_LC_I3[lc],
                    _ => return GenericNet::Unknown,
                }
            } else if let Some(suf) = name.strip_prefix("wire_mult/lc_") {
                let (lc, suf) = suf.split_once('/').unwrap();
                let lc: usize = lc.parse().unwrap();
                match suf {
                    "cout" => return GenericNet::Cout(cell, lc),
                    "out" => wire = defs::wires::OUT_LC[lc],
                    "clk" => wire = defs::wires::IMUX_CLK,
                    "s_r" => wire = defs::wires::IMUX_RST,
                    "cen" => wire = defs::wires::IMUX_CE,
                    "in_0" => wire = defs::wires::IMUX_LC_I0[lc],
                    "in_1" => wire = defs::wires::IMUX_LC_I1[lc],
                    "in_2" => wire = defs::wires::IMUX_LC_I2[lc],
                    "in_3" => wire = defs::wires::IMUX_LC_I3[lc],
                    _ => return GenericNet::Unknown,
                }
            } else if let Some(suf) = name.strip_prefix("wire_con_box/lc_") {
                let (lc, suf) = suf.split_once('/').unwrap();
                let lc: usize = lc.parse().unwrap();
                match suf {
                    "cout" => return GenericNet::Cout(cell, lc),
                    "in_0" => wire = defs::wires::IMUX_LC_I0[lc],
                    "in_1" => wire = defs::wires::IMUX_LC_I1[lc],
                    "in_3" => wire = defs::wires::IMUX_LC_I3[lc],
                    _ => return GenericNet::Unknown,
                }
            } else if let Some(suf) = name.strip_prefix("lc_trk_g") {
                let (a, b) = suf.split_once('_').unwrap();
                let a: usize = a.parse().unwrap();
                let b: usize = b.parse().unwrap();
                wire = match a {
                    0 => defs::wires::LOCAL_0[b],
                    1 => defs::wires::LOCAL_1[b],
                    2 => defs::wires::LOCAL_2[b],
                    3 => defs::wires::LOCAL_3[b],
                    _ => unreachable!(),
                };
            } else if let Some(suf) = name.strip_prefix("input_") {
                let (a, b) = suf.split_once('_').unwrap();
                let a: usize = a.parse().unwrap();
                let b: usize = b.parse().unwrap();
                wire = match a {
                    0 => defs::wires::IMUX_LC_I0[b],
                    1 => defs::wires::IMUX_LC_I1[b],
                    2 => defs::wires::IMUX_LC_I2[b],
                    3 => defs::wires::IMUX_LC_I3[b],
                    _ => unreachable!(),
                };
            } else if let Some(suf) = name.strip_prefix("input") {
                let (a, b) = suf.split_once('_').unwrap();
                let a: usize = a.parse().unwrap();
                let b: usize = b.parse().unwrap();
                if (edev.chip.kind.has_ioi_we()
                    && (cell.col == edev.chip.col_w() || cell.col == edev.chip.col_e()))
                    || cell.row == edev.chip.row_s()
                    || cell.row == edev.chip.row_n()
                {
                    wire = match b {
                        1 => defs::wires::IMUX_IO_DOUT0[a],
                        2 => defs::wires::IMUX_IO_DOUT1[a],
                        _ => unreachable!(),
                    };
                } else {
                    wire = match a {
                        0 => defs::wires::IMUX_LC_I0[b],
                        1 => defs::wires::IMUX_LC_I1[b],
                        2 => defs::wires::IMUX_LC_I2[b],
                        3 => defs::wires::IMUX_LC_I3[b],
                        _ => unreachable!(),
                    };
                }
            } else if let Some(idx) = name.strip_prefix("IPinput_") {
                let idx: usize = idx.parse().unwrap();
                let lc = idx % 8;
                wire = match idx {
                    0..8 => defs::wires::IMUX_LC_I3[lc],
                    8..16 => defs::wires::IMUX_LC_I1[lc],
                    16..24 => defs::wires::IMUX_LC_I0[lc],
                    _ => unreachable!(),
                };
            } else if let Some(idx) = name.strip_prefix("glb_netwk_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::GLOBAL[idx];
            } else if let Some(idx) = name.strip_prefix("fabout_") {
                let idx: usize = idx.parse().unwrap();
                let wire = defs::wires::IMUX_IO_EXTRA;
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
                wire = defs::wires::LOCAL_0[idx + 4];
            } else if let Some(idx) = name.strip_prefix("out_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::OUT_LC[idx];
            } else if let Some(idx) = name.strip_prefix("slf_op_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::OUT_LC[idx];
            } else if let Some(idx) = name.strip_prefix("wire_logic_cluster/ram/RDATA_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::OUT_LC[idx % 8];
            } else if let Some(idx) = name.strip_prefix("wire_bram/ram/RDATA_") {
                let mut idx: usize = idx.parse().unwrap();
                idx %= 8;
                if edev.chip.kind.has_ice40_bramv2() {
                    idx ^= 7;
                }
                wire = defs::wires::OUT_LC[idx];
            } else if let Some(idx) = name.strip_prefix("wire_bram/ram/WDATA_") {
                let idx: usize = idx.parse().unwrap();
                let idx = idx % 8;
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                wire = defs::wires::IMUX_LC_I1[xi];
            } else if let Some(idx) = name.strip_prefix("wire_logic_cluster/ram/WDATA_") {
                let idx: usize = idx.parse().unwrap();
                let idx = idx % 8;
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                wire = defs::wires::IMUX_LC_I1[xi];
            } else if let Some(idx) = name.strip_prefix("wire_bram/ram/MASK_") {
                let idx: usize = idx.parse().unwrap();
                let idx = idx % 8;
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                wire = defs::wires::IMUX_LC_I3[xi];
            } else if let Some(idx) = name.strip_prefix("wire_logic_cluster/ram/MASK_") {
                let idx: usize = idx.parse().unwrap();
                let idx = idx % 8;
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                wire = defs::wires::IMUX_LC_I3[xi];
            } else if let Some(idx) = name.strip_prefix("wire_bram/ram/RADDR_") {
                let idx: usize = idx.parse().unwrap();
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                let lc = xi % 8;
                wire = if xi >= 8 {
                    defs::wires::IMUX_LC_I2[lc]
                } else {
                    defs::wires::IMUX_LC_I0[lc]
                };
            } else if let Some(idx) = name.strip_prefix("wire_logic_cluster/ram/RADDR_") {
                let idx: usize = idx.parse().unwrap();
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                let lc = xi % 8;
                wire = if xi >= 8 {
                    defs::wires::IMUX_LC_I2[lc]
                } else {
                    defs::wires::IMUX_LC_I0[lc]
                };
            } else if let Some(idx) = name.strip_prefix("wire_bram/ram/WADDR_") {
                let idx: usize = idx.parse().unwrap();
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                let lc = xi % 8;
                wire = if xi >= 8 {
                    defs::wires::IMUX_LC_I2[lc]
                } else {
                    defs::wires::IMUX_LC_I0[lc]
                };
            } else if let Some(idx) = name.strip_prefix("wire_logic_cluster/ram/WADDR_") {
                let idx: usize = idx.parse().unwrap();
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                let lc = xi % 8;
                wire = if xi >= 8 {
                    defs::wires::IMUX_LC_I2[lc]
                } else {
                    defs::wires::IMUX_LC_I0[lc]
                };
            } else if let Some(idx) = name.strip_prefix("RADDR_") {
                let idx: usize = idx.parse().unwrap();
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                let lc = xi % 8;
                wire = if xi >= 8 {
                    defs::wires::IMUX_LC_I2[lc]
                } else {
                    defs::wires::IMUX_LC_I0[lc]
                };
            } else if let Some(idx) = name.strip_prefix("WADDR_") {
                let idx: usize = idx.parse().unwrap();
                let xi = if edev.chip.kind.has_ice40_bramv2() {
                    idx ^ 7
                } else {
                    idx
                };
                let lc = xi % 8;
                wire = if xi >= 8 {
                    defs::wires::IMUX_LC_I2[lc]
                } else {
                    defs::wires::IMUX_LC_I0[lc]
                };
            } else if let Some(idx) = name.strip_prefix("downADDR_") {
                let idx: usize = idx.parse().unwrap();
                return GenericNet::CascAddr(cell, idx);
            } else if let Some(idx) = name.strip_prefix("upADDR_") {
                let idx: usize = idx.parse().unwrap();
                return GenericNet::CascAddr(cell.delta(0, 2), idx);
            } else if let Some(idx) = name.strip_prefix("lft_op_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::OUT_LC_E[idx];
            } else if let Some(idx) = name.strip_prefix("rgt_op_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::OUT_LC_W[idx];
            } else if let Some(idx) = name.strip_prefix("bot_op_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::OUT_LC_N[idx];
            } else if let Some(idx) = name.strip_prefix("top_op_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::OUT_LC_S[idx];
            } else if let Some(idx) = name.strip_prefix("bnl_op_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::OUT_LC_EN[idx];
            } else if let Some(idx) = name.strip_prefix("bnr_op_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::OUT_LC_WN[idx];
            } else if let Some(idx) = name.strip_prefix("tnl_op_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::OUT_LC_ES[idx];
            } else if let Some(idx) = name.strip_prefix("tnr_op_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::OUT_LC_WS[idx];
            } else if let Some(idx) = name.strip_prefix("logic_op_lft_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::OUT_LC_E[idx];
            } else if let Some(idx) = name.strip_prefix("logic_op_rgt_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::OUT_LC_W[idx];
            } else if let Some(idx) = name.strip_prefix("logic_op_bot_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::OUT_LC_N[idx];
            } else if let Some(idx) = name.strip_prefix("logic_op_top_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::OUT_LC_S[idx];
            } else if let Some(idx) = name.strip_prefix("logic_op_bnl_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::OUT_LC_EN[idx];
            } else if let Some(idx) = name.strip_prefix("logic_op_bnr_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::OUT_LC_WN[idx];
            } else if let Some(idx) = name.strip_prefix("logic_op_tnl_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::OUT_LC_ES[idx];
            } else if let Some(idx) = name.strip_prefix("logic_op_tnr_") {
                let idx: usize = idx.parse().unwrap();
                wire = defs::wires::OUT_LC_WS[idx];
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
                wire = wire_sp4_h(n, false);
            } else if let Some(n) = name.strip_prefix("sp4_h_l_") {
                wire = wire_sp4_h(n, true);
            } else if let Some(n) = name.strip_prefix("span4_horz_r_") {
                wire = wire_sp4_io_h(n, false);
            } else if let Some(n) = name.strip_prefix("span4_horz_l_") {
                wire = wire_sp4_io_h(n, true);
            } else if let Some(n) = name.strip_prefix("span4_horz_") {
                if cell.col == edev.chip.col_w() {
                    wire = wire_sp4_h(n, false);
                } else if cell.col == edev.chip.col_e() {
                    wire = wire_sp4_h(n, true);
                } else {
                    return GenericNet::Unknown;
                }
            } else if let Some(n) = name.strip_prefix("sp4_v_b_") {
                wire = wire_sp4_v(n, true, false);
            } else if let Some(n) = name.strip_prefix("sp4_v_t_") {
                wire = wire_sp4_v(n, false, false);
            } else if let Some(n) = name.strip_prefix("sp4_r_v_b_") {
                wire = wire_sp4_v(n, true, true);
            } else if let Some(n) = name.strip_prefix("span4_vert_b_") {
                wire = wire_sp4_io_v(n, true);
            } else if let Some(n) = name.strip_prefix("span4_vert_t_") {
                wire = wire_sp4_io_v(n, false);
            } else if let Some(n) = name.strip_prefix("span4_vert_") {
                if cell.row == edev.chip.row_s() {
                    wire = wire_sp4_v(n, false, false);
                } else if cell.row == edev.chip.row_n() {
                    wire = wire_sp4_v(n, true, false);
                } else {
                    return GenericNet::Unknown;
                }
            } else if let Some(n) = name.strip_prefix("sp12_h_r_") {
                wire = wire_sp12_h(n, false);
            } else if let Some(n) = name.strip_prefix("sp12_h_l_") {
                wire = wire_sp12_h(n, true);
            } else if let Some(n) = name.strip_prefix("span12_horz_") {
                if cell.col == edev.chip.col_w() {
                    wire = wire_sp12_h(n, false);
                } else if cell.col == edev.chip.col_e() {
                    wire = wire_sp12_h(n, true);
                } else {
                    return GenericNet::Unknown;
                }
            } else if let Some(n) = name.strip_prefix("sp12_v_b_") {
                wire = wire_sp12_v(n, true);
            } else if let Some(n) = name.strip_prefix("sp12_v_t_") {
                wire = wire_sp12_v(n, false);
            } else if let Some(n) = name.strip_prefix("span12_vert_") {
                if cell.row == edev.chip.row_s() {
                    wire = wire_sp12_v(n, false);
                } else if cell.row == edev.chip.row_n() {
                    wire = wire_sp12_v(n, true);
                } else {
                    return GenericNet::Unknown;
                }
            } else {
                return GenericNet::Unknown;
            }
        }
    };
    let mut wire = edev.resolve_wire(cell.wire(wire)).unwrap();
    if let Some(idx) = defs::wires::OUT_LC.index_of(wire.slot) {
        if (wire.cell.col == edev.chip.col_w() || wire.cell.col == edev.chip.col_e())
            && (wire.cell.row == edev.chip.row_s() || wire.cell.row == edev.chip.row_n())
        {
            wire.slot = defs::wires::OUT_LC[0];
        } else if wire.cell.row == edev.chip.row_s()
            || wire.cell.row == edev.chip.row_n()
            || (wire.cell.col == edev.chip.col_w() && edev.chip.kind.has_ioi_we())
            || (wire.cell.col == edev.chip.col_e() && edev.chip.kind.has_ioi_we())
        {
            wire.slot = defs::wires::OUT_LC[idx % 4];
        }
    }
    GenericNet::Int(wire)
}

pub fn parse_local(slot: WireSlotId) -> Option<(usize, usize)> {
    for (group, wires) in [
        &defs::wires::LOCAL_0,
        &defs::wires::LOCAL_1,
        &defs::wires::LOCAL_2,
        &defs::wires::LOCAL_3,
    ]
    .iter()
    .enumerate()
    {
        if let Some(idx) = wires.index_of(slot) {
            return Some((group, idx));
        }
    }
    None
}

pub fn is_quad_h(slot: WireSlotId) -> bool {
    for wires in [
        &defs::wires::QUAD_H0,
        &defs::wires::QUAD_H1,
        &defs::wires::QUAD_H2,
        &defs::wires::QUAD_H3,
        &defs::wires::QUAD_H4,
    ] {
        if wires.contains(slot) {
            return true;
        }
    }
    false
}

pub fn is_quad_v(slot: WireSlotId) -> bool {
    for wires in [
        &defs::wires::QUAD_V0,
        &defs::wires::QUAD_V1,
        &defs::wires::QUAD_V2,
        &defs::wires::QUAD_V3,
        &defs::wires::QUAD_V4,
    ] {
        if wires.contains(slot) {
            return true;
        }
    }
    false
}

pub fn is_quad_v_w(slot: WireSlotId) -> bool {
    for wires in [
        &defs::wires::QUAD_V1_W,
        &defs::wires::QUAD_V2_W,
        &defs::wires::QUAD_V3_W,
        &defs::wires::QUAD_V4_W,
    ] {
        if wires.contains(slot) {
            return true;
        }
    }
    false
}

pub fn is_quad(slot: WireSlotId) -> bool {
    is_quad_h(slot) || is_quad_v(slot) || is_quad_v_w(slot)
}

pub fn is_long_h(slot: WireSlotId) -> bool {
    for wires in [
        &defs::wires::LONG_H0,
        &defs::wires::LONG_H1,
        &defs::wires::LONG_H2,
        &defs::wires::LONG_H3,
        &defs::wires::LONG_H4,
        &defs::wires::LONG_H5,
        &defs::wires::LONG_H6,
        &defs::wires::LONG_H7,
        &defs::wires::LONG_H8,
        &defs::wires::LONG_H9,
        &defs::wires::LONG_H10,
        &defs::wires::LONG_H11,
        &defs::wires::LONG_H12,
    ] {
        if wires.contains(slot) {
            return true;
        }
    }
    false
}

pub fn is_long_v(slot: WireSlotId) -> bool {
    for wires in [
        &defs::wires::LONG_V0,
        &defs::wires::LONG_V1,
        &defs::wires::LONG_V2,
        &defs::wires::LONG_V3,
        &defs::wires::LONG_V4,
        &defs::wires::LONG_V5,
        &defs::wires::LONG_V6,
        &defs::wires::LONG_V7,
        &defs::wires::LONG_V8,
        &defs::wires::LONG_V9,
        &defs::wires::LONG_V10,
        &defs::wires::LONG_V11,
        &defs::wires::LONG_V12,
    ] {
        if wires.contains(slot) {
            return true;
        }
    }
    false
}

pub fn is_long(slot: WireSlotId) -> bool {
    is_long_h(slot) || is_long_v(slot)
}

pub fn xlat_mux_in(
    edev: &ExpandedDevice,
    mut wa: WireCoord,
    wb: WireCoord,
    na: (u32, u32, &str),
    nb: (u32, u32, &str),
) -> (CellCoord, WireSlotId, WireSlotId) {
    if defs::wires::GLOBAL.contains(wa.slot) {
        return (wb.cell, wa.slot, wb.slot);
    }
    if let Some(out_idx) = defs::wires::OUT_LC.index_of(wa.slot)
        && let Some((_, local_idx)) = parse_local(wb.slot)
    {
        let is_lr = wa.cell.col == edev.chip.col_w() || wa.cell.col == edev.chip.col_e();
        let is_bt = wa.cell.row == edev.chip.row_s() || wa.cell.row == edev.chip.row_n();
        if is_lr && is_bt {
            // could be anything
        } else if (is_lr && edev.chip.kind.has_ioi_we()) || is_bt {
            assert_eq!(out_idx & 3, local_idx & 3);
        } else {
            assert_eq!(out_idx, local_idx);
        }
        wa.slot = defs::wires::OUT_LC[local_idx];
    }
    let mut locs_a: HashMap<_, HashSet<_>> = HashMap::new();
    for wire in edev.wire_tree(wa) {
        locs_a.entry(wire.cell).or_default().insert(wire.slot);
    }
    let mut locs_b: HashMap<_, HashSet<_>> = HashMap::new();
    for wire in edev.wire_tree(wb) {
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
                wires.retain(|&wire| !is_quad_v_w(wire));
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
        if defs::wires::OUT_LC.contains(wa.slot) {
            locs = HashSet::from_iter([wa.cell]);
        } else if is_long_h(wa.slot) && is_quad_h(wb.slot) {
            locs_b.retain(|_, &mut wire| defs::wires::QUAD_H1.contains(wire));
            locs.retain(|&loc| locs_b.contains_key(&loc));
        } else if is_long_v(wa.slot) && is_quad_v(wb.slot) {
            locs_b.retain(|_, &mut wire| defs::wires::QUAD_V3.contains(wire));
            locs.retain(|&loc| locs_b.contains_key(&loc));
        } else if is_quad_h(wa.slot) && is_quad_h(wb.slot) {
            locs.retain(|&cell| cell.col == edev.chip.col_w() || cell.col == edev.chip.col_e());
        } else if is_quad_v(wa.slot) && is_quad_v(wb.slot) {
            locs_a.retain(|_, &mut wire| !is_quad_v_w(wire));
            locs_b.retain(|_, &mut wire| !is_quad_v_w(wire));
            locs.retain(|&loc| locs_a.contains_key(&loc));
            locs.retain(|&loc| locs_b.contains_key(&loc));
            locs.retain(|&cell| cell.row == edev.chip.row_s() || cell.row == edev.chip.row_n());
        } else {
            locs_a.retain(|_, &mut wire| !is_quad_v_w(wire));
            locs_b.retain(|_, &mut wire| !is_quad_v_w(wire));
            locs.retain(|&loc| locs_a.contains_key(&loc));
            locs.retain(|&loc| locs_b.contains_key(&loc));
        }
        if locs.len() > 1 {
            let (ax, ay, aw) = na;
            let (bx, by, bw) = nb;
            println!("UHHHHHHHHHHH MANY POSSIBILITIES HERE {ax}:{ay}:{aw} vs {bx}:{by}:{bw}");
            println!("{wa:?} ({wna}):", wna = wa.to_string(edev.db));
            for (&cell, &wire) in &locs_a {
                println!("  {wire}", wire = cell.wire(wire).to_string(edev.db));
            }
            println!("{wb:?} ({wnb}):", wnb = wb.to_string(edev.db));
            for (&cell, &wire) in &locs_b {
                println!("  {wire}", wire = cell.wire(wire).to_string(edev.db));
            }
            println!("common {locs:?}");
            panic!("welp");
        }
    }
    if locs.is_empty() {
        let (ax, ay, aw) = na;
        let (bx, by, bw) = nb;
        println!("NO SPEAKA ENGLISH {ax}:{ay}:{aw} vs {bx}:{by}:{bw}");
        println!("{wa:?} ({wna}):", wna = wa.to_string(edev.db));
        for (&cell, &wire) in &locs_a {
            println!("  {wire}", wire = cell.wire(wire).to_string(edev.db));
        }
        println!("{wb:?} ({wnb}):", wnb = wb.to_string(edev.db));
        for (&cell, &wire) in &locs_b {
            println!("  {wire}", wire = cell.wire(wire).to_string(edev.db));
        }
        println!("common {locs:?}");
    }
    let cell = locs.iter().copied().next().unwrap();
    (cell, locs_a[&cell], locs_b[&cell])
}
