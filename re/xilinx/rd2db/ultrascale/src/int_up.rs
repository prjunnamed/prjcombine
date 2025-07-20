use std::collections::{BTreeMap, BTreeSet};

use prjcombine_interconnect::{
    db::{
        BelInfo, CellSlotId, ConnectorClass, ConnectorSlot, ConnectorSlotId, ConnectorWire, IntDb,
        TileWireCoord, WireId, WireKind,
    },
    dir::{Dir, DirMap, DirPartMap},
};
use prjcombine_re_xilinx_rawdump::{Coord, Part, TkSiteSlot};

use prjcombine_re_xilinx_naming::db::{BelNaming, NamingDb};
use prjcombine_re_xilinx_naming_ultrascale::DeviceNaming;
use prjcombine_re_xilinx_rd2db_interconnect::{IntBuilder, XNodeInfo, XNodeRef};
use prjcombine_ultrascale::{bels, expanded::REGION_LEAF, tslots};
use unnamed_entity::{EntityId, EntityPartVec};

const XLAT24: [usize; 24] = [
    0, 11, 16, 17, 18, 19, 20, 21, 22, 23, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15,
];

const BUFCE_LEAF_SWIZZLE: [[usize; 16]; 2] = [
    [0, 1, 7, 6, 5, 4, 12, 13, 14, 15, 23, 22, 21, 20, 28, 29],
    [3, 2, 8, 9, 10, 11, 19, 18, 17, 16, 24, 25, 26, 27, 31, 30],
];

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
                CellSlotId::from_idx(match side {
                    Dir::W => 0,
                    Dir::E => 1,
                    _ => unreachable!(),
                }),
                CellSlotId::from_idx(slot),
            )]
            .into_iter()
            .collect(),
        });
        self
    }
}

struct IntMaker<'a> {
    builder: IntBuilder<'a>,
    long_term_slots: DirPartMap<ConnectorSlotId>,
    long_main_passes: DirPartMap<ConnectorClass>,
    // how many mental illnesses do you think I could be diagnosed with just from this repo?
    sng_fixup_map: BTreeMap<TileWireCoord, TileWireCoord>,
    term_wires_w: EntityPartVec<WireId, ConnectorWire>,
    term_wires_e: EntityPartVec<WireId, ConnectorWire>,
    term_wires_lw: EntityPartVec<WireId, ConnectorWire>,
    term_wires_le: EntityPartVec<WireId, ConnectorWire>,
    dev_naming: &'a DeviceNaming,
}

impl IntMaker<'_> {
    fn fill_term_slots(&mut self) {
        let slot_lw = self
            .builder
            .db
            .conn_slots
            .insert(
                "LW".into(),
                ConnectorSlot {
                    opposite: ConnectorSlotId::from_idx(0),
                },
            )
            .0;
        let slot_le = self
            .builder
            .db
            .conn_slots
            .insert("LE".into(), ConnectorSlot { opposite: slot_lw })
            .0;
        self.builder.db.conn_slots[slot_lw].opposite = slot_le;

        self.long_term_slots.insert(Dir::W, slot_lw);
        self.long_term_slots.insert(Dir::E, slot_le);
    }

    fn fill_wires_long(&mut self) {
        let d2n = DirMap::from_fn(|dir| match dir {
            Dir::N => 0,
            Dir::S => 1,
            Dir::E => 2,
            Dir::W => 3,
        });

        for (dir, name, length, fts, ftn) in [
            (Dir::W, "LONG", 6, false, false),
            (Dir::E, "LONG", 6, true, true),
            (Dir::S, "LONG", 12, false, false),
            (Dir::N, "LONG", 12, false, false),
        ] {
            let ftd = d2n[!dir];
            for i in 0..8 {
                let mut w = self.builder.mux_out(
                    format!("{name}.{dir}.{i}.0"),
                    &[format!("{dir}{dir}12_BEG{i}")],
                );
                for j in 1..length {
                    let nn = (b'A' + (j - 1)) as char;
                    let wname = format!("{name}.{dir}.{i}.{j}");
                    let vwname = format!("{dir}{dir}12_{nn}_FT{ftd}_{i}");
                    if matches!(dir, Dir::W | Dir::E) {
                        let wn = self.builder.wire(
                            wname,
                            WireKind::Branch(self.long_term_slots[!dir]),
                            &[vwname],
                        );
                        self.long_main_passes
                            .get_mut(!dir)
                            .unwrap()
                            .wires
                            .insert(wn, ConnectorWire::Pass(w));
                        w = wn;
                    } else {
                        w = self.builder.branch(w, dir, wname, &[vwname]);
                    }
                }
                let wname = format!("{name}.{dir}.{i}.{length}");
                let vwname = format!("{dir}{dir}12_END{i}");
                if matches!(dir, Dir::W | Dir::E) {
                    let wn = self.builder.wire(
                        wname,
                        WireKind::Branch(self.long_term_slots[!dir]),
                        &[vwname],
                    );
                    self.long_main_passes
                        .get_mut(!dir)
                        .unwrap()
                        .wires
                        .insert(wn, ConnectorWire::Pass(w));
                    w = wn;
                } else {
                    w = self.builder.branch(w, dir, wname, &[vwname]);
                }
                if i == 0 && fts {
                    self.builder.branch(
                        w,
                        Dir::S,
                        format!("{name}.{dir}.{i}.{length}.S"),
                        &[format!("{dir}{dir}12_BLS_{i}_FT0")],
                    );
                }
                if i == 7 && ftn {
                    self.builder.branch(
                        w,
                        Dir::N,
                        format!("{name}.{dir}.{i}.{length}.N"),
                        &[format!("{dir}{dir}12_BLN_{i}_FT1")],
                    );
                }
            }
        }
        for dir in [Dir::W, Dir::E] {
            let rdir = !dir;
            for i in 0..8 {
                for seg in 0..6 {
                    let nseg = seg + 1;
                    let wt = self.builder.db.get_wire(&format!("LONG.{rdir}.{i}.{nseg}"));
                    let wf = self.builder.db.get_wire(&format!("LONG.{dir}.{i}.{seg}"));
                    let wires = match dir {
                        Dir::W => &mut self.term_wires_lw,
                        Dir::E => &mut self.term_wires_le,
                        _ => unreachable!(),
                    };
                    wires.insert(wt, ConnectorWire::Reflect(wf));
                }
            }
        }
    }

    fn fill_wires_sdqnode(&mut self) {
        for i in 0..96 {
            match i {
                0 | 2 => {
                    let w = self.builder.mux_out_pair(
                        format!("SDQNODE.{i}"),
                        &[format!("SDQNODE_W_{i}_FT1"), format!("SDQNODE_E_{i}_FT1")],
                    );
                    self.builder.branch_pair(
                        w,
                        Dir::S,
                        format!("SDQNODE.{i}.S"),
                        &[
                            format!("SDQNODE_W_BLS_{i}_FT0"),
                            format!("SDQNODE_E_BLS_{i}_FT0"),
                        ],
                    );
                }
                91 | 93 | 95 => {
                    let w = self.builder.mux_out_pair(
                        format!("SDQNODE.{i}"),
                        &[format!("SDQNODE_W_{i}_FT0"), format!("SDQNODE_E_{i}_FT0")],
                    );
                    self.builder.branch_pair(
                        w,
                        Dir::N,
                        format!("SDQNODE.{i}.N"),
                        &[
                            format!("SDQNODE_W_BLN_{i}_FT1"),
                            format!("SDQNODE_E_BLN_{i}_FT1"),
                        ],
                    );
                }
                _ => {
                    // TODO not the true permutation
                    let a = [
                        0, 11, 22, 33, 44, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 17,
                        18, 19, 20, 21, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 34, 35, 36, 37, 38,
                        39, 40, 41, 42, 43, 45, 46, 47,
                    ][i >> 1];
                    let aa = a + 48;
                    let b = i & 1;
                    self.builder.mux_out_pair(
                        format!("SDQNODE.{i}"),
                        &[
                            format!("INT_NODE_SDQ_{aa}_INT_OUT{b}"),
                            format!("INT_NODE_SDQ_{a}_INT_OUT{b}"),
                        ],
                    );
                }
            }
        }
    }

    fn fill_wires_sdq(&mut self) {
        let d2n = DirMap::from_fn(|dir| match dir {
            Dir::N => 0,
            Dir::S => 1,
            Dir::E => 2,
            Dir::W => 3,
        });

        for (dir, name, length, fts, ftn) in [
            (Dir::E, "SNG", 1, false, false),
            (Dir::W, "SNG", 1, false, true),
            (Dir::N, "SNG", 1, false, false),
            (Dir::S, "SNG", 1, false, false),
            (Dir::E, "DBL", 2, false, false),
            (Dir::W, "DBL", 2, true, false),
            (Dir::N, "DBL", 2, false, false),
            (Dir::S, "DBL", 2, false, false),
            (Dir::E, "QUAD", 4, false, false),
            (Dir::W, "QUAD", 4, false, false),
            (Dir::N, "QUAD", 4, false, true),
            (Dir::S, "QUAD", 4, true, false),
        ] {
            let length: u8 = length;
            let ftd = d2n[!dir];
            for i in 0..8 {
                let name_w = if length == 1 && dir == Dir::E {
                    let (a, b) = [
                        (60, 1),
                        (4, 0),
                        (61, 1),
                        (5, 0),
                        (62, 1),
                        (6, 0),
                        (63, 1),
                        (7, 0),
                    ][i];
                    format!("INT_INT_SDQ_{a}_INT_OUT{b}")
                } else {
                    format!("{dir}{dir}{length}_W_BEG{i}")
                };
                let name_e = if length == 1 && dir == Dir::W {
                    if i == 7 {
                        format!("{dir}{dir}{length}_E_{i}_FT0")
                    } else {
                        let (a, b) = [
                            (72, 0),
                            (32, 1),
                            (73, 0),
                            (33, 1),
                            (74, 0),
                            (34, 1),
                            (75, 0),
                        ][i];
                        format!("INT_INT_SDQ_{a}_INT_OUT{b}")
                    }
                } else {
                    format!("{dir}{dir}{length}_E_BEG{i}")
                };
                let w0 = self
                    .builder
                    .mux_out_pair(format!("{name}.{dir}.{i}.0"), &[name_w, name_e]);
                let mut w = w0;
                for j in 1..length {
                    let nn = match dir {
                        Dir::W | Dir::E => {
                            if j.is_multiple_of(2) {
                                Some((b'A' + (j / 2 - 1)) as char)
                            } else {
                                None
                            }
                        }
                        Dir::S | Dir::N => Some((b'A' + (j - 1)) as char),
                    };
                    if let Some(nn) = nn {
                        w = self.builder.branch_pair(
                            w,
                            dir,
                            format!("{name}.{dir}.{i}.{j}"),
                            &[
                                format!("{dir}{dir}{length}_W_{nn}_FT{ftd}_{i}"),
                                format!("{dir}{dir}{length}_E_{nn}_FT{ftd}_{i}"),
                            ],
                        );
                    } else {
                        w = self
                            .builder
                            .branch(w, dir, format!("{name}.{dir}.{i}.{j}"), &[""]);
                    }
                }
                w = self.builder.branch_pair(
                    w,
                    dir,
                    format!("{name}.{dir}.{i}.{length}"),
                    &if length == 1 && matches!(dir, Dir::E | Dir::W) {
                        [
                            format!("{dir}{dir}{length}_E_END{i}"),
                            format!("{dir}{dir}{length}_W_END{i}"),
                        ]
                    } else {
                        [
                            format!("{dir}{dir}{length}_W_END{i}"),
                            format!("{dir}{dir}{length}_E_END{i}"),
                        ]
                    },
                );
                match (length, dir) {
                    (1, Dir::W) => {
                        self.sng_fixup_map.insert(
                            TileWireCoord {
                                cell: CellSlotId::from_idx(1),
                                wire: w0,
                            },
                            TileWireCoord {
                                cell: CellSlotId::from_idx(0),
                                wire: w,
                            },
                        );
                    }
                    (1, Dir::E) => {
                        self.sng_fixup_map.insert(
                            TileWireCoord {
                                cell: CellSlotId::from_idx(0),
                                wire: w0,
                            },
                            TileWireCoord {
                                cell: CellSlotId::from_idx(1),
                                wire: w,
                            },
                        );
                    }
                    _ => (),
                }
                if i == 0 && fts {
                    self.builder.branch_pair(
                        w,
                        Dir::S,
                        format!("{name}.{dir}.{i}.{length}.S"),
                        &[
                            format!("{dir}{dir}{length}_W_BLS_{i}_FT0"),
                            format!("{dir}{dir}{length}_E_BLS_{i}_FT0"),
                        ],
                    );
                }
                if i == 7 && ftn {
                    self.builder.branch_pair(
                        w,
                        Dir::N,
                        format!("{name}.{dir}.{i}.{length}.N"),
                        &if length == 1 {
                            [
                                format!("{dir}{dir}{length}_E_BLN_{i}_FT1"),
                                format!("{dir}{dir}{length}_W_BLN_{i}_FT1"),
                            ]
                        } else {
                            [
                                format!("{dir}{dir}{length}_W_BLN_{i}_FT1"),
                                format!("{dir}{dir}{length}_E_BLN_{i}_FT1"),
                            ]
                        },
                    );
                }
                if (length == 2 && dir == Dir::W && i == 0)
                    || (length == 1 && dir == Dir::W && i == 7)
                {
                    self.builder
                        .branch(w, Dir::W, format!("{name}.{dir}.{i}.{length}.W"), &[""]);
                    self.builder
                        .branch(w, Dir::E, format!("{name}.{dir}.{i}.{length}.E"), &[""]);
                }
            }
        }
        for dir in [Dir::W, Dir::E] {
            for (name, length) in [("SNG", 1), ("DBL", 2), ("QUAD", 4)] {
                let rdir = !dir;
                for i in 0..8 {
                    for seg in 0..length {
                        let nseg = seg + 1;
                        let wt = self
                            .builder
                            .db
                            .get_wire(&format!("{name}.{rdir}.{i}.{nseg}"));
                        let wf = self.builder.db.get_wire(&format!("{name}.{dir}.{i}.{seg}"));
                        let wires = match dir {
                            Dir::W => &mut self.term_wires_w,
                            Dir::E => &mut self.term_wires_e,
                            _ => unreachable!(),
                        };
                        wires.insert(wt, ConnectorWire::Reflect(wf));
                    }
                }
            }
        }
    }

    fn fill_wires_inode(&mut self) {
        for i in 0..64 {
            match i {
                1 | 3 | 5 | 9 => {
                    let w = self.builder.mux_out_pair(
                        format!("INODE.{i}"),
                        &[format!("INODE_W_{i}_FT1"), format!("INODE_E_{i}_FT1")],
                    );
                    self.builder.branch_pair(
                        w,
                        Dir::S,
                        format!("INODE.{i}.S"),
                        &[
                            format!("INODE_W_BLS_{i}_FT0"),
                            format!("INODE_E_BLS_{i}_FT0"),
                        ],
                    );
                }
                54 | 58 | 60 | 62 => {
                    let w = self.builder.mux_out_pair(
                        format!("INODE.{i}"),
                        &[format!("INODE_W_{i}_FT0"), format!("INODE_E_{i}_FT0")],
                    );
                    self.builder.branch_pair(
                        w,
                        Dir::N,
                        format!("INODE.{i}.N"),
                        &[
                            format!("INODE_W_BLN_{i}_FT1"),
                            format!("INODE_E_BLN_{i}_FT1"),
                        ],
                    );
                }
                _ => {
                    // TODO not the true permutation
                    let a = [
                        0, 11, 22, 30, 31, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13, 14, 15, 16, 17,
                        18, 19, 20, 21, 23, 24, 25, 26, 27, 28, 29,
                    ][i >> 1];
                    let aa = a + 32;
                    let b = i & 1;
                    let w = self.builder.mux_out(format!("INODE.{i}"), &[""]);
                    self.builder.extra_name_tile_sub(
                        "INT",
                        format!("INT_NODE_IMUX_{aa}_INT_OUT{b}"),
                        0,
                        w,
                    );
                    self.builder.extra_name_tile_sub(
                        "INT",
                        format!("INT_NODE_IMUX_{a}_INT_OUT{b}"),
                        1,
                        w,
                    );
                }
            }
        }
    }

    fn fill_wires_imux(&mut self) {
        for i in 0..10 {
            self.builder.mux_out_pair(
                format!("IMUX.CTRL.{i}"),
                &[format!("CTRL_W{i}"), format!("CTRL_E{i}")],
            );
        }

        for i in 0..16 {
            let w = match i {
                0 | 2 => {
                    let w = self.builder.mux_out_pair(
                        format!("IMUX.BYP.{i}"),
                        &[format!("BOUNCE_W_{i}_FT1"), format!("BOUNCE_E_{i}_FT1")],
                    );
                    self.builder.branch_pair(
                        w,
                        Dir::S,
                        format!("IMUX.BYP.{i}.S"),
                        &[
                            format!("BOUNCE_W_BLS_{i}_FT0"),
                            format!("BOUNCE_E_BLS_{i}_FT0"),
                        ],
                    );
                    w
                }
                13 | 15 => {
                    let w = self.builder.mux_out_pair(
                        format!("IMUX.BYP.{i}"),
                        &[format!("BOUNCE_W_{i}_FT0"), format!("BOUNCE_E_{i}_FT0")],
                    );
                    self.builder.branch_pair(
                        w,
                        Dir::N,
                        format!("IMUX.BYP.{i}.N"),
                        &[
                            format!("BOUNCE_W_BLN_{i}_FT1"),
                            format!("BOUNCE_E_BLN_{i}_FT1"),
                        ],
                    );
                    w
                }
                _ => self.builder.mux_out_pair(
                    format!("IMUX.BYP.{i}"),
                    &[format!("BYPASS_W{i}"), format!("BYPASS_E{i}")],
                ),
            };
            self.builder.delay(w, format!("IMUX.BYP.{i}.DELAY"), &[""]);
        }

        for i in 0..48 {
            let w = self.builder.mux_out_pair(
                format!("IMUX.IMUX.{i}"),
                &[format!("IMUX_W{i}"), format!("IMUX_E{i}")],
            );
            self.builder.delay(w, format!("IMUX.IMUX.{i}.DELAY"), &[""]);
        }
    }

    fn fill_wires_rclk(&mut self) {
        for i in 0..24 {
            self.builder.mux_out_pair(
                format!("RCLK.IMUX.{i}"),
                &[
                    if i < 16 {
                        format!(
                            "CLK_LEAF_SITES_{idx}_CE_INT",
                            idx = BUFCE_LEAF_SWIZZLE[i / 8][i % 8 + 8]
                        )
                    } else {
                        format!("INT_RCLK_TO_CLK_LEFT_{a}_{b}", a = i % 2, b = (i - 16) / 2)
                    },
                    if i < 16 {
                        format!(
                            "CLK_LEAF_SITES_{idx}_CE_INT",
                            idx = BUFCE_LEAF_SWIZZLE[i / 8][i % 8]
                        )
                    } else if i < 22 {
                        format!("INT_RCLK_TO_CLK_RIGHT_{a}_{b}", a = i % 2, b = (i - 16) / 2)
                    } else if i == 22 {
                        "CLK_LEAF_SITES_0_ENSEL_PROG".into()
                    } else if i == 23 {
                        "CLK_LEAF_SITES_0_CLK_CASC_IN".into()
                    } else {
                        unreachable!()
                    },
                ],
            );
        }
        for i in 0..24 {
            let w = self.builder.mux_out(format!("RCLK.INODE.{i}"), &[""]);
            for tkn in ["RCLK_INT_L", "RCLK_INT_R"] {
                self.builder.extra_name_tile_sub(
                    tkn,
                    format!("INT_NODE_IMUX_{a}_INT_OUT{b}", a = i / 2 + 12, b = i % 2),
                    0,
                    w,
                );
                self.builder.extra_name_tile_sub(
                    tkn,
                    format!("INT_NODE_IMUX_{a}_INT_OUT{b}", a = i / 2, b = i % 2),
                    1,
                    w,
                );
            }
        }
        const RCLK_GND_SWIZZLE: [(usize, usize); 24] = [
            // absolutely not the "correct" swizzle, whatever it would even mean.
            (1, 12),
            (2, 13),
            (3, 14),
            (4, 15),
            (5, 16),
            (17, 0),
            (18, 6),
            (19, 7),
            (20, 8),
            (21, 9),
            (22, 10),
            (23, 11),
            (27, 36),
            (28, 37),
            (29, 38),
            (30, 39),
            (40, 24),
            (41, 25),
            (42, 26),
            (43, 31),
            (44, 32),
            (45, 33),
            (46, 34),
            (47, 35),
        ];
        for (i, (iw, ie)) in RCLK_GND_SWIZZLE.into_iter().enumerate() {
            let w = self
                .builder
                .wire(format!("RCLK.GND.{i}"), WireKind::Tie0, &[""]);
            for tkn in ["RCLK_INT_L", "RCLK_INT_R"] {
                self.builder
                    .extra_name_tile_sub(tkn, format!("GND_WIRE{iw}"), 0, w);
                self.builder
                    .extra_name_tile_sub(tkn, format!("GND_WIRE{ie}"), 1, w);
            }
        }
    }

    fn fill_wires(&mut self) {
        let main_pass_lw = ConnectorClass {
            slot: self.long_term_slots[Dir::W],
            wires: Default::default(),
        };
        let main_pass_le = ConnectorClass {
            slot: self.long_term_slots[Dir::E],
            wires: Default::default(),
        };
        self.long_main_passes.insert(Dir::W, main_pass_lw);
        self.long_main_passes.insert(Dir::E, main_pass_le);

        // common wires

        self.builder.wire("VCC", WireKind::Tie1, &["VCC_WIRE"]);

        for i in 0..16 {
            let w = self.builder.wire(
                format!("GCLK{i}"),
                WireKind::Regional(REGION_LEAF),
                &[format!("GCLK_B_0_{i}")],
            );
            for tkn in ["RCLK_INT_L", "RCLK_INT_R"] {
                self.builder.extra_name_tile_sub(
                    tkn,
                    format!(
                        "CLK_LEAF_SITES_{idx}_CLK_LEAF",
                        idx = BUFCE_LEAF_SWIZZLE[0][i]
                    ),
                    2,
                    w,
                );
            }
        }

        for i in 0..16 {
            for j in 0..2 {
                self.builder.mux_out(
                    format!("GNODE.{i}.{j}"),
                    &[format!("INT_NODE_GLOBAL_{i}_INT_OUT{j}")],
                );
            }
        }

        self.fill_wires_long();

        // wires belonging to interconnect left/right half-nodes

        for i in 0..32 {
            let w = self
                .builder
                .logic_out(format!("OUT.{i}"), &[format!("LOGIC_OUTS_W{i}")]);
            self.builder
                .extra_name_sub(format!("LOGIC_OUTS_E{i}"), 1, w);
        }

        self.fill_wires_sdqnode();
        self.fill_wires_sdq();
        self.fill_wires_inode();
        self.fill_wires_imux();
        self.fill_wires_rclk();

        self.builder.extract_main_passes();
        self.builder.db.conn_classes.insert(
            "MAIN.LW".into(),
            self.long_main_passes.remove(Dir::W).unwrap(),
        );
        self.builder.db.conn_classes.insert(
            "MAIN.LE".into(),
            self.long_main_passes.remove(Dir::E).unwrap(),
        );
        self.builder.db.conn_classes.insert(
            "TERM.W".into(),
            ConnectorClass {
                slot: self.builder.term_slots[Dir::W],
                wires: std::mem::take(&mut self.term_wires_w),
            },
        );
        self.builder.db.conn_classes.insert(
            "TERM.E".into(),
            ConnectorClass {
                slot: self.builder.term_slots[Dir::E],
                wires: std::mem::take(&mut self.term_wires_e),
            },
        );
        self.builder.db.conn_classes.insert(
            "TERM.LW".into(),
            ConnectorClass {
                slot: self.long_term_slots[Dir::W],
                wires: std::mem::take(&mut self.term_wires_lw),
            },
        );
        self.builder.db.conn_classes.insert(
            "TERM.LE".into(),
            ConnectorClass {
                slot: self.long_term_slots[Dir::E],
                wires: std::mem::take(&mut self.term_wires_le),
            },
        );
    }

    fn fill_tiles_int(&mut self) {
        self.builder
            .node_type(tslots::INT, bels::INT, "INT", "INT", "INT");
        let nk = self.builder.db.get_tile_class("INT");
        let node = &mut self.builder.db.tile_classes[nk];
        node.cells.push(());
        let pips = self.builder.pips.get_mut(&(nk, bels::INT)).unwrap();
        pips.pips = pips
            .pips
            .iter()
            .map(|(&(wt, wf), &mode)| {
                let wtn = self.builder.db.wires.key(wt.wire);
                if wtn.starts_with("INODE") || wtn.starts_with("SDQNODE") {
                    let nwf = self.sng_fixup_map.get(&wf).copied().unwrap_or(wf);
                    ((wt, nwf), mode)
                } else {
                    ((wt, wf), mode)
                }
            })
            .collect();
        let naming = self.builder.ndb.get_tile_class_naming("INT");
        let naming = &mut self.builder.ndb.tile_class_namings[naming];
        for (&wf, &wt) in &self.sng_fixup_map {
            let name = naming.wires[&wf].clone();
            naming.wires.insert(wt, name);
        }
    }

    fn extract_sn_term(&mut self, dir: Dir, int_xy: Coord) {
        let pass_rev = &self.builder.db.conn_classes[self
            .builder
            .db
            .get_conn_class(&format!("MAIN.{rd}", rd = !dir))];
        let naming =
            &self.builder.ndb.tile_class_namings[self.builder.ndb.get_tile_class_naming("INT")];
        let mut node2target = BTreeMap::new();
        for &ti in pass_rev.wires.values() {
            let ConnectorWire::Pass(wf) = ti else {
                unreachable!()
            };
            for tile in [0, 1] {
                let tile = CellSlotId::from_idx(tile);
                let Some(name) = naming.wires.get(&TileWireCoord {
                    cell: tile,
                    wire: wf,
                }) else {
                    continue;
                };
                let node = self.builder.rd.lookup_wire_force(int_xy, name);
                let mut twf = (tile, wf);
                // sigh. no hope. no hope at all.
                if self.builder.db.wires.key(wf) == "DBL.W.0.2" {
                    twf = (
                        CellSlotId::from_idx(tile.to_idx() ^ 1),
                        self.builder
                            .db
                            .get_wire(&format!("DBL.W.0.2.{d}", d = ["E", "W"][tile.to_idx()])),
                    );
                }
                if self.builder.db.wires.key(wf) == "SNG.W.7.1" {
                    twf = (
                        CellSlotId::from_idx(tile.to_idx() ^ 1),
                        self.builder
                            .db
                            .get_wire(&format!("SNG.W.7.1.{d}", d = ["E", "W"][tile.to_idx()])),
                    );
                }
                assert!(node2target.insert(node, twf).is_none());
            }
        }
        for tile in [0, 1] {
            let pass = &self.builder.db.conn_classes
                [self.builder.db.get_conn_class(&format!("MAIN.{dir}"))];
            let naming =
                &self.builder.ndb.tile_class_namings[self.builder.ndb.get_tile_class_naming("INT")];
            let mut wires = EntityPartVec::new();
            for wt in pass.wires.ids() {
                let tile = CellSlotId::from_idx(tile);
                let Some(name) = naming.wires.get(&TileWireCoord {
                    cell: tile,
                    wire: wt,
                }) else {
                    continue;
                };
                let node = self.builder.rd.lookup_wire_force(int_xy, name);
                if let Some(&(tf, wf)) = node2target.get(&node) {
                    assert_eq!(tile, tf);
                    wires.insert(wt, ConnectorWire::Reflect(wf));
                }
            }
            let term = ConnectorClass {
                slot: self.builder.term_slots[dir],
                wires,
            };
            self.builder
                .insert_term_merge(&format!("TERM.{dir}{tile}"), term);
        }
    }

    fn fill_io_term_short(&mut self, xy_w: Coord, xy_e: Coord) {
        let mut e2w = EntityPartVec::new();
        let mut w2e = EntityPartVec::new();
        let pass_w = &self.builder.db.conn_classes[self.builder.db.get_conn_class("MAIN.W")];
        for (wt, &ti) in &pass_w.wires {
            let ConnectorWire::Pass(wf) = ti else {
                unreachable!()
            };
            w2e.insert(wf, wt);
            e2w.insert(wt, wf);
        }
        let pass_e = &self.builder.db.conn_classes[self.builder.db.get_conn_class("MAIN.E")];
        for (wt, &ti) in &pass_e.wires {
            let ConnectorWire::Pass(wf) = ti else {
                unreachable!()
            };
            e2w.insert(wf, wt);
            w2e.insert(wt, wf);
        }
        let switch_tile = [w2e, e2w];
        for (dir, xy_to, xy_from, tile_to, tile_from) in
            [(Dir::W, xy_e, xy_w, 0, 1), (Dir::E, xy_w, xy_e, 1, 0)]
        {
            let pass = &self.builder.db.conn_classes
                [self.builder.db.get_conn_class(&format!("MAIN.{dir}"))];
            let naming =
                &self.builder.ndb.tile_class_namings[self.builder.ndb.get_tile_class_naming("INT")];
            let mut node2target = BTreeMap::new();
            for &ti in pass.wires.values() {
                let ConnectorWire::Pass(wf) = ti else {
                    unreachable!()
                };
                let name = if let Some(name) = naming.wires.get(&TileWireCoord {
                    cell: CellSlotId::from_idx(tile_from),
                    wire: wf,
                }) {
                    name
                } else if let Some(&owf) = switch_tile[tile_from].get(wf) {
                    if let Some(name) = naming.wires.get(&TileWireCoord {
                        cell: CellSlotId::from_idx(tile_from ^ 1),
                        wire: owf,
                    }) {
                        name
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };
                let node = self.builder.rd.lookup_wire_force(xy_from, name);
                assert!(node2target.insert(node, wf).is_none());
            }
            let mut wires = EntityPartVec::new();
            for wt in pass.wires.ids() {
                let name = if let Some(name) = naming.wires.get(&TileWireCoord {
                    cell: CellSlotId::from_idx(tile_to),
                    wire: wt,
                }) {
                    name
                } else if let Some(&owt) = switch_tile[tile_to].get(wt) {
                    if let Some(name) = naming.wires.get(&TileWireCoord {
                        cell: CellSlotId::from_idx(tile_to ^ 1),
                        wire: owt,
                    }) {
                        name
                    } else {
                        continue;
                    }
                } else {
                    continue;
                };
                let node = self.builder.rd.lookup_wire_force(xy_to, name);
                if let Some(&wf) = node2target.get(&node) {
                    wires.insert(wt, ConnectorWire::Pass(wf));
                }
            }
            let term = ConnectorClass {
                slot: self.builder.term_slots[dir],
                wires,
            };
            self.builder.insert_term_merge(&format!("IO.{dir}"), term);
        }
    }

    fn fill_io_term_long(&mut self, xy_w: Coord, xy_e: Coord) {
        for (dir, xy_to, xy_from) in [(Dir::W, xy_e, xy_w), (Dir::E, xy_w, xy_e)] {
            let pass = &self.builder.db.conn_classes
                [self.builder.db.get_conn_class(&format!("MAIN.L{dir}"))];
            let naming =
                &self.builder.ndb.tile_class_namings[self.builder.ndb.get_tile_class_naming("INT")];
            let mut node2target = BTreeMap::new();
            for &ti in pass.wires.values() {
                let ConnectorWire::Pass(wf) = ti else {
                    unreachable!()
                };
                let tile = CellSlotId::from_idx(0);
                let Some(name) = naming.wires.get(&TileWireCoord {
                    cell: tile,
                    wire: wf,
                }) else {
                    continue;
                };
                let node = self.builder.rd.lookup_wire_force(xy_from, name);
                assert!(node2target.insert(node, wf).is_none());
            }
            let mut wires = EntityPartVec::new();
            for wt in pass.wires.ids() {
                let tile = CellSlotId::from_idx(0);
                let Some(name) = naming.wires.get(&TileWireCoord {
                    cell: tile,
                    wire: wt,
                }) else {
                    continue;
                };
                let node = self.builder.rd.lookup_wire_force(xy_to, name);
                if let Some(&wf) = node2target.get(&node) {
                    wires.insert(wt, ConnectorWire::Pass(wf));
                }
            }
            let term = ConnectorClass {
                slot: self.long_term_slots[dir],
                wires,
            };
            self.builder.insert_term_merge(&format!("IO.L{dir}"), term);
        }
    }

    fn fill_terms(&mut self) {
        for tkn in ["INT_TERM_B", "INT_TERM_P", "INT_INT_TERM_H_FT"] {
            for &xy in self.builder.rd.tiles_by_kind_name(tkn) {
                let int_xy = self.builder.walk_to_int(xy, Dir::N, true).unwrap();
                self.extract_sn_term(Dir::S, int_xy);
            }
        }
        for tkn in ["INT_TERM_T"] {
            for &xy in self.builder.rd.tiles_by_kind_name(tkn) {
                let int_xy = self.builder.walk_to_int(xy, Dir::S, true).unwrap();
                self.extract_sn_term(Dir::N, int_xy);
            }
        }

        for &xy in self.builder.rd.tiles_by_kind_name("INT_IBRK_FSR2IO") {
            let Some(xy_w) = self.builder.walk_to_int(xy, Dir::W, true) else {
                continue;
            };
            let Some(xy_e) = self.builder.walk_to_int(xy, Dir::E, true) else {
                continue;
            };
            self.fill_io_term_short(xy_w, xy_e);
            self.fill_io_term_long(xy_w, xy_e);
        }
    }

    fn fill_tiles_rclk_int(&mut self) {
        for tkn in ["RCLK_INT_L", "RCLK_INT_R"] {
            for &xy in self.builder.rd.tiles_by_kind_name(tkn) {
                let mut bels = vec![];
                for (ud, slots) in [('D', bels::BUFCE_LEAF_S), ('U', bels::BUFCE_LEAF_N)] {
                    for i in 0..16 {
                        let mut bel = self
                            .builder
                            .bel_xy(
                                slots[i],
                                "BUFCE_LEAF",
                                i & 7,
                                i / 8 + 2 * usize::from(ud == 'U'),
                            )
                            .pins_name_only(&["CLK_CASC_OUT", "CLK_IN"]);
                        if i != 0 || ud == 'U' {
                            bel = bel.pins_name_only(&["CLK_CASC_IN"]);
                        }
                        bels.push(bel);
                    }
                }
                let mut bel = self
                    .builder
                    .bel_virtual(bels::RCLK_INT_CLK)
                    .extra_wire("VCC", &["VCC_WIRE"]);
                for i in 0..24 {
                    bel = bel.extra_wire(format!("HDISTR{i}"), &[format!("CLK_HDISTR_FT0_{i}")]);
                }
                bels.push(bel);
                self.builder
                    .xnode(tslots::RCLK_INT, "RCLK_INT", "RCLK_INT", xy)
                    .num_tiles(4)
                    .ref_int_side(xy.delta(0, 1), Dir::W, 0)
                    .ref_int_side(xy.delta(0, 1), Dir::E, 1)
                    .extract_muxes(bels::RCLK_INT)
                    .bels(bels)
                    .extract();
            }
        }
    }

    fn fill_tiles_intf(&mut self) {
        for (kind, naming, dir, tkn, sb_delay) in [
            ("INTF", "INTF.W", Dir::W, "INT_INTF_L", None),
            ("INTF", "INTF.E", Dir::E, "INT_INTF_R", None),
            (
                "INTF.IO",
                "INTF.PSS",
                Dir::W,
                "INT_INTF_LEFT_TERM_PSS",
                Some(bels::INTF_DELAY),
            ),
            (
                "INTF.IO",
                "INTF.W.IO",
                Dir::W,
                "INT_INTF_LEFT_TERM_IO_FT",
                Some(bels::INTF_DELAY),
            ),
            (
                "INTF.IO",
                "INTF.W.IO",
                Dir::W,
                "INT_INTF_L_CMT",
                Some(bels::INTF_DELAY),
            ),
            (
                "INTF.IO",
                "INTF.W.IO",
                Dir::W,
                "INT_INTF_L_IO",
                Some(bels::INTF_DELAY),
            ),
            (
                "INTF.IO",
                "INTF.E.IO",
                Dir::E,
                "INT_INTF_RIGHT_TERM_IO",
                Some(bels::INTF_DELAY),
            ),
            (
                "INTF.IO",
                "INTF.E.IO",
                Dir::E,
                "INT_INTF_RIGHT_TERM_XP5IO_FT",
                Some(bels::INTF_DELAY),
            ),
            (
                "INTF.DELAY",
                "INTF.W.PCIE",
                Dir::W,
                "INT_INTF_L_PCIE4",
                Some(bels::INTF_DELAY),
            ),
            (
                "INTF.DELAY",
                "INTF.E.PCIE",
                Dir::E,
                "INT_INTF_R_PCIE4",
                Some(bels::INTF_DELAY),
            ),
            (
                "INTF.DELAY",
                "INTF.W.GT",
                Dir::W,
                "INT_INTF_L_TERM_GT",
                Some(bels::INTF_DELAY),
            ),
            (
                "INTF.DELAY",
                "INTF.E.GT",
                Dir::E,
                "INT_INTF_R_TERM_GT",
                Some(bels::INTF_DELAY),
            ),
            (
                "INTF.DELAY",
                "INTF.E.GT",
                Dir::E,
                "INT_INTF_20_2_RIGHT_TERM_GT_FT",
                Some(bels::INTF_DELAY),
            ),
            (
                "INTF.DELAY",
                "INTF.E.GT",
                Dir::E,
                "INT_INTF_RIGHT_TERM_HDIO_FT",
                Some(bels::INTF_DELAY),
            ),
        ] {
            for &xy in self.builder.rd.tiles_by_kind_name(tkn.as_ref()) {
                let int_xy = self.builder.walk_to_int(xy, !dir, false).unwrap();
                let mut xn = self
                    .builder
                    .xnode(tslots::INTF, kind, naming, xy)
                    .extract_intfs(true)
                    .ref_int_side(int_xy, dir, 0);
                if let Some(sb) = sb_delay {
                    xn = xn.extract_delay(sb);
                }
                xn.extract();
            }
        }
    }

    fn fill_tiles_clb(&mut self) {
        for (tkn, kind, side) in [
            ("CLEL_L", "CLEL", Dir::W),
            ("CLEL_R", "CLEL", Dir::E),
            ("CLEM", "CLEM", Dir::W),
            ("CLEM_R", "CLEM", Dir::W),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let int_xy = xy.delta(if side == Dir::W { 1 } else { -1 }, 0);
                let bel = self
                    .builder
                    .bel_xy(bels::SLICE, "SLICE", 0, 0)
                    .pin_name_only("CIN", 1)
                    .pin_name_only("COUT", 0);
                self.builder
                    .xnode(tslots::BEL, kind, kind, xy)
                    .ref_int_side(int_xy, side, 0)
                    .bel(bel)
                    .extract();
            }
        }
    }

    fn fill_tiles_lag(&mut self) {
        if let Some(&xy) = self.builder.rd.tiles_by_kind_name("LAG_LAG").iter().next() {
            let mut bels = vec![];
            for i in 0..4 {
                let mut bel = self
                    .builder
                    .bel_xy(bels::LAGUNA[i], "LAGUNA", i >> 1, i & 1);
                for j in 0..6 {
                    bel = bel
                        .pin_name_only(&format!("RXQ{j}"), 0)
                        .pin_name_only(&format!("RXD{j}"), 0)
                        .pin_name_only(&format!("TXQ{j}"), 0)
                        .extra_int_out(format!("RXOUT{j}"), &[format!("RXD{ii}", ii = i * 6 + j)])
                        .extra_wire(
                            format!("TXOUT{j}"),
                            &[format!(
                                "LAG_MUX_ATOM_{ii}_TXOUT",
                                ii = match (i, j) {
                                    (0, 0) => 0,
                                    (0, 1) => 11,
                                    (0, 2) => 16,
                                    (0, 3) => 17,
                                    (0, 4) => 18,
                                    (0, 5) => 19,
                                    (1, 0) => 20,
                                    (1, 1) => 21,
                                    (1, 2) => 22,
                                    (1, 3) => 23,
                                    (1, 4) => 1,
                                    (1, 5) => 2,
                                    (2, 0) => 3,
                                    (2, 1) => 4,
                                    (2, 2) => 5,
                                    (2, 3) => 6,
                                    (2, 4) => 7,
                                    (2, 5) => 8,
                                    (3, 0) => 9,
                                    (3, 1) => 10,
                                    (3, 2) => 12,
                                    (3, 3) => 13,
                                    (3, 4) => 14,
                                    (3, 5) => 15,
                                    _ => unreachable!(),
                                }
                            )],
                        )
                        .extra_wire(format!("UBUMP{j}"), &[format!("UBUMP{ii}", ii = i * 6 + j)]);
                }
                bels.push(bel);
            }
            bels.push(
                self.builder
                    .bel_virtual(bels::VCC_LAGUNA)
                    .extra_wire("VCC", &["VCC_WIRE"]),
            );
            self.builder
                .xnode(tslots::BEL, "LAGUNA", "LAGUNA", xy)
                .ref_int_side(xy.delta(2, 0), Dir::W, 0)
                .bels(bels)
                .extract();
        }
    }

    fn fill_tiles_bram(&mut self) {
        if let Some(&xy) = self.builder.rd.tiles_by_kind_name("BRAM").iter().next() {
            let intf = self.builder.ndb.get_tile_class_naming("INTF.W");
            let mut bel_bram_f = self
                .builder
                .bel_xy(bels::BRAM_F, "RAMB36", 0, 0)
                .pin_name_only("CASINSBITERR", 1)
                .pin_name_only("CASINDBITERR", 1)
                .pin_name_only("CASOUTSBITERR", 0)
                .pin_name_only("CASOUTDBITERR", 0)
                .pin_name_only("CASPRVEMPTY", 1)
                .pin_name_only("CASPRVRDEN", 1)
                .pin_name_only("CASNXTEMPTY", 1)
                .pin_name_only("CASNXTRDEN", 1)
                .pin_name_only("CASMBIST12OUT", 0)
                .pin_name_only("ENABLE_BIST", 1)
                .pin_name_only("START_RSR_NEXT", 0);
            let mut bel_bram_h0 = self
                .builder
                .bel_xy(bels::BRAM_H0, "RAMB18", 0, 0)
                .pin_name_only("CASPRVEMPTY", 0)
                .pin_name_only("CASPRVRDEN", 0)
                .pin_name_only("CASNXTEMPTY", 0)
                .pin_name_only("CASNXTRDEN", 0);
            let mut bel_bram_h1 = self.builder.bel_xy(bels::BRAM_H1, "RAMB18", 0, 1);
            for ab in ['A', 'B'] {
                for ul in ['U', 'L'] {
                    for i in 0..16 {
                        bel_bram_f = bel_bram_f.pin_name_only(&format!("CASDI{ab}{ul}{i}"), 1);
                        bel_bram_f = bel_bram_f.pin_name_only(&format!("CASDO{ab}{ul}{i}"), 1);
                    }
                    for i in 0..2 {
                        bel_bram_f = bel_bram_f.pin_name_only(&format!("CASDIP{ab}{ul}{i}"), 1);
                        bel_bram_f = bel_bram_f.pin_name_only(&format!("CASDOP{ab}{ul}{i}"), 1);
                    }
                }
                for i in 0..16 {
                    bel_bram_h0 = bel_bram_h0.pin_name_only(&format!("CASDI{ab}L{i}"), 0);
                    bel_bram_h0 = bel_bram_h0.pin_name_only(&format!("CASDO{ab}L{i}"), 0);
                }
                for i in 0..2 {
                    bel_bram_h0 = bel_bram_h0.pin_name_only(&format!("CASDIP{ab}L{i}"), 0);
                    bel_bram_h0 = bel_bram_h0.pin_name_only(&format!("CASDOP{ab}L{i}"), 0);
                }
                for i in 0..16 {
                    bel_bram_h1 = bel_bram_h1.pin_name_only(&format!("CASDI{ab}U{i}"), 0);
                    bel_bram_h1 = bel_bram_h1.pin_name_only(&format!("CASDO{ab}U{i}"), 0);
                }
                for i in 0..2 {
                    bel_bram_h1 = bel_bram_h1.pin_name_only(&format!("CASDIP{ab}U{i}"), 0);
                    bel_bram_h1 = bel_bram_h1.pin_name_only(&format!("CASDOP{ab}U{i}"), 0);
                }
            }
            let mut xn = self
                .builder
                .xnode(tslots::BEL, "BRAM", "BRAM", xy)
                .num_tiles(5);
            for i in 0..5 {
                xn = xn
                    .ref_int_side(xy.delta(2, i as i32), Dir::W, i)
                    .ref_single(xy.delta(1, i as i32), i, intf);
            }
            xn.bels([bel_bram_f, bel_bram_h0, bel_bram_h1]).extract();
        }

        for tkn in [
            "RCLK_BRAM_INTF_L",
            "RCLK_BRAM_INTF_TD_L",
            "RCLK_BRAM_INTF_TD_R",
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let intf = self.builder.ndb.get_tile_class_naming("INTF.W");
                let mut bels = vec![];
                for (i, x, y) in [(0, 0, 0), (1, 0, 1), (2, 1, 0), (3, 1, 1)] {
                    bels.push(self.builder.bel_xy(bels::HARD_SYNC[i], "HARD_SYNC", x, y));
                }
                self.builder
                    .xnode(tslots::RCLK_BEL, "HARD_SYNC", "HARD_SYNC", xy)
                    .ref_int_side(xy.delta(2, 1), Dir::W, 0)
                    .ref_single(xy.delta(1, 1), 0, intf)
                    .bels(bels)
                    .extract();
            }
        }
    }

    fn fill_tiles_dsp(&mut self) {
        if let Some(&xy) = self.builder.rd.tiles_by_kind_name("DSP").iter().next() {
            let intf = self.builder.ndb.get_tile_class_naming("INTF.E");

            let mut bels_dsp = vec![];
            for i in 0..2 {
                let mut bel = self.builder.bel_xy(bels::DSP[i], "DSP48E2", 0, i);
                let buf_cnt = match i {
                    0 => 1,
                    1 => 0,
                    _ => unreachable!(),
                };
                bel = bel.pin_name_only("MULTSIGNIN", buf_cnt);
                bel = bel.pin_name_only("MULTSIGNOUT", 0);
                bel = bel.pin_name_only("CARRYCASCIN", buf_cnt);
                bel = bel.pin_name_only("CARRYCASCOUT", 0);
                for j in 0..30 {
                    bel = bel.pin_name_only(&format!("ACIN_B{j}"), buf_cnt);
                    bel = bel.pin_name_only(&format!("ACOUT_B{j}"), 0);
                }
                for j in 0..18 {
                    bel = bel.pin_name_only(&format!("BCIN_B{j}"), buf_cnt);
                    bel = bel.pin_name_only(&format!("BCOUT_B{j}"), 0);
                }
                for j in 0..48 {
                    bel = bel.pin_name_only(&format!("PCIN{j}"), buf_cnt);
                    bel = bel.pin_name_only(&format!("PCOUT{j}"), 0);
                }
                bels_dsp.push(bel);
            }
            let mut xn = self
                .builder
                .xnode(tslots::BEL, "DSP", "DSP", xy)
                .num_tiles(5);
            for i in 0..5 {
                xn = xn
                    .ref_int_side(xy.delta(-2, i as i32), Dir::E, i)
                    .ref_single(xy.delta(-1, i as i32), i, intf);
            }
            xn.bels(bels_dsp).extract();
        }
    }

    fn fill_tiles_bli(&mut self) {
        if let Some(&xy) = self
            .builder
            .rd
            .tiles_by_kind_name("BLI_BLI_FT")
            .iter()
            .next()
        {
            let intf = self.builder.ndb.get_tile_class_naming("INTF.E");
            let bels = [
                self.builder
                    .bel_xy(bels::BLI_HBM_APB_INTF, "BLI_HBM_APB_INTF", 0, 0),
                self.builder
                    .bel_xy(bels::BLI_HBM_AXI_INTF, "BLI_HBM_AXI_INTF", 0, 0),
            ];
            let mut xn = self
                .builder
                .xnode(tslots::BEL, "BLI", "BLI", xy)
                .num_tiles(15);
            for i in 0..15 {
                xn = xn
                    .ref_int_side(xy.delta(-2, i as i32), Dir::E, i)
                    .ref_single(xy.delta(-1, i as i32), i, intf);
            }
            xn.bels(bels).extract();
        }
    }

    fn fill_tiles_uram(&mut self) {
        for tkn in ["URAM_URAM_FT", "URAM_URAM_DELAY_FT"] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let intf_e = self.builder.ndb.get_tile_class_naming("INTF.E");
                let intf_w = self.builder.ndb.get_tile_class_naming("INTF.W");
                let mut bels = vec![];
                for i in 0..4 {
                    let mut bel = self.builder.bel_xy(bels::URAM[i], "URAM288", 0, i);
                    let buf_cnt = match i {
                        0 => 1,
                        _ => 0,
                    };
                    for ab in ['A', 'B'] {
                        for j in 0..23 {
                            bel = bel.pin_name_only(&format!("CAS_IN_ADDR_{ab}{j}"), buf_cnt);
                            bel = bel.pin_name_only(&format!("CAS_OUT_ADDR_{ab}{j}"), 0);
                        }
                        for j in 0..9 {
                            bel = bel.pin_name_only(&format!("CAS_IN_BWE_{ab}{j}"), buf_cnt);
                            bel = bel.pin_name_only(&format!("CAS_OUT_BWE_{ab}{j}"), 0);
                        }
                        for j in 0..72 {
                            bel = bel.pin_name_only(&format!("CAS_IN_DIN_{ab}{j}"), buf_cnt);
                            bel = bel.pin_name_only(&format!("CAS_OUT_DIN_{ab}{j}"), 0);
                            bel = bel.pin_name_only(&format!("CAS_IN_DOUT_{ab}{j}"), buf_cnt);
                            bel = bel.pin_name_only(&format!("CAS_OUT_DOUT_{ab}{j}"), 0);
                        }
                        for pin in ["EN", "RDACCESS", "RDB_WR", "DBITERR", "SBITERR"] {
                            bel = bel.pin_name_only(&format!("CAS_IN_{pin}_{ab}"), buf_cnt);
                            bel = bel.pin_name_only(&format!("CAS_OUT_{pin}_{ab}"), 0);
                        }
                    }
                    bels.push(bel);
                }
                let mut xn = self
                    .builder
                    .xnode(tslots::BEL, "URAM", "URAM", xy)
                    .num_tiles(30);
                for i in 0..15 {
                    xn = xn
                        .ref_int_side(xy.delta(-2, i as i32), Dir::E, i)
                        .ref_single(xy.delta(-1, i as i32), i, intf_e);
                }
                for i in 0..15 {
                    xn = xn
                        .ref_int_side(xy.delta(2, i as i32), Dir::W, i + 15)
                        .ref_single(xy.delta(1, i as i32), i + 15, intf_w);
                }
                xn.bels(bels).extract();
            }
        }
    }

    fn fill_tiles_hard(&mut self) {
        for (slot, kind, tkn, bk) in [
            (bels::PCIE4, "PCIE4", "PCIE4_PCIE4_FT", "PCIE40E4"),
            (bels::PCIE4C, "PCIE4C", "PCIE4C_PCIE4C_FT", "PCIE4CE4"),
            (bels::PCIE4CE, "PCIE4CE", "PCIE4CE_PCIE4CE_FT", "PCIE4CE"),
            (bels::CMAC, "CMAC", "CMAC", "CMACE4"),
            (bels::ILKN, "ILKN", "ILKN_ILKN_FT", "ILKNE4"),
            (bels::DFE_A, "DFE_A", "DFE_DFE_TILEA_FT", "DFE_A"),
            (bels::DFE_C, "DFE_C", "DFE_DFE_TILEC_FT", "DFE_C"),
            (bels::DFE_D, "DFE_D", "DFE_DFE_TILED_FT", "DFE_D"),
            (bels::DFE_E, "DFE_E", "DFE_DFE_TILEE_FT", "DFE_E"),
            (bels::DFE_F, "DFE_F", "DFE_DFE_TILEF_FT", "DFE_F"),
            (bels::DFE_G, "DFE_G", "DFE_DFE_TILEG_FT", "DFE_G"),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let int_w_xy = self.builder.walk_to_int(xy, Dir::W, false).unwrap();
                let int_e_xy = self.builder.walk_to_int(xy, Dir::E, false).unwrap();
                let intf_w = self.builder.ndb.get_tile_class_naming("INTF.E.PCIE");
                let intf_e = self.builder.ndb.get_tile_class_naming("INTF.W.PCIE");
                let mut bel = self.builder.bel_xy(slot, bk, 0, 0);
                let mut naming = kind;
                if kind == "PCIE4" {
                    let mut has_mcap = false;
                    if let Some(wire) = self
                        .builder
                        .rd
                        .wires
                        .get("PCIE4_PCIE4_CORE_0_MCAP_PERST0_B_PIN")
                    {
                        let tk = &self.builder.rd.tile_kinds[self.builder.rd.tiles[&xy].kind];
                        if tk.wires.contains_key(&wire) {
                            has_mcap = true;
                        }
                    }
                    if has_mcap {
                        bel = bel
                            .pin_name_only("MCAP_PERST0_B", 1)
                            .pin_name_only("MCAP_PERST1_B", 1);
                    } else {
                        bel = bel
                            .pin_name_only("MCAP_PERST0_B", 0)
                            .pin_name_only("MCAP_PERST1_B", 0);
                        naming = "PCIE4.NOCFG";
                    }
                } else if kind == "PCIE4C" {
                    let mut has_mcap = false;
                    if let Some(wire) = self.builder.rd.wires.get("PCIE4C_CORE_0_MCAP_PERST0_B_PIN")
                    {
                        let tk = &self.builder.rd.tile_kinds[self.builder.rd.tiles[&xy].kind];
                        if tk.wires.contains_key(&wire) {
                            has_mcap = true;
                        }
                    }
                    if has_mcap {
                        bel = bel
                            .pin_name_only("MCAP_PERST0_B", 1)
                            .pin_name_only("MCAP_PERST1_B", 1);
                    } else {
                        bel = bel
                            .pin_name_only("MCAP_PERST0_B", 0)
                            .pin_name_only("MCAP_PERST1_B", 0);
                        naming = "PCIE4C.NOCFG";
                    }
                } else if kind == "PCIE4CE" {
                    bel = bel
                        .pin_name_only("MCAP_PERST0_B", 0)
                        .pin_name_only("MCAP_PERST1_B", 0);
                }
                let mut xn = self
                    .builder
                    .xnode(tslots::BEL, kind, naming, xy)
                    .num_tiles(120);
                for i in 0..60 {
                    xn = xn
                        .ref_int_side(int_w_xy.delta(0, (i + i / 30) as i32), Dir::E, i)
                        .ref_int_side(int_e_xy.delta(0, (i + i / 30) as i32), Dir::W, i + 60)
                        .ref_single(int_w_xy.delta(1, (i + i / 30) as i32), i, intf_w)
                        .ref_single(int_e_xy.delta(-1, (i + i / 30) as i32), i + 60, intf_e)
                }
                xn.bel(bel).extract();
            }
        }

        if let Some(&xy) = self
            .builder
            .rd
            .tiles_by_kind_name("DFE_DFE_TILEB_FT")
            .iter()
            .next()
        {
            let int_xy = self.builder.walk_to_int(xy, Dir::W, false).unwrap();
            let intf = self.builder.ndb.get_tile_class_naming("INTF.E.PCIE");
            let bel = self.builder.bel_xy(bels::DFE_B, "DFE_B", 0, 0);
            let mut xn = self
                .builder
                .xnode(tslots::BEL, "DFE_B", "DFE_B", xy)
                .num_tiles(60);
            for i in 0..60 {
                xn = xn
                    .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), Dir::E, i)
                    .ref_single(int_xy.delta(1, (i + i / 30) as i32), i, intf);
            }
            xn.bel(bel).extract();
        }

        if let Some(&xy) = self.builder.rd.tiles_by_kind_name("FE_FE_FT").iter().next() {
            let int_xy = self.builder.walk_to_int(xy, Dir::E, false).unwrap();
            let intf = self.builder.ndb.get_tile_class_naming("INTF.W.PCIE");
            let bel = self.builder.bel_xy(bels::FE, "FE", 0, 0);
            let mut xn = self
                .builder
                .xnode(tslots::BEL, "FE", "FE", xy)
                .num_tiles(60);
            for i in 0..60 {
                xn = xn
                    .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), Dir::W, i)
                    .ref_single(int_xy.delta(-1, (i + i / 30) as i32), i, intf);
            }
            xn.bel(bel).extract();
        }
    }

    fn fill_tiles_ps(&mut self) {
        if let Some(&xy) = self.builder.rd.tiles_by_kind_name("PSS_ALTO").iter().next() {
            let int_r_xy = self.builder.walk_to_int(xy, Dir::E, false).unwrap();
            let intf = self.builder.ndb.get_tile_class_naming("INTF.PSS");
            let mut bel = self.builder.bel_xy(bels::PS, "PS8", 0, 0).pins_name_only(&[
                "DP_AUDIO_REF_CLK",
                "DP_VIDEO_REF_CLK",
                "DDR_DTO0",
                "DDR_DTO1",
                "APLL_TEST_CLK_OUT0",
                "APLL_TEST_CLK_OUT1",
                "RPLL_TEST_CLK_OUT0",
                "RPLL_TEST_CLK_OUT1",
                "DPLL_TEST_CLK_OUT0",
                "DPLL_TEST_CLK_OUT1",
                "IOPLL_TEST_CLK_OUT0",
                "IOPLL_TEST_CLK_OUT1",
                "VPLL_TEST_CLK_OUT0",
                "VPLL_TEST_CLK_OUT1",
                "FMIO_GEM0_FIFO_RX_CLK_TO_PL_BUFG",
                "FMIO_GEM0_FIFO_TX_CLK_TO_PL_BUFG",
                "FMIO_GEM1_FIFO_RX_CLK_TO_PL_BUFG",
                "FMIO_GEM1_FIFO_TX_CLK_TO_PL_BUFG",
                "FMIO_GEM2_FIFO_RX_CLK_TO_PL_BUFG",
                "FMIO_GEM2_FIFO_TX_CLK_TO_PL_BUFG",
                "FMIO_GEM3_FIFO_RX_CLK_TO_PL_BUFG",
                "FMIO_GEM3_FIFO_TX_CLK_TO_PL_BUFG",
                "FMIO_GEM_TSU_CLK_TO_PL_BUFG",
                "PL_CLK0",
                "PL_CLK1",
                "PL_CLK2",
                "PL_CLK3",
                "O_DBG_L0_RXCLK",
                "O_DBG_L0_TXCLK",
                "O_DBG_L1_RXCLK",
                "O_DBG_L1_TXCLK",
                "O_DBG_L2_RXCLK",
                "O_DBG_L2_TXCLK",
                "O_DBG_L3_RXCLK",
                "O_DBG_L3_TXCLK",
                "PS_PL_SYSOSC_CLK",
                "BSCAN_RESET_TAP_B",
                "BSCAN_CLOCKDR",
                "BSCAN_SHIFTDR",
                "BSCAN_UPDATEDR",
                "BSCAN_INTEST",
                "BSCAN_EXTEST",
                "BSCAN_INIT_MEMORY",
                "BSCAN_AC_TEST",
                "BSCAN_AC_MODE",
                "BSCAN_MISR_JTAG_LOAD",
                "PSS_CFG_RESET_B",
                "PSS_FST_CFG_B",
                "PSS_GTS_CFG_B",
                "PSS_GTS_USR_B",
                "PSS_GHIGH_B",
                "PSS_GPWRDWN_B",
                "PCFG_POR_B",
            ]);
            for pin in [
                "IDCODE15",
                "IDCODE16",
                "IDCODE17",
                "IDCODE18",
                "IDCODE20",
                "IDCODE21",
                "IDCODE28",
                "IDCODE29",
                "IDCODE30",
                "IDCODE31",
                "PS_VERSION_0",
                "PS_VERSION_2",
                "PS_VERSION_3",
            ] {
                bel = bel.pin_dummy(pin);
            }
            let mut xn = self
                .builder
                .xnode(tslots::BEL, "PS", "PS", xy)
                .num_tiles(180);
            for i in 0..180 {
                xn = xn
                    .ref_int_side(int_r_xy.delta(0, (i + i / 30) as i32), Dir::W, i)
                    .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i, intf);
            }
            xn.bel(bel).extract();
        }

        if let Some(&xy) = self
            .builder
            .rd
            .tiles_by_kind_name("VCU_VCU_FT")
            .iter()
            .next()
        {
            let int_r_xy = self
                .builder
                .walk_to_int(xy.delta(0, 2), Dir::E, false)
                .unwrap();
            let intf = self.builder.ndb.get_tile_class_naming("INTF.PSS");
            let bel = self
                .builder
                .bel_xy(bels::VCU, "VCU", 0, 0)
                .pins_name_only(&["VCU_PLL_TEST_CLK_OUT0", "VCU_PLL_TEST_CLK_OUT1"]);
            let mut xn = self
                .builder
                .xnode(tslots::BEL, "VCU", "VCU", xy)
                .num_tiles(60);
            for i in 0..60 {
                xn = xn
                    .ref_int_side(int_r_xy.delta(0, (i + i / 30) as i32), Dir::W, i)
                    .ref_single(int_r_xy.delta(-1, (i + i / 30) as i32), i, intf);
            }
            xn.bel(bel).extract();
        }

        for tkn in [
            "RCLK_INTF_LEFT_TERM_ALTO",
            "RCLK_RCLK_INTF_LEFT_TERM_DA6_FT",
            "RCLK_INTF_LEFT_TERM_DA7",
            "RCLK_RCLK_INTF_LEFT_TERM_DA8_FT",
            "RCLK_RCLK_INTF_LEFT_TERM_DC12_FT",
            "RCLK_RCLK_INTF_LEFT_TERM_MX8_FT",
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let rclk_int = self.builder.ndb.get_tile_class_naming("RCLK_INT");
                let mut bels = vec![];
                for i in 0..24 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::BUFG_PS[i], "BUFG_PS", 0, i)
                            .pins_name_only(&["CLK_IN", "CLK_OUT"])
                            .extra_wire(
                                "CLK_IN_DUMMY",
                                &[format!(
                                    "VCC_WIRE{ii}",
                                    ii = [
                                        0, 3, 8, 9, 10, 11, 13, 14, 15, 16, 1, 12, 17, 18, 19, 20,
                                        21, 22, 23, 2, 4, 5, 6, 7
                                    ][i]
                                )],
                            ),
                    );
                }
                let mut bel = self
                    .builder
                    .bel_virtual(bels::RCLK_PS)
                    .extra_int_in("CKINT", &["INT_RCLK_TO_CLK_0_FT1_0"]);
                for i in 0..18 {
                    bel = bel.extra_wire(format!("PS_TO_PL_CLK{i}"), &[format!("PS_TO_PL_CLK{i}")]);
                }
                for i in 0..24 {
                    bel = bel.extra_wire(format!("HROUTE{i}"), &[format!("CLK_HROUTE{i}")]);
                }
                bels.push(bel);
                bels.push(
                    self.builder
                        .bel_virtual(bels::VCC_RCLK_PS)
                        .extra_wire("VCC", &["VCC_WIRE"]),
                );
                self.builder
                    .xnode(tslots::RCLK_BEL, "RCLK_PS", "RCLK_PS", xy)
                    .ref_xlat(xy.delta(1, 0), &[Some(0), None, None, None], rclk_int)
                    .bels(bels)
                    .extract();
            }
        }
    }

    fn fill_tiles_cfg(&mut self) {
        for (kind, tkn, bkind) in [
            ("CFG", "CFG_CONFIG", "CONFIG_SITE"),
            ("CFG_CSEC", "CSEC_CONFIG_FT", "CSEC_SITE"),
            ("CFG_CSEC_V2", "CSEC_CONFIG_VER2_FT", "CSEC_SITE"),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let int_xy = self.builder.walk_to_int(xy, Dir::W, false).unwrap();
                let intf = self.builder.ndb.get_tile_class_naming("INTF.E.PCIE");
                let bels = [
                    self.builder.bel_xy(bels::CFG, bkind, 0, 0),
                    self.builder
                        .bel_xy(bels::ABUS_SWITCH_CFG, "ABUS_SWITCH", 0, 0)
                        .pins_name_only(&["TEST_ANALOGBUS_SEL_B"]),
                ];
                let mut xn = self
                    .builder
                    .xnode(tslots::BEL, kind, kind, xy)
                    .num_tiles(60);
                for i in 0..60 {
                    xn = xn
                        .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), Dir::E, i)
                        .ref_single(int_xy.delta(1, (i + i / 30) as i32), i, intf)
                }
                xn.bels(bels).extract();
            }
        }

        for tkn in ["CFGIO_IOB20", "CFGIOLC_IOB20_FT"] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let int_xy = self.builder.walk_to_int(xy, Dir::W, false).unwrap();
                let intf = self.builder.ndb.get_tile_class_naming("INTF.E.PCIE");
                let bels = [
                    self.builder.bel_xy(bels::PMV, "PMV", 0, 0),
                    self.builder.bel_xy(bels::PMV2, "PMV2", 0, 0),
                    self.builder.bel_xy(bels::PMVIOB, "PMVIOB", 0, 0),
                    self.builder.bel_xy(bels::MTBF3, "MTBF3", 0, 0),
                    self.builder.bel_xy(bels::CFGIO, "CFGIO_SITE", 0, 0),
                ];
                let mut xn = self
                    .builder
                    .xnode(tslots::BEL, "CFGIO", "CFGIO", xy)
                    .num_tiles(30);
                for i in 0..30 {
                    xn = xn
                        .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), Dir::E, i)
                        .ref_single(int_xy.delta(1, (i + i / 30) as i32), i, intf)
                }
                xn.bels(bels).extract();
            }
        }

        if let Some(&xy) = self.builder.rd.tiles_by_kind_name("AMS").iter().next() {
            let int_xy = self.builder.walk_to_int(xy, Dir::W, false).unwrap();
            let intf = self.builder.ndb.get_tile_class_naming("INTF.E.PCIE");
            let mut bel = self.builder.bel_xy(bels::SYSMON, "SYSMONE4", 0, 0);
            for i in 0..16 {
                bel = bel
                    .pin_name_only(&format!("VP_AUX{i}"), 1)
                    .pin_name_only(&format!("VN_AUX{i}"), 1);
            }
            let mut xn = self
                .builder
                .xnode(tslots::BEL, "AMS", "AMS", xy)
                .num_tiles(30);
            for i in 0..30 {
                xn = xn
                    .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), Dir::E, i)
                    .ref_single(int_xy.delta(1, (i + i / 30) as i32), i, intf)
            }
            xn.bel(bel).extract();
        }

        if let Some(&xy) = self
            .builder
            .rd
            .tiles_by_kind_name("CFRM_CFRAME_TERM_H_FT")
            .iter()
            .next()
        {
            let mut bels = vec![];
            for i in 0..8 {
                bels.push(
                    self.builder
                        .bel_xy(bels::ABUS_SWITCH_HBM[i], "ABUS_SWITCH", i >> 1, i & 1)
                        .pins_name_only(&["TEST_ANALOGBUS_SEL_B"]),
                );
            }
            self.builder
                .xnode(tslots::CMT, "HBM_ABUS_SWITCH", "HBM_ABUS_SWITCH", xy)
                .num_tiles(0)
                .bels(bels)
                .extract();
        }
    }

    fn fill_tiles_hdio(&mut self) {
        for tkn in ["HDIO_BOT_RIGHT", "HDIO_TOP_RIGHT"] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let is_bot = tkn == "HDIO_BOT_RIGHT";
                let int_xy = self.builder.walk_to_int(xy, Dir::W, false).unwrap();
                let intf = self.builder.ndb.get_tile_class_naming("INTF.E.PCIE");
                let mut bels = vec![];
                for i in 0..6 {
                    bels.extend([
                        self.builder
                            .bel_xy(bels::HDIOB[2 * i], "IOB", 0, 2 * i)
                            .pins_name_only(&[
                                "OP",
                                "TSP",
                                "O_B",
                                "TSTATEB",
                                "OUTB_B",
                                "OUTB_B_IN",
                                "TSTATE_IN",
                                "TSTATE_OUT",
                                "LVDS_TRUE",
                                "PAD_RES",
                                "I",
                            ])
                            .pin_name_only("SWITCH_OUT", 1)
                            .pin_dummy("IO"),
                        self.builder
                            .bel_xy(bels::HDIOB[2 * i + 1], "IOB", 0, 2 * i + 1)
                            .pins_name_only(&[
                                "OP",
                                "TSP",
                                "O_B",
                                "TSTATEB",
                                "OUTB_B",
                                "OUTB_B_IN",
                                "TSTATE_IN",
                                "TSTATE_OUT",
                                "LVDS_TRUE",
                                "PAD_RES",
                                "I",
                            ])
                            .pin_name_only("SWITCH_OUT", 1)
                            .pin_dummy("IO"),
                    ]);
                }
                for i in 0..6 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::HDIOB_DIFF_IN[i], "HDIOBDIFFINBUF", 0, i)
                            .pins_name_only(&["LVDS_TRUE", "LVDS_COMP", "PAD_RES_0", "PAD_RES_1"]),
                    );
                }
                for i in 0..6 {
                    bels.extend([
                        self.builder
                            .bel_xy(bels::HDIOLOGIC[2 * i], "HDIOLOGIC_M", 0, i)
                            .pins_name_only(&["OPFFM_Q", "TFFM_Q", "IPFFM_D"]),
                        self.builder
                            .bel_xy(bels::HDIOLOGIC[2 * i + 1], "HDIOLOGIC_S", 0, i)
                            .pins_name_only(&["OPFFS_Q", "TFFS_Q", "IPFFS_D"]),
                    ]);
                }
                bels.push(
                    self.builder
                        .bel_xy(bels::HDLOGIC_CSSD0, "HDLOGIC_CSSD", 0, 0),
                );
                if is_bot {
                    bels.push(self.builder.bel_xy(bels::HDIO_VREF0, "HDIO_VREF", 0, 0));
                } else {
                    bels.push(self.builder.bel_xy(bels::HDIO_BIAS, "HDIO_BIAS", 0, 0));
                }
                let kind = if is_bot { "HDIO_S" } else { "HDIO_N" };
                let mut xn = self
                    .builder
                    .xnode(tslots::BEL, kind, kind, xy)
                    .num_tiles(30);
                for i in 0..30 {
                    xn = xn
                        .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), Dir::E, i)
                        .ref_single(int_xy.delta(1, (i + i / 30) as i32), i, intf)
                }
                xn.bels(bels).extract();
            }
        }

        for (kind, tkn, side) in [
            ("HDIOL_S", "HDIOLC_HDIOL_BOT_LEFT_FT", Dir::W),
            ("HDIOL_N", "HDIOLC_HDIOL_TOP_LEFT_FT", Dir::W),
            ("HDIOL_S", "HDIOLC_HDIOL_BOT_RIGHT_CFG_FT", Dir::E),
            ("HDIOL_N", "HDIOLC_HDIOL_TOP_RIGHT_CFG_FT", Dir::E),
            ("HDIOL_S", "HDIOLC_HDIOL_BOT_RIGHT_AUX_FT", Dir::E),
            ("HDIOL_N", "HDIOLC_HDIOL_TOP_RIGHT_AUX_FT", Dir::E),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let int_xy = self.builder.walk_to_int(xy, !side, false).unwrap();
                let intf = self.builder.ndb.get_tile_class_naming(if side == Dir::W {
                    "INTF.W.IO"
                } else {
                    "INTF.E.PCIE"
                });
                let mut bels = vec![];
                for i in 0..21 {
                    bels.extend([
                        self.builder
                            .bel_xy(bels::HDIOB[2 * i], "IOB", 0, 2 * i)
                            .pins_name_only(&[
                                "OP",
                                "TSP",
                                "O_B",
                                "TSTATEB",
                                "OUTB_B",
                                "OUTB_B_IN",
                                "TSTATE_IN",
                                "TSTATE_OUT",
                                "LVDS_TRUE",
                                "PAD_RES",
                                "I",
                            ])
                            .pin_name_only("SWITCH_OUT", 0)
                            .pin_dummy("IO"),
                        self.builder
                            .bel_xy(bels::HDIOB[2 * i + 1], "IOB", 0, 2 * i + 1)
                            .pins_name_only(&[
                                "OP",
                                "TSP",
                                "O_B",
                                "TSTATEB",
                                "OUTB_B",
                                "OUTB_B_IN",
                                "TSTATE_IN",
                                "TSTATE_OUT",
                                "LVDS_TRUE",
                                "PAD_RES",
                                "I",
                            ])
                            .pin_name_only("SWITCH_OUT", 0)
                            .pin_dummy("IO"),
                    ]);
                }
                for i in 0..21 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::HDIOB_DIFF_IN[i], "HDIOBDIFFINBUF", 0, i)
                            .pins_name_only(&["LVDS_TRUE", "LVDS_COMP", "PAD_RES_0", "PAD_RES_1"]),
                    );
                }
                for i in 0..21 {
                    bels.extend([
                        self.builder
                            .bel_xy(bels::HDIOLOGIC[2 * i], "HDIOLOGIC_M", 0, i)
                            .pins_name_only(&["OPFFM_Q", "TFFM_Q", "IPFFM_D"]),
                        self.builder
                            .bel_xy(bels::HDIOLOGIC[2 * i + 1], "HDIOLOGIC_S", 0, i)
                            .pins_name_only(&["OPFFS_Q", "TFFS_Q", "IPFFS_D"]),
                    ]);
                }
                for i in 0..3 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::HDLOGIC_CSSD[i], "HDLOGIC_CSSD", 0, i),
                    );
                }
                for i in 0..2 {
                    bels.push(self.builder.bel_xy(bels::HDIO_VREF[i], "HDIO_VREF", 0, i));
                }
                bels.push(self.builder.bel_xy(bels::HDIO_BIAS, "HDIO_BIAS", 0, 0));
                let mut xn = self.builder.xnode(tslots::BEL, kind, tkn, xy).num_tiles(30);
                for i in 0..30 {
                    xn = xn
                        .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), side, i)
                        .ref_single(
                            int_xy.delta(if side == Dir::W { -1 } else { 1 }, (i + i / 30) as i32),
                            i,
                            intf,
                        )
                }
                xn.bels(bels).extract();
            }
        }

        for tkn in [
            "HDIOLC_HDIOS_BOT_LEFT_FT",
            "HDIOLC_HDIOS_BOT_LEFT_CFG_FT",
            "HDIOLC_HDIOS_BOT_LEFT_AUX_FT",
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let int_w_xy = self.builder.walk_to_int(xy, Dir::W, false).unwrap();
                let int_e_xy = self.builder.walk_to_int(xy, Dir::E, false).unwrap();
                let intf_w = self.builder.ndb.get_tile_class_naming("INTF.E.PCIE");
                let intf_e = self.builder.ndb.get_tile_class_naming("INTF.W.PCIE");
                let mut bels = vec![];
                for i in 0..21 {
                    bels.extend([
                        self.builder
                            .bel_xy(bels::HDIOB[2 * i], "IOB", 0, 2 * (i % 11))
                            .raw_tile(i / 11)
                            .pins_name_only(&[
                                "OP",
                                "TSP",
                                "O_B",
                                "TSTATEB",
                                "OUTB_B",
                                "OUTB_B_IN",
                                "TSTATE_IN",
                                "TSTATE_OUT",
                                "LVDS_TRUE",
                                "PAD_RES",
                                "I",
                            ])
                            .pin_name_only("SWITCH_OUT", 0)
                            .pin_dummy("IO"),
                        self.builder
                            .bel_xy(bels::HDIOB[2 * i + 1], "IOB", 0, 2 * (i % 11) + 1)
                            .raw_tile(i / 11)
                            .pins_name_only(&[
                                "OP",
                                "TSP",
                                "O_B",
                                "TSTATEB",
                                "OUTB_B",
                                "OUTB_B_IN",
                                "TSTATE_IN",
                                "TSTATE_OUT",
                                "LVDS_TRUE",
                                "PAD_RES",
                                "I",
                            ])
                            .pin_name_only("SWITCH_OUT", 0)
                            .pin_dummy("IO"),
                    ]);
                }
                for i in 0..21 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::HDIOB_DIFF_IN[i], "HDIOBDIFFINBUF", 0, i % 11)
                            .raw_tile(i / 11)
                            .pins_name_only(&["LVDS_TRUE", "LVDS_COMP", "PAD_RES_0", "PAD_RES_1"]),
                    );
                }
                for i in 0..21 {
                    bels.extend([
                        self.builder
                            .bel_xy(bels::HDIOLOGIC[2 * i], "HDIOLOGIC_M", 0, i % 11)
                            .raw_tile(i / 11)
                            .pins_name_only(&["OPFFM_Q", "TFFM_Q", "IPFFM_D"]),
                        self.builder
                            .bel_xy(bels::HDIOLOGIC[2 * i + 1], "HDIOLOGIC_S", 0, i % 11)
                            .raw_tile(i / 11)
                            .pins_name_only(&["OPFFS_Q", "TFFS_Q", "IPFFS_D"]),
                    ]);
                }
                for i in 0..4 {
                    bels.push(
                        self.builder
                            .bel_xy(
                                bels::HDLOGIC_CSSD[i],
                                if i < 2 {
                                    "HDIOS_HDLOGIC_CSSD"
                                } else {
                                    "HDIOS_HDLOGIC_CSSD_TOP"
                                },
                                0,
                                i % 2,
                            )
                            .raw_tile(i / 2),
                    );
                }
                for i in 0..3 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::HDIO_VREF[i], "HDIO_VREF", 0, i % 2)
                            .raw_tile(i / 2),
                    );
                }
                bels.push(
                    self.builder
                        .bel_xy(bels::HDIO_BIAS, "HDIO_BIAS", 0, 0)
                        .raw_tile(1),
                );
                let mut xn = self
                    .builder
                    .xnode(tslots::BEL, "HDIOS", tkn, xy)
                    .raw_tile(xy.delta(0, 31))
                    .num_tiles(120);
                for i in 0..60 {
                    xn = xn
                        .ref_int_side(int_w_xy.delta(0, (i + i / 30) as i32), Dir::E, i)
                        .ref_single(int_w_xy.delta(1, (i + i / 30) as i32), i, intf_w);
                    xn = xn
                        .ref_int_side(int_e_xy.delta(0, (i + i / 30) as i32), Dir::E, i + 60)
                        .ref_single(int_e_xy.delta(-1, (i + i / 30) as i32), i + 60, intf_e);
                }
                xn.bels(bels).extract();
            }
        }

        for (kind, tkn) in [
            ("RCLK_HDIO", "RCLK_HDIO"),
            ("RCLK_HDIO", "RCLK_RCLK_HDIO_R_FT"),
            ("RCLK_HDIO", "RCLK_RCLK_HDIO_LAST_R_FT"),
            ("RCLK_HDIOS", "RCLK_RCLK_HDIOS_L_FT"),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let top_xy = xy.delta(0, -30);
                let int_xy = self.builder.walk_to_int(top_xy, Dir::W, false).unwrap();
                let intf = self.builder.ndb.get_tile_class_naming("INTF.E.PCIE");
                let mut bels = vec![];
                for i in 0..4 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::BUFGCE_HDIO[i], "BUFGCE_HDIO", i >> 1, i & 1)
                            .pins_name_only(&["CLK_IN", "CLK_OUT"])
                            .extra_wire("CLK_IN_MUX", &[format!("CLK_CMT_MUX_4TO1_{i}_CLK_OUT")]),
                    );
                }
                for (i, x, y) in [
                    (0, 0, 0),
                    (1, 0, 1),
                    (2, 1, 0),
                    (3, 1, 1),
                    (4, 2, 0),
                    (5, 2, 1),
                    (6, 3, 0),
                ] {
                    bels.push(
                        self.builder
                            .bel_xy(bels::ABUS_SWITCH_HDIO[i], "ABUS_SWITCH", x, y),
                    );
                }
                let mut bel = self
                    .builder
                    .bel_virtual(bels::RCLK_HDIO)
                    .extra_int_in("CKINT", &["CLK_INT_TOP"]);
                for i in 0..4 {
                    bel = bel.extra_wire(format!("CCIO{i}"), &[format!("CCIO_IO2RCLK{i}")]);
                }
                for i in 0..24 {
                    bel = bel
                        .extra_wire(format!("HROUTE{i}_L"), &[format!("CLK_HROUTE_L{i}")])
                        .extra_wire(format!("HROUTE{i}_R"), &[format!("CLK_HROUTE_R{i}")])
                        .extra_wire(format!("HDISTR{i}_L"), &[format!("CLK_HDISTR_FT0_{i}")])
                        .extra_wire(
                            format!("HROUTE{i}_L_MUX"),
                            &[format!(
                                "CLK_CMT_MUX_2TO1_{ii}_CLK_OUT",
                                ii = XLAT24[i] * 2 + 5
                            )],
                        )
                        .extra_wire(
                            format!("HROUTE{i}_R_MUX"),
                            &[format!(
                                "CLK_CMT_MUX_2TO1_{ii}_CLK_OUT",
                                ii = XLAT24[i] * 2 + 4
                            )],
                        )
                        .extra_wire(
                            format!("HDISTR{i}_MUX"),
                            &[format!("CLK_CMT_MUX_4TO1_{ii}_CLK_OUT", ii = XLAT24[i] + 4)],
                        );
                }
                bels.push(bel);
                bels.push(
                    self.builder
                        .bel_virtual(bels::VCC_RCLK_HDIO)
                        .extra_wire("VCC", &["VCC_WIRE"]),
                );
                let mut xn = self
                    .builder
                    .xnode(tslots::RCLK_BEL, kind, kind, xy)
                    .num_tiles(60);
                for i in 0..60 {
                    xn = xn
                        .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), Dir::E, i)
                        .ref_single(int_xy.delta(1, (i + i / 30) as i32), i, intf)
                }
                xn.bels(bels).extract();
            }
        }

        for (naming, tkn, side) in [
            ("RCLK_HDIOL_L", "RCLK_RCLK_HDIOL_L_FT", Dir::W),
            ("RCLK_HDIOL_L", "RCLK_RCLK_HDIOL_MRC_L_FT", Dir::W),
            ("RCLK_HDIOL_R", "RCLK_RCLK_HDIOL_R_FT", Dir::E),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let top_xy = xy.delta(0, -30);
                let int_xy = self.builder.walk_to_int(top_xy, !side, false).unwrap();
                let intf = self.builder.ndb.get_tile_class_naming(if side == Dir::W {
                    "INTF.W.IO"
                } else {
                    "INTF.E.PCIE"
                });
                let mut bels = vec![];
                for i in 0..4 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::BUFGCE_HDIO[i], "BUFGCE_HDIO", i >> 1, i & 1)
                            .pins_name_only(&["CLK_IN", "CLK_OUT"])
                            .extra_wire("CLK_IN_MUX", &[format!("CLK_CMT_MUX_4TO1_{i}_CLK_OUT")]),
                    );
                }
                for (i, x, y) in [
                    (0, 0, 0),
                    (1, 0, 1),
                    (2, 1, 0),
                    (3, 1, 1),
                    (4, 2, 0),
                    (5, 2, 1),
                    (6, 3, 0),
                    (7, 3, 1),
                    (8, 4, 0),
                    (9, 4, 1),
                    (10, 5, 0),
                    (11, 5, 1),
                ] {
                    bels.push(
                        self.builder
                            .bel_xy(bels::ABUS_SWITCH_HDIO[i], "ABUS_SWITCH", x, y),
                    );
                }
                let mut bel = self
                    .builder
                    .bel_virtual(bels::RCLK_HDIOL)
                    .extra_int_in("CKINT", &["CLK_INT_TOP"]);
                for i in 0..4 {
                    bel = bel.extra_wire(format!("CCIO{i}"), &[format!("CCIO_IO2RCLK{i}")]);
                }
                for i in 0..24 {
                    bel = bel
                        .extra_wire(
                            format!("HROUTE{i}_L"),
                            &[
                                format!("CLK_HROUTE_FT0_{i}"),
                                format!(
                                    "CLK_CMT_DRVR_TRI_HROUTE_{ii}_CLK_OUT_B",
                                    ii = XLAT24[i] * 2 + 1
                                ),
                            ],
                        )
                        .extra_wire(
                            format!("HROUTE{i}_R"),
                            &[
                                format!("CLK_HROUTE_FT1_{i}"),
                                format!(
                                    "CLK_CMT_DRVR_TRI_HROUTE_{ii}_CLK_OUT_B",
                                    ii = XLAT24[i] * 2
                                ),
                            ],
                        )
                        .extra_wire(
                            if side == Dir::W {
                                format!("HDISTR{i}_R")
                            } else {
                                format!("HDISTR{i}_L")
                            },
                            &[format!("CLK_HDISTR_FT0_{i}"), format!("CLK_HDISTR_FT1_{i}")],
                        )
                        .extra_wire(
                            format!("HROUTE{i}_L_MUX"),
                            &[format!(
                                "CLK_CMT_MUX_2TO1_{ii}_CLK_OUT",
                                ii = XLAT24[i] * 2 + 5
                            )],
                        )
                        .extra_wire(
                            format!("HROUTE{i}_R_MUX"),
                            &[format!(
                                "CLK_CMT_MUX_2TO1_{ii}_CLK_OUT",
                                ii = XLAT24[i] * 2 + 4
                            )],
                        )
                        .extra_wire(
                            format!("HDISTR{i}_MUX"),
                            &[format!("CLK_CMT_MUX_4TO1_{ii}_CLK_OUT", ii = XLAT24[i] + 4)],
                        );
                }
                bels.push(bel);
                bels.push(
                    self.builder
                        .bel_virtual(bels::VCC_RCLK_HDIO)
                        .extra_wire("VCC", &["VCC_WIRE"]),
                );
                let mut xn = self
                    .builder
                    .xnode(tslots::RCLK_BEL, "RCLK_HDIOL", naming, xy)
                    .num_tiles(60);
                for i in 0..60 {
                    xn = xn
                        .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), side, i)
                        .ref_single(
                            int_xy.delta(if side == Dir::W { -1 } else { 1 }, (i + i / 30) as i32),
                            i,
                            intf,
                        )
                }
                xn.bels(bels).extract();
            }
        }
    }

    fn fill_tiles_cmt(&mut self) {
        for (kind, naming, tkn, side) in [
            ("CMT", "CMT_L", "CMT_L", Dir::W),
            ("CMT", "CMT_L", "CMT_CMT_LEFT_DL3_FT", Dir::W),
            ("CMT_HBM", "CMT_L_HBM", "CMT_LEFT_H", Dir::W),
            ("CMT", "CMT_R", "CMT_RIGHT", Dir::E),
            ("CMTXP", "CMTXP_R", "CMTXP_CMTXP_RIGHT_FT", Dir::E),
        ] {
            let is_hbm = kind == "CMT_HBM";
            let is_xp = kind == "CMTXP";
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let int_xy = self.builder.walk_to_int(xy, !side, false).unwrap();
                let intf = self.builder.ndb.get_tile_class_naming(if side == Dir::W {
                    "INTF.W.IO"
                } else {
                    "INTF.E.IO"
                });
                let rclk_int = self.builder.ndb.get_tile_class_naming("RCLK_INT");
                let mut bels = vec![];
                for i in 0..24 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::BUFCE_ROW_CMT[i], "BUFCE_ROW", 0, i)
                            .pins_name_only(&["CLK_IN", "CLK_OUT", "CLK_OUT_OPT_DLY"]),
                    );
                }
                for i in 0..24 {
                    bels.push(
                        self.builder
                            .bel_xy(
                                bels::GCLK_TEST_BUF_CMT[i],
                                "GCLK_TEST_BUFE3",
                                0,
                                if i < 18 { i } else { i + 1 },
                            )
                            .pins_name_only(&["CLK_IN", "CLK_OUT"]),
                    );
                }
                for i in 0..24 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::BUFGCE[i], "BUFGCE", 0, i)
                            .pins_name_only(&["CLK_OUT"])
                            .pin_name_only("CLK_IN", usize::from(matches!(i, 5 | 11 | 17 | 23)))
                            .extra_wire(
                                "CLK_IN_MUX_HROUTE",
                                &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = i * 2 + 1)],
                            )
                            .extra_wire(
                                "CLK_IN_MUX_PLL_CKINT",
                                &[format!(
                                    "CLK_CMT_MUX_3TO1_{ii}_CLK_OUT",
                                    ii = i % 3 + i / 3 * 5
                                )],
                            )
                            .extra_wire(
                                "CLK_IN_MUX_TEST",
                                &[format!("CLK_CMT_MUX_4TO1_{i}_CLK_OUT")],
                            )
                            .extra_int_in("CLK_IN_CKINT", &[format!("CLK_INT{i}")]),
                    );
                }
                for i in 0..8 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::BUFGCTRL[i], "BUFGCTRL", 0, i)
                            .pins_name_only(&["CLK_I0", "CLK_I1", "CLK_OUT"]),
                    );
                }
                for i in 0..4 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::BUFGCE_DIV[i], "BUFGCE_DIV", 0, i)
                            .pins_name_only(&["CLK_IN", "CLK_OUT"]),
                    );
                }
                for i in 0..2 {
                    if !is_xp {
                        let mut bel = self
                            .builder
                            .bel_xy(bels::PLL[i], "PLL", 0, i)
                            .pins_name_only(&[
                                "CLKOUT0",
                                "CLKOUT0B",
                                "CLKOUT1",
                                "CLKOUT1B",
                                "CLKFBOUT",
                                "TMUXOUT",
                                "CLKOUTPHY_P",
                                "CLKIN",
                                "CLKFBIN",
                            ])
                            .extra_wire(
                                "CLKIN_MUX_MMCM",
                                &[format!("CLK_CMT_MUX_4TO1_{ii}_CLK_OUT", ii = 24 + i)],
                            )
                            .extra_wire(
                                "CLKIN_MUX_HDISTR",
                                &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = 60 + i * 3)],
                            )
                            .extra_wire(
                                "CLKIN_MUX_HROUTE",
                                &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = 61 + i * 3)],
                            )
                            .extra_wire(
                                "CLKIN_MUX_BUFCE_ROW_DLY",
                                &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = 62 + i * 3)],
                            );
                        if is_hbm {
                            // the muxes are repurposed for HBM reference
                            bel = bel.pin_name_only("CLKFBIN", 1);
                        } else {
                            bel = bel
                                .extra_wire(
                                    "CLKFBIN_MUX_HDISTR",
                                    &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = 56 + i * 2)],
                                )
                                .extra_wire(
                                    "CLKFBIN_MUX_BUFCE_ROW_DLY",
                                    &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = 57 + i * 2)],
                                );
                        }
                        bels.push(bel);
                    } else {
                        let bel = self
                            .builder
                            .bel_xy(bels::PLLXP[i], "PLLXP", 0, i)
                            .pins_name_only(&[
                                "CLKOUT0",
                                "CLKOUT0B",
                                "CLKOUT1",
                                "CLKOUT1B",
                                "CLKFBOUT",
                                "TMUXOUT",
                                "CLKOUTPHY_P",
                                "CLKIN",
                                "CLKFBIN",
                                "LOCKED_DMC",
                            ])
                            .pin_name_only("CLKOUTPHY_DMCEN", 1)
                            .pin_name_only("RST_DMC", 1)
                            .extra_wire(
                                "CLKIN_MUX_MMCM",
                                &[format!("CLK_CMT_MUX_4TO1_{ii}_CLK_OUT", ii = 24 + i)],
                            )
                            .extra_wire(
                                "CLKIN_MUX_HDISTR",
                                &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = 60 + i * 3)],
                            )
                            .extra_wire(
                                "CLKIN_MUX_HROUTE",
                                &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = 61 + i * 3)],
                            )
                            .extra_wire(
                                "CLKIN_MUX_BUFCE_ROW_DLY",
                                &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = 62 + i * 3)],
                            )
                            .extra_wire(
                                "CLKFBIN_MUX_HDISTR",
                                &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = 56 + i * 2)],
                            )
                            .extra_wire(
                                "CLKFBIN_MUX_BUFCE_ROW_DLY",
                                &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = 57 + i * 2)],
                            );
                        bels.push(bel);
                    }
                }
                bels.push(
                    self.builder
                        .bel_xy(bels::MMCM, "MMCM", 0, 0)
                        .pins_name_only(&[
                            "CLKOUT0",
                            "CLKOUT0B",
                            "CLKOUT1",
                            "CLKOUT1B",
                            "CLKOUT2",
                            "CLKOUT2B",
                            "CLKOUT3",
                            "CLKOUT3B",
                            "CLKOUT4",
                            "CLKOUT5",
                            "CLKOUT6",
                            "CLKFBOUT",
                            "CLKFBOUTB",
                            "TMUXOUT",
                            "CLKIN1",
                            "CLKIN2",
                            "CLKFBIN",
                        ])
                        .extra_wire("CLKFBIN_MUX_HDISTR", &["CLK_CMT_MUX_24_ENC_48_CLK_OUT"])
                        .extra_wire(
                            "CLKFBIN_MUX_BUFCE_ROW_DLY",
                            &["CLK_CMT_MUX_24_ENC_49_CLK_OUT"],
                        )
                        .extra_wire("CLKFBIN_MUX_DUMMY0", &["VCC_WIRE51"])
                        .extra_wire("CLKFBIN_MUX_DUMMY1", &["VCC_WIRE52"])
                        .extra_wire("CLKIN1_MUX_HDISTR", &["CLK_CMT_MUX_24_ENC_50_CLK_OUT"])
                        .extra_wire("CLKIN1_MUX_HROUTE", &["CLK_CMT_MUX_24_ENC_51_CLK_OUT"])
                        .extra_wire(
                            "CLKIN1_MUX_BUFCE_ROW_DLY",
                            &["CLK_CMT_MUX_24_ENC_52_CLK_OUT"],
                        )
                        .extra_wire("CLKIN1_MUX_DUMMY0", &["GND_WIRE0"])
                        .extra_wire("CLKIN2_MUX_HDISTR", &["CLK_CMT_MUX_24_ENC_53_CLK_OUT"])
                        .extra_wire("CLKIN2_MUX_HROUTE", &["CLK_CMT_MUX_24_ENC_54_CLK_OUT"])
                        .extra_wire(
                            "CLKIN2_MUX_BUFCE_ROW_DLY",
                            &["CLK_CMT_MUX_24_ENC_55_CLK_OUT"],
                        )
                        .extra_wire("CLKIN2_MUX_DUMMY0", &["GND_WIRE1"]),
                );
                bels.push(
                    self.builder
                        .bel_xy(bels::ABUS_SWITCH_CMT, "ABUS_SWITCH", 0, 0),
                );
                if is_hbm {
                    for i in 0..2 {
                        bels.push(
                            self.builder
                                .bel_xy(bels::HBM_REF_CLK[i], "HBM_REF_CLK", 0, i)
                                .pins_name_only(&["REF_CLK"])
                                .extra_wire(
                                    "REF_CLK_MUX_HDISTR",
                                    &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = 56 + i * 2)],
                                )
                                .extra_wire(
                                    "REF_CLK_MUX_BUFCE_ROW_DLY",
                                    &[format!("CLK_CMT_MUX_24_ENC_{ii}_CLK_OUT", ii = 57 + i * 2)],
                                ),
                        );
                    }
                }
                let mut bel = self
                    .builder
                    .bel_virtual(if is_xp { bels::CMTXP } else { bels::CMT });
                if !is_xp {
                    for i in 0..4 {
                        bel = bel.extra_wire(format!("CCIO{i}"), &[format!("IOB2CLK_CCIO{i}")]);
                    }
                    for i in 0..8 {
                        bel = bel.extra_wire(
                            format!("FIFO_WRCLK{i}"),
                            &[format!("PHY2RCLK_SS_DIVCLK_{j}_{k}", j = i / 2, k = i % 2)],
                        );
                    }
                } else {
                    for i in 0..4 {
                        bel = bel
                            .extra_wire(format!("CCIO_BOT{i}"), &[format!("CLK_CCIO_BOT{i}")])
                            .extra_wire(format!("CCIO_MID{i}"), &[format!("CLK_CCIO_MID{i}")])
                            .extra_wire(format!("CCIO_TOP{i}"), &[format!("CLK_CCIO_TOP{i}")]);
                    }
                }
                for i in 0..24 {
                    let dummy_base = [
                        0, 3, 36, 53, 56, 59, 62, 65, 68, 71, 6, 9, 12, 15, 18, 21, 24, 27, 30, 33,
                        39, 42, 45, 48,
                    ][i];
                    bel = bel
                        .extra_wire(format!("VDISTR{i}_B"), &[format!("CLK_VDISTR_BOT{i}")])
                        .extra_wire(format!("VDISTR{i}_T"), &[format!("CLK_VDISTR_TOP{i}")])
                        .extra_wire(format!("HDISTR{i}_L"), &[format!("CLK_HDISTR_0_{i}")])
                        .extra_wire(
                            format!("HDISTR{i}_R"),
                            &[format!("CLK_CMT_DRVR_TRI_{ii}_CLK_OUT_B", ii = i * 4)],
                        )
                        .extra_wire(
                            format!("HDISTR{i}_L_MUX"),
                            &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 1 + i * 8)],
                        )
                        .extra_wire(
                            format!("HDISTR{i}_R_MUX"),
                            &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = i * 8)],
                        )
                        .extra_wire(
                            format!("HDISTR{i}_OUT_MUX"),
                            &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 4 + i * 8)],
                        )
                        .extra_wire(
                            format!("VDISTR{i}_B_MUX"),
                            &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 6 + i * 8)],
                        )
                        .extra_wire(
                            format!("VDISTR{i}_T_MUX"),
                            &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 7 + i * 8)],
                        )
                        .extra_wire(
                            format!("OUT_MUX{i}"),
                            &[format!("CLK_CMT_MUX_16_ENC_{i}_CLK_OUT")],
                        )
                        .extra_wire(
                            format!("OUT_MUX{i}_DUMMY0"),
                            &[format!("VCC_WIRE{dummy_base}")],
                        )
                        .extra_wire(
                            format!("OUT_MUX{i}_DUMMY1"),
                            &[format!("VCC_WIRE{ii}", ii = dummy_base + 1)],
                        )
                        .extra_wire(
                            format!("OUT_MUX{i}_DUMMY2"),
                            &[format!("VCC_WIRE{ii}", ii = dummy_base + 2)],
                        );
                    if side == Dir::W {
                        bel = bel
                            .extra_wire(format!("HROUTE{i}_L"), &[format!("CLK_HROUTE_0_{i}")])
                            .extra_wire(format!("HROUTE{i}_R"), &[format!("CLK_HROUTE_1_{i}")])
                            .extra_wire(
                                format!("HROUTE{i}_L_MUX"),
                                &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 3 + i * 8)],
                            )
                            .extra_wire(
                                format!("HROUTE{i}_R_MUX"),
                                &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 2 + i * 8)],
                            );
                    } else {
                        bel = bel
                            .extra_wire(format!("HROUTE{i}_L"), &[format!("CLK_HROUTE_1_{i}")])
                            .extra_wire(format!("HROUTE{i}_R"), &[format!("CLK_HROUTE_0_{i}")])
                            .extra_wire(
                                format!("HROUTE{i}_L_MUX"),
                                &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 2 + i * 8)],
                            )
                            .extra_wire(
                                format!("HROUTE{i}_R_MUX"),
                                &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 3 + i * 8)],
                            );
                    }
                }
                bels.push(bel);
                bels.push(
                    self.builder
                        .bel_virtual(bels::VCC_CMT)
                        .extra_wire("VCC", &["VCC_WIRE"]),
                );
                let mut xn = self
                    .builder
                    .xnode(tslots::CMT, kind, naming, xy)
                    .num_tiles(60)
                    .ref_xlat(
                        int_xy.delta(0, 30),
                        if side == Dir::W {
                            &[Some(30), None, None, None]
                        } else {
                            &[None, Some(30), None, None]
                        },
                        rclk_int,
                    );
                for i in 0..60 {
                    xn = xn
                        .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), side, i)
                        .ref_single(
                            int_xy.delta(if side == Dir::W { -1 } else { 1 }, (i + i / 30) as i32),
                            i,
                            intf,
                        )
                }
                xn.bels(bels).extract();
            }
        }
    }

    fn fill_tiles_xiphy(&mut self) {
        for (tkn, side) in [("XIPHY_BYTE_L", Dir::W), ("XIPHY_BYTE_RIGHT", Dir::E)] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let int_xy = self.builder.walk_to_int(xy, !side, false).unwrap();
                let intf = self.builder.ndb.get_tile_class_naming(if side == Dir::W {
                    "INTF.W.IO"
                } else {
                    "INTF.E.IO"
                });
                let mut bels = vec![];
                for i in 0..13 {
                    let mut bel = self
                        .builder
                        .bel_xy(bels::BITSLICE[i], "BITSLICE_RX_TX", 0, i)
                        .pins_name_only(&[
                            "TX_CLK",
                            "TX_OCLK",
                            "TX_DIV2_CLK",
                            "TX_DIV4_CLK",
                            "TX_DDR_CLK",
                            "TX_CTRL_CLK",
                            "TX_CTRL_CE",
                            "TX_CTRL_INC",
                            "TX_CTRL_LD",
                            "TX_OCLKDIV",
                            "TX_TBYTE_IN",
                            "TX_WL_TRAIN",
                            "TX_MUX_360_N_SEL",
                            "TX_MUX_360_P_SEL",
                            "TX_MUX_720_P0_SEL",
                            "TX_MUX_720_P1_SEL",
                            "TX_MUX_720_P2_SEL",
                            "TX_MUX_720_P3_SEL",
                            "TX_VTC_READY",
                            "TX_TOGGLE_DIV2_SEL",
                            "TX_BS_RESET",
                            "TX_REGRST",
                            "TX_RST",
                            "TX_Q",
                            "RX_CLK_C",
                            "RX_CLK_C_B",
                            "RX_CLK_P",
                            "RX_CLK_N",
                            "RX_CTRL_CLK",
                            "RX_CTRL_CE",
                            "RX_CTRL_INC",
                            "RX_CTRL_LD",
                            "RX_RST",
                            "RX_CLKDIV",
                            "RX_DCC0",
                            "RX_DCC1",
                            "RX_DCC2",
                            "RX_DCC3",
                            "RX_VTC_READY",
                            "RX_RESET",
                            "RX_BS_RESET",
                            "RX_DQS_OUT",
                            "TX2RX_CASC_IN",
                            "TX2RX_CASC_OUT",
                            "RX2TX_CASC_RETURN_IN",
                            "PHY2CLB_FIFO_WRCLK",
                            "CLB2PHY_FIFO_CLK",
                            "CTL2BS_FIFO_BYPASS",
                            "CTL2BS_RX_RECALIBRATE_EN",
                            "CTL2BS_TX_DDR_PHASE_SEL",
                            "CTL2BS_DYNAMIC_MODE_EN",
                            "BS2CTL_IDELAY_DELAY_FORMAT",
                            "BS2CTL_ODELAY_DELAY_FORMAT",
                            "BS2CTL_TX_DDR_PHASE_SEL",
                            "BS2CTL_RX_P0_DQ_OUT",
                            "BS2CTL_RX_N0_DQ_OUT",
                            "BS2CTL_RX_DDR_EN_DQS",
                        ])
                        .pin_name_only("RX_CLK", 1)
                        .pin_name_only("RX_D", 1)
                        .extra_wire("DYN_DCI_OUT", &[format!("PHY2IOB_ODT_OUT_BYTE{i}")])
                        .extra_int_in(
                            "DYN_DCI_OUT_INT",
                            &[if i < 6 {
                                format!("CLB2PHY_ODT_LOW{i}")
                            } else {
                                format!("CLB2PHY_ODT_UPP{ii}", ii = i - 6)
                            }],
                        );
                    for i in 0..18 {
                        bel = bel.pins_name_only(&[
                            format!("BS2CTL_IDELAY_FIXED_DLY_RATIO{i}"),
                            format!("BS2CTL_ODELAY_FIXED_DLY_RATIO{i}"),
                        ]);
                    }
                    for i in 0..9 {
                        bel = bel.pins_name_only(&[
                            format!("BS2CTL_RX_CNTVALUEOUT{i}"),
                            format!("BS2CTL_TX_CNTVALUEOUT{i}"),
                            format!("RX_CTRL_DLY{i}"),
                            format!("TX_CTRL_DLY{i}"),
                        ]);
                    }
                    bels.push(bel);
                }
                for i in 0..2 {
                    let mut bel = self
                        .builder
                        .bel_xy(bels::BITSLICE_T[i], "BITSLICE_TX", 0, i)
                        .pins_name_only(&[
                            "CLK",
                            "DIV2_CLK",
                            "DIV4_CLK",
                            "DDR_CLK",
                            "CTRL_CLK",
                            "CTRL_CE",
                            "CTRL_INC",
                            "CTRL_LD",
                            "TX_MUX_360_N_SEL",
                            "TX_MUX_360_P_SEL",
                            "TX_MUX_720_P0_SEL",
                            "TX_MUX_720_P1_SEL",
                            "TX_MUX_720_P2_SEL",
                            "TX_MUX_720_P3_SEL",
                            "TOGGLE_DIV2_SEL",
                            "D0",
                            "D1",
                            "D2",
                            "D3",
                            "D4",
                            "D5",
                            "D6",
                            "D7",
                            "Q",
                            "RST",
                            "REGRST",
                            "BS_RESET",
                            "CDATAIN0",
                            "CDATAIN1",
                            "CDATAOUT",
                            "CTL2BS_TX_DDR_PHASE_SEL",
                            "CTL2BS_DYNAMIC_MODE_EN",
                            "BS2CTL_TX_DDR_PHASE_SEL",
                            "FORCE_OE_B",
                            "VTC_READY",
                        ]);
                    for i in 0..9 {
                        bel = bel.pins_name_only(&[
                            format!("BS2CTL_CNTVALUEOUT{i}"),
                            format!("CTRL_DLY{i}"),
                        ]);
                    }
                    bels.push(bel);
                }
                for i in 0..2 {
                    let mut bel = self
                        .builder
                        .bel_xy(bels::BITSLICE_CONTROL[i], "BITSLICE_CONTROL", 0, i)
                        .pins_name_only(&[
                            "PDQS_GT_IN",
                            "NDQS_GT_IN",
                            "PDQS_GT_OUT",
                            "NDQS_GT_OUT",
                            "FORCE_OE_B",
                            "PLL_CLK",
                            "PLL_CLK_EN",
                            "REFCLK_DFD",
                            "CLK_TO_EXT_SOUTH",
                            "CLK_TO_EXT_NORTH",
                            "CLB2PHY_CTRL_RST",
                            "LOCAL_DIV_CLK",
                            "BS_RESET_TRI",
                            "TRISTATE_ODELAY_CE_OUT",
                            "TRISTATE_ODELAY_INC_OUT",
                            "TRISTATE_ODELAY_LD_OUT",
                            "TRISTATE_VTC_READY",
                            "SCAN_INT",
                            "RIU2CLB_VALID",
                            "CLK_STOP",
                            "CLK_FROM_EXT",
                        ])
                        .pin_name_only("CLK_FROM_EXT", 1);
                    for i in 0..7 {
                        bel = bel.pins_name_only(&[
                            format!("RX_DCC{i:02}_0"),
                            format!("RX_DCC{i:02}_1"),
                            format!("RX_DCC{i:02}_2"),
                            format!("RX_DCC{i:02}_3"),
                            format!("RX_PDQ{i}_IN"),
                            format!("RX_NDQ{i}_IN"),
                            format!("IDELAY_CTRL_CLK{i}"),
                            format!("IDELAY_CE_OUT{i}"),
                            format!("IDELAY_INC_OUT{i}"),
                            format!("IDELAY_LD_OUT{i}"),
                            format!("FIXED_IDELAY{i:02}"),
                            format!("ODELAY_CE_OUT{i}"),
                            format!("ODELAY_INC_OUT{i}"),
                            format!("ODELAY_LD_OUT{i}"),
                            format!("FIXED_ODELAY{i:02}"),
                            format!("VTC_READY_IDELAY{i:02}"),
                            format!("VTC_READY_ODELAY{i:02}"),
                            format!("WL_TRAIN{i}"),
                            format!("DYN_DCI_OUT{i}"),
                            format!("DQS_IN{i}"),
                            format!("RX_BS_RESET{i}"),
                            format!("TX_BS_RESET{i}"),
                            format!("PDQS_OUT{i}"),
                            format!("NDQS_OUT{i}"),
                            format!("REFCLK_EN{i}"),
                            format!("IFIFO_BYPASS{i}"),
                            format!("BS2CTL_RIU_BS_DQS_EN{i}"),
                        ]);
                        for j in 0..9 {
                            bel = bel.pins_name_only(&[
                                format!("IDELAY{i:02}_IN{j}"),
                                format!("IDELAY{i:02}_OUT{j}"),
                                format!("ODELAY{i:02}_IN{j}"),
                                format!("ODELAY{i:02}_OUT{j}"),
                            ]);
                        }
                        for j in 0..18 {
                            bel = bel.pins_name_only(&[
                                format!("FIXDLYRATIO_IDELAY{i:02}_{j}"),
                                format!("FIXDLYRATIO_ODELAY{i:02}_{j}"),
                            ]);
                        }
                    }
                    for i in 0..8 {
                        bel = bel.pins_name_only(&[
                            format!("ODELAY_CTRL_CLK{i}"),
                            format!("DYNAMIC_MODE_EN{i}"),
                            format!("EN_DIV_DLY_OE{i}"),
                            format!("TOGGLE_DIV2_SEL{i}"),
                            format!("TX_DATA_PHASE{i}"),
                            format!("BS2CTL_RIU_TX_DATA_PHASE{i}"),
                            format!("DIV2_CLK_OUT{i}"),
                            format!("DIV_CLK_OUT{i}"),
                            format!("DDR_CLK_OUT{i}"),
                            format!("PH02_DIV2_360_{i}"),
                            format!("PH13_DIV2_360_{i}"),
                            format!("PH0_DIV_720_{i}"),
                            format!("PH1_DIV_720_{i}"),
                            format!("PH2_DIV_720_{i}"),
                            format!("PH3_DIV_720_{i}"),
                        ]);
                    }
                    for i in 0..9 {
                        bel = bel.pins_name_only(&[
                            format!("TRISTATE_ODELAY_IN{i}"),
                            format!("TRISTATE_ODELAY_OUT{i}"),
                        ]);
                    }
                    for i in 0..16 {
                        bel = bel.pins_name_only(&[format!("RIU2CLB_RD_DATA{i}")]);
                    }
                    bels.push(bel);
                }
                for i in 0..2 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::PLL_SELECT[i], "PLL_SELECT_SITE", 0, i)
                            .pins_name_only(&["REFCLK_DFD", "Z", "PLL_CLK_EN"])
                            .pin_name_only("D0", 1)
                            .pin_name_only("D1", 1),
                    );
                }
                let mut bel = self
                    .builder
                    .bel_xy(bels::RIU_OR0, "RIU_OR", 0, 0)
                    .pins_name_only(&["RIU_RD_VALID_LOW", "RIU_RD_VALID_UPP"]);
                for i in 0..16 {
                    bel = bel.pins_name_only(&[
                        format!("RIU_RD_DATA_LOW{i}"),
                        format!("RIU_RD_DATA_UPP{i}"),
                    ]);
                }
                bels.push(bel);
                let mut bel = self
                    .builder
                    .bel_xy(bels::XIPHY_FEEDTHROUGH0, "XIPHY_FEEDTHROUGH", 0, 0)
                    .pins_name_only(&[
                        "CLB2PHY_CTRL_RST_LOW_SMX",
                        "CLB2PHY_CTRL_RST_UPP_SMX",
                        "CLB2PHY_TRISTATE_ODELAY_RST_SMX0",
                        "CLB2PHY_TRISTATE_ODELAY_RST_SMX1",
                        "CLB2PHY_TXBIT_TRI_RST_SMX0",
                        "CLB2PHY_TXBIT_TRI_RST_SMX1",
                        "SCAN_INT_LOWER",
                        "SCAN_INT_UPPER",
                        "DIV_CLK_OUT_LOW",
                        "DIV_CLK_OUT_UPP",
                        "XIPHY_CLK_STOP_CTRL_LOW",
                        "XIPHY_CLK_STOP_CTRL_UPP",
                        "RCLK2PHY_CLKDR",
                        "RCLK2PHY_SHIFTDR",
                    ]);
                for i in 0..13 {
                    bel = bel.pins_name_only(&[
                        format!("CLB2PHY_TXBIT_RST_SMX{i}"),
                        format!("CLB2PHY_RXBIT_RST_SMX{i}"),
                        format!("CLB2PHY_FIFO_CLK_SMX{i}"),
                        format!("CLB2PHY_IDELAY_RST_SMX{i}"),
                        format!("CLB2PHY_ODELAY_RST_SMX{i}"),
                    ]);
                }
                for i in 0..6 {
                    bel = bel.pins_name_only(&[format!("CTL2BS_REFCLK_EN_LOW_SMX{i}")]);
                }
                for i in 0..7 {
                    bel = bel.pins_name_only(&[
                        format!("CTL2BS_REFCLK_EN_LOW{i}"),
                        format!("CTL2BS_REFCLK_EN_UPP{i}"),
                        format!("CTL2BS_REFCLK_EN_UPP_SMX{i}"),
                    ]);
                }
                bels.push(bel);
                let mut bel = self.builder.bel_virtual(bels::XIPHY_BYTE);
                for i in 0..6 {
                    bel = bel.extra_wire(format!("XIPHY_CLK{i}"), &[format!("GCLK_FT0_{i}")]);
                }
                bels.push(bel);
                let mut xn = self
                    .builder
                    .xnode(tslots::BEL, "XIPHY", "XIPHY", xy)
                    .num_tiles(15);
                for i in 0..15 {
                    xn = xn
                        .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), side, i)
                        .ref_single(
                            int_xy.delta(if side == Dir::W { -1 } else { 1 }, (i + i / 30) as i32),
                            i,
                            intf,
                        )
                }
                xn.bels(bels).extract();
            }
        }

        for (tkn, naming) in [
            ("RCLK_RCLK_XIPHY_INNER_FT", "RCLK_XIPHY_L"),
            ("RCLK_XIPHY_OUTER_RIGHT", "RCLK_XIPHY_R"),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let mut bel = self.builder.bel_virtual(bels::RCLK_XIPHY);
                for i in 0..24 {
                    if naming == "RCLK_XIPHY_L" {
                        bel = bel
                            .extra_wire(format!("HDISTR{i}_L"), &[format!("CLK_HDISTR_ZERO{i}")])
                            .extra_wire(format!("HDISTR{i}_R"), &[format!("CLK_HDISTR_ONE{i}")]);
                    } else {
                        bel = bel
                            .extra_wire(format!("HDISTR{i}_L"), &[format!("CLK_HDISTR_ONE{i}")])
                            .extra_wire(format!("HDISTR{i}_R"), &[format!("CLK_HDISTR_ZERO{i}")]);
                    }
                }
                for i in 0..6 {
                    bel = bel
                        .extra_wire(
                            format!("XIPHY_CLK{i}_B"),
                            &[format!("CLK_TO_XIPHY_BYTES_BOT{i}")],
                        )
                        .extra_wire(
                            format!("XIPHY_CLK{i}_T"),
                            &[format!("CLK_TO_XIPHY_BYTES_TOP{i}")],
                        );
                }
                let bel_vcc = self
                    .builder
                    .bel_virtual(bels::VCC_RCLK_XIPHY)
                    .extra_wire("VCC", &["VCC_WIRE"]);
                self.builder
                    .xnode(tslots::RCLK_BEL, "RCLK_XIPHY", naming, xy)
                    .num_tiles(0)
                    .bel(bel)
                    .bel(bel_vcc)
                    .extract();
            }
        }
    }

    fn fill_tiles_hpio(&mut self) {
        for (tkn, side) in [("HPIO_L", Dir::W), ("HPIO_RIGHT", Dir::E)] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let int_xy = self.builder.walk_to_int(xy, !side, false).unwrap();
                let intf = self.builder.ndb.get_tile_class_naming(if side == Dir::W {
                    "INTF.W.IO"
                } else {
                    "INTF.E.IO"
                });
                let mut bels = vec![];
                let mut is_alt = false;
                let mut is_nocfg = true;
                let mut is_noams = true;
                if let Some(wire) = self.builder.rd.wires.get("HPIO_IOBSNGL_19_TSDI_PIN") {
                    let tk = &self.builder.rd.tile_kinds[self.builder.rd.tiles[&xy].kind];
                    if tk.wires.contains_key(&wire) {
                        is_alt = true;
                    }
                }
                if let Some(wire) = self.builder.rd.wires.get("HPIO_IOBPAIR_26_TSDI_PIN") {
                    let tk = &self.builder.rd.tile_kinds[self.builder.rd.tiles[&xy].kind];
                    if tk.wires.contains_key(&wire) {
                        is_nocfg = false;
                    }
                }
                if let Some(wire) = self.builder.rd.wires.get("HPIO_IOBPAIR_26_SWITCH_OUT_PIN") {
                    let tk = &self.builder.rd.tile_kinds[self.builder.rd.tiles[&xy].kind];
                    if tk.wires.contains_key(&wire) {
                        is_noams = false;
                    }
                }
                for i in 0..26 {
                    let mut bel = self
                        .builder
                        .bel_xy(bels::HPIOB[i], "IOB", 0, i)
                        .pins_name_only(&[
                            "I",
                            "OUTB_B_IN",
                            "OUTB_B",
                            "TSTATE_IN",
                            "TSTATE_OUT",
                            "DOUT",
                            "IO",
                            "LVDS_TRUE",
                            "PAD_RES",
                            "O_B",
                            "TSTATEB",
                            "DYNAMIC_DCI_TS",
                            "VREF",
                        ])
                        .pin_name_only("SWITCH_OUT", if is_noams { 0 } else { 1 })
                        .pin_name_only("OP", 1)
                        .pin_name_only("TSP", 1)
                        .pin_name_only("TSDI", 1);
                    if matches!(i, 12 | 25) {
                        bel = bel
                            .pin_dummy("IO")
                            .pin_dummy("LVDS_TRUE")
                            .pin_dummy("OUTB_B_IN")
                            .pin_dummy("TSTATE_IN");
                        if !is_alt {
                            bel = bel.pin_name_only("TSDI", 0);
                        }
                    }
                    if is_nocfg {
                        bel = bel.pin_name_only("TSDI", 0);
                    }
                    bels.push(bel);
                }
                for i in 0..12 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::HPIOB_DIFF_IN[i], "HPIOBDIFFINBUF", 0, i)
                            .pins_name_only(&[
                                "LVDS_TRUE",
                                "LVDS_COMP",
                                "PAD_RES_0",
                                "PAD_RES_1",
                                "VREF",
                            ]),
                    );
                }
                for i in 0..12 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::HPIOB_DIFF_OUT[i], "HPIOBDIFFOUTBUF", 0, i)
                            .pins_name_only(&["AOUT", "BOUT", "O_B", "TSTATEB"]),
                    );
                }
                for i in 0..2 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::HPIOB_DCI[i], "HPIOB_DCI_SNGL", 0, i),
                    );
                }
                bels.push(
                    self.builder
                        .bel_xy(bels::HPIO_VREF, "HPIO_VREF_SITE", 0, 0)
                        .pins_name_only(&["VREF1", "VREF2"]),
                );
                bels.push(self.builder.bel_xy(bels::HPIO_BIAS, "BIAS", 0, 0));
                let naming = if is_noams {
                    "HPIO.NOAMS"
                } else if is_nocfg {
                    "HPIO.NOCFG"
                } else if is_alt {
                    "HPIO.ALTCFG"
                } else {
                    "HPIO"
                };
                let mut xn = self
                    .builder
                    .xnode(tslots::IOB, "HPIO", naming, xy)
                    .num_tiles(30);
                for i in 0..30 {
                    xn = xn
                        .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), side, i)
                        .ref_single(
                            int_xy.delta(if side == Dir::W { -1 } else { 1 }, (i + i / 30) as i32),
                            i,
                            intf,
                        )
                }
                xn.bels(bels).extract();
            }
        }

        for (tkn, side) in [("RCLK_HPIO_L", Dir::W), ("RCLK_HPIO_R", Dir::E)] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let int_xy = self
                    .builder
                    .walk_to_int(xy.delta(0, -30), !side, false)
                    .unwrap();
                let intf = self.builder.ndb.get_tile_class_naming(if side == Dir::W {
                    "INTF.W.IO"
                } else {
                    "INTF.E.IO"
                });
                let mut bels = vec![];
                for i in 0..7 {
                    bels.push(self.builder.bel_xy(
                        bels::ABUS_SWITCH_HPIO[i],
                        "ABUS_SWITCH",
                        if side == Dir::W {
                            i
                        } else {
                            [0, 6, 1, 3, 2, 4, 5][i]
                        },
                        0,
                    ));
                }
                bels.push(
                    self.builder
                        .bel_xy(bels::HPIO_ZMATCH, "HPIO_ZMATCH_BLK_HCLK", 0, 0),
                );
                bels.push(self.builder.bel_xy(bels::HPIO_PRBS, "HPIO_RCLK_PRBS", 0, 0));
                let mut xn = self
                    .builder
                    .xnode(tslots::RCLK_IOB, "RCLK_HPIO", "RCLK_HPIO", xy)
                    .num_tiles(60);
                for i in 0..60 {
                    xn = xn
                        .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), side, i)
                        .ref_single(
                            int_xy.delta(if side == Dir::W { -1 } else { 1 }, (i + i / 30) as i32),
                            i,
                            intf,
                        )
                }
                xn.bels(bels).extract();
            }
        }
    }

    fn fill_tiles_xp5io(&mut self) {
        if let Some(&xy) = self
            .builder
            .rd
            .tiles_by_kind_name("HSM_XP5IO_FT")
            .iter()
            .next()
        {
            let int_xy = self.builder.walk_to_int(xy, Dir::W, false).unwrap();
            let intf = self.builder.ndb.get_tile_class_naming("INTF.E.IO");

            let mut bels = vec![];
            let tile = &self.builder.rd.tiles[&xy];
            let tk = &self.builder.rd.tile_kinds[tile.kind];

            for i in 0..33 {
                let slot = self.builder.rd.slot_kinds.get("IOB").unwrap();
                let slot = TkSiteSlot::Xy(slot, 0, i as u8);
                let tksite = tk.sites.get(&slot).unwrap().1;
                let mut bel = self.builder.bel_xy(bels::XP5IOB[i], "IOB", 0, i);
                for pin in tksite.pins.keys() {
                    bel = bel.pin_name_only(pin, 0);
                }
                bels.push(bel);
            }

            for i in 0..11 {
                let slot = self.builder.rd.slot_kinds.get("XP5IO_VREF").unwrap();
                let slot = TkSiteSlot::Xy(slot, 0, i as u8);
                let tksite = tk.sites.get(&slot).unwrap().1;
                let mut bel = self.builder.bel_xy(bels::XP5IO_VREF[i], "XP5IO_VREF", 0, i);
                for pin in tksite.pins.keys() {
                    bel = bel.pin_name_only(pin, 0);
                }
                bels.push(bel);
            }

            for i in 0..11 {
                let slot = self.builder.rd.slot_kinds.get("X5PHY_LS").unwrap();
                let slot = TkSiteSlot::Xy(slot, 0, i as u8);
                let tksite = tk.sites.get(&slot).unwrap().1;
                let mut bel = self.builder.bel_xy(bels::X5PHY_LS[i], "X5PHY_LS", 0, i);
                for pin in tksite.pins.keys() {
                    let mut buf_cnt = 0;
                    if pin.starts_with("RTRIM_M2L_IN") && i == 0 {
                        buf_cnt = 1;
                    }
                    bel = bel.pin_name_only(pin, buf_cnt);
                }
                bels.push(bel);
            }

            for i in 0..11 {
                let slot = self.builder.rd.slot_kinds.get("X5PHY_HS").unwrap();
                let slot = TkSiteSlot::Xy(slot, 0, i as u8);
                let tksite = tk.sites.get(&slot).unwrap().1;
                let mut bel = self.builder.bel_xy(bels::X5PHY_HS[i], "X5PHY_HS", 0, i);
                for pin in tksite.pins.keys() {
                    bel = bel.pin_name_only(pin, 0);
                }
                bels.push(bel);
            }

            for i in 0..11 {
                let slot = self.builder.rd.slot_kinds.get("X5PHY_PLL_SELECT").unwrap();
                let slot = TkSiteSlot::Xy(slot, 0, i as u8);
                let tksite = tk.sites.get(&slot).unwrap().1;
                let mut bel =
                    self.builder
                        .bel_xy(bels::X5PHY_PLL_SELECT[i], "X5PHY_PLL_SELECT", 0, i);
                for pin in tksite.pins.keys() {
                    bel = bel.pin_name_only(pin, 0);
                }
                bels.push(bel);
            }

            let slot = self.builder.rd.slot_kinds.get("LPDDRMC").unwrap();
            let slot = TkSiteSlot::Xy(slot, 0, 0);
            let tksite = tk.sites.get(&slot).unwrap().1;
            let mut bel = self.builder.bel_xy(bels::LPDDRMC, "LPDDRMC", 0, 0);
            for pin in tksite.pins.keys() {
                if pin.starts_with("IF_DMC_CLB2PHY")
                    || pin.starts_with("IF_DMC2PHY")
                    || pin.starts_with("DMC2PHY")
                    || pin.starts_with("DMC_XPLL")
                    || pin.starts_with("PHY_DIV4_CLK")
                    || pin.starts_with("IF_XPIO_DFX_DFXCNTRL_DMC_IABUT")
                    || pin.starts_with("IF_XPIO_DCI_FABRIC_DMC_IABUT")
                    || pin.starts_with("IF_XPIO_MMCM_DMC_IABUT")
                    || pin.starts_with("IF_XPIO_MMCM_DMC_OABUT_XPIO_CCIO")
                    || matches!(
                        pin.as_str(),
                        "CLOCK_DR_OABUT"
                            | "RESET_TAP_OABUT"
                            | "SHIFT_DR_OABUT"
                            | "IJTAG_TDO_EXT"
                            | "IJTAG_TDO_IABUT"
                            | "UPDATE_DR_OABUT"
                            | "SELECT_DR_OABUT"
                            | "IJTAG_CLOCK_DR"
                            | "IJTAG_SHIFT_DR"
                            | "IJTAG_UPDATE_DR"
                            | "CAPTURE_DR_OABUT"
                            | "BSCAN_CFG2IOB_PUDC_B_IABUT"
                            | "BSCAN_GTS_USR_B_IABUT"
                            | "BSCAN_EXTEST_IABUT"
                            | "BSCAN_EXTEST_SMPL_IABUT"
                            | "PLL0_CLKOUTPHY_OABUT"
                            | "PLL1_CLKOUTPHY_OABUT"
                            | "IF_XPIO_MMCM_DMC_OABUT_XPIO_PHY_CLK1"
                            | "IF_XPIO_MMCM_DMC_OABUT_XPIO_PHY_CLK2"
                            | "IF_XPIO_MMCM_DMC_OABUT_XPIO_PHY_CLK3"
                            | "IF_XPIO_MMCM_DMC_OABUT_XPIO_PHY_CLK4"
                            | "IF_XPIO_MMCM_DMC_OABUT_XPIO_PHY_CLK5"
                            | "IF_XPIO_MMCM_DMC_OABUT_XPIO_PHY_CLK6"
                            | "IF_XPIO_MMCM_DMC_OABUT_XPIO_PHY_CLK7"
                            | "IF_XPIO_MMCM_DMC_OABUT_XPIO_PHY_CLK8"
                    )
                {
                    bel = bel.pin_name_only(pin, 0);
                }
            }
            for pin in [
                "PLL0_CLKOUTPHY_IABUT",
                "PLL1_CLKOUTPHY_IABUT",
                "XPLL0_DMC_LOCK",
                "XPLL1_DMC_LOCK",
            ] {
                bel = bel.pin_name_only(pin, 1);
            }
            bel = bel
                .extra_wire("NIBBLE0_CCIO_DUMMY", &["XP5IO_ROUTE_MUX2TO1_EN_0_Z"])
                .extra_wire("NIBBLE1_CCIO_DUMMY", &["XP5IO_ROUTE_MUX2TO1_EN_1_Z"])
                .extra_wire("NIBBLE2_CCIO_DUMMY", &["XP5IO_ROUTE_MUX2TO1_EN_3_Z"])
                .extra_wire("NIBBLE3_CCIO_DUMMY", &["XP5IO_ROUTE_MUX2TO1_EN_4_Z"])
                .extra_wire("NIBBLE7_CCIO_DUMMY", &["XP5IO_ROUTE_MUX2TO1_EN_8_Z"])
                .extra_wire("NIBBLE8_CCIO_DUMMY", &["XP5IO_ROUTE_MUX2TO1_EN_9_Z"])
                .extra_wire("NIBBLE9_CCIO_DUMMY", &["XP5IO_ROUTE_MUX2TO1_EN_10_Z"])
                // ???
                .extra_int_in("CFG2IOB_PUDC_B", &["CFG2IOB_PUDC_B"])
                .extra_int_out("CFG2IOB_PUDC_B_O", &["CFG2IOB_PUDC_B"])
                .extra_int_out("CAPTURE_DR_O", &["LPDDRMC_DMC_SITE_0_CAPTURE_DR"])
                .extra_int_out("SELECT_DR_O", &["LPDDRMC_DMC_SITE_0_SELECT_DR"])
                .extra_int_out("IJTAG_RESET_TAP_O", &["LPDDRMC_DMC_SITE_0_IJTAG_RESET_TAP"]);
            bels.push(bel);

            let slot = self.builder.rd.slot_kinds.get("XP5PIO_CMU_ANA").unwrap();
            let slot = TkSiteSlot::Xy(slot, 0, 0);
            let tksite = tk.sites.get(&slot).unwrap().1;
            let mut bel = self
                .builder
                .bel_xy(bels::XP5PIO_CMU_ANA, "XP5PIO_CMU_ANA", 0, 0);
            for pin in tksite.pins.keys() {
                bel = bel.pin_name_only(pin, 0);
            }
            bels.push(bel);

            let slot = self
                .builder
                .rd
                .slot_kinds
                .get("XP5PIO_CMU_DIG_TOP")
                .unwrap();
            let slot = TkSiteSlot::Xy(slot, 0, 0);
            let tksite = tk.sites.get(&slot).unwrap().1;
            let mut bel = self
                .builder
                .bel_xy(bels::XP5PIO_CMU_DIG_TOP, "XP5PIO_CMU_DIG_TOP", 0, 0);
            for pin in tksite.pins.keys() {
                bel = bel.pin_name_only(pin, 0);
            }
            bels.push(bel);

            for i in 0..2 {
                bels.push(
                    self.builder
                        .bel_xy(bels::ABUS_SWITCH_XP5IO[i], "ABUS_SWITCH", 0, i)
                        .raw_tile(1),
                )
            }

            bels.push(
                self.builder
                    .bel_virtual(bels::VCC_XP5IO)
                    .extra_wire("VCC", &["VCC_WIRE"]),
            );

            let mut xn = self
                .builder
                .xnode(tslots::BEL, "XP5IO", "XP5IO", xy)
                .raw_tile(xy.delta(-2, 30))
                .num_tiles(60);
            for i in 0..60 {
                xn = xn
                    .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), Dir::E, i)
                    .ref_single(int_xy.delta(1, (i + i / 30) as i32), i, intf)
            }
            xn.bels(bels).extract();

            let tcls = self.builder.db.tile_classes.get_mut("XP5IO").unwrap().1;
            let BelInfo::Bel(ref mut bel) = tcls.bels[bels::LPDDRMC] else {
                unreachable!()
            };
            let tncls = self
                .builder
                .ndb
                .tile_class_namings
                .get_mut("XP5IO")
                .unwrap()
                .1;
            let BelNaming::Bel(ref mut beln) = tncls.bels[bels::LPDDRMC] else {
                unreachable!()
            };
            for (pin, wire) in [
                ("CFG2IOB_PUDC_B", "IMUX.IMUX.27.DELAY"),
                ("IJTAG_RESET_TAP", "IMUX.IMUX.28.DELAY"),
                ("CAPTURE_DR", "IMUX.IMUX.30.DELAY"),
                ("SELECT_DR", "IMUX.IMUX.31.DELAY"),
            ] {
                let wire = TileWireCoord {
                    cell: CellSlotId::from_idx(34),
                    wire: self.builder.db.wires.get(wire).unwrap().0,
                };
                let bpin = bel.pins.get_mut(pin).unwrap();
                bpin.wires = BTreeSet::from_iter([wire]);
                let bnpin = beln.pins.get_mut(pin).unwrap();
                bnpin.is_intf = false;
            }
        }
    }

    fn fill_tiles_rclk(&mut self) {
        for (node, tkn) in [
            ("RCLK_HROUTE_SPLITTER.HARD", "PCIE4_PCIE4_FT"),
            ("RCLK_HROUTE_SPLITTER.HARD", "PCIE4C_PCIE4C_FT"),
            ("RCLK_HROUTE_SPLITTER.HARD", "PCIE4CE_PCIE4CE_FT"),
            ("RCLK_HROUTE_SPLITTER.HARD", "CMAC"),
            ("RCLK_HROUTE_SPLITTER.HARD", "ILKN_ILKN_FT"),
            ("RCLK_HROUTE_SPLITTER.HARD", "DFE_DFE_TILEA_FT"),
            ("RCLK_HROUTE_SPLITTER.HARD", "DFE_DFE_TILEB_FT"),
            ("RCLK_HROUTE_SPLITTER.HARD", "DFE_DFE_TILEE_FT"),
            ("RCLK_HROUTE_SPLITTER.HARD", "DFE_DFE_TILEG_FT"),
            ("RCLK_HROUTE_SPLITTER.HARD", "CFG_CONFIG"),
            ("RCLK_HROUTE_SPLITTER.HARD", "CSEC_CONFIG_FT"),
            ("RCLK_HROUTE_SPLITTER.HARD", "CSEC_CONFIG_VER2_FT"),
            ("RCLK_HROUTE_SPLITTER.HARD", "RCLK_AMS_CFGIO"),
            ("RCLK_HROUTE_SPLITTER.CLE", "RCLK_CLEM_CLKBUF_L"),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let mut bel = self.builder.bel_virtual(bels::RCLK_HROUTE_SPLITTER);
                for i in 0..24 {
                    bel = bel
                        .extra_wire(format!("HROUTE{i}_L"), &[format!("CLK_HROUTE_L{i}")])
                        .extra_wire(format!("HROUTE{i}_R"), &[format!("CLK_HROUTE_R{i}")]);
                }
                let bel_vcc = self
                    .builder
                    .bel_virtual(bels::VCC_RCLK_HROUTE_SPLITTER)
                    .extra_wire("VCC", &["VCC_WIRE"]);
                self.builder
                    .xnode(tslots::RCLK_SPLITTER, node, "RCLK_HROUTE_SPLITTER", xy)
                    .num_tiles(0)
                    .bel(bel)
                    .bel(bel_vcc)
                    .extract();
            }
        }

        if let Some(&xy) = self
            .builder
            .rd
            .tiles_by_kind_name("RCLK_DSP_INTF_CLKBUF_L")
            .iter()
            .next()
        {
            let mut bel = self.builder.bel_virtual(bels::RCLK_SPLITTER);
            for i in 0..24 {
                bel = bel
                    .extra_wire(format!("HDISTR{i}_L"), &[format!("CLK_HDISTR_L{i}")])
                    .extra_wire(format!("HDISTR{i}_R"), &[format!("CLK_HDISTR_R{i}")])
                    .extra_wire(format!("HROUTE{i}_L"), &[format!("CLK_HROUTE_L{i}")])
                    .extra_wire(format!("HROUTE{i}_R"), &[format!("CLK_HROUTE_R{i}")]);
            }
            let bel_vcc = self
                .builder
                .bel_virtual(bels::VCC_RCLK_SPLITTER)
                .extra_wire("VCC", &["VCC_WIRE"]);
            self.builder
                .xnode(tslots::RCLK_SPLITTER, "RCLK_SPLITTER", "RCLK_SPLITTER", xy)
                .num_tiles(0)
                .bel(bel)
                .bel(bel_vcc)
                .extract();
        }

        for (node, tkn) in [
            ("RCLK_V_SINGLE.CLE", "RCLK_CLEL_L_L"),
            ("RCLK_V_SINGLE.CLE", "RCLK_CLEL_L_R"),
            ("RCLK_V_SINGLE.CLE", "RCLK_CLEM_L"),
            ("RCLK_V_SINGLE.CLE", "RCLK_CLEM_DMC_L"),
            ("RCLK_V_SINGLE.CLE", "RCLK_CLEM_R"),
            ("RCLK_V_SINGLE.LAG", "RCLK_LAG_L"),
            ("RCLK_V_SINGLE.LAG", "RCLK_LAG_R"),
            ("RCLK_V_SINGLE.LAG", "RCLK_LAG_DMC_L"),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let is_alt = self.dev_naming.rclk_alt_pins[tkn];
                let rclk_int = self.builder.ndb.get_tile_class_naming("RCLK_INT");
                let int_xy = xy.delta(if tkn.starts_with("RCLK_LAG") { 2 } else { 1 }, 0);
                let bels = vec![
                    self.builder
                        .bel_xy(bels::BUFCE_ROW_RCLK0, "BUFCE_ROW_FSR", 0, 0)
                        .pins_name_only(&["CLK_IN", "CLK_OUT", "CLK_OUT_OPT_DLY"])
                        .extra_wire("VDISTR_B", &["CLK_VDISTR_BOT"])
                        .extra_wire("VDISTR_T", &["CLK_VDISTR_TOP"])
                        .extra_wire("VROUTE_B", &["CLK_VROUTE_BOT"])
                        .extra_wire("VROUTE_T", &["CLK_VROUTE_TOP"])
                        .extra_wire("HROUTE", &["CLK_HROUTE_CORE_OPT"])
                        .extra_wire("VDISTR_B_MUX", &["CLK_CMT_MUX_3TO1_0_CLK_OUT"])
                        .extra_wire("VDISTR_T_MUX", &["CLK_CMT_MUX_3TO1_1_CLK_OUT"])
                        .extra_wire("VROUTE_B_MUX", &["CLK_CMT_MUX_3TO1_2_CLK_OUT"])
                        .extra_wire("VROUTE_T_MUX", &["CLK_CMT_MUX_3TO1_3_CLK_OUT"])
                        .extra_wire("HROUTE_MUX", &["CLK_CMT_MUX_2TO1_1_CLK_OUT"])
                        .extra_wire(
                            "VDISTR_B_BUF",
                            &["CLK_CMT_DRVR_TRI_ESD_0_CLK_OUT_SCHMITT_B"],
                        )
                        .extra_wire(
                            "VDISTR_T_BUF",
                            &["CLK_CMT_DRVR_TRI_ESD_1_CLK_OUT_SCHMITT_B"],
                        )
                        .extra_wire(
                            "VROUTE_B_BUF",
                            &["CLK_CMT_DRVR_TRI_ESD_2_CLK_OUT_SCHMITT_B"],
                        )
                        .extra_wire(
                            "VROUTE_T_BUF",
                            &["CLK_CMT_DRVR_TRI_ESD_3_CLK_OUT_SCHMITT_B"],
                        ),
                    self.builder
                        .bel_xy(bels::GCLK_TEST_BUF_RCLK0, "GCLK_TEST_BUFE3", 0, 0)
                        .pin_name_only("CLK_OUT", 0)
                        .pin_name_only("CLK_IN", usize::from(is_alt)),
                    self.builder
                        .bel_virtual(bels::VCC_RCLK_V)
                        .extra_wire("VCC", &["VCC_WIRE"]),
                ];
                self.builder
                    .xnode(
                        tslots::RCLK_V,
                        node,
                        if is_alt {
                            "RCLK_V_SINGLE.ALT"
                        } else {
                            "RCLK_V_SINGLE"
                        },
                        xy,
                    )
                    .ref_xlat(int_xy, &[Some(0), None, None, None], rclk_int)
                    .bels(bels)
                    .extract();
            }
        }

        for tkn in [
            "RCLK_DSP_INTF_L",
            "RCLK_DSP_INTF_R",
            "RCLK_RCLK_DSP_INTF_DC12_L_FT",
            "RCLK_RCLK_DSP_INTF_DC12_R_FT",
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let is_alt = self.dev_naming.rclk_alt_pins[tkn];
                let rclk_int = self.builder.ndb.get_tile_class_naming("RCLK_INT");
                let int_xy = xy.delta(-1, 0);
                let mut bels = vec![];
                for i in 0..2 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::BUFCE_ROW_RCLK[i], "BUFCE_ROW_FSR", i, 0)
                            .pins_name_only(&["CLK_IN", "CLK_OUT", "CLK_OUT_OPT_DLY"])
                            .extra_wire("VDISTR_B", &[format!("CLK_VDISTR_BOT{i}")])
                            .extra_wire("VDISTR_T", &[format!("CLK_VDISTR_TOP{i}")])
                            .extra_wire("VROUTE_B", &[format!("CLK_VROUTE_BOT{i}")])
                            .extra_wire("VROUTE_T", &[format!("CLK_VROUTE_TOP{i}")])
                            .extra_wire("HROUTE", &[format!("CLK_HROUTE_CORE_OPT{i}")])
                            .extra_wire(
                                "VDISTR_B_MUX",
                                &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4)],
                            )
                            .extra_wire(
                                "VDISTR_T_MUX",
                                &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4 + 1)],
                            )
                            .extra_wire(
                                "VROUTE_B_MUX",
                                &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4 + 2)],
                            )
                            .extra_wire(
                                "VROUTE_T_MUX",
                                &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4 + 3)],
                            )
                            .extra_wire(
                                "HROUTE_MUX",
                                &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = i * 2 + 1)],
                            )
                            .extra_wire(
                                "VDISTR_B_BUF",
                                &[format!(
                                    "CLK_CMT_DRVR_TRI_ESD_{ii}_CLK_OUT_SCHMITT_B",
                                    ii = i * 4
                                )],
                            )
                            .extra_wire(
                                "VDISTR_T_BUF",
                                &[format!(
                                    "CLK_CMT_DRVR_TRI_ESD_{ii}_CLK_OUT_SCHMITT_B",
                                    ii = i * 4 + 1
                                )],
                            )
                            .extra_wire(
                                "VROUTE_B_BUF",
                                &[format!(
                                    "CLK_CMT_DRVR_TRI_ESD_{ii}_CLK_OUT_SCHMITT_B",
                                    ii = i * 4 + 2
                                )],
                            )
                            .extra_wire(
                                "VROUTE_T_BUF",
                                &[format!(
                                    "CLK_CMT_DRVR_TRI_ESD_{ii}_CLK_OUT_SCHMITT_B",
                                    ii = i * 4 + 3
                                )],
                            ),
                    );
                }
                for i in 0..2 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::GCLK_TEST_BUF_RCLK[i], "GCLK_TEST_BUFE3", i, 0)
                            .pin_name_only("CLK_OUT", 0)
                            .pin_name_only("CLK_IN", usize::from(is_alt)),
                    );
                }
                bels.push(
                    self.builder
                        .bel_virtual(bels::VCC_RCLK_V)
                        .extra_wire("VCC", &["VCC_WIRE"]),
                );
                self.builder
                    .xnode(
                        tslots::RCLK_V,
                        "RCLK_V_DOUBLE.DSP",
                        if is_alt {
                            "RCLK_V_DOUBLE.ALT"
                        } else {
                            "RCLK_V_DOUBLE"
                        },
                        xy,
                    )
                    .ref_xlat(int_xy, &[None, Some(0), None, None], rclk_int)
                    .bels(bels)
                    .extract();
            }
        }

        for (node, tkn) in [
            ("RCLK_V_QUAD.BRAM", "RCLK_BRAM_INTF_L"),
            ("RCLK_V_QUAD.BRAM", "RCLK_BRAM_INTF_TD_L"),
            ("RCLK_V_QUAD.BRAM", "RCLK_BRAM_INTF_TD_R"),
            ("RCLK_V_QUAD.URAM", "RCLK_RCLK_URAM_INTF_L_FT"),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let is_alt = self.dev_naming.rclk_alt_pins[tkn];
                let is_uram = tkn == "RCLK_RCLK_URAM_INTF_L_FT";
                let rclk_int = self.builder.ndb.get_tile_class_naming("RCLK_INT");
                let int_xy = xy.delta(if is_uram { 3 } else { 2 }, 0);
                let mut bels = vec![];
                for i in 0..4 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::BUFCE_ROW_RCLK[i], "BUFCE_ROW_FSR", i, 0)
                            .pins_name_only(&["CLK_IN", "CLK_OUT", "CLK_OUT_OPT_DLY"])
                            .extra_wire("VDISTR_B", &[format!("CLK_VDISTR_BOT{i}")])
                            .extra_wire("VDISTR_T", &[format!("CLK_VDISTR_TOP{i}")])
                            .extra_wire("VROUTE_B", &[format!("CLK_VROUTE_BOT{i}")])
                            .extra_wire("VROUTE_T", &[format!("CLK_VROUTE_TOP{i}")])
                            .extra_wire("HROUTE", &[format!("CLK_HROUTE_CORE_OPT{i}")])
                            .extra_wire(
                                "VDISTR_B_MUX",
                                &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4)],
                            )
                            .extra_wire(
                                "VDISTR_T_MUX",
                                &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4 + 1)],
                            )
                            .extra_wire(
                                "VROUTE_B_MUX",
                                &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4 + 2)],
                            )
                            .extra_wire(
                                "VROUTE_T_MUX",
                                &[format!("CLK_CMT_MUX_3TO1_{ii}_CLK_OUT", ii = i * 4 + 3)],
                            )
                            .extra_wire(
                                "HROUTE_MUX",
                                &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = i * 2 + 1)],
                            )
                            .extra_wire(
                                "VDISTR_B_BUF",
                                &[format!(
                                    "CLK_CMT_DRVR_TRI_ESD_{ii}_CLK_OUT_SCHMITT_B",
                                    ii = i * 4
                                )],
                            )
                            .extra_wire(
                                "VDISTR_T_BUF",
                                &[format!(
                                    "CLK_CMT_DRVR_TRI_ESD_{ii}_CLK_OUT_SCHMITT_B",
                                    ii = i * 4 + 1
                                )],
                            )
                            .extra_wire(
                                "VROUTE_B_BUF",
                                &[format!(
                                    "CLK_CMT_DRVR_TRI_ESD_{ii}_CLK_OUT_SCHMITT_B",
                                    ii = i * 4 + 2
                                )],
                            )
                            .extra_wire(
                                "VROUTE_T_BUF",
                                &[format!(
                                    "CLK_CMT_DRVR_TRI_ESD_{ii}_CLK_OUT_SCHMITT_B",
                                    ii = i * 4 + 3
                                )],
                            ),
                    );
                }
                for i in 0..4 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::GCLK_TEST_BUF_RCLK[i], "GCLK_TEST_BUFE3", i, 0)
                            .pin_name_only("CLK_OUT", 0)
                            .pin_name_only("CLK_IN", usize::from(is_alt)),
                    );
                }
                for (i, x, y) in [(0, 0, 0), (1, 0, 1), (2, 1, 0)] {
                    bels.push(
                        self.builder
                            .bel_xy(bels::VBUS_SWITCH[i], "VBUS_SWITCH", x, y),
                    );
                }
                bels.push(
                    self.builder
                        .bel_virtual(bels::VCC_RCLK_V)
                        .extra_wire("VCC", &["VCC_WIRE"]),
                );
                self.builder
                    .xnode(
                        tslots::RCLK_V,
                        node,
                        &if is_alt {
                            format!("{node}.ALT")
                        } else {
                            node.to_string()
                        },
                        xy,
                    )
                    .ref_xlat(int_xy, &[Some(0), None, None, None], rclk_int)
                    .bels(bels)
                    .extract();
            }
        }
    }

    fn fill_tiles_gt(&mut self) {
        for (kind, naming, tkn, side) in [
            ("GTH", "GTH_L", "GTH_QUAD_LEFT", Dir::W),
            ("GTH", "GTH_R", "GTH_QUAD_RIGHT", Dir::E),
            ("GTY", "GTY_L", "GTY_L", Dir::W),
            ("GTY", "GTY_R", "GTY_R", Dir::E),
            ("GTF", "GTF_L", "GTFY_QUAD_LEFT_FT", Dir::W),
            ("GTF", "GTF_R", "GTFY_QUAD_RIGHT_FT", Dir::E),
            ("GTM", "GTM_L", "GTM_DUAL_LEFT_FT", Dir::W),
            ("GTM", "GTM_R", "GTM_DUAL_RIGHT_FT", Dir::E),
            ("HSADC", "HSADC_R", "HSADC_HSADC_RIGHT_FT", Dir::E),
            ("HSDAC", "HSDAC_R", "HSDAC_HSDAC_RIGHT_FT", Dir::E),
            ("RFADC", "RFADC_R", "RFADC_RFADC_RIGHT_FT", Dir::E),
            ("RFDAC", "RFDAC_R", "RFDAC_RFDAC_RIGHT_FT", Dir::E),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let int_xy = self.builder.walk_to_int(xy, !side, false).unwrap();
                let intf = self.builder.ndb.get_tile_class_naming(if side == Dir::W {
                    "INTF.W.GT"
                } else {
                    "INTF.E.GT"
                });
                let rclk_int = self.builder.ndb.get_tile_class_naming("RCLK_INT");
                let mut bels = vec![];
                for i in 0..24 {
                    let mut bel = self
                        .builder
                        .bel_xy(bels::BUFG_GT[i], "BUFG_GT", 0, i)
                        .pins_name_only(&["CLK_IN", "CLK_OUT", "CE", "RST_PRE_OPTINV"]);
                    if !kind.starts_with("GT") {
                        let bi = [
                            10, 43, 61, 64, 67, 70, 73, 76, 79, 13, 16, 19, 22, 25, 28, 31, 34, 37,
                            40, 46, 49, 52, 55, 58,
                        ][i];
                        bel = bel
                            .pin_name_only("DIV0", 1)
                            .pin_name_only("DIV1", 1)
                            .pin_name_only("DIV2", 1)
                            .extra_wire("DIV0_DUMMY", &[format!("GND_WIRE{bi}")])
                            .extra_wire("DIV1_DUMMY", &[format!("GND_WIRE{ii}", ii = bi + 1)])
                            .extra_wire("DIV2_DUMMY", &[format!("GND_WIRE{ii}", ii = bi + 2)]);
                    }
                    if kind.starts_with("GT") {
                        let bi = [
                            (0, 1, 12),
                            (27, 28, 29),
                            (43, 44, 46),
                            (47, 48, 49),
                            (50, 51, 52),
                            (53, 54, 55),
                            (57, 58, 59),
                            (60, 61, 62),
                            (63, 64, 65),
                            (66, 68, 69),
                            (23, 34, 45),
                            (56, 67, 70),
                            (71, 2, 3),
                            (4, 5, 6),
                            (7, 8, 9),
                            (10, 11, 13),
                            (14, 15, 16),
                            (17, 18, 19),
                            (20, 21, 22),
                            (24, 25, 26),
                            (30, 31, 32),
                            (33, 35, 36),
                            (37, 38, 39),
                            (40, 41, 42),
                        ][i];
                        bel = bel
                            .extra_wire("CE_MUX_DUMMY0", &[format!("VCC_WIRE{ii}", ii = bi.0)])
                            .extra_wire("CLK_IN_MUX_DUMMY0", &[format!("VCC_WIRE{ii}", ii = bi.1)])
                            .extra_wire("RST_MUX_DUMMY0", &[format!("VCC_WIRE{ii}", ii = bi.2)]);
                    } else {
                        let bi = [
                            (20, 21, 82),
                            (137, 138, 149),
                            (213, 214, 226),
                            (227, 228, 239),
                            (240, 241, 252),
                            (253, 254, 265),
                            (267, 268, 279),
                            (280, 281, 292),
                            (293, 294, 305),
                            (306, 318, 329),
                            (123, 164, 225),
                            (266, 307, 330),
                            (331, 32, 43),
                            (44, 45, 56),
                            (57, 58, 69),
                            (70, 71, 83),
                            (84, 85, 96),
                            (97, 98, 109),
                            (110, 111, 122),
                            (124, 125, 136),
                            (150, 151, 162),
                            (163, 175, 186),
                            (187, 188, 199),
                            (200, 201, 212),
                        ][i];
                        bel = bel
                            .extra_wire("CE_MUX_DUMMY0", &[format!("VCC_WIRE{ii}", ii = bi.0)])
                            .extra_wire("RST_MUX_DUMMY0", &[format!("VCC_WIRE{ii}", ii = bi.2)]);
                        for j in 0..11 {
                            bel = bel.extra_wire(
                                format!("CLK_IN_MUX_DUMMY{j}"),
                                &[format!("VCC_WIRE{ii}", ii = bi.1 + j)],
                            );
                        }
                    }
                    bels.push(bel);
                }
                for i in 0..15 {
                    let mut bel = self
                        .builder
                        .bel_xy(bels::BUFG_GT_SYNC[i], "BUFG_GT_SYNC", 0, i)
                        .pins_name_only(&["CE_OUT", "RST_OUT"]);
                    if !kind.starts_with("GT") && (4..14).contains(&i) {
                        bel = bel.pins_name_only(&["CE_IN", "RST_IN"]);
                    }
                    if i != 14 {
                        bel = bel.pins_name_only(&["CLK_IN"]);
                    }
                    if kind.starts_with("GTM") && matches!(i, 6 | 13) {
                        bel = bel.extra_wire(
                            "CLK_IN",
                            &[format!(
                                "CLK_BUFG_GT_SYNC_BOTH_{ii}_CLK_IN",
                                ii = if side == Dir::W { 24 + i } else { 26 + i }
                            )],
                        );
                    }
                    bels.push(bel);
                }
                for i in 0..5 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::ABUS_SWITCH_GT[i], "ABUS_SWITCH", 0, i),
                    );
                }

                if kind.starts_with("GTM") {
                    bels.push(
                        self.builder
                            .bel_xy(bels::GTM_DUAL, "GTM_DUAL", 0, 0)
                            .pins_name_only(&[
                                "CLK_BUFGT_CLK_IN_BOT0",
                                "CLK_BUFGT_CLK_IN_BOT1",
                                "CLK_BUFGT_CLK_IN_BOT2",
                                "CLK_BUFGT_CLK_IN_BOT3",
                                "CLK_BUFGT_CLK_IN_BOT4",
                                "CLK_BUFGT_CLK_IN_BOT5",
                                "CLK_BUFGT_CLK_IN_TOP0",
                                "CLK_BUFGT_CLK_IN_TOP1",
                                "CLK_BUFGT_CLK_IN_TOP2",
                                "CLK_BUFGT_CLK_IN_TOP3",
                                "CLK_BUFGT_CLK_IN_TOP4",
                                "CLK_BUFGT_CLK_IN_TOP5",
                                "HROW_TEST_CK_SA",
                                "MGTREFCLK_CLEAN",
                                "RXRECCLK0_INT",
                                "RXRECCLK1_INT",
                                "REFCLKPDB_SA",
                                "RCALSEL0",
                                "RCALSEL1",
                                "REFCLK_DIST2PLL0",
                                "REFCLK_DIST2PLL1",
                                "REFCLK2HROW",
                            ])
                            .extra_wire("SOUTHCLKOUT", &["SOUTHCLKOUT"])
                            .extra_wire("SOUTHCLKOUT_DUMMY0", &["VCC_WIRE72"])
                            .extra_wire("SOUTHCLKOUT_DUMMY1", &["VCC_WIRE73"])
                            .extra_wire("NORTHCLKOUT", &["NORTHCLKOUT"])
                            .extra_wire("NORTHCLKOUT_DUMMY0", &["VCC_WIRE74"])
                            .extra_wire("NORTHCLKOUT_DUMMY1", &["VCC_WIRE75"]),
                    );
                    bels.push(
                        self.builder
                            .bel_xy(bels::GTM_REFCLK, "GTM_REFCLK", 0, 0)
                            .pins_name_only(&[
                                "HROW_TEST_CK_FS",
                                "MGTREFCLK_CLEAN",
                                "REFCLK2HROW",
                                "REFCLKPDB_SA",
                                "RXRECCLK0_INT",
                                "RXRECCLK1_INT",
                                "RXRECCLK2_INT",
                                "RXRECCLK3_INT",
                            ]),
                    );
                } else if kind.starts_with("GT") {
                    let (common, channel) = match kind {
                        "GTH" => (bels::GTH_COMMON, bels::GTH_CHANNEL),
                        "GTY" => (bels::GTY_COMMON, bels::GTY_CHANNEL),
                        "GTF" => (bels::GTF_COMMON, bels::GTF_CHANNEL),
                        _ => unreachable!(),
                    };
                    let pref = if kind == "GTF" {
                        "GTF".to_string()
                    } else {
                        format!("{kind}E4")
                    };
                    for i in 0..4 {
                        bels.push(
                            self.builder
                                .bel_xy(channel[i], &format!("{pref}_CHANNEL"), 0, i)
                                .pins_name_only(&[
                                    "MGTREFCLK0",
                                    "MGTREFCLK1",
                                    "NORTHREFCLK0",
                                    "NORTHREFCLK1",
                                    "SOUTHREFCLK0",
                                    "SOUTHREFCLK1",
                                    "QDCMREFCLK0_INT",
                                    "QDCMREFCLK1_INT",
                                    "QDPLL0CLK0P_INT",
                                    "QDPLL1CLK0P_INT",
                                    "RING_OSC_CLK_INT",
                                    "RXRECCLKOUT",
                                    "RXRECCLK_INT",
                                    "TXOUTCLK_INT",
                                    "DMONOUTCLK_INT",
                                ]),
                        );
                    }
                    bels.push(
                        self.builder
                            .bel_xy(common, &format!("{pref}_COMMON"), 0, 0)
                            .pins_name_only(&[
                                "RXRECCLK0",
                                "RXRECCLK1",
                                "RXRECCLK2",
                                "RXRECCLK3",
                                "QDCMREFCLK_INT_0",
                                "QDCMREFCLK_INT_1",
                                "QDPLLCLK0P_0",
                                "QDPLLCLK0P_1",
                                "COM0_REFCLKOUT0",
                                "COM0_REFCLKOUT1",
                                "COM0_REFCLKOUT2",
                                "COM0_REFCLKOUT3",
                                "COM0_REFCLKOUT4",
                                "COM0_REFCLKOUT5",
                                "COM2_REFCLKOUT0",
                                "COM2_REFCLKOUT1",
                                "COM2_REFCLKOUT2",
                                "COM2_REFCLKOUT3",
                                "COM2_REFCLKOUT4",
                                "COM2_REFCLKOUT5",
                                "MGTREFCLK0",
                                "MGTREFCLK1",
                                "REFCLK2HROW0",
                                "REFCLK2HROW1",
                                "SARC_CLK0",
                                "SARC_CLK1",
                                "SARC_CLK2",
                                "SARC_CLK3",
                            ])
                            .extra_wire("CLKOUT_NORTH0", &["CLKOUT_NORTH0"])
                            .extra_wire("CLKOUT_NORTH1", &["CLKOUT_NORTH1"])
                            .extra_wire("CLKOUT_SOUTH0", &["CLKOUT_SOUTH0"])
                            .extra_wire("CLKOUT_SOUTH1", &["CLKOUT_SOUTH1"])
                            .extra_wire(
                                "NORTHREFCLK0",
                                &[
                                    "GTH_CHANNEL_BLH_0_NORTHREFCLK0",
                                    "GTH_CHANNEL_BLH_44_NORTHREFCLK0",
                                    "GTF_CHANNEL_BLH_1_NORTHREFCLK0",
                                    "GTF_CHANNEL_BLH_45_NORTHREFCLK0",
                                    "GTY_CHANNEL_BLH_1_NORTHREFCLK0",
                                    "GTY_CHANNEL_BLH_45_NORTHREFCLK0",
                                ],
                            )
                            .extra_wire(
                                "NORTHREFCLK1",
                                &[
                                    "GTH_CHANNEL_BLH_0_NORTHREFCLK1",
                                    "GTH_CHANNEL_BLH_44_NORTHREFCLK1",
                                    "GTF_CHANNEL_BLH_1_NORTHREFCLK1",
                                    "GTF_CHANNEL_BLH_45_NORTHREFCLK1",
                                    "GTY_CHANNEL_BLH_1_NORTHREFCLK1",
                                    "GTY_CHANNEL_BLH_45_NORTHREFCLK1",
                                ],
                            )
                            .extra_wire(
                                "SOUTHREFCLK0",
                                &[
                                    "GTH_CHANNEL_BLH_0_SOUTHREFCLK0",
                                    "GTH_CHANNEL_BLH_44_SOUTHREFCLK0",
                                    "GTF_CHANNEL_BLH_1_SOUTHREFCLK0",
                                    "GTF_CHANNEL_BLH_45_SOUTHREFCLK0",
                                    "GTY_CHANNEL_BLH_1_SOUTHREFCLK0",
                                    "GTY_CHANNEL_BLH_45_SOUTHREFCLK0",
                                ],
                            )
                            .extra_wire(
                                "SOUTHREFCLK1",
                                &[
                                    "GTH_CHANNEL_BLH_0_SOUTHREFCLK1",
                                    "GTH_CHANNEL_BLH_44_SOUTHREFCLK1",
                                    "GTF_CHANNEL_BLH_1_SOUTHREFCLK1",
                                    "GTF_CHANNEL_BLH_45_SOUTHREFCLK1",
                                    "GTY_CHANNEL_BLH_1_SOUTHREFCLK1",
                                    "GTY_CHANNEL_BLH_45_SOUTHREFCLK1",
                                ],
                            ),
                    );
                } else {
                    let slot = match kind {
                        "HSADC" => bels::HSADC,
                        "HSDAC" => bels::HSDAC,
                        "RFADC" => bels::RFADC,
                        "RFDAC" => bels::RFDAC,
                        _ => unreachable!(),
                    };
                    let mut bel = self
                        .builder
                        .bel_xy(slot, kind, 0, 0)
                        .pins_name_only(&[
                            "SYSREF_OUT_SOUTH_P",
                            "SYSREF_OUT_NORTH_P",
                            "PLL_DMON_OUT",
                            "PLL_REFCLK_OUT",
                        ])
                        .pin_name_only("SYSREF_IN_SOUTH_P", 1)
                        .pin_name_only("SYSREF_IN_NORTH_P", 1);
                    if kind.ends_with("ADC") {
                        bel = bel.pins_name_only(&["CLK_ADC", "CLK_ADC_SPARE"]);
                    } else {
                        bel = bel.pins_name_only(&["CLK_DAC", "CLK_DAC_SPARE"]);
                    }
                    if kind.starts_with("RF") {
                        bel = bel
                            .pins_name_only(&[
                                "CLK_DISTR_IN_NORTH",
                                "CLK_DISTR_IN_SOUTH",
                                "CLK_DISTR_OUT_NORTH",
                                "CLK_DISTR_OUT_SOUTH",
                                "T1_ALLOWED_SOUTH",
                                "T1_ALLOWED_NORTH",
                            ])
                            .pin_name_only("CLK_DISTR_IN_NORTH", 1)
                            .pin_name_only("CLK_DISTR_IN_SOUTH", 1)
                            .pin_name_only("T1_ALLOWED_NORTH", 1);
                    }
                    bels.push(bel);
                }
                let mut bel = self.builder.bel_virtual(bels::RCLK_GT);
                for i in 0..24 {
                    if side == Dir::W {
                        bel = bel
                            .extra_wire(format!("HDISTR{i}_R"), &[format!("CLK_HDISTR_FT1_{i}")])
                            .extra_wire(format!("HROUTE{i}_R"), &[format!("CLK_HROUTE_FT1_{i}")]);
                    } else {
                        bel = bel
                            .extra_wire(format!("HDISTR{i}_L"), &[format!("CLK_HDISTR_FT0_{i}")])
                            .extra_wire(format!("HROUTE{i}_L"), &[format!("CLK_HROUTE_FT0_{i}")])
                    }
                }
                bels.push(bel);
                bels.push(
                    self.builder
                        .bel_virtual(bels::VCC_GT)
                        .extra_wire("VCC", &["VCC_WIRE"]),
                );
                let mut xn = self
                    .builder
                    .xnode(tslots::BEL, kind, naming, xy)
                    .num_tiles(60)
                    .ref_xlat(
                        int_xy.delta(0, 30),
                        if side == Dir::W {
                            &[Some(30), None, None, None]
                        } else {
                            &[None, Some(30), None, None]
                        },
                        rclk_int,
                    );
                for i in 0..60 {
                    xn = xn
                        .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), side, i)
                        .ref_single(
                            int_xy.delta(if side == Dir::W { -1 } else { 1 }, (i + i / 30) as i32),
                            i,
                            intf,
                        )
                }
                xn.bels(bels).extract();
            }
        }
    }
}

pub fn make_int_db(rd: &Part, dev_naming: &DeviceNaming) -> (IntDb, NamingDb) {
    let mut maker = IntMaker {
        builder: IntBuilder::new(rd),
        long_term_slots: DirPartMap::new(),
        long_main_passes: DirPartMap::new(),
        sng_fixup_map: BTreeMap::new(),
        term_wires_w: EntityPartVec::new(),
        term_wires_e: EntityPartVec::new(),
        term_wires_lw: EntityPartVec::new(),
        term_wires_le: EntityPartVec::new(),
        dev_naming,
    };

    assert_eq!(
        maker.builder.db.region_slots.insert("LEAF".into()).0,
        REGION_LEAF
    );

    maker.builder.db.init_slots(tslots::SLOTS, bels::SLOTS);

    maker.fill_term_slots();
    maker.fill_wires();

    maker.fill_tiles_int();
    maker.fill_terms();
    maker.fill_tiles_rclk_int();
    maker.fill_tiles_intf();
    maker.fill_tiles_clb();
    maker.fill_tiles_lag();
    maker.fill_tiles_bram();
    maker.fill_tiles_dsp();
    maker.fill_tiles_bli();
    maker.fill_tiles_uram();
    maker.fill_tiles_hard();
    maker.fill_tiles_ps();
    maker.fill_tiles_cfg();
    maker.fill_tiles_hdio();
    maker.fill_tiles_cmt();
    maker.fill_tiles_xiphy();
    maker.fill_tiles_hpio();
    maker.fill_tiles_xp5io();
    maker.fill_tiles_rclk();
    maker.fill_tiles_gt();

    maker.builder.build()
}
