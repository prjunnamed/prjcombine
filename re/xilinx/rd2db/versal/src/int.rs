use prjcombine_interconnect::db::{
    IntDb, NodeTileId, NodeWireId, TermInfo, TermKind, TermSlotId, TermSlotInfo, WireId, WireKind,
};
use prjcombine_interconnect::dir::{Dir, DirMap, DirPartMap};
use prjcombine_re_xilinx_naming::db::NamingDb;
use prjcombine_re_xilinx_naming_versal::DeviceNaming;
use prjcombine_re_xilinx_naming_versal::{
    BUFDIV_LEAF_SWZ_A, BUFDIV_LEAF_SWZ_AH, BUFDIV_LEAF_SWZ_B, BUFDIV_LEAF_SWZ_BH,
};
use prjcombine_re_xilinx_rawdump::{Coord, Part, TkWire};
use prjcombine_versal::bels;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use unnamed_entity::{EntityId, EntityPartVec};

use prjcombine_re_xilinx_rd2db_interconnect::{IntBuilder, XNodeInfo, XNodeRef};

trait IntBuilderExt {
    fn mux_out_pair(&mut self, name: impl Into<String>, raw_names: &[impl AsRef<str>; 2])
    -> WireId;

    fn branch_pair(
        &mut self,
        src: WireId,
        dir: Dir,
        name: impl Into<String>,
        raw_names: &[impl AsRef<str>; 2],
    ) -> WireId;
}

impl IntBuilderExt for IntBuilder<'_> {
    fn mux_out_pair(
        &mut self,
        name: impl Into<String>,
        raw_names: &[impl AsRef<str>; 2],
    ) -> WireId {
        let w = self.mux_out(name, &[""]);
        for (sub, name) in raw_names.iter().enumerate() {
            self.extra_name_sub(name.as_ref(), sub, w);
        }
        w
    }

    fn branch_pair(
        &mut self,
        src: WireId,
        dir: Dir,
        name: impl Into<String>,
        raw_names: &[impl AsRef<str>; 2],
    ) -> WireId {
        let w = self.branch(src, dir, name, &[""]);
        for (sub, name) in raw_names.iter().enumerate() {
            self.extra_name_sub(name.as_ref(), sub, w);
        }
        w
    }
}

trait XNodeInfoExt {
    fn ref_int_side(self, xy: Coord, side: Dir, slot: usize) -> Self;
}

impl XNodeInfoExt for XNodeInfo<'_, '_> {
    fn ref_int_side(mut self, xy: Coord, side: Dir, slot: usize) -> Self {
        self.refs.push(XNodeRef {
            xy,
            naming: None,
            tile_map: [(
                NodeTileId::from_idx(match side {
                    Dir::W => 0,
                    Dir::E => 1,
                    _ => unreachable!(),
                }),
                NodeTileId::from_idx(slot),
            )]
            .into_iter()
            .collect(),
        });
        self
    }
}

const INTF_KINDS: &[(Dir, &str, &str, bool)] = &[
    (Dir::W, "INTF_LOCF_BL_TILE", "INTF.W", false),
    (Dir::W, "INTF_LOCF_TL_TILE", "INTF.W", false),
    (Dir::E, "INTF_LOCF_BR_TILE", "INTF.E", false),
    (Dir::E, "INTF_LOCF_TR_TILE", "INTF.E", false),
    (Dir::W, "INTF_ROCF_BL_TILE", "INTF.W", false),
    (Dir::W, "INTF_ROCF_TL_TILE", "INTF.W", false),
    (Dir::E, "INTF_ROCF_BR_TILE", "INTF.E", false),
    (Dir::E, "INTF_ROCF_TR_TILE", "INTF.E", false),
    (Dir::W, "INTF_HB_LOCF_BL_TILE", "INTF.W.HB", false),
    (Dir::W, "INTF_HB_LOCF_TL_TILE", "INTF.W.HB", false),
    (Dir::E, "INTF_HB_LOCF_BR_TILE", "INTF.E.HB", false),
    (Dir::E, "INTF_HB_LOCF_TR_TILE", "INTF.E.HB", false),
    (Dir::W, "INTF_HB_ROCF_BL_TILE", "INTF.W.HB", false),
    (Dir::W, "INTF_HB_ROCF_TL_TILE", "INTF.W.HB", false),
    (Dir::E, "INTF_HB_ROCF_BR_TILE", "INTF.E.HB", false),
    (Dir::E, "INTF_HB_ROCF_TR_TILE", "INTF.E.HB", false),
    (Dir::W, "INTF_HDIO_LOCF_BL_TILE", "INTF.W.HDIO", false),
    (Dir::W, "INTF_HDIO_LOCF_TL_TILE", "INTF.W.HDIO", false),
    (Dir::E, "INTF_HDIO_LOCF_BR_TILE", "INTF.E.HDIO", false),
    (Dir::E, "INTF_HDIO_LOCF_TR_TILE", "INTF.E.HDIO", false),
    (Dir::W, "INTF_HDIO_ROCF_BL_TILE", "INTF.W.HDIO", false),
    (Dir::W, "INTF_HDIO_ROCF_TL_TILE", "INTF.W.HDIO", false),
    (Dir::E, "INTF_HDIO_ROCF_BR_TILE", "INTF.E.HDIO", false),
    (Dir::E, "INTF_HDIO_ROCF_TR_TILE", "INTF.E.HDIO", false),
    (Dir::W, "INTF_CFRM_BL_TILE", "INTF.W.PSS", false),
    (Dir::W, "INTF_CFRM_TL_TILE", "INTF.W.PSS", false),
    (Dir::W, "INTF_PSS_BL_TILE", "INTF.W.TERM.PSS", true),
    (Dir::W, "INTF_PSS_TL_TILE", "INTF.W.TERM.PSS", true),
    (Dir::W, "INTF_GT_BL_TILE", "INTF.W.TERM.GT", true),
    (Dir::W, "INTF_GT_TL_TILE", "INTF.W.TERM.GT", true),
    (Dir::E, "INTF_GT_BR_TILE", "INTF.E.TERM.GT", true),
    (Dir::E, "INTF_GT_TR_TILE", "INTF.E.TERM.GT", true),
];

const BLI_CLE_INTF_KINDS: &[(Dir, &str, &str, bool)] = &[
    (Dir::E, "BLI_CLE_BOT_CORE", "INTF.BLI_CLE.E.S", false),
    (Dir::E, "BLI_CLE_TOP_CORE", "INTF.BLI_CLE.E.N", true),
    (Dir::W, "BLI_CLE_BOT_CORE_MY", "INTF.BLI_CLE.W.S", false),
    (Dir::W, "BLI_CLE_TOP_CORE_MY", "INTF.BLI_CLE.W.N", true),
];

struct IntMaker<'a> {
    builder: IntBuilder<'a>,
    long_term_slots: DirPartMap<TermSlotId>,
    term_slot_intf: TermSlotId,
    long_main_passes: DirPartMap<TermKind>,
    // how many mental illnesses do you think I could be diagnosed with just from this repo?
    sng_fixup_map: BTreeMap<NodeWireId, NodeWireId>,
    term_wires: DirMap<EntityPartVec<WireId, TermInfo>>,
    term_wires_l: DirPartMap<EntityPartVec<WireId, TermInfo>>,
    bnodes: Vec<WireId>,
    bnode_outs: Vec<WireId>,
    bounces: Vec<WireId>,
    term_logic_outs: EntityPartVec<WireId, TermInfo>,
    dev_naming: &'a DeviceNaming,
}

impl IntMaker<'_> {
    fn fill_term_slots(&mut self) {
        let slot_lw = self
            .builder
            .db
            .term_slots
            .insert(
                "LW".into(),
                TermSlotInfo {
                    opposite: TermSlotId::from_idx(0),
                },
            )
            .0;
        let slot_le = self
            .builder
            .db
            .term_slots
            .insert("LE".into(), TermSlotInfo { opposite: slot_lw })
            .0;
        self.builder.db.term_slots[slot_lw].opposite = slot_le;

        self.long_term_slots.insert(Dir::W, slot_lw);
        self.long_term_slots.insert(Dir::E, slot_le);

        self.term_slot_intf = self
            .builder
            .db
            .term_slots
            .insert(
                "INTF".into(),
                TermSlotInfo {
                    opposite: self.term_slot_intf,
                },
            )
            .0;
        self.builder.db.term_slots[self.term_slot_intf].opposite = self.term_slot_intf;
    }

    fn fill_wires_long(&mut self) {
        for (fwd, name, l, ll) in [
            (Dir::E, "LONG.6", 3, 6),
            (Dir::N, "LONG.7", 7, 7),
            (Dir::E, "LONG.10", 5, 10),
            (Dir::N, "LONG.12", 12, 12),
        ] {
            let (slot_f, slot_b) = if fwd == Dir::E {
                (self.long_term_slots[fwd], self.long_term_slots[!fwd])
            } else {
                (self.builder.term_slots[fwd], self.builder.term_slots[!fwd])
            };
            let bwd = !fwd;
            for i in 0..8 {
                let mut w_f = self.builder.mux_out(
                    format!("{name}.{fwd}.{i}.0"),
                    &[format!("OUT_{fwd}{fwd}{ll}_BEG{i}")],
                );
                let mut w_b = self.builder.mux_out(
                    format!("{name}.{bwd}.{i}.0"),
                    &[format!("OUT_{bwd}{bwd}{ll}_BEG{i}")],
                );
                for j in 1..=l {
                    let n_f = self.builder.wire(
                        format!("{name}.{fwd}.{i}.{j}"),
                        WireKind::Branch(slot_b),
                        &[""],
                    );
                    let n_b = self.builder.wire(
                        format!("{name}.{bwd}.{i}.{j}"),
                        WireKind::Branch(slot_f),
                        &[""],
                    );
                    if matches!(fwd, Dir::W | Dir::E) {
                        self.long_main_passes
                            .get_mut(!fwd)
                            .unwrap()
                            .wires
                            .insert(n_f, TermInfo::PassFar(w_f));
                        self.long_main_passes
                            .get_mut(!bwd)
                            .unwrap()
                            .wires
                            .insert(n_b, TermInfo::PassFar(w_b));
                        self.term_wires_l
                            .get_mut(fwd)
                            .unwrap()
                            .insert(n_b, TermInfo::PassNear(w_f));
                        self.term_wires_l
                            .get_mut(bwd)
                            .unwrap()
                            .insert(n_f, TermInfo::PassNear(w_b));
                    } else {
                        self.builder.conn_branch(w_f, fwd, n_f);
                        self.builder.conn_branch(w_b, bwd, n_b);
                        self.term_wires[fwd].insert(n_b, TermInfo::PassNear(w_f));
                        self.term_wires[bwd].insert(n_f, TermInfo::PassNear(w_b));
                    }
                    w_f = n_f;
                    w_b = n_b;
                }
                self.builder
                    .extra_name(format!("IN_{fwd}{fwd}{ll}_END{i}"), w_f);
                self.builder
                    .extra_name(format!("IN_{bwd}{bwd}{ll}_END{i}"), w_b);
                if i == 0 && fwd == Dir::E && ll == 6 {
                    self.builder.branch(
                        w_f,
                        Dir::S,
                        format!("{name}.{fwd}.{i}.{l}.S"),
                        &[format!("IN_{fwd}{fwd}{ll}_BLS_{i}")],
                    );
                }
                if i == 7 && fwd == Dir::E && ll == 10 {
                    self.builder.branch(
                        w_f,
                        Dir::N,
                        format!("{name}.{fwd}.{i}.{l}.N"),
                        &[format!("IN_{fwd}{fwd}{ll}_BLN_{i}")],
                    );
                    self.builder
                        .branch(w_f, Dir::E, format!("{name}.{fwd}.{i}.{l}.E"), &[""]);
                }
            }
        }
    }

    fn fill_wires_sdqnode(&mut self) {
        for (iq, q) in ['E', 'N', 'S', 'W'].into_iter().enumerate() {
            for i in 0..32 {
                match (q, i) {
                    ('E', 0 | 2) | ('W', 0 | 2) | ('N', 0) => {
                        let w = self.builder.mux_out_pair(
                            format!("SDQNODE.{q}.{i}"),
                            &[format!("OUT_{q}NODE_W_{i}"), format!("OUT_{q}NODE_E_{i}")],
                        );
                        self.builder.branch_pair(
                            w,
                            Dir::S,
                            format!("SDQNODE.{q}.{i}.S"),
                            &[
                                format!("IN_{q}NODE_W_BLS_{i}"),
                                format!("IN_{q}NODE_E_BLS_{i}"),
                            ],
                        );
                    }
                    ('E', 29 | 31) | ('W', 31) | ('S', 31) => {
                        let w = self.builder.mux_out_pair(
                            format!("SDQNODE.{q}.{i}"),
                            &[format!("OUT_{q}NODE_W_{i}"), format!("OUT_{q}NODE_E_{i}")],
                        );
                        self.builder.branch_pair(
                            w,
                            Dir::N,
                            format!("SDQNODE.{q}.{i}.N"),
                            &[
                                format!("IN_{q}NODE_W_BLN_{i}"),
                                format!("IN_{q}NODE_E_BLN_{i}"),
                            ],
                        );
                    }
                    _ => {
                        // TODO not the true permutation
                        let a = [0, 11, 1, 2, 3, 4, 5, 6, 7, 8, 9, 13, 14, 15, 10, 12][i >> 1];
                        let aaw = a + 16 + iq * 32;
                        let aae = a + iq * 32;
                        let b = i & 1;
                        self.builder.mux_out_pair(
                            format!("SDQNODE.{q}.{i}"),
                            &[
                                format!("INT_NODE_SDQ_ATOM_{aaw}_INT_OUT{b}"),
                                format!("INT_NODE_SDQ_ATOM_{aae}_INT_OUT{b}"),
                            ],
                        );
                    }
                }
            }
        }
    }

    fn fill_wires_sdq(&mut self) {
        for (fwd, name, length, num) in [
            (Dir::E, "SNG", 1, 16),
            (Dir::N, "SNG", 1, 16),
            (Dir::E, "DBL", 2, 8),
            (Dir::N, "DBL", 2, 8),
            (Dir::E, "QUAD", 4, 8),
            (Dir::N, "QUAD", 4, 8),
        ] {
            let bwd = !fwd;
            for i in 0..num {
                let mut w_f = self.builder.mux_out_pair(
                    format!("{name}.{fwd}.{i}.0"),
                    &[
                        format!("OUT_{fwd}{fwd}{length}_W_BEG{i}"),
                        format!("OUT_{fwd}{fwd}{length}_E_BEG{i}"),
                    ],
                );
                let mut w_b = self.builder.mux_out_pair(
                    format!("{name}.{bwd}.{i}.0"),
                    &[
                        format!("OUT_{bwd}{bwd}{length}_W_BEG{i}"),
                        format!("OUT_{bwd}{bwd}{length}_E_BEG{i}"),
                    ],
                );
                if fwd == Dir::E && length == 1 {
                    self.builder.extra_name(format!("IF_HBUS_EBUS{i}"), w_f);
                    self.builder.extra_name(format!("IF_HBUS_W_EBUS{i}"), w_f);
                    self.builder.extra_name(format!("IF_HBUS_WBUS{i}"), w_b);
                    self.builder.extra_name(format!("IF_HBUS_E_WBUS{i}"), w_b);
                }
                if bwd == Dir::W && i == 0 && length == 1 {
                    let w =
                        self.builder
                            .branch(w_b, Dir::S, format!("{name}.{bwd}.{i}.0.S"), &[""]);
                    self.builder
                        .extra_name_tile_sub("CLE_BC_CORE", "BNODE_TAP0", 1, w);
                    self.builder
                        .extra_name_tile_sub("CLE_BC_CORE_MX", "BNODE_TAP0", 1, w);
                    self.builder.extra_name_tile_sub("SLL", "BNODE_TAP0", 1, w);
                    self.builder.extra_name_tile_sub("SLL2", "BNODE_TAP0", 1, w);
                }
                for j in 1..=length {
                    let n_f = self
                        .builder
                        .branch(w_f, fwd, format!("{name}.{fwd}.{i}.{j}"), &[""]);
                    let n_b = self
                        .builder
                        .branch(w_b, bwd, format!("{name}.{bwd}.{i}.{j}"), &[""]);
                    self.term_wires[fwd].insert(n_b, TermInfo::PassNear(w_f));
                    self.term_wires[bwd].insert(n_f, TermInfo::PassNear(w_b));
                    if length == 1 && fwd == Dir::E {
                        let swz = [0, 7, 8, 9, 10, 11, 12, 13, 14, 15, 1, 2, 3, 4, 5, 6][i];
                        self.builder.extra_name_sub(
                            format!("INT_SDQ_RED_ATOM_{ii}_INT_OUT0", ii = 16 + swz),
                            0,
                            w_f,
                        );
                        self.builder.extra_name_sub(
                            format!("INT_SDQ_RED_ATOM_{ii}_INT_OUT0", ii = 32 + swz),
                            1,
                            w_b,
                        );
                        self.sng_fixup_map.insert(
                            (NodeTileId::from_idx(0), w_f),
                            (NodeTileId::from_idx(1), n_f),
                        );
                        self.sng_fixup_map.insert(
                            (NodeTileId::from_idx(1), w_b),
                            (NodeTileId::from_idx(0), n_b),
                        );
                    }
                    w_f = n_f;
                    w_b = n_b;
                    match (fwd, length, j) {
                        (Dir::E, 2, 1) => {
                            for &(side, tkn, _, term) in INTF_KINDS {
                                if term && side == Dir::W {
                                    let ii = i + 24;
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_WBUS{ii}"),
                                        w_f,
                                    );
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_EBUS{ii}"),
                                        w_b,
                                    );
                                } else {
                                    let ii = i + 24;
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_EBUS{ii}"),
                                        w_f,
                                    );
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_W_EBUS{ii}"),
                                        w_f,
                                    );
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_WBUS{ii}"),
                                        w_b,
                                    );
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_E_WBUS{ii}"),
                                        w_b,
                                    );
                                }
                                if term && side == Dir::E {
                                    let ii = i + 16;
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_WBUS{ii}"),
                                        w_f,
                                    );
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_EBUS{ii}"),
                                        w_b,
                                    );
                                } else {
                                    let ii = i + 16;
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_EBUS{ii}"),
                                        w_f,
                                    );
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_W_EBUS{ii}"),
                                        w_f,
                                    );
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_WBUS{ii}"),
                                        w_b,
                                    );
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_E_WBUS{ii}"),
                                        w_b,
                                    );
                                }
                            }
                        }
                        (Dir::E, 4, 3) => {
                            for &(side, tkn, _, term) in INTF_KINDS {
                                if term && side == Dir::W {
                                    let ii = i + 56;
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_WBUS{ii}"),
                                        w_f,
                                    );
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_EBUS{ii}"),
                                        w_b,
                                    );
                                } else {
                                    let ii = i + 56;
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_EBUS{ii}"),
                                        w_f,
                                    );
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_W_EBUS{ii}"),
                                        w_f,
                                    );
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_WBUS{ii}"),
                                        w_b,
                                    );
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_E_WBUS{ii}"),
                                        w_b,
                                    );
                                }
                                if term && side == Dir::E {
                                    let ii = i + 40;
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_WBUS{ii}"),
                                        w_f,
                                    );
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_EBUS{ii}"),
                                        w_b,
                                    );
                                } else {
                                    let ii = i + 40;
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_EBUS{ii}"),
                                        w_f,
                                    );
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_W_EBUS{ii}"),
                                        w_f,
                                    );
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_WBUS{ii}"),
                                        w_b,
                                    );
                                    self.builder.extra_name_tile(
                                        tkn,
                                        format!("IF_HBUS_E_WBUS{ii}"),
                                        w_b,
                                    );
                                }
                            }
                            if i == 0 {
                                let w = self.builder.branch(
                                    w_f,
                                    Dir::S,
                                    format!("{name}.{fwd}.{i}.{j}.S"),
                                    &[""],
                                );
                                self.builder.extra_name("IF_LBC_N_BNODE_SOUTHBUS", w);
                            }
                        }
                        (Dir::N, 1, 1) => {
                            let ii = i;
                            self.builder.extra_name(format!("IF_INT_VSINGLE{ii}"), w_b);
                            let ii = i + 16;
                            self.builder.extra_name(format!("IF_INT_VSINGLE{ii}"), w_f);
                        }
                        (Dir::N, 2, 1) => {
                            let ii = i + 32;
                            self.builder
                                .extra_name_sub(format!("IF_VBUS_S_NBUS{ii}"), 1, n_f);
                            let ii = i + 48;
                            self.builder
                                .extra_name_sub(format!("IF_VBUS_S_NBUS{ii}"), 0, n_f);
                        }
                        (Dir::N, 4, 1) => {
                            let ii = i + 64;
                            self.builder
                                .extra_name_sub(format!("IF_VBUS_S_NBUS{ii}"), 1, n_f);
                            let ii = i + 96;
                            self.builder
                                .extra_name_sub(format!("IF_VBUS_S_NBUS{ii}"), 0, n_f);
                        }
                        _ => (),
                    }
                }
                if length == 1 && fwd == Dir::E {
                    self.builder
                        .extra_name_sub(format!("IN_{fwd}{fwd}{length}_E_END{i}"), 0, w_f);
                    self.builder
                        .extra_name_sub(format!("IN_{fwd}{fwd}{length}_W_END{i}"), 1, w_f);
                    self.builder
                        .extra_name_sub(format!("IN_{bwd}{bwd}{length}_E_END{i}"), 0, w_b);
                    self.builder
                        .extra_name_sub(format!("IN_{bwd}{bwd}{length}_W_END{i}"), 1, w_b);
                } else {
                    self.builder
                        .extra_name_sub(format!("IN_{fwd}{fwd}{length}_W_END{i}"), 0, w_f);
                    self.builder
                        .extra_name_sub(format!("IN_{fwd}{fwd}{length}_E_END{i}"), 1, w_f);
                    self.builder
                        .extra_name_sub(format!("IN_{bwd}{bwd}{length}_W_END{i}"), 0, w_b);
                    self.builder
                        .extra_name_sub(format!("IN_{bwd}{bwd}{length}_E_END{i}"), 1, w_b);
                }
            }
        }
    }

    fn fill_wires_cle_imux(&mut self) {
        for i in 0..13 {
            let w = self.builder.mux_out_pair(
                format!("CLE.IMUX.CTRL.{i}"),
                &[format!("CTRL_L_B{i}"), format!("CTRL_R_B{i}")],
            );
            for tkn in ["CLE_W_CORE", "CLE_E_CORE"] {
                self.builder.extra_name_tile(
                    tkn,
                    match i {
                        0 => "CLE_SLICEL_TOP_0_CLK",
                        1 => "CLE_SLICEM_TOP_1_CLK",
                        2 => "CLE_SLICEL_TOP_0_RST",
                        3 => "CLE_SLICEM_TOP_1_RST",
                        4 => "CLE_SLICEL_TOP_0_CKEN1",
                        5 => "CLE_SLICEL_TOP_0_CKEN2",
                        6 => "CLE_SLICEL_TOP_0_CKEN3",
                        7 => "CLE_SLICEL_TOP_0_CKEN4",
                        8 => "CLE_SLICEM_TOP_1_WE",
                        9 => "CLE_SLICEM_TOP_1_CKEN1",
                        10 => "CLE_SLICEM_TOP_1_CKEN2",
                        11 => "CLE_SLICEM_TOP_1_CKEN3",
                        12 => "CLE_SLICEM_TOP_1_CKEN4",
                        _ => unreachable!(),
                    },
                    w,
                );
            }
            for tkn in ["CLE_W_VR_CORE", "CLE_E_VR_CORE"] {
                self.builder.extra_name_tile(
                    tkn,
                    match i {
                        0 => "CLE_SLICEL_VR_TOP_0_CLK",
                        1 => "CLE_SLICEM_VR_TOP_1_CLK",
                        2 => "CLE_SLICEL_VR_TOP_0_RST",
                        3 => "CLE_SLICEM_VR_TOP_1_RST",
                        4 => "CLE_SLICEL_VR_TOP_0_CKEN1",
                        5 => "CLE_SLICEL_VR_TOP_0_CKEN2",
                        6 => "CLE_SLICEL_VR_TOP_0_CKEN3",
                        7 => "CLE_SLICEL_VR_TOP_0_CKEN4",
                        8 => "CLE_SLICEM_VR_TOP_1_WE",
                        9 => "CLE_SLICEM_VR_TOP_1_CKEN1",
                        10 => "CLE_SLICEM_VR_TOP_1_CKEN2",
                        11 => "CLE_SLICEM_VR_TOP_1_CKEN3",
                        12 => "CLE_SLICEM_VR_TOP_1_CKEN4",
                        _ => unreachable!(),
                    },
                    w,
                );
            }
        }
    }

    fn fill_term_sn_extra(&mut self) {
        for (dir, wt, wf) in [
            (Dir::S, "LONG.10.E.7.5.N", "LONG.6.E.0.3"),
            (Dir::S, "SDQNODE.E.29.N", "SDQNODE.E.0"),
            (Dir::S, "SDQNODE.E.31.N", "SDQNODE.E.2"),
            (Dir::S, "SDQNODE.S.31.N", "SDQNODE.N.0"),
            (Dir::S, "SDQNODE.W.31.N", "SDQNODE.W.0"),
            (Dir::N, "LONG.6.E.0.3.S", "LONG.10.E.7.5"),
            (Dir::N, "OUT.1.S", "LONG.10.E.7.5.E"),
            (Dir::N, "SDQNODE.E.0.S", "SDQNODE.E.29"),
            (Dir::N, "SDQNODE.E.2.S", "SDQNODE.E.31"),
            (Dir::N, "SDQNODE.N.0.S", "SDQNODE.S.31"),
            (Dir::N, "SDQNODE.W.0.S", "SDQNODE.W.31"),
            (Dir::N, "SDQNODE.W.2.S", "SDQNODE.W.31"),
        ] {
            self.term_wires[dir].insert(
                self.builder.db.wires.get(wt).unwrap().0,
                TermInfo::PassNear(self.builder.db.wires.get(wf).unwrap().0),
            );
        }
    }

    fn fill_wires(&mut self) {
        let main_pass_lw = TermKind {
            slot: self.long_term_slots[Dir::W],
            wires: Default::default(),
        };
        let main_pass_le = TermKind {
            slot: self.long_term_slots[Dir::E],
            wires: Default::default(),
        };
        self.long_main_passes.insert(Dir::W, main_pass_lw);
        self.long_main_passes.insert(Dir::E, main_pass_le);

        // common wires
        self.builder.wire("VCC", WireKind::Tie1, &["VCC_WIRE"]);
        self.fill_wires_long();

        for i in 0..6 {
            let w = self.builder.mux_out(format!("IMUX.LAG{i}"), &[""]);
            self.builder
                .extra_name_tile_sub("SLL", format!("LAG_CASCOUT_TXI{i}"), 0, w);
            self.builder
                .extra_name_tile_sub("SLL2", format!("LAG_CASCOUT_TXI{i}"), 0, w);
        }

        for i in 0..6 {
            let w = self.builder.logic_out(format!("OUT.LAG{i}"), &[""]);
            self.builder
                .extra_name_tile_sub("CLE_BC_CORE", format!("VCC_WIRE{i}"), 0, w);
            self.builder
                .extra_name_tile_sub("CLE_BC_CORE_MX", format!("VCC_WIRE{i}"), 0, w);
            self.builder
                .extra_name_tile_sub("SLL", format!("LAG_OUT{i}"), 0, w);
            self.builder
                .extra_name_tile_sub("SLL2", format!("LAG_OUT{i}"), 0, w);
        }

        // wires belonging to interconnect left/right half-nodes
        for i in 0..48 {
            let w = self.builder.logic_out(format!("OUT.{i}"), &[""]);
            self.builder
                .extra_name_tile_sub("INT", format!("LOGIC_OUTS_W{i}"), 0, w);
            self.builder
                .extra_name_tile_sub("INT", format!("LOGIC_OUTS_E{i}"), 1, w);
            match i {
                1 | 4 | 5 => {
                    self.builder.branch_pair(
                        w,
                        Dir::S,
                        format!("OUT.{i}.S"),
                        &[
                            format!("IN_LOGIC_OUTS_W_BLS_{i}"),
                            format!("IN_LOGIC_OUTS_E_BLS_{i}"),
                        ],
                    );
                }
                _ => (),
            }
            let cw = self.builder.wire(
                format!("CLE.OUT.{i}"),
                WireKind::Branch(self.term_slot_intf),
                &[""],
            );
            self.builder.test_mux_pass(cw);
            for tkn in ["CLE_BC_CORE", "CLE_BC_CORE_MX", "SLL", "SLL2"] {
                self.builder
                    .extra_name_tile_sub(tkn, format!("LOGIC_OUTS_W{i}"), 0, cw);
                self.builder
                    .extra_name_tile_sub(tkn, format!("LOGIC_OUTS_E{i}"), 1, cw);
            }

            self.term_logic_outs.insert(cw, TermInfo::PassNear(w));
        }

        self.fill_wires_sdqnode();
        self.fill_wires_sdq();

        for i in 0..64 {
            for j in 0..2 {
                self.builder.mux_out_pair(
                    format!("INODE.{i}.{j}"),
                    &[
                        format!(
                            "INT_NODE_IMUX_ATOM_{ii}_INT_OUT{j}",
                            ii = 32 + (i % 32) + (i / 32) * 64
                        ),
                        format!(
                            "INT_NODE_IMUX_ATOM_{ii}_INT_OUT{j}",
                            ii = (i % 32) + (i / 32) * 64
                        ),
                    ],
                );
            }
        }

        for i in 0..96 {
            self.builder.mux_out_pair(
                format!("IMUX.IMUX.{i}"),
                &[format!("IMUX_B_W{i}"), format!("IMUX_B_E{i}")],
            );
        }

        for i in 0..32 {
            let w = self.builder.mux_out(format!("IMUX.BOUNCE.{i}"), &[""]);
            self.builder
                .extra_name_tile_sub("INT", format!("BOUNCE_W{i}"), 0, w);
            self.builder
                .extra_name_tile_sub("INT", format!("BOUNCE_E{i}"), 1, w);
            self.bounces.push(w);
        }

        for i in 0..64 {
            let w = self.builder.wire(
                format!("BNODE.{i}"),
                WireKind::Branch(self.term_slot_intf),
                &[""],
            );
            self.builder
                .extra_name_tile_sub("INT", format!("BNODE_W{i}"), 0, w);
            self.builder
                .extra_name_tile_sub("INT", format!("BNODE_E{i}"), 1, w);

            self.bnodes.push(w);
        }

        let w = self
            .builder
            .test_out("TEST.TMR_DFT", &["INTF_MUX2_TMR_GREEN_TMR_DFT"]);
        for &(_, tkn, _, _) in BLI_CLE_INTF_KINDS {
            for i in 0..4 {
                self.builder
                    .extra_name_tile(tkn, format!("INTF_MUX2_TMR_GREEN_{i}_TMR_DFT"), w);
            }
        }

        for i in 0..32 {
            let w = self.builder.mux_out(format!("CLE.BNODE.{i}"), &[""]);
            self.builder
                .extra_name_sub(format!("BNODE_OUTS_W{i}"), 0, w);
            self.builder
                .extra_name_sub(format!("BNODE_OUTS_E{i}"), 1, w);
            self.bnode_outs.push(w);
        }

        for i in 0..12 {
            let w = self.builder.mux_out(format!("CLE.CNODE.{i}"), &[""]);
            self.builder
                .extra_name_sub(format!("CNODE_OUTS_W{i}"), 0, w);
            self.builder
                .extra_name_sub(format!("CNODE_OUTS_E{i}"), 1, w);
            self.bnode_outs.push(w);
        }

        self.fill_wires_cle_imux();

        for i in 0..4 {
            for j in 1..4 {
                let w = self.builder.wire(
                    format!("BLI_CLE.IMUX.IRI{i}.FAKE_CE{j}"),
                    WireKind::Tie0,
                    &[""],
                );
                for (_, tkn, _, _) in BLI_CLE_INTF_KINDS {
                    let idxs = match (i, j) {
                        (0, 1) => [24, 30, 39, 45],
                        (0, 2) => [25, 31, 40, 46],
                        (0, 3) => [26, 32, 41, 47],
                        (1, 1) => [0, 6, 15, 21],
                        (1, 2) => [1, 7, 16, 22],
                        (1, 3) => [2, 8, 17, 23],
                        (2, 1) => [27, 33, 36, 42],
                        (2, 2) => [28, 34, 37, 43],
                        (2, 3) => [29, 35, 38, 44],
                        (3, 1) => [3, 9, 12, 18],
                        (3, 2) => [4, 10, 13, 19],
                        (3, 3) => [5, 11, 14, 20],
                        _ => unreachable!(),
                    };
                    for idx in idxs {
                        self.builder
                            .extra_name_tile(tkn, format!("GND_WIRE{idx}"), w);
                    }
                }
            }
        }

        for i in 0..16 {
            let w = self
                .builder
                .wire(format!("CLE.GCLK.{i}"), WireKind::ClkOut, &[""]);
            self.builder.extra_name_sub(format!("GCLK_B{i}"), 1, w);
        }

        for i in 0..4 {
            let rg = match i % 2 {
                0 => "RED",
                1 => "GREEN",
                _ => unreachable!(),
            };
            let w = self.builder.mux_out(format!("INTF.IMUX.IRI{i}.CLK"), &[""]);
            for (_, tkn, _, _) in INTF_KINDS {
                self.builder
                    .extra_name_tile(tkn, format!("INTF_IRI_QUADRANT_{rg}_{i}_CLK"), w);
            }
            let w = self.builder.mux_out(format!("INTF.IMUX.IRI{i}.RST"), &[""]);
            for (_, tkn, _, _) in INTF_KINDS {
                self.builder
                    .extra_name_tile(tkn, format!("INTF_IRI_QUADRANT_{rg}_{i}_RST"), w);
            }
            for j in 0..4 {
                let w = self
                    .builder
                    .mux_out(format!("INTF.IMUX.IRI{i}.CE{j}"), &[""]);
                for (_, tkn, _, _) in INTF_KINDS {
                    self.builder.extra_name_tile(
                        tkn,
                        format!("INTF_IRI_QUADRANT_{rg}_{i}_CE{j}"),
                        w,
                    );
                }
            }
        }

        for i in 0..12 {
            for j in 0..2 {
                self.builder.mux_out(
                    format!("INTF.CNODE.{i}.{j}"),
                    &[format!("INTF_CNODE_ATOM_{i}_INT_OUT{j}")],
                );
            }
        }

        for i in 0..16 {
            self.builder.wire(
                format!("INTF.GCLK.{i}"),
                WireKind::ClkOut,
                &[format!("IF_GCLK_GCLK_B{i}")],
            );
        }

        for i in 0..20 {
            for j in 0..2 {
                self.builder.mux_out_pair(
                    format!("RCLK.INODE.{i}.{j}"),
                    &[
                        format!(
                            "INT_NODE_IMUX_ATOM_RCLK_{ii}_INT_OUT{j}",
                            ii = i % 10 + i / 10 * 20 + 10
                        ),
                        format!(
                            "INT_NODE_IMUX_ATOM_RCLK_{ii}_INT_OUT{j}",
                            ii = i % 10 + i / 10 * 20
                        ),
                    ],
                );
            }
        }

        for i in 0..2 {
            for j in 0..20 {
                self.builder.mux_out_pair(
                    format!("RCLK.IMUX.{i}.{j}"),
                    &[
                        format!("IF_INT2COE_W_INT_RCLK_TO_CLK_B_{i}_{j}"),
                        format!("IF_INT2COE_E_INT_RCLK_TO_CLK_B_{i}_{j}"),
                    ],
                );
            }
        }

        self.fill_term_sn_extra();

        self.builder.extract_main_passes();
        self.builder.db.terms.insert(
            "MAIN.LW".into(),
            self.long_main_passes.remove(Dir::W).unwrap(),
        );
        self.builder.db.terms.insert(
            "MAIN.LE".into(),
            self.long_main_passes.remove(Dir::E).unwrap(),
        );
        for (dir, wires) in std::mem::take(&mut self.term_wires) {
            self.builder.db.terms.insert(
                format!("TERM.{dir}"),
                TermKind {
                    slot: self.builder.term_slots[dir],
                    wires,
                },
            );
        }
        for (dir, wires) in std::mem::take(&mut self.term_wires_l) {
            self.builder.db.terms.insert(
                format!("TERM.L{dir}"),
                TermKind {
                    slot: self.long_term_slots[dir],
                    wires,
                },
            );
        }
    }

    fn fill_tiles_int(&mut self) {
        self.builder.node_type("INT", "INT", "INT");
        let nk = self.builder.db.get_node("INT");
        let node = &mut self.builder.db.nodes[nk];
        node.tiles.push(());
        for (wt, mux) in &mut node.muxes {
            let wtn = self.builder.db.wires.key(wt.1);
            if !wtn.starts_with("INODE") && !wtn.starts_with("SDQNODE") {
                continue;
            }
            mux.ins = BTreeSet::from_iter(
                mux.ins
                    .iter()
                    .map(|w| self.sng_fixup_map.get(w).copied().unwrap_or(*w)),
            );
        }
        let naming = self.builder.ndb.get_node_naming("INT");
        let naming = &mut self.builder.ndb.node_namings[naming];
        for (&wf, &wt) in &self.sng_fixup_map {
            let name = naming.wires[&wf].clone();
            naming.wires.insert(wt, name);
        }
    }

    fn fill_tiles_cle_bc(&mut self) {
        for (kind, tkn) in [
            ("CLE_BC", "CLE_BC_CORE"),
            ("CLE_BC", "CLE_BC_CORE_MX"),
            ("CLE_BC.SLL", "SLL"),
            ("CLE_BC.SLL2", "SLL2"),
        ] {
            for &xy in self.builder.rd.tiles_by_kind_name(tkn) {
                let td = &self.builder.rd.tiles[&self.builder.delta(xy, 0, -1)];
                if self.builder.rd.tile_kinds.key(td.kind) != tkn {
                    continue;
                }
                let tu = &self.builder.rd.tiles[&self.builder.delta(xy, 0, 1)];
                if self.builder.rd.tile_kinds.key(tu.kind) != tkn {
                    continue;
                }
                let int_xy_w = self.builder.walk_to_int(xy, Dir::W, false).unwrap();
                let int_xy_e = self.builder.walk_to_int(xy, Dir::E, false).unwrap();
                let mut bels = vec![];
                if kind != "CLE_BC" {
                    let mut bel = self.builder.bel_virtual(bels::LAGUNA);
                    for i in 0..6 {
                        bel = bel
                            .extra_int_in(format!("IN{i}"), &[format!("LAG_CASCOUT_TXI{i}")])
                            .extra_int_out(format!("OUT{i}"), &[format!("LAG_OUT{i}")])
                            .extra_wire(format!("UBUMP{i}"), &[format!("UBUMP{i}")]);
                    }
                    bels.push(bel);
                }
                self.builder
                    .xnode(kind, kind, xy)
                    .num_tiles(2)
                    .ref_int_side(int_xy_w, Dir::E, 0)
                    .ref_int_side(int_xy_e, Dir::W, 1)
                    .extract_muxes()
                    .bels(bels)
                    .extract();
                let tile = &self.builder.rd.tiles[&xy];
                let tk = &self.builder.rd.tile_kinds[tile.kind];
                let naming = self.builder.ndb.get_node_naming("CLE_BC");
                let int_naming = self.builder.ndb.get_node_naming("INT");
                for (int_xy, int_subtile, cle_tile, side) in [
                    (
                        int_xy_e,
                        NodeTileId::from_idx(0),
                        NodeTileId::from_idx(1),
                        Dir::W,
                    ),
                    (
                        int_xy_w,
                        NodeTileId::from_idx(1),
                        NodeTileId::from_idx(0),
                        Dir::E,
                    ),
                ] {
                    let naming = &self.builder.ndb.node_namings[naming];
                    let mut nodes = HashMap::new();
                    for &w in &self.bnode_outs {
                        if let Some(n) = naming.wires.get(&(cle_tile, w)) {
                            let n = self.builder.rd.wires.get(n).unwrap();
                            if let &TkWire::Connected(idx) = tk.wires.get(&n).unwrap().1 {
                                nodes.insert(tile.conn_wires[idx], w);
                            }
                        }
                    }
                    let int_tile = &self.builder.rd.tiles[&int_xy];
                    let int_tk = &self.builder.rd.tile_kinds[int_tile.kind];
                    let int_naming = &self.builder.ndb.node_namings[int_naming];
                    for &w in &self.bounces {
                        if let Some(n) = int_naming.wires.get(&(int_subtile, w)) {
                            let n = self.builder.rd.wires.get(n).unwrap();
                            if let &TkWire::Connected(idx) = int_tk.wires.get(&n).unwrap().1 {
                                nodes.insert(int_tile.conn_wires[idx], w);
                            }
                        }
                    }
                    let mut wires = EntityPartVec::new();
                    for &w in &self.bnodes {
                        if self.builder.db.wires[w] != WireKind::Branch(self.term_slot_intf) {
                            continue;
                        }
                        if let Some(n) = int_naming.wires.get(&(int_subtile, w)) {
                            let n = self.builder.rd.wires.get(n).unwrap();
                            if let &TkWire::Connected(idx) = int_tk.wires.get(&n).unwrap().1 {
                                if let Some(&cw) = nodes.get(&int_tile.conn_wires[idx]) {
                                    wires.insert(w, TermInfo::PassNear(cw));
                                }
                            }
                        }
                    }
                    self.builder.insert_term_merge(
                        &format!("CLE.{side}"),
                        TermKind {
                            slot: self.term_slot_intf,
                            wires,
                        },
                    );
                }
                break;
            }
        }

        for side in [Dir::W, Dir::E] {
            let t = self
                .builder
                .db
                .terms
                .get(&format!("CLE.{side}"))
                .unwrap()
                .1
                .clone();
            self.builder.db.terms.insert(format!("CLE.BLI.{side}"), t);
            self.builder.insert_term_merge(
                &format!("CLE.{side}"),
                TermKind {
                    slot: self.term_slot_intf,
                    wires: self.term_logic_outs.clone(),
                },
            );
        }
    }

    fn fill_tiles_intf(&mut self) {
        for &(side, tkn, name, _) in INTF_KINDS {
            for &xy in self.builder.rd.tiles_by_kind_name(tkn) {
                let td = &self.builder.rd.tiles[&self.builder.delta(xy, 0, -1)];
                if self.builder.rd.tile_kinds.key(td.kind) != tkn {
                    continue;
                }
                let tu = &self.builder.rd.tiles[&self.builder.delta(xy, 0, 1)];
                if self.builder.rd.tile_kinds.key(tu.kind) != tkn {
                    continue;
                }
                let int_xy = self.builder.walk_to_int(xy, !side, false).unwrap();
                self.builder
                    .xnode(name, name, xy)
                    .ref_int_side(int_xy, side, 0)
                    .extract_muxes()
                    .extract_intfs(true)
                    .iris(&[
                        ("IRI_QUAD", 0, 0),
                        ("IRI_QUAD", 0, 1),
                        ("IRI_QUAD", 0, 2),
                        ("IRI_QUAD", 0, 3),
                    ])
                    .extract();
                break;
            }
        }
    }

    fn fill_tiles_bli_cle_intf(&mut self) {
        let cle_bc = self.builder.ndb.get_node_naming("CLE_BC");
        for &(side, tkn, name, is_top) in BLI_CLE_INTF_KINDS {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let int_xy = self.builder.walk_to_int(xy, !side, false).unwrap();
                let cle_xy = self
                    .builder
                    .delta(xy, if side == Dir::E { 1 } else { -1 }, 0);
                for i in 0..4 {
                    let iriy = if is_top { 4 * i } else { 4 * (3 - i) };
                    let cur_int_xy = self.builder.delta(int_xy, 0, i as i32);
                    let cur_cle_xy = self.builder.delta(cle_xy, 0, i as i32);
                    self.builder
                        .xnode(format!("{name}.{i}"), format!("{name}.{i}"), xy)
                        .ref_int_side(cur_int_xy, side, 0)
                        .ref_xlat(
                            cur_cle_xy,
                            if side == Dir::E {
                                &[Some(0), None]
                            } else {
                                &[None, Some(0)]
                            },
                            cle_bc,
                        )
                        .extract_intfs(true)
                        .iris(&[
                            ("IRI_QUAD", 0, iriy),
                            ("IRI_QUAD", 0, iriy + 1),
                            ("IRI_QUAD", 0, iriy + 2),
                            ("IRI_QUAD", 0, iriy + 3),
                        ])
                        .extract();
                }
            }
        }
    }

    fn fill_tiles_rclk(&mut self) {
        for tkn in [
            "RCLK_INT_L_FT",
            "RCLK_INT_R_FT",
            "RCLK_INT_L_VR_FT",
            "RCLK_INT_R_VR_FT",
        ] {
            for &xy in self.builder.rd.tiles_by_kind_name(tkn) {
                let int_xy = self.builder.delta(xy, 0, 1);
                let mut int_xy_b = self.builder.delta(xy, 0, -1);
                if self
                    .builder
                    .rd
                    .tile_kinds
                    .key(self.builder.rd.tiles[&int_xy_b].kind)
                    != "INT"
                {
                    int_xy_b = self.builder.delta(int_xy_b, 0, -1);
                    if self
                        .builder
                        .rd
                        .tile_kinds
                        .key(self.builder.rd.tiles[&int_xy_b].kind)
                        != "INT"
                    {
                        continue;
                    }
                }
                self.builder
                    .xnode("RCLK", "RCLK", xy)
                    .num_tiles(2)
                    .ref_int_side(int_xy, Dir::W, 0)
                    .ref_int_side(int_xy, Dir::E, 1)
                    .extract_muxes()
                    .extract();
                break;
            }
        }
    }

    fn fill_tiles_rclk_cle(&mut self) {
        let cle_bc = self.builder.ndb.get_node_naming("CLE_BC");
        let rclk_int = self.builder.ndb.get_node_naming("RCLK");
        for (tkn, naming_f, naming_h, bkind, swz) in [
            (
                "RCLK_CLE_CORE",
                "RCLK_CLE",
                "RCLK_CLE.HALF",
                "BUFDIV_LEAF",
                BUFDIV_LEAF_SWZ_A,
            ),
            (
                "RCLK_CLE_VR_CORE",
                "RCLK_CLE.VR",
                "RCLK_CLE.HALF.VR",
                "BUFDIV_LEAF_ULVT",
                BUFDIV_LEAF_SWZ_B,
            ),
            (
                "RCLK_CLE_LAG_CORE",
                "RCLK_CLE.LAG",
                "RCLK_CLE.HALF.LAG",
                "BUFDIV_LEAF",
                BUFDIV_LEAF_SWZ_B,
            ),
        ] {
            let mut done_full = false;
            let mut done_half = false;
            for &xy in self.builder.rd.tiles_by_kind_name(tkn) {
                let td = &self.builder.rd.tiles[&self.builder.delta(xy, 0, -1)];
                let is_full = matches!(
                    &self.builder.rd.tile_kinds.key(td.kind)[..],
                    "CLE_W_CORE" | "CLE_W_VR_CORE"
                );
                if is_full {
                    if done_full {
                        continue;
                    }
                    done_full = true;
                } else {
                    if done_half {
                        continue;
                    }
                    done_half = true;
                }
                let mut bels = vec![];
                for (i, &y) in swz.iter().enumerate() {
                    let mut bel = self
                        .builder
                        .bel_xy(
                            if i < 16 {
                                bels::BUFDIV_LEAF_S[i]
                            } else {
                                bels::BUFDIV_LEAF_N[i - 16]
                            },
                            bkind,
                            0,
                            y as usize,
                        )
                        .pin_name_only("I", 1)
                        .pin_name_only("O_CASC", 1);
                    if i != 0 {
                        bel = bel.pin_name_only("I_CASC", 0);
                    }
                    if !is_full && i < 16 {
                        bel = bel.pin_name_only("O", 1);
                    }
                    bels.push(bel);
                }
                let mut bel = self.builder.bel_virtual(bels::RCLK_HDISTR_LOC);
                for i in 0..24 {
                    bel = bel.extra_wire(
                        format!("HDISTR_LOC{i}"),
                        &[format!("IF_HCLK_CLK_HDISTR_LOC{i}")],
                    );
                }
                bels.push(bel);
                bels.push(
                    self.builder
                        .bel_virtual(bels::VCC_RCLK)
                        .extra_wire("VCC", &["VCC_WIRE"]),
                );
                let kind = if is_full { "RCLK_CLE" } else { "RCLK_CLE.HALF" };
                let naming = if is_full { naming_f } else { naming_h };
                let int_e_xy = self.builder.delta(
                    self.builder
                        .walk_to_int(self.builder.delta(xy, 0, 1), Dir::E, false)
                        .unwrap(),
                    0,
                    -1,
                );
                let int_u_xy = self.builder.delta(xy, 1, 1);
                let int_d_xy = self.builder.delta(xy, 1, -1);
                let mut xn = self
                    .builder
                    .xnode(kind, naming, xy)
                    .num_tiles(if is_full { 2 } else { 1 })
                    .ref_xlat(int_e_xy, &[Some(0), None], rclk_int)
                    .ref_xlat(int_u_xy, &[None, Some(0)], cle_bc);
                if is_full {
                    xn = xn.ref_xlat(int_d_xy, &[None, Some(1)], cle_bc);
                }
                xn.bels(bels).extract();
            }
        }
    }

    fn fill_tiles_rclk_intf(&mut self) {
        let rclk_int = self.builder.ndb.get_node_naming("RCLK");

        for (side, naming, tkn, bkind, intf_dx, swz, has_dfx) in [
            (
                Dir::E,
                "DSP",
                "RCLK_DSP_CORE",
                "BUFDIV_LEAF",
                0,
                BUFDIV_LEAF_SWZ_A,
                false,
            ),
            (
                Dir::W,
                "DSP",
                "RCLK_DSP_CORE",
                "BUFDIV_LEAF",
                3,
                BUFDIV_LEAF_SWZ_AH,
                true,
            ),
            (
                Dir::E,
                "DSP.VR",
                "RCLK_DSP_VR_CORE",
                "BUFDIV_LEAF_ULVT",
                0,
                BUFDIV_LEAF_SWZ_B,
                false,
            ),
            (
                Dir::W,
                "DSP.VR",
                "RCLK_DSP_VR_CORE",
                "BUFDIV_LEAF_ULVT",
                3,
                BUFDIV_LEAF_SWZ_BH,
                true,
            ),
            (
                Dir::E,
                "HB",
                "RCLK_HB_CORE",
                "BUFDIV_LEAF",
                0,
                BUFDIV_LEAF_SWZ_A,
                false,
            ),
            (
                Dir::E,
                "HB.VR",
                "RCLK_HB_VR_CORE",
                "BUFDIV_LEAF_ULVT",
                0,
                BUFDIV_LEAF_SWZ_B,
                false,
            ),
            (
                Dir::W,
                "HB",
                "RCLK_HB_CORE",
                "BUFDIV_LEAF",
                2,
                BUFDIV_LEAF_SWZ_AH,
                false,
            ),
            (
                Dir::W,
                "HB.VR",
                "RCLK_HB_VR_CORE",
                "BUFDIV_LEAF_ULVT",
                2,
                BUFDIV_LEAF_SWZ_BH,
                false,
            ),
            (
                Dir::E,
                "SDFEC",
                "RCLK_SDFEC_CORE",
                "BUFDIV_LEAF_ULVT",
                0,
                BUFDIV_LEAF_SWZ_B,
                false,
            ),
            (
                Dir::W,
                "SDFEC",
                "RCLK_SDFEC_CORE",
                "BUFDIV_LEAF_ULVT",
                2,
                BUFDIV_LEAF_SWZ_BH,
                false,
            ),
            (
                Dir::E,
                "HDIO",
                "RCLK_HDIO_CORE",
                "BUFDIV_LEAF",
                0,
                BUFDIV_LEAF_SWZ_A,
                false,
            ),
            (
                Dir::E,
                "HDIO.VR",
                "RCLK_HDIO_VR_CORE",
                "BUFDIV_LEAF_ULVT",
                0,
                BUFDIV_LEAF_SWZ_B,
                false,
            ),
            (
                Dir::W,
                "HDIO",
                "RCLK_HDIO_CORE",
                "BUFDIV_LEAF",
                2,
                BUFDIV_LEAF_SWZ_AH,
                false,
            ),
            (
                Dir::W,
                "HDIO.VR",
                "RCLK_HDIO_VR_CORE",
                "BUFDIV_LEAF_ULVT",
                2,
                BUFDIV_LEAF_SWZ_BH,
                false,
            ),
            (
                Dir::E,
                "HB_HDIO",
                "RCLK_HB_HDIO_CORE",
                "BUFDIV_LEAF",
                0,
                BUFDIV_LEAF_SWZ_B,
                false,
            ),
            (
                Dir::E,
                "HB_HDIO.VR",
                "RCLK_HB_HDIO_VR_CORE",
                "BUFDIV_LEAF_ULVT",
                0,
                BUFDIV_LEAF_SWZ_B,
                false,
            ),
            (
                Dir::W,
                "HB_HDIO",
                "RCLK_HB_HDIO_CORE",
                "BUFDIV_LEAF",
                2,
                BUFDIV_LEAF_SWZ_BH,
                false,
            ),
            (
                Dir::W,
                "HB_HDIO.VR",
                "RCLK_HB_HDIO_VR_CORE",
                "BUFDIV_LEAF_ULVT",
                2,
                BUFDIV_LEAF_SWZ_BH,
                false,
            ),
            (
                Dir::W,
                "VNOC",
                "RCLK_INTF_L_CORE",
                "BUFDIV_LEAF",
                0,
                BUFDIV_LEAF_SWZ_B,
                false,
            ),
            (
                Dir::W,
                "VNOC.VR",
                "RCLK_INTF_L_VR_CORE",
                "BUFDIV_LEAF_ULVT",
                0,
                BUFDIV_LEAF_SWZ_B,
                false,
            ),
            (
                Dir::E,
                "VNOC",
                "RCLK_INTF_R_CORE",
                "BUFDIV_LEAF",
                0,
                BUFDIV_LEAF_SWZ_B,
                false,
            ),
            (
                Dir::E,
                "VNOC.VR",
                "RCLK_INTF_R_VR_CORE",
                "BUFDIV_LEAF_ULVT",
                0,
                BUFDIV_LEAF_SWZ_B,
                false,
            ),
            (
                Dir::W,
                "CFRM",
                "RCLK_INTF_OPT_CORE",
                "BUFDIV_LEAF",
                0,
                BUFDIV_LEAF_SWZ_A,
                false,
            ),
            (
                Dir::W,
                "CFRM.VR",
                "RCLK_INTF_OPT_VR_CORE",
                "BUFDIV_LEAF_ULVT",
                0,
                BUFDIV_LEAF_SWZ_B,
                false,
            ),
            (
                Dir::W,
                "GT",
                "RCLK_INTF_TERM_LEFT_CORE",
                "BUFDIV_LEAF",
                1,
                BUFDIV_LEAF_SWZ_A,
                false,
            ),
            (
                Dir::W,
                "GT.VR",
                "RCLK_INTF_TERM_LEFT_VR_CORE",
                "BUFDIV_LEAF_ULVT",
                1,
                BUFDIV_LEAF_SWZ_B,
                false,
            ),
            (
                Dir::E,
                "GT",
                "RCLK_INTF_TERM_RIGHT_CORE",
                "BUFDIV_LEAF",
                0,
                BUFDIV_LEAF_SWZ_A,
                false,
            ),
            (
                Dir::E,
                "GT.VR",
                "RCLK_INTF_TERM_RIGHT_VR_CORE",
                "BUFDIV_LEAF_ULVT",
                0,
                BUFDIV_LEAF_SWZ_B,
                false,
            ),
            (
                Dir::E,
                "GT.ALT",
                "RCLK_INTF_TERM2_RIGHT_CORE",
                "BUFDIV_LEAF",
                0,
                BUFDIV_LEAF_SWZ_B,
                false,
            ),
            (
                Dir::W,
                "BRAM",
                "RCLK_BRAM_CORE_MY",
                "BUFDIV_LEAF",
                1,
                BUFDIV_LEAF_SWZ_A,
                true,
            ),
            (
                Dir::W,
                "BRAM.VR",
                "RCLK_BRAM_VR_CORE_MY",
                "BUFDIV_LEAF_ULVT",
                1,
                BUFDIV_LEAF_SWZ_B,
                true,
            ),
            (
                Dir::W,
                "URAM",
                "RCLK_URAM_CORE_MY",
                "BUFDIV_LEAF",
                1,
                BUFDIV_LEAF_SWZ_A,
                true,
            ),
            (
                Dir::W,
                "URAM.VR",
                "RCLK_URAM_VR_CORE_MY",
                "BUFDIV_LEAF_ULVT",
                1,
                BUFDIV_LEAF_SWZ_B,
                true,
            ),
            (
                Dir::E,
                "BRAM",
                "RCLK_BRAM_CORE",
                "BUFDIV_LEAF",
                0,
                BUFDIV_LEAF_SWZ_A,
                true,
            ),
            (
                Dir::E,
                "BRAM.VR",
                "RCLK_BRAM_VR_CORE",
                "BUFDIV_LEAF_ULVT",
                0,
                BUFDIV_LEAF_SWZ_B,
                true,
            ),
            (
                Dir::E,
                "BRAM.CLKBUF",
                "RCLK_BRAM_CLKBUF_CORE",
                "BUFDIV_LEAF",
                0,
                BUFDIV_LEAF_SWZ_A,
                true,
            ),
            (
                Dir::E,
                "BRAM.CLKBUF.VR",
                "RCLK_BRAM_CLKBUF_VR_CORE",
                "BUFDIV_LEAF_ULVT",
                0,
                BUFDIV_LEAF_SWZ_B,
                true,
            ),
            (
                Dir::E,
                "BRAM.CLKBUF.NOPD",
                "RCLK_BRAM_CLKBUF_NOPD_CORE",
                "BUFDIV_LEAF",
                0,
                BUFDIV_LEAF_SWZ_B,
                true,
            ),
            (
                Dir::E,
                "BRAM.CLKBUF.NOPD.VR",
                "RCLK_BRAM_CLKBUF_NOPD_VR_CORE",
                "BUFDIV_LEAF_ULVT",
                0,
                BUFDIV_LEAF_SWZ_B,
                true,
            ),
            (
                Dir::W,
                "HB_FULL",
                "RCLK_HB_FULL_R_CORE",
                "BUFDIV_LEAF",
                0,
                BUFDIV_LEAF_SWZ_B,
                false,
            ),
            (
                Dir::E,
                "HB_FULL",
                "RCLK_HB_FULL_L_CORE",
                "BUFDIV_LEAF",
                0,
                BUFDIV_LEAF_SWZ_B,
                false,
            ),
        ] {
            let mut done_full = false;
            let mut done_half = false;
            for &xy in self.builder.rd.tiles_by_kind_name(tkn) {
                let int_xy = self.builder.delta(
                    self.builder
                        .walk_to_int(self.builder.delta(xy, 0, 1), !side, false)
                        .unwrap(),
                    0,
                    -1,
                );
                if int_xy.x.abs_diff(xy.x) > 5 {
                    continue;
                }
                if self
                    .builder
                    .rd
                    .tile_kinds
                    .key(self.builder.rd.tiles[&self.builder.delta(int_xy, 0, 1)].kind)
                    != "INT"
                {
                    continue;
                }
                let td = &self.builder.rd.tiles[&self.builder.delta(int_xy, 0, -1)];
                let is_full = self.builder.rd.tile_kinds.key(td.kind) == "INT";
                if is_full {
                    if done_full {
                        continue;
                    }
                    done_full = true;
                } else {
                    if done_half {
                        continue;
                    }
                    done_half = true;
                }
                let mut bels = vec![];
                for (i, &y) in swz.iter().enumerate() {
                    let mut bel = self
                        .builder
                        .bel_xy(
                            if i < 16 {
                                bels::BUFDIV_LEAF_S[i]
                            } else {
                                bels::BUFDIV_LEAF_N[i - 16]
                            },
                            bkind,
                            0,
                            y as usize,
                        )
                        .pin_name_only("I", 1)
                        .pin_name_only("O_CASC", 1);
                    if i != 0 {
                        bel = bel.pin_name_only("I_CASC", 0);
                    }
                    if !is_full && i < 16 {
                        bel = bel.pin_name_only("O", 1);
                    }
                    bels.push(bel);
                }
                let mut bel = self.builder.bel_virtual(bels::RCLK_HDISTR_LOC);
                for i in 0..24 {
                    bel = bel.extra_wire(
                        format!("HDISTR_LOC{i}"),
                        &[
                            format!("IF_HCLK_CLK_HDISTR_LOC{i}"),
                            format!("IF_HCLK_L_CLK_HDISTR_LOC{i}"),
                        ],
                    );
                }
                bels.push(bel);
                bels.push(
                    self.builder
                        .bel_virtual(bels::VCC_RCLK)
                        .extra_wire("VCC", &["VCC_WIRE"]),
                );
                let intf = self.builder.ndb.get_node_naming(if side == Dir::E {
                    "INTF.E"
                } else {
                    "INTF.W"
                });
                let half = if is_full { "" } else { ".HALF" };
                let intf_u_xy = self.builder.delta(xy, intf_dx, 1);
                let intf_d_xy = self.builder.delta(xy, intf_dx, -1);
                let mut xn = self
                    .builder
                    .xnode(
                        format!("RCLK_INTF.{side}{half}"),
                        format!("RCLK_INTF.{side}{half}.{naming}"),
                        xy,
                    )
                    .num_tiles(if is_full { 2 } else { 1 })
                    .ref_xlat(
                        int_xy,
                        &if side == Dir::W {
                            [Some(0), None]
                        } else {
                            [None, Some(0)]
                        },
                        rclk_int,
                    )
                    .ref_single(intf_u_xy, 0, intf);
                if is_full {
                    xn = xn.ref_single(intf_d_xy, 1, intf);
                }
                xn.bels(bels).extract();
                if has_dfx {
                    let bel = self.builder.bel_xy(bels::RCLK_DFX_TEST, "RCLK", 0, 0);
                    self.builder
                        .xnode(
                            format!("RCLK_DFX.{side}"),
                            format!("RCLK_DFX.{side}.{naming}"),
                            xy,
                        )
                        .ref_xlat(
                            int_xy,
                            &if side == Dir::W {
                                [Some(0), None]
                            } else {
                                [None, Some(0)]
                            },
                            rclk_int,
                        )
                        .bel(bel)
                        .extract();
                }
            }
        }
    }

    fn fill_tiles_rclk_clkbuf(&mut self) {
        for (tkn, kind) in [
            ("RCLK_BRAM_CLKBUF_CORE", "RCLK_CLKBUF"),
            ("RCLK_BRAM_CLKBUF_VR_CORE", "RCLK_CLKBUF.VR"),
            ("RCLK_BRAM_CLKBUF_NOPD_CORE", "RCLK_CLKBUF.NOPD"),
            ("RCLK_BRAM_CLKBUF_NOPD_VR_CORE", "RCLK_CLKBUF.NOPD.VR"),
        ] {
            for &xy in self.builder.rd.tiles_by_kind_name(tkn) {
                let mut bels = vec![];
                for i in 0..24 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::GCLK_PD_CLKBUF[i], "GCLK_PD", 0, i)
                            .pin_name_only("CLK_IN0", 1)
                            .pin_name_only("CLK_IN1", 1)
                            .pin_name_only("PD_OUT", 1),
                    );
                }
                let mut bel = self.builder.bel_virtual(bels::RCLK_CLKBUF);
                for i in 0..24 {
                    if kind == "RCLK_CLKBUF.NOPD.VR" && matches!(i, 8..12 | 20..24) {
                        continue;
                    }
                    bel = bel
                        .extra_wire(
                            format!("HDISTR{i}_W"),
                            &[&format!("IF_HCLK_L_CLK_HDISTR{i}")],
                        )
                        .extra_wire(
                            format!("HDISTR{i}_E"),
                            &[&format!("IF_HCLK_R_CLK_HDISTR{i}")],
                        );
                }
                for i in 0..12 {
                    bel = bel
                        .extra_wire(
                            format!("HROUTE{i}_W"),
                            &[&format!("IF_HCLK_L_CLK_HROUTE{i}")],
                        )
                        .extra_wire(
                            format!("HROUTE{i}_E"),
                            &[&format!("IF_HCLK_R_CLK_HROUTE{i}")],
                        );
                }
                bels.push(bel);
                self.builder.xnode(kind, kind, xy).bels(bels).extract();
            }
        }
    }

    fn fill_tiles_cle(&mut self) {
        for (tkn, kind, side) in [
            ("CLE_W_CORE", "CLE_E", Dir::E),
            ("CLE_E_CORE", "CLE_W", Dir::W),
            ("CLE_W_VR_CORE", "CLE_E.VR", Dir::E),
            ("CLE_E_VR_CORE", "CLE_W.VR", Dir::W),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let int_xy = self.builder.walk_to_int(xy, !side, false).unwrap();
                let bel_slicel = self
                    .builder
                    .bel_xy(bels::SLICE0, "SLICE", 0, 0)
                    .pin_name_only("CIN", 1)
                    .pin_name_only("COUT", 1);
                let bel_slicem = self
                    .builder
                    .bel_xy(bels::SLICE1, "SLICE", 1, 0)
                    .pin_name_only("SRL_IN_B", 1)
                    .pin_name_only("SRL_OUT_B", 1)
                    .pin_name_only("CIN", 1)
                    .pin_name_only("COUT", 1);
                let cle_bc_xy = self
                    .builder
                    .delta(xy, if side == Dir::E { 1 } else { -1 }, 0);
                let cle_bc_kind = self.builder.rd.tiles[&cle_bc_xy].kind;
                let cle_bc_naming = match &self.builder.rd.tile_kinds.key(cle_bc_kind)[..] {
                    "CLE_BC_CORE" | "CLE_BC_CORE_MX" => "CLE_BC",
                    "SLL" => "CLE_BC.SLL",
                    "SLL2" => "CLE_BC.SLL2",
                    _ => unreachable!(),
                };
                let cle_bc_naming = self.builder.ndb.get_node_naming(cle_bc_naming);
                let mut xn = self
                    .builder
                    .xnode(kind, kind, xy)
                    .num_tiles(if side == Dir::W { 2 } else { 1 })
                    .bel(bel_slicel)
                    .bel(bel_slicem)
                    .ref_int_side(int_xy, side, 0);
                if side == Dir::E {
                    xn = xn.ref_xlat(cle_bc_xy, &[Some(0), None], cle_bc_naming);
                } else {
                    xn = xn.ref_xlat(cle_bc_xy, &[Some(1), None], cle_bc_naming);
                }
                xn.extract();
            }
        }
    }

    fn fill_tiles_bram(&mut self) {
        for (side, tkn) in [
            (Dir::E, "BRAM_LOCF_BR_TILE"),
            (Dir::E, "BRAM_LOCF_TR_TILE"),
            (Dir::W, "BRAM_LOCF_BL_TILE"),
            (Dir::W, "BRAM_LOCF_TL_TILE"),
            (Dir::E, "BRAM_ROCF_BR_TILE"),
            (Dir::E, "BRAM_ROCF_TR_TILE"),
            (Dir::W, "BRAM_ROCF_BL_TILE"),
            (Dir::W, "BRAM_ROCF_TL_TILE"),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let (kind, intf) = match side {
                    Dir::E => ("BRAM_E", "INTF.E"),
                    Dir::W => ("BRAM_W", "INTF.W"),
                    _ => unreachable!(),
                };
                let intf = self.builder.ndb.get_node_naming(intf);
                let intf_xy = if side == Dir::E {
                    self.builder.delta(xy, -1, 0)
                } else {
                    self.builder.delta(xy, 1, 0)
                };
                let mut bel_f = self
                    .builder
                    .bel_xy(bels::BRAM_F, "RAMB36", 0, 0)
                    .pin_name_only("CASINSBITERR", 1)
                    .pin_name_only("CASINDBITERR", 1)
                    .pin_name_only("CASOUTSBITERR", 1)
                    .pin_name_only("CASOUTDBITERR", 1);
                for ab in ['A', 'B'] {
                    for i in 0..32 {
                        bel_f = bel_f
                            .pin_name_only(&format!("CASDIN{ab}_{i}_"), 1)
                            .pin_name_only(&format!("CASDOUT{ab}_{i}_"), 1);
                    }
                    for i in 0..4 {
                        bel_f = bel_f
                            .pin_name_only(&format!("CASDINP{ab}_{i}_"), 1)
                            .pin_name_only(&format!("CASDOUTP{ab}_{i}_"), 1);
                    }
                }
                let mut bel_h0 = self.builder.bel_xy(bels::BRAM_H0, "RAMB18", 0, 0);
                let mut bel_h1 = self.builder.bel_xy(bels::BRAM_H1, "RAMB18", 0, 1);
                for ab in ['A', 'B'] {
                    for i in 0..16 {
                        bel_h0 = bel_h0
                            .pin_name_only(&format!("CASDIN{ab}_{i}_"), 0)
                            .pin_name_only(&format!("CASDOUT{ab}_{i}_"), 0);
                        bel_h1 = bel_h1
                            .pin_name_only(&format!("CASDIN{ab}_{i}_"), 0)
                            .pin_name_only(&format!("CASDOUT{ab}_{i}_"), 0);
                    }
                    for i in 0..2 {
                        bel_h0 = bel_h0
                            .pin_name_only(&format!("CASDINP{ab}_{i}_"), 0)
                            .pin_name_only(&format!("CASDOUTP{ab}_{i}_"), 0);
                        bel_h1 = bel_h1
                            .pin_name_only(&format!("CASDINP{ab}_{i}_"), 0)
                            .pin_name_only(&format!("CASDOUTP{ab}_{i}_"), 0);
                    }
                }
                let bels = [bel_f, bel_h0, bel_h1];
                let mut xn = self.builder.xnode(kind, kind, xy).num_tiles(4);
                for i in 0..4 {
                    let cur_intf_xy = xn.builder.delta(intf_xy, 0, i as i32);
                    xn = xn.ref_single(cur_intf_xy, i, intf)
                }
                xn.bels(bels).extract();
            }
        }
    }

    fn fill_tiles_uram(&mut self) {
        for (tkn, kind) in [
            ("URAM_LOCF_BL_TILE", "URAM"),
            ("URAM_LOCF_TL_TILE", "URAM"),
            ("URAM_ROCF_BL_TILE", "URAM"),
            ("URAM_ROCF_TL_TILE", "URAM"),
            ("URAM_DELAY_LOCF_TL_TILE", "URAM_DELAY"),
            ("URAM_DELAY_ROCF_TL_TILE", "URAM_DELAY"),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let intf = self.builder.ndb.get_node_naming("INTF.W");
                let intf_xy = self.builder.delta(xy, 1, 0);
                let mut bels = vec![self.builder.bel_xy(bels::URAM, "URAM288", 0, 0)];
                if kind == "URAM_DELAY" {
                    bels.push(
                        self.builder
                            .bel_xy(bels::URAM_CAS_DLY, "URAM_CAS_DLY", 0, 0),
                    );
                }
                let bels: Vec<_> = bels
                    .into_iter()
                    .map(|mut bel| {
                        for ab in ['A', 'B'] {
                            bel = bel
                                .pin_name_only(&format!("CAS_IN_EN_{ab}"), 1)
                                .pin_name_only(&format!("CAS_OUT_EN_{ab}"), 1)
                                .pin_name_only(&format!("CAS_IN_SBITERR_{ab}"), 1)
                                .pin_name_only(&format!("CAS_OUT_SBITERR_{ab}"), 1)
                                .pin_name_only(&format!("CAS_IN_DBITERR_{ab}"), 1)
                                .pin_name_only(&format!("CAS_OUT_DBITERR_{ab}"), 1)
                                .pin_name_only(&format!("CAS_IN_RDACCESS_{ab}"), 1)
                                .pin_name_only(&format!("CAS_OUT_RDACCESS_{ab}"), 1)
                                .pin_name_only(&format!("CAS_IN_RDB_WR_{ab}"), 1)
                                .pin_name_only(&format!("CAS_OUT_RDB_WR_{ab}"), 1);
                            for i in 0..72 {
                                bel = bel
                                    .pin_name_only(&format!("CAS_IN_DIN_{ab}_{i}_"), 1)
                                    .pin_name_only(&format!("CAS_IN_DOUT_{ab}_{i}_"), 1)
                                    .pin_name_only(&format!("CAS_OUT_DIN_{ab}_{i}_"), 1)
                                    .pin_name_only(&format!("CAS_OUT_DOUT_{ab}_{i}_"), 1);
                            }
                            for i in 0..26 {
                                bel = bel
                                    .pin_name_only(&format!("CAS_IN_ADDR_{ab}_{i}_"), 1)
                                    .pin_name_only(&format!("CAS_OUT_ADDR_{ab}_{i}_"), 1);
                            }
                            for i in 0..9 {
                                bel = bel
                                    .pin_name_only(&format!("CAS_IN_BWE_{ab}_{i}_"), 1)
                                    .pin_name_only(&format!("CAS_OUT_BWE_{ab}_{i}_"), 1);
                            }
                        }
                        bel
                    })
                    .collect();
                let mut xn = self.builder.xnode(kind, kind, xy).num_tiles(4);
                for i in 0..4 {
                    let cur_intf_xy = xn.builder.delta(intf_xy, 0, i as i32);
                    xn = xn.ref_single(cur_intf_xy, i, intf)
                }
                xn.bels(bels).extract();
            }
        }
    }

    fn fill_tiles_dsp(&mut self) {
        for tkn in [
            "DSP_LOCF_B_TILE",
            "DSP_LOCF_T_TILE",
            "DSP_ROCF_B_TILE",
            "DSP_ROCF_T_TILE",
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let mut bels = vec![];
                for i in 0..2 {
                    let mut bel = self
                        .builder
                        .bel_xy(bels::DSP[i], "DSP", i, 0)
                        .pin_name_only("MULTSIGNIN", 1)
                        .pin_name_only("MULTSIGNOUT", 1)
                        .pin_name_only("CARRYCASCIN", 1)
                        .pin_name_only("CARRYCASCOUT", 1)
                        .pin_name_only("CONJ_CPLX_OUT", 1)
                        .pin_name_only("CONJ_CPLX_MULT_IN", 0)
                        .pin_name_only("CONJ_CPLX_PREADD_IN", 0);
                    for i in 0..34 {
                        bel = bel
                            .pin_name_only(&format!("ACIN_{i}_"), 1)
                            .pin_name_only(&format!("ACOUT_{i}_"), 1);
                    }
                    for i in 0..32 {
                        bel = bel
                            .pin_name_only(&format!("BCIN_{i}_"), 1)
                            .pin_name_only(&format!("BCOUT_{i}_"), 1);
                    }
                    for i in 0..58 {
                        bel = bel
                            .pin_name_only(&format!("PCIN_{i}_"), 1)
                            .pin_name_only(&format!("PCOUT_{i}_"), 1);
                    }
                    for i in 0..10 {
                        bel = bel
                            .pin_name_only(&format!("AD_CPLX_{i}_"), 0)
                            .pin_name_only(&format!("AD_DATA_CPLX_{i}_"), 1);
                    }
                    for i in 0..18 {
                        bel = bel
                            .pin_name_only(&format!("A_TO_D_CPLX_{i}_"), 1)
                            .pin_name_only(&format!("D_FROM_A_CPLX_{i}_"), 1)
                            .pin_name_only(&format!("A_CPLX_{i}_"), 1)
                            .pin_name_only(&format!("B2B1_CPLX_{i}_"), 1);
                    }
                    for i in 0..37 {
                        bel = bel
                            .pin_name_only(&format!("U_CPLX_{i}_"), 0)
                            .pin_name_only(&format!("V_CPLX_{i}_"), 0);
                    }
                    bels.push(bel);
                }
                let mut bel = self
                    .builder
                    .bel_xy(bels::DSP_CPLX, "DSP58_CPLX", 0, 0)
                    .pin_name_only("CONJ_DSP_L_IN", 0)
                    .pin_name_only("CONJ_DSP_R_IN", 0)
                    .pin_name_only("CONJ_DSP_L_MULT_OUT", 1)
                    .pin_name_only("CONJ_DSP_R_MULT_OUT", 1)
                    .pin_name_only("CONJ_DSP_L_PREADD_OUT", 1)
                    .pin_name_only("CONJ_DSP_R_PREADD_OUT", 1);
                for i in 0..10 {
                    bel = bel
                        .pin_name_only(&format!("AD_CPLX_DSPL_{i}_"), 1)
                        .pin_name_only(&format!("AD_CPLX_DSPR_{i}_"), 1)
                        .pin_name_only(&format!("AD_DATA_CPLX_DSPL_{i}_"), 0)
                        .pin_name_only(&format!("AD_DATA_CPLX_DSPR_{i}_"), 0);
                }
                for i in 0..18 {
                    bel = bel
                        .pin_name_only(&format!("A_CPLX_L_{i}_"), 0)
                        .pin_name_only(&format!("B2B1_CPLX_L_{i}_"), 0)
                        .pin_name_only(&format!("B2B1_CPLX_R_{i}_"), 0);
                }
                for i in 0..37 {
                    bel = bel
                        .pin_name_only(&format!("U_CPLX_{i}_"), 1)
                        .pin_name_only(&format!("V_CPLX_{i}_"), 1);
                }
                bels.push(bel);
                let intf_e = self.builder.ndb.get_node_naming("INTF.E");
                let intf_w = self.builder.ndb.get_node_naming("INTF.W");
                let naming = if self.dev_naming.is_dsp_v2 {
                    "DSP.V2"
                } else {
                    "DSP.V1"
                };
                let intf0_xy = self.builder.delta(xy, -1, 0);
                let intf1_xy = self.builder.delta(xy, -1, 1);
                let intf2_xy = self.builder.delta(xy, 2, 0);
                let intf3_xy = self.builder.delta(xy, 2, 1);
                self.builder
                    .xnode("DSP", naming, xy)
                    .num_tiles(4)
                    .ref_single(intf0_xy, 0, intf_e)
                    .ref_single(intf1_xy, 1, intf_e)
                    .ref_single(intf2_xy, 2, intf_w)
                    .ref_single(intf3_xy, 3, intf_w)
                    .bels(bels)
                    .extract();
            }
        }
    }

    fn fill_tiles_hard(&mut self) {
        for (slot, kind, tkn, sk, is_large) in [
            (bels::PCIE4, "PCIE4", "PCIEB_BOT_TILE", "PCIE40", false),
            (bels::PCIE4, "PCIE4", "PCIEB_TOP_TILE", "PCIE40", false),
            (bels::PCIE5, "PCIE5", "PCIEB5_BOT_TILE", "PCIE50", false),
            (bels::PCIE5, "PCIE5", "PCIEB5_TOP_TILE", "PCIE50", false),
            (bels::MRMAC, "MRMAC", "MRMAC_BOT_TILE", "MRMAC", false),
            (bels::MRMAC, "MRMAC", "MRMAC_TOP_TILE", "MRMAC", false),
            (
                bels::DFE_CFC_BOT,
                "DFE_CFC_BOT",
                "DFE_CFC_BOT_TILE",
                "DFE_CFC_BOT",
                false,
            ),
            (
                bels::DFE_CFC_TOP,
                "DFE_CFC_TOP",
                "DFE_CFC_TOP_TILE",
                "DFE_CFC_TOP",
                false,
            ),
            (bels::SDFEC, "SDFEC", "SDFECA_TOP_TILE", "SDFEC_A", false),
            (bels::DCMAC, "DCMAC", "DCMAC_TILE", "DCMAC", true),
            (bels::ILKN, "ILKN", "ILKN_TILE", "ILKNF", true),
            (bels::HSC, "HSC", "HSC_TILE", "HSC", true),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let bel = self.builder.bel_xy(slot, sk, 0, 0);
                let intf_e = self.builder.ndb.get_node_naming("INTF.E.HB");
                let intf_w = self.builder.ndb.get_node_naming("INTF.W.HB");
                let height = if is_large { 96 } else { 48 };
                let mut xn = self.builder.xnode(kind, kind, xy).num_tiles(height * 2);
                for i in 0..height {
                    let intf_w_xy = xn.builder.delta(xy, -1, (i + i / 4) as i32);
                    let intf_e_xy = xn.builder.delta(xy, 1, (i + i / 4) as i32);
                    xn = xn.ref_single(intf_w_xy, i, intf_e).ref_single(
                        intf_e_xy,
                        i + height,
                        intf_w,
                    )
                }
                xn.bel(bel).extract();
            }
        }
    }

    fn fill_tiles_hdio(&mut self) {
        for tkn in ["HDIO_TILE", "HDIO_BOT_TILE"] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let mut bels = vec![];
                for i in 0..11 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::HDIOLOGIC[i], "HDIOLOGIC", 0, i)
                            .pin_name_only("TFFM_Q", 1)
                            .pin_name_only("TFFS_Q", 1)
                            .pin_name_only("OPFFM_Q", 1)
                            .pin_name_only("OPFFS_Q", 1)
                            .pin_name_only("IPFFM_D", 0)
                            .pin_name_only("IPFFS_D", 0),
                    );
                }
                for i in 0..11 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::HDIOB[i], "IOB", 0, i)
                            .pin_name_only("RXOUT_M", 1)
                            .pin_name_only("RXOUT_S", 1)
                            .pin_name_only("OP_M", 0)
                            .pin_name_only("OP_S", 0)
                            .pin_name_only("TRISTATE_M", 0)
                            .pin_name_only("TRISTATE_S", 0),
                    );
                }
                for i in 0..4 {
                    let mut bel = self
                        .builder
                        .bel_xy(bels::BUFGCE_HDIO[i], "BUFGCE_HDIO", 0, i)
                        .pin_name_only("O", 1)
                        .pin_name_only("I", 1);
                    for j in 0..8 {
                        bel = bel.extra_wire(
                            format!("I_DUMMY{j}"),
                            &[format!("VCC_WIRE{k}", k = i * 8 + j)],
                        );
                    }
                    bels.push(bel);
                }
                bels.push(
                    self.builder
                        .bel_xy(bels::DPLL_HDIO, "DPLL", 0, 0)
                        .pin_name_only("CLKIN", 1)
                        .extra_int_in("CLKIN_INT", &["IF_COE_W24_CTRL14"])
                        .extra_wire("CLKIN_RCLK", &["IF_RCLK_CLK_TO_DPLL"])
                        .pin_name_only("CLKIN_DESKEW", 1)
                        .extra_wire("CLKIN_DESKEW_DUMMY0", &["VCC_WIRE32"])
                        .extra_wire("CLKIN_DESKEW_DUMMY1", &["VCC_WIRE33"])
                        .pin_name_only("CLKOUT0", 1)
                        .pin_name_only("CLKOUT1", 1)
                        .pin_name_only("CLKOUT2", 1)
                        .pin_name_only("CLKOUT3", 1)
                        .pin_name_only("TMUXOUT", 1),
                );
                bels.push(self.builder.bel_xy(bels::HDIO_BIAS, "HDIO_BIAS", 0, 0));
                bels.push(self.builder.bel_xy(bels::RPI_HD_APB, "RPI_HD_APB", 0, 0));
                bels.push(self.builder.bel_xy(bels::HDLOGIC_APB, "HDLOGIC_APB", 0, 0));
                bels.push(
                    self.builder
                        .bel_virtual(bels::VCC_HDIO)
                        .extra_wire("VCC", &["VCC_WIRE"]),
                );
                let intf_e = self.builder.ndb.get_node_naming("INTF.E.HB");
                let intf_w = self.builder.ndb.get_node_naming("INTF.W.HB");
                let mut xn = self.builder.xnode("HDIO", "HDIO", xy).num_tiles(96);
                for i in 0..48 {
                    let intf_w_xy = xn.builder.delta(xy, -1, (i + i / 4) as i32);
                    let intf_e_xy = xn.builder.delta(xy, 1, (i + i / 4) as i32);
                    xn = xn
                        .ref_single(intf_w_xy, i, intf_e)
                        .ref_single(intf_e_xy, i + 48, intf_w)
                }
                xn.bels(bels).extract();
            }
        }
    }

    fn fill_tiles_rclk_hdio(&mut self) {
        for (tkn, naming) in [
            ("RCLK_HDIO_CORE", "RCLK_HDIO"),
            ("RCLK_HDIO_VR_CORE", "RCLK_HDIO.VR"),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let bel_dpll = self
                    .builder
                    .bel_virtual(bels::RCLK_HDIO_DPLL)
                    .extra_wire("OUT_S", &["IF_RCLK_BOT_CLK_TO_DPLL"])
                    .extra_wire("OUT_N", &["IF_RCLK_TOP_CLK_TO_DPLL"]);
                let mut bel_hdio = self.builder.bel_virtual(bels::RCLK_HDIO);
                for i in 0..4 {
                    bel_hdio = bel_hdio
                        .extra_wire(
                            format!("BUFGCE_OUT_S{i}"),
                            &[format!("IF_RCLK_BOT_CLK_FROM_BUFG{i}")],
                        )
                        .extra_wire(
                            format!("BUFGCE_OUT_N{i}"),
                            &[format!("IF_RCLK_TOP_CLK_FROM_BUFG{i}")],
                        );
                }
                let swz = [
                    0, 3, 4, 5, 6, 7, 8, 9, 10, 11, 1, 2, 12, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                    13, 14,
                ];
                for (i, si) in swz.into_iter().enumerate() {
                    bel_hdio = bel_hdio
                        .extra_wire(
                            format!("HDISTR{i}"),
                            &[
                                format!("IF_HCLK_CLK_HDISTR{i}"),
                                match i {
                                    0..8 => format!("CLK_HDISTR_LSB{i}"),
                                    8..12 | 20..24 => {
                                        format!("CLK_CMT_DRVR_TRI_ULVT_{si}_CLK_OUT_B")
                                    }
                                    12..20 => format!("CLK_HDISTR_MSB{ii}", ii = i - 12),
                                    _ => unreachable!(),
                                },
                            ],
                        )
                        .extra_wire(
                            format!("HDISTR{i}_MUX"),
                            &[
                                format!("CLK_CMT_MUX_8TO1_{si}_CLK_OUT"),
                                format!("CLK_CMT_MUX_8TO1_ULVT_{si}_CLK_OUT"),
                            ],
                        );
                }
                for i in 0..12 {
                    bel_hdio = bel_hdio
                        .extra_wire(format!("HROUTE{i}"), &[format!("IF_HCLK_CLK_HROUTE{i}")])
                        .extra_wire(
                            format!("HROUTE{i}_MUX"),
                            &[
                                format!("CLK_CMT_MUX_8TO1_{si}_CLK_OUT", si = 24 + i),
                                format!("CLK_CMT_MUX_8TO1_ULVT_{si}_CLK_OUT", si = 24 + i),
                            ],
                        );
                }
                self.builder
                    .xnode("RCLK_HDIO", naming, xy)
                    .num_tiles(0)
                    .bel(bel_hdio)
                    .bel(bel_dpll)
                    .extract();
            }
        }

        for (tkn, naming) in [
            ("RCLK_HB_HDIO_CORE", "RCLK_HB_HDIO"),
            ("RCLK_HB_HDIO_VR_CORE", "RCLK_HB_HDIO.VR"),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let bel_dpll = self
                    .builder
                    .bel_virtual(bels::RCLK_HDIO_DPLL)
                    .extra_wire("OUT_S", &["IF_RCLK_BOT_CLK_TO_DPLL"])
                    .extra_wire(
                        "OUT_N",
                        &[
                            "CLK_CMT_MUX_24_ENC_1_CLK_OUT",
                            "CLK_CMT_MUX_24_ENC_ULVT_1_CLK_OUT",
                        ],
                    );
                let mut bel_hdio = self.builder.bel_virtual(bels::RCLK_HB_HDIO);
                for i in 0..4 {
                    bel_hdio = bel_hdio.extra_wire(
                        format!("BUFGCE_OUT_S{i}"),
                        &[format!("IF_RCLK_BOT_CLK_FROM_BUFG{i}")],
                    );
                }
                let swz = [
                    0, 3, 4, 5, 6, 7, 8, 9, 10, 11, 1, 2, 12, 15, 16, 17, 18, 19, 20, 21, 22, 23,
                    13, 14,
                ];
                for (i, si) in swz.into_iter().enumerate() {
                    bel_hdio = bel_hdio
                        .extra_wire(
                            format!("HDISTR{i}"),
                            &[
                                format!("IF_HCLK_CLK_HDISTR{i}"),
                                match i {
                                    0..8 => format!("CLK_HDISTR_LSB{i}"),
                                    8..12 | 20..24 => {
                                        format!("CLK_CMT_DRVR_TRI_ULVT_{si}_CLK_OUT_B")
                                    }
                                    12..20 => format!("CLK_HDISTR_MSB{ii}", ii = i - 12),
                                    _ => unreachable!(),
                                },
                            ],
                        )
                        .extra_wire(
                            format!("HDISTR{i}_MUX"),
                            &[
                                format!("CLK_CMT_MUX_8TO1_{si}_CLK_OUT"),
                                format!("CLK_CMT_MUX_8TO1_ULVT_{si}_CLK_OUT"),
                            ],
                        );
                    let b = [
                        0, 92, 120, 124, 128, 132, 136, 140, 8, 12, 4, 48, 16, 28, 32, 36, 40, 44,
                        52, 56, 60, 64, 20, 24,
                    ][i];
                    for j in 0..4 {
                        bel_hdio = bel_hdio.extra_wire(
                            format!("HDISTR{i}_MUX_DUMMY{j}"),
                            &[format!("GND_WIRE{k}", k = b + j)],
                        );
                    }
                }
                for i in 0..12 {
                    bel_hdio = bel_hdio
                        .extra_wire(format!("HROUTE{i}"), &[format!("IF_HCLK_CLK_HROUTE{i}")])
                        .extra_wire(
                            format!("HROUTE{i}_MUX"),
                            &[
                                format!("CLK_CMT_MUX_8TO1_{si}_CLK_OUT", si = 24 + i),
                                format!("CLK_CMT_MUX_8TO1_ULVT_{si}_CLK_OUT", si = 24 + i),
                            ],
                        );
                    let b = [68, 72, 76, 80, 84, 88, 96, 100, 104, 108, 112, 116][i];
                    for j in 0..4 {
                        bel_hdio = bel_hdio.extra_wire(
                            format!("HROUTE{i}_MUX_DUMMY{j}"),
                            &[format!("GND_WIRE{k}", k = b + j)],
                        );
                    }
                }
                self.builder
                    .xnode("RCLK_HB_HDIO", naming, xy)
                    .num_tiles(0)
                    .bel(bel_hdio)
                    .bel(bel_dpll)
                    .extract();
            }
        }
    }

    pub fn fill_tiles_vnoc(&mut self) {
        if let Some(&xy) = self
            .builder
            .rd
            .tiles_by_kind_name("AMS_SAT_VNOC_TILE")
            .iter()
            .next()
        {
            let bel = self
                .builder
                .bel_xy(bels::SYSMON_SAT_VNOC, "SYSMON_SAT", 0, 0);
            let intf_e = self.builder.ndb.get_node_naming("INTF.E");
            let mut xn = self
                .builder
                .xnode("SYSMON_SAT.VNOC", "SYSMON_SAT.VNOC", xy)
                .num_tiles(96);
            for i in 0..48 {
                let intf_xy = xn.builder.delta(xy, -1, -49 + (i + i / 4) as i32);
                xn = xn.ref_single(intf_xy, i, intf_e)
            }
            xn.bel(bel).extract();
        }

        if let Some(&xy) = self
            .builder
            .rd
            .tiles_by_kind_name("MISR_TILE")
            .iter()
            .next()
        {
            let bel = self.builder.bel_xy(bels::MISR, "MISR", 0, 0);
            let intf_w = self.builder.ndb.get_node_naming("INTF.W");
            let mut xn = self.builder.xnode("MISR", "MISR", xy).num_tiles(96);
            for i in 0..48 {
                let intf_xy = xn.builder.delta(xy, 1, 1 + (i + i / 4) as i32);
                xn = xn.ref_single(intf_xy, i + 48, intf_w)
            }
            xn.bel(bel).extract();
        }

        if let Some(&nsu_xy) = self
            .builder
            .rd
            .tiles_by_kind_name("NOC_NSU512_TOP")
            .iter()
            .next()
        {
            let nps_a_xy = self.builder.delta(nsu_xy, 0, 9);
            let nps_b_xy = self.builder.delta(nsu_xy, 0, 19);
            let nmu_xy = self.builder.delta(nsu_xy, 0, 29);
            let intf_w = self.builder.ndb.get_node_naming("INTF.E");
            let intf_e = self.builder.ndb.get_node_naming("INTF.W");
            let bels = [
                self.builder
                    .bel_xy(bels::VNOC_NSU512, "NOC_NSU512", 0, 0)
                    .pin_name_only("TO_NOC", 1)
                    .pin_name_only("FROM_NOC", 1),
                self.builder
                    .bel_xy(bels::VNOC_NPS_A, "NOC_NPS_VNOC", 0, 0)
                    .raw_tile(1)
                    .pin_name_only("IN_0", 1)
                    .pin_name_only("IN_1", 1)
                    .pin_name_only("IN_2", 1)
                    .pin_name_only("IN_3", 1)
                    .pin_name_only("OUT_0", 1)
                    .pin_name_only("OUT_1", 1)
                    .pin_name_only("OUT_2", 1)
                    .pin_name_only("OUT_3", 1),
                self.builder
                    .bel_xy(bels::VNOC_NPS_B, "NOC_NPS_VNOC", 0, 0)
                    .raw_tile(2)
                    .pin_name_only("IN_0", 1)
                    .pin_name_only("IN_1", 1)
                    .pin_name_only("IN_2", 1)
                    .pin_name_only("IN_3", 1)
                    .pin_name_only("OUT_0", 1)
                    .pin_name_only("OUT_1", 1)
                    .pin_name_only("OUT_2", 1)
                    .pin_name_only("OUT_3", 1),
                self.builder
                    .bel_xy(bels::VNOC_NMU512, "NOC_NMU512", 0, 0)
                    .raw_tile(3)
                    .pin_name_only("TO_NOC", 1)
                    .pin_name_only("FROM_NOC", 1),
            ];
            let mut xn = self
                .builder
                .xnode("VNOC", "VNOC", nsu_xy)
                .num_tiles(96)
                .raw_tile(nps_a_xy)
                .raw_tile(nps_b_xy)
                .raw_tile(nmu_xy);
            for i in 0..48 {
                let intf_w_xy = xn.builder.delta(nsu_xy, -1, -9 + (i + i / 4) as i32);
                let intf_e_xy = xn.builder.delta(nsu_xy, 2, -9 + (i + i / 4) as i32);
                xn = xn
                    .ref_single(intf_w_xy, i, intf_w)
                    .ref_single(intf_e_xy, i + 48, intf_e)
            }
            xn.bels(bels).extract();
        }

        if let Some(&nsu_xy) = self
            .builder
            .rd
            .tiles_by_kind_name("NOC2_NSU512_VNOC_TILE")
            .iter()
            .next()
        {
            let mut nps_a_xy = self.builder.delta(nsu_xy, 0, 4);
            if self
                .builder
                .rd
                .tile_kinds
                .key(self.builder.rd.tiles[&nps_a_xy].kind)
                == "NULL"
            {
                nps_a_xy = self.builder.delta(nps_a_xy, -1, 0);
            }
            let nps_b_xy = self.builder.delta(nps_a_xy, 0, 4);
            let nmu_xy = self.builder.delta(nps_a_xy, 0, 7);
            let scan_xy = self.builder.delta(nsu_xy, 1, 0);
            let intf_e = self.builder.ndb.get_node_naming("INTF.E");
            let intf_w = self.builder.ndb.get_node_naming("INTF.W");
            let mut bel_scan = self
                .builder
                .bel_xy(bels::VNOC2_SCAN, "NOC2_SCAN", 0, 0)
                .raw_tile(4);
            for i in 6..15 {
                bel_scan = bel_scan
                    .pin_name_only(&format!("NOC2_SCAN_CHNL_FROM_PL_{i}_"), 1)
                    .pin_name_only(&format!("NOC2_SCAN_CHNL_TO_PL_{i}_"), 1);
            }
            for i in 5..14 {
                bel_scan = bel_scan.pin_name_only(&format!("NOC2_SCAN_CHNL_MASK_FROM_PL_{i}_"), 1);
            }
            let bels = [
                self.builder
                    .bel_xy(bels::VNOC2_NSU512, "NOC2_NSU512", 0, 0)
                    .pin_name_only("TO_NOC", 1)
                    .pin_name_only("FROM_NOC", 1),
                self.builder
                    .bel_xy(bels::VNOC2_NPS_A, "NOC2_NPS5555", 0, 0)
                    .raw_tile(1)
                    .pin_name_only("IN_0", 1)
                    .pin_name_only("IN_1", 1)
                    .pin_name_only("IN_2", 1)
                    .pin_name_only("IN_3", 1)
                    .pin_name_only("OUT_0", 1)
                    .pin_name_only("OUT_1", 1)
                    .pin_name_only("OUT_2", 1)
                    .pin_name_only("OUT_3", 1),
                self.builder
                    .bel_xy(bels::VNOC2_NPS_B, "NOC2_NPS5555", 0, 0)
                    .raw_tile(2)
                    .pin_name_only("IN_0", 1)
                    .pin_name_only("IN_1", 1)
                    .pin_name_only("IN_2", 1)
                    .pin_name_only("IN_3", 1)
                    .pin_name_only("OUT_0", 1)
                    .pin_name_only("OUT_1", 1)
                    .pin_name_only("OUT_2", 1)
                    .pin_name_only("OUT_3", 1),
                self.builder
                    .bel_xy(bels::VNOC2_NMU512, "NOC2_NMU512", 0, 0)
                    .raw_tile(3)
                    .pin_name_only("TO_NOC", 1)
                    .pin_name_only("FROM_NOC", 1),
                bel_scan,
            ];
            let mut xn = self
                .builder
                .xnode("VNOC2", "VNOC2", nsu_xy)
                .num_tiles(96)
                .raw_tile(nps_a_xy)
                .raw_tile(nps_b_xy)
                .raw_tile(nmu_xy)
                .raw_tile(scan_xy);
            for i in 0..48 {
                let intf_l_xy = xn.builder.delta(nps_a_xy, -1, -13 + (i + i / 4) as i32);
                let intf_r_xy = xn.builder.delta(nps_a_xy, 3, -13 + (i + i / 4) as i32);
                xn = xn
                    .ref_single(intf_l_xy, i, intf_e)
                    .ref_single(intf_r_xy, i + 48, intf_w)
            }
            xn.bels(bels).extract();
        }

        if let Some(&nsu_xy) = self
            .builder
            .rd
            .tiles_by_kind_name("NOC2_NSU512_VNOC4_TILE")
            .iter()
            .next()
        {
            let nps_a_xy = self.builder.delta(nsu_xy, -1, 4);
            let nps_b_xy = self.builder.delta(nps_a_xy, 0, 4);
            let nmu_xy = self.builder.delta(nps_a_xy, 0, 7);
            let scan_xy = self.builder.delta(nsu_xy, 1, 0);
            let intf_w = self.builder.ndb.get_node_naming("INTF.E");
            let intf_e = self.builder.ndb.get_node_naming("INTF.W");
            let mut bel_scan = self
                .builder
                .bel_xy(bels::VNOC4_SCAN, "NOC2_SCAN", 0, 0)
                .raw_tile(4);
            for i in 7..15 {
                bel_scan = bel_scan
                    .pin_name_only(&format!("NOC2_SCAN_CHNL_FROM_PL_{i}_"), 1)
                    .pin_name_only(&format!("NOC2_SCAN_CHNL_TO_PL_{i}_"), 1);
            }
            for i in 7..14 {
                bel_scan = bel_scan.pin_name_only(&format!("NOC2_SCAN_CHNL_MASK_FROM_PL_{i}_"), 1);
            }
            let bels = [
                self.builder
                    .bel_xy(bels::VNOC4_NSU512, "NOC2_NSU512", 0, 0)
                    .pin_name_only("TO_NOC", 1)
                    .pin_name_only("FROM_NOC", 1),
                self.builder
                    .bel_xy(bels::VNOC4_NPS_A, "NOC2_NPS6X", 0, 0)
                    .raw_tile(1)
                    .pin_name_only("IN_0", 1)
                    .pin_name_only("IN_1", 1)
                    .pin_name_only("IN_2", 1)
                    .pin_name_only("IN_3", 1)
                    .pin_name_only("IN_4", 1)
                    .pin_name_only("IN_5", 1)
                    .pin_name_only("OUT_0", 1)
                    .pin_name_only("OUT_1", 1)
                    .pin_name_only("OUT_2", 1)
                    .pin_name_only("OUT_3", 1)
                    .pin_name_only("OUT_4", 1)
                    .pin_name_only("OUT_5", 1),
                self.builder
                    .bel_xy(bels::VNOC4_NPS_B, "NOC2_NPS6X", 0, 0)
                    .raw_tile(2)
                    .pin_name_only("IN_0", 1)
                    .pin_name_only("IN_1", 1)
                    .pin_name_only("IN_2", 1)
                    .pin_name_only("IN_3", 1)
                    .pin_name_only("IN_4", 1)
                    .pin_name_only("IN_5", 1)
                    .pin_name_only("OUT_0", 1)
                    .pin_name_only("OUT_1", 1)
                    .pin_name_only("OUT_2", 1)
                    .pin_name_only("OUT_3", 1)
                    .pin_name_only("OUT_4", 1)
                    .pin_name_only("OUT_5", 1),
                self.builder
                    .bel_xy(bels::VNOC4_NMU512, "NOC2_NMU512", 0, 0)
                    .raw_tile(3)
                    .pin_name_only("TO_NOC", 1)
                    .pin_name_only("FROM_NOC", 1),
                bel_scan,
            ];
            let mut xn = self
                .builder
                .xnode("VNOC4", "VNOC4", nsu_xy)
                .num_tiles(96)
                .raw_tile(nps_a_xy)
                .raw_tile(nps_b_xy)
                .raw_tile(nmu_xy)
                .raw_tile(scan_xy);
            for i in 0..48 {
                let intf_w_xy = xn.builder.delta(nps_a_xy, -1, -13 + (i + i / 4) as i32);
                let intf_e_xy = xn.builder.delta(nps_a_xy, 5, -13 + (i + i / 4) as i32);

                xn = xn
                    .ref_single(intf_w_xy, i, intf_w)
                    .ref_single(intf_e_xy, i + 48, intf_e)
            }
            xn.bels(bels).extract();
        }
    }

    pub fn fill_tiles_gt_misc(&mut self) {
        for (kind, dpll_kind, tkn, intf_kind, int_dir) in [
            (
                "SYSMON_SAT.LGT",
                "DPLL.LGT",
                "AMS_SAT_GT_BOT_TILE_MY",
                "INTF.W.TERM.GT",
                Dir::E,
            ),
            (
                "SYSMON_SAT.LGT",
                "DPLL.LGT",
                "AMS_SAT_GT_TOP_TILE_MY",
                "INTF.W.TERM.GT",
                Dir::E,
            ),
            (
                "SYSMON_SAT.RGT",
                "DPLL.RGT",
                "AMS_SAT_GT_BOT_TILE",
                "INTF.E.TERM.GT",
                Dir::W,
            ),
            (
                "SYSMON_SAT.RGT",
                "DPLL.RGT",
                "AMS_SAT_GT_TOP_TILE",
                "INTF.E.TERM.GT",
                Dir::W,
            ),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let bel = self.builder.bel_xy(bels::SYSMON_SAT_GT, "SYSMON_SAT", 0, 0);
                let intf = self.builder.ndb.get_node_naming(intf_kind);
                let base_xy = self.builder.delta(xy, 0, -24);
                let int_xy = self.builder.walk_to_int(base_xy, int_dir, true).unwrap();
                let intf_xy = self
                    .builder
                    .delta(int_xy, if int_dir == Dir::E { -1 } else { 1 }, 0);
                let mut xn = self.builder.xnode(kind, kind, xy).num_tiles(48);
                for i in 0..48 {
                    let intf_xy = xn.builder.delta(intf_xy, 0, (i + i / 4) as i32);
                    xn = xn.ref_single(intf_xy, i, intf)
                }
                xn.bel(bel).extract();
                let bel = self
                    .builder
                    .bel_xy(bels::DPLL_GT, "DPLL", 0, 0)
                    .pin_name_only("CLKIN", 1)
                    .pin_name_only("CLKIN_DESKEW", 1)
                    .pin_name_only("CLKOUT0", 1)
                    .pin_name_only("CLKOUT1", 1)
                    .pin_name_only("CLKOUT2", 1)
                    .pin_name_only("CLKOUT3", 1)
                    .pin_name_only("TMUXOUT", 1);
                let dpll_xy = self.builder.delta(xy, 0, -15);
                let mut xn = self
                    .builder
                    .xnode(dpll_kind, dpll_kind, dpll_xy)
                    .num_tiles(48);
                for i in 0..48 {
                    let intf_xy = xn.builder.delta(intf_xy, 0, (i + i / 4) as i32);
                    xn = xn.ref_single(intf_xy, i, intf)
                }
                xn.bel(bel).extract();
            }
        }
    }

    pub fn fill_tiles_gt(&mut self) {
        if let Some(&xy) = self
            .builder
            .rd
            .tiles_by_kind_name("VDU_CORE_MY")
            .iter()
            .next()
        {
            let bel = self
                .builder
                .bel_xy(bels::VDU, "VDU", 0, 0)
                .pin_name_only("VDUCORECLK", 1)
                .pin_name_only("VDUMCUCLK", 1);
            let intf_e = self.builder.ndb.get_node_naming("INTF.E.TERM.GT");
            let int_xy = self.builder.walk_to_int(xy, Dir::W, false).unwrap();
            let mut xn = self.builder.xnode("VDU.E", "VDU.E", xy).num_tiles(48);
            for i in 0..48 {
                xn = xn.ref_single(int_xy.delta(1, (i + i / 4) as i32), i, intf_e)
            }
            xn.bel(bel).extract();
        }
        //
        for tkn in ["BFR_TILE_B_BOT_CORE", "BFR_TILE_B_TOP_CORE"] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let bel = self.builder.bel_xy(bels::BFR_B, "BFR_B", 0, 0);
                let intf_e = self.builder.ndb.get_node_naming("INTF.E.TERM.GT");
                let int_xy = self.builder.walk_to_int(xy, Dir::W, false).unwrap();
                let mut xn = self.builder.xnode("BFR_B.E", "BFR_B.E", xy).num_tiles(48);
                for i in 0..48 {
                    xn = xn.ref_single(int_xy.delta(1, (i + i / 4) as i32), i, intf_e)
                }
                xn.bel(bel).extract();
            }
        }
    }
}

pub fn make_int_db(rd: &Part, dev_naming: &DeviceNaming) -> (IntDb, NamingDb) {
    let mut maker = IntMaker {
        builder: IntBuilder::new(rd),
        long_term_slots: DirPartMap::new(),
        term_slot_intf: TermSlotId::from_idx(0),
        long_main_passes: DirPartMap::new(),
        sng_fixup_map: BTreeMap::new(),
        term_wires: DirMap::from_fn(|_| EntityPartVec::new()),
        term_wires_l: DirPartMap::new(),
        bnodes: vec![],
        bnode_outs: vec![],
        bounces: vec![],
        term_logic_outs: EntityPartVec::new(),
        dev_naming,
    };

    for &slot in bels::SLOTS {
        maker.builder.db.bel_slots.insert(slot.into());
    }

    for dir in [Dir::W, Dir::E] {
        maker.term_wires_l.insert(dir, EntityPartVec::new());
    }
    let crd = rd.tiles_by_kind_name("INT").first().unwrap();
    let tile = &rd.tiles[crd];
    if tile.name.contains("_S") {
        maker.builder.set_mirror_square();
    }
    maker.fill_term_slots();
    maker.fill_wires();
    maker.fill_tiles_int();
    maker.fill_tiles_cle_bc();
    maker.fill_tiles_intf();
    maker.fill_tiles_bli_cle_intf();
    maker.fill_tiles_rclk();
    maker.fill_tiles_rclk_cle();
    maker.fill_tiles_rclk_intf();
    maker.fill_tiles_rclk_clkbuf();
    maker.fill_tiles_cle();
    maker.fill_tiles_bram();
    maker.fill_tiles_uram();
    maker.fill_tiles_dsp();
    maker.fill_tiles_hard();
    maker.fill_tiles_hdio();
    maker.fill_tiles_rclk_hdio();
    maker.fill_tiles_vnoc();
    maker.fill_tiles_gt_misc();
    maker.fill_tiles_gt();

    maker.builder.build()
}
