use std::collections::BTreeMap;

use prjcombine_interconnect::{
    db::{
        CellSlotId, ConnectorClass, ConnectorSlot, ConnectorSlotId, ConnectorWire, IntDb,
        TileWireCoord, WireId, WireKind,
    },
    dir::{Dir, DirPartMap},
};
use prjcombine_re_xilinx_rawdump::{Coord, Part};

use prjcombine_re_xilinx_naming::db::NamingDb;
use prjcombine_re_xilinx_naming_ultrascale::DeviceNaming;
use prjcombine_re_xilinx_rd2db_interconnect::{IntBuilder, XNodeInfo, XNodeRef};
use prjcombine_ultrascale::{bels, expanded::REGION_LEAF, tslots};
use unnamed_entity::{EntityId, EntityPartVec};

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

    fn fill_wires_ql(&mut self) {
        for (dir, name, l, n, fts, ftn) in [
            (Dir::E, "QUAD", 2, 16, true, true),
            (Dir::W, "QUAD", 2, 16, false, false),
            (Dir::N, "QUAD.4", 4, 8, false, false),
            (Dir::N, "QUAD.5", 5, 8, false, true),
            (Dir::S, "QUAD.4", 4, 8, false, false),
            (Dir::S, "QUAD.5", 5, 8, false, false),
            (Dir::E, "LONG", 6, 8, true, false),
            (Dir::W, "LONG", 6, 8, false, true),
            (Dir::N, "LONG.12", 12, 4, false, false),
            (Dir::N, "LONG.16", 16, 4, false, true),
            (Dir::S, "LONG.12", 12, 4, true, false),
            (Dir::S, "LONG.16", 16, 4, false, false),
        ] {
            let ftd = !dir;
            let ll = if matches!(dir, Dir::E | Dir::W) {
                l * 2
            } else {
                l
            };
            for i in 0..n {
                let mut w = self.builder.mux_out(
                    format!("{name}.{dir}.{i}.0"),
                    &[format!("{dir}{dir}{ll}_BEG{i}")],
                );
                for j in 1..l {
                    let nn = (b'A' + (j - 1)) as char;
                    let wname = format!("{name}.{dir}.{i}.{j}");
                    let vwname = format!("{dir}{dir}{ll}_{nn}_FT{ftd}{i}");
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
                let wname = format!("{name}.{dir}.{i}.{l}");
                let vwname = format!("{dir}{dir}{ll}_END{i}");
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
                        format!("{name}.{dir}.{i}.{l}.S"),
                        &[format!("{dir}{dir}{ll}_BLS_{i}_FTN")],
                    );
                }
                if i == (n - 1) && ftn {
                    self.builder.branch(
                        w,
                        Dir::N,
                        format!("{name}.{dir}.{i}.{l}.N"),
                        &[format!("{dir}{dir}{ll}_BLN_{i}_FTS")],
                    );
                }
            }
        }
        for dir in [Dir::W, Dir::E] {
            let rdir = !dir;
            for i in 0..16 {
                for seg in 0..2 {
                    let nseg = seg + 1;
                    let wt = self.builder.db.get_wire(&format!("QUAD.{rdir}.{i}.{nseg}"));
                    let wf = self.builder.db.get_wire(&format!("QUAD.{dir}.{i}.{seg}"));
                    let wires = match dir {
                        Dir::W => &mut self.term_wires_lw,
                        Dir::E => &mut self.term_wires_le,
                        _ => unreachable!(),
                    };
                    wires.insert(wt, ConnectorWire::Reflect(wf));
                }
            }
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

    fn fill_wires_sdnode(&mut self) {
        for (iq, q) in ["NE", "NW", "SE", "SW"].into_iter().enumerate() {
            for i in 0..16 {
                match (iq, i) {
                    (1 | 3, 0) => {
                        let w = self.builder.mux_out_pair(
                            format!("SDNODE.{q}.{i}"),
                            &[format!("SDND{q}_W_{i}_FTS"), format!("SDND{q}_E_{i}_FTS")],
                        );
                        self.builder.branch_pair(
                            w,
                            Dir::S,
                            format!("SDNODE.{q}.{i}.S"),
                            &[
                                format!("SDND{q}_W_BLS_{i}_FTN"),
                                format!("SDND{q}_E_BLS_{i}_FTN"),
                            ],
                        );
                    }
                    (1 | 3, 15) => {
                        let w = self.builder.mux_out_pair(
                            format!("SDNODE.{q}.{i}"),
                            &[format!("SDND{q}_W_{i}_FTN"), format!("SDND{q}_E_{i}_FTN")],
                        );
                        self.builder.branch_pair(
                            w,
                            Dir::N,
                            format!("SDNODE.{q}.{i}.N"),
                            &[
                                format!("SDND{q}_W_BLN_{i}_FTS"),
                                format!("SDND{q}_E_BLN_{i}_FTS"),
                            ],
                        );
                    }
                    _ => {
                        let xlat = [0, 7, 8, 9, 10, 11, 12, 13, 14, 15, 1, 2, 3, 4, 5, 6];
                        self.builder.mux_out_pair(
                            format!("SDNODE.{q}.{i}"),
                            &[
                                format!(
                                    "INT_NODE_SINGLE_DOUBLE_{n}_INT_OUT",
                                    n = iq * 32 + 16 + xlat[i]
                                ),
                                format!(
                                    "INT_NODE_SINGLE_DOUBLE_{n}_INT_OUT",
                                    n = iq * 32 + xlat[i]
                                ),
                            ],
                        );
                    }
                }
            }
        }
    }

    fn fill_wires_sng(&mut self) {
        for i in 0..8 {
            let beg = self.builder.mux_out_pair(
                format!("SNG.E.{i}.0"),
                &[
                    if i == 0 {
                        format!("EE1_W_{i}_FTS")
                    } else {
                        format!("INT_INT_SINGLE_{n}_INT_OUT", n = i + 8)
                    },
                    format!("EE1_E_BEG{i}"),
                ],
            );
            let end = self
                .builder
                .branch(beg, Dir::E, format!("SNG.E.{i}.1"), &[""]);
            self.builder.extra_name_sub(format!("EE1_E_END{i}"), 0, end);
            if i == 0 {
                self.builder.branch_pair(
                    end,
                    Dir::S,
                    format!("SNG.E.{i}.1.S"),
                    &[format!("EE1_E_BLS_{i}_FTN"), format!("EE1_W_BLS_{i}_FTN")],
                );
            }
            self.sng_fixup_map.insert(
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: beg,
                },
                TileWireCoord {
                    cell: CellSlotId::from_idx(1),
                    wire: end,
                },
            );
            if i == 0 {
                self.builder
                    .branch(end, Dir::W, format!("SNG.E.{i}.1.W"), &[""]);
                self.builder
                    .branch(end, Dir::E, format!("SNG.E.{i}.1.E"), &[""]);
            }
        }
        for i in 0..8 {
            let beg = self.builder.mux_out_pair(
                format!("SNG.W.{i}.0"),
                &[
                    format!("WW1_W_BEG{i}"),
                    format!("INT_INT_SINGLE_{n}_INT_OUT", n = i + 48),
                ],
            );
            let end = self
                .builder
                .branch(beg, Dir::W, format!("SNG.W.{i}.1"), &[""]);
            self.builder.extra_name_sub(format!("WW1_W_END{i}"), 1, end);
            self.sng_fixup_map.insert(
                TileWireCoord {
                    cell: CellSlotId::from_idx(1),
                    wire: beg,
                },
                TileWireCoord {
                    cell: CellSlotId::from_idx(0),
                    wire: end,
                },
            );
        }
        for dir in [Dir::W, Dir::E] {
            let rdir = !dir;
            for i in 0..8 {
                let wt = self.builder.db.get_wire(&format!("SNG.{rdir}.{i}.1"));
                let wf = self.builder.db.get_wire(&format!("SNG.{dir}.{i}.0"));
                let wires = match dir {
                    Dir::W => &mut self.term_wires_w,
                    Dir::E => &mut self.term_wires_e,
                    _ => unreachable!(),
                };
                wires.insert(wt, ConnectorWire::Reflect(wf));
            }
        }
        for dir in [Dir::N, Dir::S] {
            for i in 0..8 {
                let beg = self.builder.mux_out_pair(
                    format!("SNG.{dir}.{i}.0"),
                    &[
                        format!("{dir}{dir}1_W_BEG{i}"),
                        format!("{dir}{dir}1_E_BEG{i}"),
                    ],
                );
                let end = self.builder.branch_pair(
                    beg,
                    dir,
                    format!("SNG.{dir}.{i}.1"),
                    &[
                        format!("{dir}{dir}1_W_END{i}"),
                        format!("{dir}{dir}1_E_END{i}"),
                    ],
                );
                if i == 0 && dir == Dir::S {
                    self.builder.branch_pair(
                        end,
                        Dir::S,
                        format!("SNG.{dir}.{i}.1.S"),
                        &[
                            format!("{dir}{dir}1_W_BLS_{i}_FTN"),
                            format!("{dir}{dir}1_E_BLS_{i}_FTN"),
                        ],
                    );
                }
            }
        }
    }

    fn fill_wires_dbl(&mut self) {
        for dir in [Dir::E, Dir::W] {
            for i in 0..8 {
                let beg = self.builder.mux_out_pair(
                    format!("DBL.{dir}.{i}.0"),
                    &[
                        format!("{dir}{dir}2_W_BEG{i}"),
                        format!("{dir}{dir}2_E_BEG{i}"),
                    ],
                );
                let mid = self
                    .builder
                    .branch(beg, dir, format!("DBL.{dir}.{i}.1"), &[""]);
                let end = self.builder.branch_pair(
                    mid,
                    dir,
                    format!("DBL.{dir}.{i}.2"),
                    &[
                        format!("{dir}{dir}2_W_END{i}"),
                        format!("{dir}{dir}2_E_END{i}"),
                    ],
                );
                if i == 7 && dir == Dir::E {
                    self.builder.branch_pair(
                        end,
                        Dir::N,
                        format!("DBL.{dir}.{i}.2.N"),
                        &[
                            format!("{dir}{dir}2_W_BLN_{i}_FTS"),
                            format!("{dir}{dir}2_E_BLN_{i}_FTS"),
                        ],
                    );
                }
                if i == 7 && dir == Dir::E {
                    self.builder
                        .branch(end, Dir::W, format!("DBL.{dir}.{i}.2.W"), &[""]);
                    self.builder
                        .branch(end, Dir::E, format!("DBL.{dir}.{i}.2.E"), &[""]);
                }
            }
        }
        for dir in [Dir::W, Dir::E] {
            let rdir = !dir;
            for i in 0..8 {
                for seg in 0..2 {
                    let nseg = seg + 1;
                    // AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAaaaaaaaaaaaaaaaaaaaa
                    let xseg = seg ^ 1;
                    let wt = self.builder.db.get_wire(&format!("DBL.{rdir}.{i}.{nseg}"));
                    let wf = self.builder.db.get_wire(&format!("DBL.{dir}.{i}.{xseg}"));
                    let wires = match dir {
                        Dir::W => &mut self.term_wires_w,
                        Dir::E => &mut self.term_wires_e,
                        _ => unreachable!(),
                    };
                    wires.insert(wt, ConnectorWire::Reflect(wf));
                }
            }
        }
        for dir in [Dir::N, Dir::S] {
            let ftd = !dir;
            for i in 0..8 {
                let beg = self.builder.mux_out_pair(
                    format!("DBL.{dir}.{i}.0"),
                    &[
                        format!("{dir}{dir}2_W_BEG{i}"),
                        format!("{dir}{dir}2_E_BEG{i}"),
                    ],
                );
                let a = self.builder.branch_pair(
                    beg,
                    dir,
                    format!("DBL.{dir}.{i}.1"),
                    &[
                        format!("{dir}{dir}2_W_A_FT{ftd}{i}"),
                        format!("{dir}{dir}2_E_A_FT{ftd}{i}"),
                    ],
                );
                let end = self.builder.branch_pair(
                    a,
                    dir,
                    format!("DBL.{dir}.{i}.2"),
                    &[
                        format!("{dir}{dir}2_W_END{i}"),
                        format!("{dir}{dir}2_E_END{i}"),
                    ],
                );
                if i == 7 && dir == Dir::N {
                    self.builder.branch_pair(
                        end,
                        Dir::N,
                        format!("DBL.{dir}.{i}.2.N"),
                        &[
                            format!("{dir}{dir}2_W_BLN_{i}_FTS"),
                            format!("{dir}{dir}2_E_BLN_{i}_FTS"),
                        ],
                    );
                }
            }
        }
    }

    fn fill_wires_qlnode(&mut self) {
        for (iq, q) in ["NE", "NW", "SE", "SW"].into_iter().enumerate() {
            for i in 0..16 {
                let xlat = [0, 7, 8, 9, 10, 11, 12, 13, 14, 15, 1, 2, 3, 4, 5, 6];
                let w = self.builder.mux_out_pair(
                    format!("QLNODE.{q}.{i}"),
                    &[
                        format!("INT_NODE_QUAD_LONG_{n}_INT_OUT", n = iq * 32 + 16 + xlat[i]),
                        format!("INT_NODE_QUAD_LONG_{n}_INT_OUT", n = iq * 32 + xlat[i]),
                    ],
                );
                match (q, i) {
                    ("NW", 0) | ("SW", 0) | ("NW", 1) => {
                        self.builder
                            .extra_name_sub(format!("QLND{q}_W_{i}_FTS"), 0, w);
                        self.builder
                            .extra_name_sub(format!("QLND{q}_E_{i}_FTS"), 1, w);
                        self.builder.branch_pair(
                            w,
                            Dir::S,
                            format!("QLNODE.{q}.{i}.S"),
                            &[
                                format!("QLND{q}_W_BLS_{i}_FTN"),
                                format!("QLND{q}_E_BLS_{i}_FTN"),
                            ],
                        );
                    }
                    ("NW", 15) | ("SW", 15) | ("SE", 15) => {
                        self.builder
                            .extra_name_sub(format!("QLND{q}_W_{i}_FTN"), 0, w);
                        self.builder
                            .extra_name_sub(format!("QLND{q}_E_{i}_FTN"), 1, w);
                        self.builder.branch_pair(
                            w,
                            Dir::N,
                            format!("QLNODE.{q}.{i}.N"),
                            &[
                                format!("QLND{q}_W_BLN_{i}_FTS"),
                                format!("QLND{q}_E_BLN_{i}_FTS"),
                            ],
                        );
                    }
                    _ => (),
                }
            }
        }
    }

    fn fill_wires_inode(&mut self) {
        for (iq, q) in ["1", "2"].into_iter().enumerate() {
            for i in 0..32 {
                match i {
                    1 | 3 => {
                        let w = self.builder.mux_out_pair(
                            format!("INODE.{q}.{i}"),
                            &[
                                format!("INODE_{q}_W_{i}_FTS"),
                                format!("INODE_{q}_E_{i}_FTS"),
                            ],
                        );
                        self.builder.branch_pair(
                            w,
                            Dir::S,
                            format!("INODE.{q}.{i}.S"),
                            &[
                                format!("INODE_{q}_W_BLS_{i}_FTN"),
                                format!("INODE_{q}_E_BLS_{i}_FTN"),
                            ],
                        );
                    }
                    28 | 30 => {
                        let w = self.builder.mux_out_pair(
                            format!("INODE.{q}.{i}"),
                            &[
                                format!("INODE_{q}_W_{i}_FTN"),
                                format!("INODE_{q}_E_{i}_FTN"),
                            ],
                        );
                        self.builder.branch_pair(
                            w,
                            Dir::N,
                            format!("INODE.{q}.{i}.N"),
                            &[
                                format!("INODE_{q}_W_BLN_{i}_FTS"),
                                format!("INODE_{q}_E_BLN_{i}_FTS"),
                            ],
                        );
                    }
                    _ => {
                        let xlat = [
                            0, 11, 22, 25, 26, 27, 28, 29, 30, 31, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10,
                            12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 23, 24,
                        ];
                        let w = self.builder.mux_out(format!("INODE.{q}.{i}"), &[""]);
                        self.builder.extra_name_tile_sub(
                            "INT",
                            format!("INT_NODE_IMUX_{n}_INT_OUT", n = iq * 64 + 32 + xlat[i]),
                            0,
                            w,
                        );
                        self.builder.extra_name_tile_sub(
                            "INT",
                            format!("INT_NODE_IMUX_{n}_INT_OUT", n = iq * 64 + xlat[i]),
                            1,
                            w,
                        );
                    }
                }
            }
        }
    }

    fn fill_wires_imux(&mut self) {
        for i in 0..10 {
            self.builder.mux_out_pair(
                format!("IMUX.CTRL.{i}"),
                &[format!("CTRL_W_B{i}"), format!("CTRL_E_B{i}")],
            );
        }

        for i in 0..16 {
            match i {
                1 | 3 | 5 | 7 | 11 => {
                    let w = self.builder.mux_out_pair(
                        format!("IMUX.BYP.{i}"),
                        &[format!("BOUNCE_W_{i}_FTS"), format!("BOUNCE_E_{i}_FTS")],
                    );
                    self.builder.branch_pair(
                        w,
                        Dir::S,
                        format!("IMUX.BYP.{i}.S"),
                        &[
                            format!("BOUNCE_W_BLS_{i}_FTN"),
                            format!("BOUNCE_E_BLS_{i}_FTN"),
                        ],
                    );
                }
                8 | 10 | 12 | 14 => {
                    let w = self.builder.mux_out_pair(
                        format!("IMUX.BYP.{i}"),
                        &[format!("BOUNCE_W_{i}_FTN"), format!("BOUNCE_E_{i}_FTN")],
                    );
                    self.builder.branch_pair(
                        w,
                        Dir::N,
                        format!("IMUX.BYP.{i}.N"),
                        &[
                            format!("BOUNCE_W_BLN_{i}_FTS"),
                            format!("BOUNCE_E_BLN_{i}_FTS"),
                        ],
                    );
                }
                _ => {
                    self.builder.mux_out_pair(
                        format!("IMUX.BYP.{i}"),
                        &[format!("BYPASS_W{i}"), format!("BYPASS_E{i}")],
                    );
                }
            }
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
        #[allow(clippy::type_complexity)]
        const BUFCE_XLAT: [Option<((usize, usize), (usize, usize))>; 24] = [
            None,
            None,
            None,
            None,
            Some(((1, 8), (1, 4))),
            Some(((1, 13), (1, 6))),
            None,
            None,
            None,
            None,
            Some(((0, 9), (0, 7))),
            Some(((1, 11), (1, 2))),
            Some(((0, 0), (0, 1))),
            Some(((0, 4), (0, 5))),
            Some(((0, 6), (0, 8))),
            Some(((0, 12), (0, 13))),
            Some(((1, 3), (1, 10))),
            Some(((1, 15), (1, 14))),
            Some(((0, 2), (0, 3))),
            Some(((0, 10), (0, 11))),
            Some(((0, 14), (0, 15))),
            Some(((1, 1), (1, 0))),
            Some(((1, 5), (1, 9))),
            Some(((1, 7), (1, 12))),
        ];
        for i in 0..24 {
            if let Some(((wa, wb), (ea, eb))) = BUFCE_XLAT[i] {
                self.builder.mux_out_pair(
                    format!("RCLK.IMUX.{i}"),
                    &[
                        format!("CLK_BUFCE_LEAF_X16_{wa}_CE_INT{wb}"),
                        format!("CLK_BUFCE_LEAF_X16_{ea}_CE_INT{eb}",),
                    ],
                );
            } else {
                self.builder.mux_out_pair(
                    format!("RCLK.IMUX.{i}"),
                    &[
                        format!("INT_RCLK_TO_CLK_LEFT_{a}_{b}", a = i / 6, b = i % 6),
                        format!("INT_RCLK_TO_CLK_RIGHT_{a}_{b}", a = i / 6, b = i % 6),
                    ],
                );
            }
        }
        for i in 0..24 {
            let w = self.builder.mux_out(format!("RCLK.INODE.{i}"), &[""]);
            for sub in 0..2 {
                for tkn in ["RCLK_INT_L", "RCLK_INT_R"] {
                    let ii = (i % 12) + (1 - sub) * 12 + (i / 12) * 24;
                    let name = format!("INT_NODE_IMUX_{ii}_INT_OUT");
                    self.builder.extra_name_tile_sub(tkn, name, sub, w);
                }
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

        self.builder.wire("GND", WireKind::Tie0, &["GND_WIRE"]);
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
                    format!("CLK_BUFCE_LEAF_X16_0_CLK_OUT{ii}", ii = i / 2 + (i % 2) * 8),
                    2,
                    w,
                );
            }
        }

        for i in 0..16 {
            for j in 0..2 {
                self.builder.mux_out(
                    format!("GNODE.{i}.{j}"),
                    &[format!("INT_NODE_GLOBAL_{i}_OUT{j}")],
                );
            }
        }

        self.fill_wires_ql();

        // wires belonging to interconnect left/right half-nodes

        for i in 0..32 {
            let w = self
                .builder
                .logic_out(format!("OUT.{i}"), &[format!("LOGIC_OUTS_W{i}")]);
            self.builder
                .extra_name_sub(format!("LOGIC_OUTS_E{i}"), 1, w);
        }

        for i in 0..4 {
            self.builder
                .test_out(format!("TEST.{i}"), &[format!("BLOCK_OUTS{i}")]);
        }

        self.fill_wires_sdnode();
        self.fill_wires_sng();
        self.fill_wires_dbl();
        self.fill_wires_qlnode();
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
                if wtn.starts_with("INODE")
                    || wtn.starts_with("SDNODE")
                    || wtn.starts_with("QLNODE")
                {
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
                if self.builder.db.wires.key(wf) == "SNG.E.0.1" {
                    twf = (
                        CellSlotId::from_idx(tile.to_idx() ^ 1),
                        self.builder
                            .db
                            .get_wire(&format!("SNG.E.0.1.{d}", d = ["E", "W"][tile.to_idx()])),
                    );
                }
                if self.builder.db.wires.key(wf) == "DBL.E.7.2" {
                    twf = (
                        CellSlotId::from_idx(tile.to_idx() ^ 1),
                        self.builder
                            .db
                            .get_wire(&format!("DBL.E.7.2.{d}", d = ["E", "W"][tile.to_idx()])),
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

    fn fill_terms(&mut self) {
        for &xy in self.builder.rd.tiles_by_kind_name("INT_TERM_B") {
            let int_xy = self.builder.walk_to_int(xy, Dir::N, true).unwrap();
            self.extract_sn_term(Dir::S, int_xy);
        }
        for &xy in self.builder.rd.tiles_by_kind_name("INT_TERM_T") {
            let int_xy = self.builder.walk_to_int(xy, Dir::S, true).unwrap();
            self.extract_sn_term(Dir::N, int_xy);
        }
    }

    fn fill_tiles_rclk_int(&mut self) {
        for tkn in ["RCLK_INT_L", "RCLK_INT_R"] {
            for &xy in self.builder.rd.tiles_by_kind_name(tkn) {
                let mut bels = vec![];
                for i in 0..2 {
                    let slot = [bels::BUFCE_LEAF_X16_S, bels::BUFCE_LEAF_X16_N][i];
                    let mut bel = self.builder.bel_xy(slot, "BUFCE_LEAF_X16", 0, i);
                    for j in 0..16 {
                        bel = bel.pin_name_only(&format!("CLK_IN{j}"), 0);
                    }
                    bels.push(bel);
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
            ("INTF", "INTF.W", Dir::W, "INT_INTERFACE_L", None),
            ("INTF", "INTF.E", Dir::E, "INT_INTERFACE_R", None),
            (
                "INTF.DELAY",
                "INTF.W.IO",
                Dir::W,
                "INT_INT_INTERFACE_XIPHY_FT",
                Some(bels::INTF_DELAY),
            ),
            (
                "INTF.DELAY",
                "INTF.W.PCIE",
                Dir::W,
                "INT_INTERFACE_PCIE_L",
                Some(bels::INTF_DELAY),
            ),
            (
                "INTF.DELAY",
                "INTF.E.PCIE",
                Dir::E,
                "INT_INTERFACE_PCIE_R",
                Some(bels::INTF_DELAY),
            ),
            (
                "INTF.DELAY",
                "INTF.W.GT",
                Dir::W,
                "INT_INT_INTERFACE_GT_LEFT_FT",
                Some(bels::INTF_DELAY),
            ),
            (
                "INTF.DELAY",
                "INTF.E.GT",
                Dir::E,
                "INT_INTERFACE_GT_R",
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
            ("CLE_M", "CLEM", Dir::W),
            ("CLE_M_R", "CLEM", Dir::W),
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
        if let Some(&xy) = self
            .builder
            .rd
            .tiles_by_kind_name("LAGUNA_TILE")
            .iter()
            .next()
        {
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
                    .bel_virtual(bels::LAGUNA_EXTRA)
                    .extra_wire("UBUMP", &["UBUMP_EXTRA"])
                    .extra_wire("RXD", &["LAG_IOBUF_ATOM_16_RXO"])
                    .extra_wire("TXOUT", &["VCC_WIRE0"]),
            );
            bels.push(
                self.builder
                    .bel_virtual(bels::VCC_LAGUNA)
                    .extra_wire("VCC", &["VCC_WIRE"]),
            );
            self.builder
                .xnode(tslots::BEL, "LAGUNA", "LAGUNA", xy)
                .ref_int_side(xy.delta(1, 0), Dir::W, 0)
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
            "RCLK_BRAM_L",
            "RCLK_BRAM_R",
            "RCLK_RCLK_BRAM_L_AUXCLMP_FT",
            "RCLK_RCLK_BRAM_L_BRAMCLMP_FT",
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

    fn fill_tiles_hard(&mut self) {
        for (slot, kind, tkn, bk) in [
            (bels::PCIE3, "PCIE", "PCIE", "PCIE_3_1"),
            (bels::CMAC, "CMAC", "CMAC_CMAC_FT", "CMAC_SITE"),
            (bels::ILKN, "ILKN", "ILMAC_ILMAC_FT", "ILKN_SITE"),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let int_w_xy = self.builder.walk_to_int(xy, Dir::W, false).unwrap();
                let int_e_xy = self.builder.walk_to_int(xy, Dir::E, false).unwrap();
                let intf_w = self.builder.ndb.get_tile_class_naming("INTF.E.PCIE");
                let intf_e = self.builder.ndb.get_tile_class_naming("INTF.W.PCIE");
                let mut bel = self.builder.bel_xy(slot, bk, 0, 0);
                if kind == "PCIE" {
                    bel = bel
                        .pin_name_only("MCAP_PERST0_B", 1)
                        .pin_name_only("MCAP_PERST1_B", 1);
                }
                let mut xn = self
                    .builder
                    .xnode(tslots::BEL, kind, kind, xy)
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
    }

    fn fill_tiles_cfg(&mut self) {
        if let Some(&xy) = self.builder.rd.tiles_by_kind_name("CFG_CFG").iter().next() {
            let int_xy = self.builder.walk_to_int(xy, Dir::W, false).unwrap();
            let intf = self.builder.ndb.get_tile_class_naming("INTF.E.PCIE");
            let bels = [
                self.builder.bel_xy(bels::CFG, "CONFIG_SITE", 0, 0),
                self.builder
                    .bel_xy(bels::ABUS_SWITCH_CFG, "ABUS_SWITCH", 0, 0),
            ];
            let mut xn = self
                .builder
                .xnode(tslots::BEL, "CFG", "CFG", xy)
                .num_tiles(60);
            for i in 0..60 {
                xn = xn
                    .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), Dir::E, i)
                    .ref_single(int_xy.delta(1, (i + i / 30) as i32), i, intf)
            }
            xn.bels(bels).extract();
        }

        if let Some(&xy) = self
            .builder
            .rd
            .tiles_by_kind_name("CFGIO_IOB")
            .iter()
            .next()
        {
            let int_xy = self.builder.walk_to_int(xy, Dir::W, false).unwrap();
            let intf = self.builder.ndb.get_tile_class_naming("INTF.E.PCIE");
            let bels = [
                self.builder.bel_xy(bels::PMV, "PMV", 0, 0),
                self.builder.bel_xy(bels::PMV2, "PMV2", 0, 0),
                self.builder.bel_xy(bels::PMVIOB, "PMVIOB", 0, 0),
                self.builder.bel_xy(bels::MTBF3, "MTBF3", 0, 0),
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

        if let Some(&xy) = self.builder.rd.tiles_by_kind_name("AMS").iter().next() {
            let int_xy = self.builder.walk_to_int(xy, Dir::W, false).unwrap();
            let intf = self.builder.ndb.get_tile_class_naming("INTF.E.PCIE");
            let mut bel = self
                .builder
                .bel_xy(bels::SYSMON, "SYSMONE1", 0, 0)
                .pins_name_only(&["I2C_SCLK_TS", "I2C_SDA_TS"])
                .pin_name_only("I2C_SCLK_IN", 1)
                .pin_name_only("I2C_SDA_IN", 1);
            for i in 0..16 {
                bel = bel.pins_name_only(&[format!("VP_AUX{i}"), format!("VN_AUX{i}")]);
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
    }

    fn fill_tiles_xiphy(&mut self) {
        if let Some(&xy) = self.builder.rd.tiles_by_kind_name("XIPHY_L").iter().next() {
            let int_xy = self.builder.walk_to_int(xy, Dir::E, false).unwrap();
            let intf = self.builder.ndb.get_tile_class_naming("INTF.W.IO");
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
                        .bel_xy(bels::GCLK_TEST_BUF_CMT[i], "GCLK_TEST_BUFE3", 0, i)
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
                            &[format!("CLK_LEAF_MUX_{ii}_CLK_LEAF", ii = i * 2 + 1)],
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
                        .extra_int_in("CLK_IN_CKINT", &[format!("CLB2CMT_CLK_INT{i}")]),
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
                bels.push(
                    self.builder
                        .bel_xy(bels::PLL[i], "PLLE3_ADV", 0, i)
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
                            "CLKFBIN_MUX_HDISTR",
                            &[format!("CLK_LEAF_MUX_{ii}_CLK_LEAF", ii = 56 + i * 2)],
                        )
                        .extra_wire(
                            "CLKFBIN_MUX_BUFCE_ROW_DLY",
                            &[format!("CLK_LEAF_MUX_{ii}_CLK_LEAF", ii = 57 + i * 2)],
                        )
                        .extra_wire(
                            "CLKIN_MUX_MMCM",
                            &[format!("CLK_CMT_MUX_4TO1_{ii}_CLK_OUT", ii = 24 + i)],
                        )
                        .extra_wire(
                            "CLKIN_MUX_HDISTR",
                            &[format!("CLK_LEAF_MUX_{ii}_CLK_LEAF", ii = 60 + i * 3)],
                        )
                        .extra_wire(
                            "CLKIN_MUX_HROUTE",
                            &[format!("CLK_LEAF_MUX_{ii}_CLK_LEAF", ii = 61 + i * 3)],
                        )
                        .extra_wire(
                            "CLKIN_MUX_BUFCE_ROW_DLY",
                            &[format!("CLK_LEAF_MUX_{ii}_CLK_LEAF", ii = 62 + i * 3)],
                        ),
                );
            }
            bels.push(
                self.builder
                    .bel_xy(bels::MMCM, "MMCME3_ADV", 0, 0)
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
                    .extra_wire("CLKFBIN_MUX_HDISTR", &["CLK_LEAF_MUX_48_CLK_LEAF"])
                    .extra_wire("CLKFBIN_MUX_BUFCE_ROW_DLY", &["CLK_LEAF_MUX_49_CLK_LEAF"])
                    .extra_wire("CLKFBIN_MUX_DUMMY0", &["VCC_WIRE51"])
                    .extra_wire("CLKFBIN_MUX_DUMMY1", &["VCC_WIRE52"])
                    .extra_wire("CLKIN1_MUX_HDISTR", &["CLK_LEAF_MUX_50_CLK_LEAF"])
                    .extra_wire("CLKIN1_MUX_HROUTE", &["CLK_LEAF_MUX_51_CLK_LEAF"])
                    .extra_wire("CLKIN1_MUX_BUFCE_ROW_DLY", &["CLK_LEAF_MUX_52_CLK_LEAF"])
                    .extra_wire("CLKIN1_MUX_DUMMY0", &["GND_WIRE0"])
                    .extra_wire("CLKIN2_MUX_HDISTR", &["CLK_LEAF_MUX_53_CLK_LEAF"])
                    .extra_wire("CLKIN2_MUX_HROUTE", &["CLK_LEAF_MUX_54_CLK_LEAF"])
                    .extra_wire("CLKIN2_MUX_BUFCE_ROW_DLY", &["CLK_LEAF_MUX_55_CLK_LEAF"])
                    .extra_wire("CLKIN2_MUX_DUMMY0", &["GND_WIRE1"]),
            );
            bels.push(
                self.builder
                    .bel_xy(bels::ABUS_SWITCH_CMT, "ABUS_SWITCH", 0, 0),
            );

            // XIPHY
            for i in 0..52 {
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
                        "TX_REGRST_B",
                        "TX_RST_B",
                        "TX_Q",
                        "RX_CLK_C",
                        "RX_CLK_C_B",
                        "RX_CLK_P",
                        "RX_CLK_N",
                        "RX_CTRL_CLK",
                        "RX_CTRL_CE",
                        "RX_CTRL_INC",
                        "RX_CTRL_LD",
                        "RX_RST_B",
                        "RX_CLKDIV",
                        "RX_DCC0",
                        "RX_DCC1",
                        "RX_DCC2",
                        "RX_DCC3",
                        "RX_VTC_READY",
                        "RX_RESET_B",
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
                    .extra_wire(
                        "DYN_DCI_OUT",
                        &[match i {
                            0..=11 => format!("DYNAMIC_DCI_TS_BOT{i}"),
                            12 => "DYNAMIC_DCI_TS_BOT_VR1".to_string(),
                            13..=24 => format!("DYNAMIC_DCI_TS_BOT{ii}", ii = i - 1),
                            25 => "DYNAMIC_DCI_TS_BOT_VR2".to_string(),
                            26..=37 => format!("DYNAMIC_DCI_TS_TOP{ii}", ii = i - 26),
                            38 => "DYNAMIC_DCI_TS_TOP_VR1".to_string(),
                            39..=50 => format!("DYNAMIC_DCI_TS_TOP{ii}", ii = i - 27),
                            51 => "DYNAMIC_DCI_TS_TOP_VR2".to_string(),
                            _ => unreachable!(),
                        }],
                    )
                    .extra_int_in(
                        "DYN_DCI_OUT_INT",
                        &[match i {
                            0..=11 => format!("CLB2PHY_DYNAMIC_DCI_TS_BOT{i}"),
                            12 => "CLB2PHY_DYNAMIC_DCI_TS_BOT_VR1".to_string(),
                            13..=24 => format!("CLB2PHY_DYNAMIC_DCI_TS_BOT{ii}", ii = i - 1),
                            25 => "CLB2PHY_DYNAMIC_DCI_TS_BOT_VR2".to_string(),
                            26..=37 => format!("CLB2PHY_DYNAMIC_DCI_TS_TOP{ii}", ii = i - 26),
                            38 => "CLB2PHY_DYNAMIC_DCI_TS_TOP_VR1".to_string(),
                            39..=50 => format!("CLB2PHY_DYNAMIC_DCI_TS_TOP{ii}", ii = i - 27),
                            51 => "CLB2PHY_DYNAMIC_DCI_TS_TOP_VR2".to_string(),
                            _ => unreachable!(),
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
            for i in 0..8 {
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
                        "RST_B",
                        "REGRST_B",
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
            for i in 0..8 {
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
                        "CLB2PHY_CTRL_RST_B",
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
                    ]);
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
            for i in 0..8 {
                bels.push(
                    self.builder
                        .bel_xy(bels::PLL_SELECT[i], "PLL_SELECT_SITE", 0, i ^ 1)
                        .pins_name_only(&["REFCLK_DFD", "Z", "PLL_CLK_EN"])
                        .pin_name_only("D0", 1)
                        .pin_name_only("D1", 1),
                );
            }
            for i in 0..4 {
                let mut bel = self
                    .builder
                    .bel_xy(bels::RIU_OR[i], "RIU_OR", 0, i)
                    .pins_name_only(&["RIU_RD_VALID_LOW", "RIU_RD_VALID_UPP"]);
                for i in 0..16 {
                    bel = bel.pins_name_only(&[
                        format!("RIU_RD_DATA_LOW{i}"),
                        format!("RIU_RD_DATA_UPP{i}"),
                    ]);
                }
                bels.push(bel);
            }
            for i in 0..4 {
                let mut bel = self
                    .builder
                    .bel_xy(bels::XIPHY_FEEDTHROUGH[i], "XIPHY_FEEDTHROUGH", i, 0)
                    .pins_name_only(&[
                        "CLB2PHY_CTRL_RST_B_LOW_SMX",
                        "CLB2PHY_CTRL_RST_B_UPP_SMX",
                        "CLB2PHY_TRISTATE_ODELAY_RST_B_SMX0",
                        "CLB2PHY_TRISTATE_ODELAY_RST_B_SMX1",
                        "CLB2PHY_TXBIT_TRI_RST_B_SMX0",
                        "CLB2PHY_TXBIT_TRI_RST_B_SMX1",
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
                        format!("CLB2PHY_TXBIT_RST_B_SMX{i}"),
                        format!("CLB2PHY_RXBIT_RST_B_SMX{i}"),
                        format!("CLB2PHY_FIFO_CLK_SMX{i}"),
                        format!("CLB2PHY_IDELAY_RST_B_SMX{i}"),
                        format!("CLB2PHY_ODELAY_RST_B_SMX{i}"),
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
            }

            let mut bel = self.builder.bel_virtual(bels::CMT);
            for i in 0..4 {
                bel = bel.extra_wire(format!("CCIO{i}"), &[format!("IOB2CLK_CCIO{i}")]);
            }
            for i in 0..24 {
                let dummy_base = [
                    0, 3, 36, 53, 56, 59, 62, 65, 68, 71, 6, 9, 12, 15, 18, 21, 24, 27, 30, 33, 39,
                    42, 45, 48,
                ][i];
                bel = bel
                    .extra_wire(format!("VDISTR{i}_B"), &[format!("CLK_VDISTR_BOT{i}")])
                    .extra_wire(format!("VDISTR{i}_T"), &[format!("CLK_VDISTR_TOP{i}")])
                    .extra_wire(format!("HROUTE{i}_L"), &[format!("CLK_HROUTE_0_{i}")])
                    .extra_wire(format!("HROUTE{i}_R"), &[format!("CLK_HROUTE_1_{i}")])
                    .extra_wire(format!("HDISTR{i}_L"), &[format!("CLK_HDISTR_0_{i}")])
                    .extra_wire(format!("HDISTR{i}_R"), &[format!("CLK_HDISTR_1_{i}")])
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
                        format!("HROUTE{i}_L_MUX"),
                        &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 3 + i * 8)],
                    )
                    .extra_wire(
                        format!("HROUTE{i}_R_MUX"),
                        &[format!("CLK_CMT_MUX_2TO1_{ii}_CLK_OUT", ii = 2 + i * 8)],
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
            }
            for i in 0..6 {
                bel = bel
                    .extra_wire(
                        format!("XIPHY_CLK{i}_B"),
                        &[format!("CLK_LEAF_MUX_XIPHY_{ii}_CLK_LEAF", ii = i + 6)],
                    )
                    .extra_wire(
                        format!("XIPHY_CLK{i}_T"),
                        &[format!("CLK_LEAF_MUX_XIPHY_{i}_CLK_LEAF")],
                    );
            }
            bels.push(bel);
            bels.push(
                self.builder
                    .bel_virtual(bels::VCC_CMT)
                    .extra_wire("VCC", &["VCC_WIRE"]),
            );
            let mut xn = self
                .builder
                .xnode(tslots::BEL, "XIPHY", "XIPHY", xy)
                .num_tiles(60)
                .ref_xlat(int_xy.delta(0, 30), &[Some(30), None, None, None], rclk_int);
            for i in 0..60 {
                xn = xn
                    .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), Dir::W, i)
                    .ref_single(int_xy.delta(-1, (i + i / 30) as i32), i, intf)
            }
            xn.bels(bels).extract();
        }
    }

    fn fill_tiles_hpio(&mut self) {
        if let Some(&xy) = self.builder.rd.tiles_by_kind_name("HPIO_L").iter().next() {
            let int_xy = self.builder.walk_to_int(xy, Dir::E, false).unwrap();
            let intf = self.builder.ndb.get_tile_class_naming("INTF.W.IO");
            let mut bels = vec![];
            let mut is_nocfg = true;
            if let Some(wire) = self.builder.rd.wires.get("HPIO_IOB_3_TSDI_PIN") {
                let tk = &self.builder.rd.tile_kinds[self.builder.rd.tiles[&xy].kind];
                if tk.wires.contains_key(&wire) {
                    is_nocfg = false;
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
                        "CTLE_IN",
                        "DOUT",
                        "IO",
                        "LVDS_TRUE",
                        "PAD_RES",
                        "O_B",
                        "TSTATEB",
                        "DYNAMIC_DCI_TS",
                        "VREF",
                    ])
                    .pin_name_only("SWITCH_OUT", usize::from(matches!(i, 4..=11 | 13..=20)))
                    .pin_name_only("OP", 1)
                    .pin_name_only("TSP", 1)
                    .pin_name_only("TSDI", 1);
                if is_nocfg || i == 25 {
                    bel = bel.pin_name_only("TSDI", 0);
                }
                if matches!(i, 12 | 25) {
                    bel = bel.pin_dummy("IO");
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
                            "CTLE_IN_1",
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
            bels.push(
                self.builder
                    .bel_xy(bels::HPIO_VREF, "HPIO_VREF_SITE", 0, 0)
                    .pins_name_only(&["VREF1", "VREF2"]),
            );
            let naming = if is_nocfg { "HPIO.NOCFG" } else { "HPIO" };
            let mut xn = self
                .builder
                .xnode(tslots::IOB, "HPIO", naming, xy)
                .num_tiles(30);
            for i in 0..30 {
                xn = xn
                    .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), Dir::W, i)
                    .ref_single(int_xy.delta(-1, (i + i / 30) as i32), i, intf)
            }
            xn.bels(bels).extract();
        }

        if let Some(&xy) = self
            .builder
            .rd
            .tiles_by_kind_name("RCLK_HPIO_L")
            .iter()
            .next()
        {
            let int_xy = self
                .builder
                .walk_to_int(xy.delta(0, -30), Dir::E, false)
                .unwrap();
            let intf = self.builder.ndb.get_tile_class_naming("INTF.W.IO");
            let mut bels = vec![];
            for i in 0..5 {
                bels.push(
                    self.builder
                        .bel_xy(bels::ABUS_SWITCH_HPIO[i], "ABUS_SWITCH", i, 0),
                );
            }
            bels.push(
                self.builder
                    .bel_xy(bels::HPIO_ZMATCH, "HPIO_ZMATCH_BLK_HCLK", 0, 0),
            );
            let mut xn = self
                .builder
                .xnode(tslots::RCLK_IOB, "RCLK_HPIO", "RCLK_HPIO", xy)
                .num_tiles(60);
            for i in 0..60 {
                xn = xn
                    .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), Dir::W, i)
                    .ref_single(int_xy.delta(-1, (i + i / 30) as i32), i, intf)
            }
            xn.bels(bels).extract();
        }
    }

    fn fill_tiles_hrio(&mut self) {
        if let Some(&xy) = self.builder.rd.tiles_by_kind_name("HRIO_L").iter().next() {
            let int_xy = self.builder.walk_to_int(xy, Dir::E, false).unwrap();
            let intf = self.builder.ndb.get_tile_class_naming("INTF.W.IO");
            let mut bels = vec![];
            let mut is_nocfg = true;
            if let Some(wire) = self.builder.rd.wires.get("HRIO_IOB_5_TSDI_PIN") {
                let tk = &self.builder.rd.tile_kinds[self.builder.rd.tiles[&xy].kind];
                if tk.wires.contains_key(&wire) {
                    is_nocfg = false;
                }
            }
            for i in 0..26 {
                let mut bel = self
                    .builder
                    .bel_xy(bels::HRIOB[i], "IOB", 0, i)
                    .pins_name_only(&[
                        "DOUT",
                        "OUTB_B_IN",
                        "OUTB_B",
                        "TSTATEIN",
                        "TSTATEOUT",
                        "IO",
                        "TMDS_IBUF_OUT",
                        "DRIVER_BOT_IBUF",
                        "O_B",
                        "TSTATEB",
                        "DYNAMIC_DCI_TS",
                    ])
                    .pin_name_only("SWITCH_OUT", usize::from(matches!(i, 4..=11 | 13..=20)))
                    .pin_name_only("OP", 1)
                    .pin_name_only("TSP", 1)
                    .pin_name_only("TSDI", 1);
                if is_nocfg || i == 25 {
                    bel = bel.pin_name_only("TSDI", 0);
                }
                if matches!(i, 12 | 25) {
                    bel = bel.pin_dummy("IO");
                }
                bels.push(bel);
            }
            for i in 0..12 {
                bels.push(
                    self.builder
                        .bel_xy(bels::HRIOB_DIFF_IN[i], "HRIODIFFINBUF", 0, i)
                        .pins_name_only(&[
                            "LVDS_IBUF_OUT",
                            "LVDS_IBUF_OUT_B",
                            "LVDS_IN_P",
                            "LVDS_IN_N",
                        ]),
                );
            }
            for i in 0..12 {
                bels.push(
                    self.builder
                        .bel_xy(bels::HRIOB_DIFF_OUT[i], "HRIODIFFOUTBUF", 0, i)
                        .pins_name_only(&["AOUT", "BOUT", "O_B", "TSTATEB"]),
                );
            }
            let naming = if is_nocfg { "HRIO.NOCFG" } else { "HRIO" };
            let mut xn = self
                .builder
                .xnode(tslots::IOB, "HRIO", naming, xy)
                .num_tiles(30);
            for i in 0..30 {
                xn = xn
                    .ref_int_side(int_xy.delta(0, (i + i / 30) as i32), Dir::W, i)
                    .ref_single(int_xy.delta(-1, (i + i / 30) as i32), i, intf)
            }
            xn.bels(bels).extract();
        }

        if let Some(&xy) = self
            .builder
            .rd
            .tiles_by_kind_name("RCLK_HRIO_L")
            .iter()
            .next()
        {
            let mut bels = vec![];
            for i in 0..8 {
                bels.push(
                    self.builder
                        .bel_xy(bels::ABUS_SWITCH_HRIO[i], "ABUS_SWITCH", i, 0),
                );
            }
            self.builder
                .xnode(tslots::RCLK_IOB, "RCLK_HRIO", "RCLK_HRIO", xy)
                .num_tiles(0)
                .bels(bels)
                .extract();
        }
    }

    fn fill_tiles_rclk(&mut self) {
        for (node, tkn) in [
            ("RCLK_HROUTE_SPLITTER.HARD", "PCIE"),
            ("RCLK_HROUTE_SPLITTER.HARD", "CMAC_CMAC_FT"),
            ("RCLK_HROUTE_SPLITTER.HARD", "ILMAC_ILMAC_FT"),
            ("RCLK_HROUTE_SPLITTER.HARD", "CFG_CFG"),
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
            .tiles_by_kind_name("RCLK_DSP_CLKBUF_L")
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

        for (tkn, side) in [
            ("RCLK_CLEL_L", Dir::W),
            ("RCLK_CLEL_R", Dir::W),
            ("RCLK_CLEL_R_L", Dir::E),
            ("RCLK_CLEL_R_R", Dir::E),
            ("RCLK_CLE_M_L", Dir::W),
            ("RCLK_CLE_M_R", Dir::W),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let is_alt = self.dev_naming.rclk_alt_pins[tkn];
                let rclk_int = self.builder.ndb.get_tile_class_naming("RCLK_INT");
                let int_xy = xy.delta(if side == Dir::W { 1 } else { -1 }, 0);
                let bels = vec![
                    self.builder
                        .bel_xy(bels::BUFCE_ROW_RCLK0, "BUFCE_ROW", 0, 0)
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
                        .extra_wire("HROUTE_MUX", &["CLK_CMT_MUX_2TO1_1_CLK_OUT"]),
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
                        "RCLK_V_SINGLE.CLE",
                        if is_alt {
                            "RCLK_V_SINGLE.ALT"
                        } else {
                            "RCLK_V_SINGLE"
                        },
                        xy,
                    )
                    .ref_xlat(
                        int_xy,
                        &if side == Dir::W {
                            [Some(0), None, None, None]
                        } else {
                            [None, Some(0), None, None]
                        },
                        rclk_int,
                    )
                    .bels(bels)
                    .extract();
            }
        }

        for (tkn, kind, side) in [
            ("RCLK_BRAM_L", "RCLK_V_DOUBLE.BRAM", Dir::W),
            ("RCLK_BRAM_R", "RCLK_V_DOUBLE.BRAM", Dir::W),
            ("RCLK_RCLK_BRAM_L_AUXCLMP_FT", "RCLK_V_DOUBLE.BRAM", Dir::W),
            ("RCLK_RCLK_BRAM_L_BRAMCLMP_FT", "RCLK_V_DOUBLE.BRAM", Dir::W),
            ("RCLK_DSP_L", "RCLK_V_DOUBLE.DSP", Dir::E),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let is_alt = self.dev_naming.rclk_alt_pins[tkn];
                let rclk_int = self.builder.ndb.get_tile_class_naming("RCLK_INT");
                let int_xy = xy.delta(if side == Dir::W { 2 } else { -2 }, 0);
                let mut bels = vec![];
                for i in 0..2 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::BUFCE_ROW_RCLK[i], "BUFCE_ROW", i, 0)
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
                        kind,
                        if is_alt {
                            "RCLK_V_DOUBLE.ALT"
                        } else {
                            "RCLK_V_DOUBLE"
                        },
                        xy,
                    )
                    .ref_xlat(
                        int_xy,
                        &if side == Dir::W {
                            [Some(0), None, None, None]
                        } else {
                            [None, Some(0), None, None]
                        },
                        rclk_int,
                    )
                    .bels(bels)
                    .extract();
            }
        }
    }

    fn fill_tiles_gt(&mut self) {
        for (common, channel, tkn, kind, naming, side) in [
            (
                bels::GTH_COMMON,
                bels::GTH_CHANNEL,
                "GTH_QUAD_LEFT_FT",
                "GTH",
                "GTH_L",
                Dir::W,
            ),
            (
                bels::GTY_COMMON,
                bels::GTY_CHANNEL,
                "GTY_QUAD_LEFT_FT",
                "GTY",
                "GTY_L",
                Dir::W,
            ),
            (
                bels::GTH_COMMON,
                bels::GTH_CHANNEL,
                "GTH_R",
                "GTH",
                "GTH_R",
                Dir::E,
            ),
        ] {
            if let Some(&xy) = self.builder.rd.tiles_by_kind_name(tkn).iter().next() {
                let int_xy = self.builder.walk_to_int(xy, !side, false).unwrap();
                let intf = self.builder.ndb.get_tile_class_naming(if side == Dir::W {
                    "INTF.W.GT"
                } else {
                    "INTF.E.GT"
                });
                let rclk_int = self.builder.ndb.get_tile_class_naming("RCLK_INT");
                let gtk = &kind[..3];
                let mut bels = vec![];
                for i in 0..24 {
                    let bi = [
                        (0, 5, 60),
                        (135, 140, 145),
                        (215, 220, 230),
                        (235, 240, 245),
                        (250, 255, 260),
                        (265, 270, 275),
                        (285, 290, 295),
                        (300, 305, 310),
                        (315, 320, 325),
                        (330, 340, 345),
                        (115, 170, 225),
                        (280, 335, 350),
                        (355, 10, 15),
                        (20, 25, 30),
                        (35, 40, 45),
                        (50, 55, 65),
                        (70, 75, 80),
                        (85, 90, 95),
                        (100, 105, 110),
                        (120, 125, 130),
                        (150, 155, 160),
                        (165, 175, 180),
                        (185, 190, 195),
                        (200, 205, 210),
                    ][i];
                    let mut bel = self
                        .builder
                        .bel_xy(bels::BUFG_GT[i], "BUFG_GT", 0, i)
                        .pins_name_only(&["CLK_IN", "CLK_OUT", "CE", "RST_PRE_OPTINV"]);
                    for j in 0..5 {
                        bel = bel
                            .extra_wire(
                                format!("CE_MUX_DUMMY{j}"),
                                &[format!("VCC_WIRE{ii}", ii = bi.0 + j)],
                            )
                            .extra_wire(
                                format!("CLK_IN_MUX_DUMMY{j}"),
                                &[format!("VCC_WIRE{ii}", ii = bi.1 + j)],
                            )
                            .extra_wire(
                                format!("RST_MUX_DUMMY{j}"),
                                &[format!("VCC_WIRE{ii}", ii = bi.2 + j)],
                            );
                    }
                    bels.push(bel);
                }
                for i in 0..11 {
                    let mut bel = self
                        .builder
                        .bel_xy(bels::BUFG_GT_SYNC[i], "BUFG_GT_SYNC", 0, i)
                        .pins_name_only(&["CE_OUT", "RST_OUT"]);
                    if i != 10 {
                        bel = bel.pins_name_only(&["CLK_IN"]);
                    }
                    bels.push(bel);
                }
                for i in 0..4 {
                    bels.push(
                        self.builder
                            .bel_xy(bels::ABUS_SWITCH_GT[i], "ABUS_SWITCH", 0, i),
                    );
                }

                for i in 0..4 {
                    bels.push(
                        self.builder
                            .bel_xy(channel[i], &format!("{gtk}E3_CHANNEL"), 0, i)
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
                            ]),
                    );
                }
                bels.push(
                    self.builder
                        .bel_xy(common, &format!("{gtk}E3_COMMON"), 0, 0)
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
                                "GTY_CHANNEL_BLH_1_NORTHREFCLK0",
                            ],
                        )
                        .extra_wire(
                            "NORTHREFCLK1",
                            &[
                                "GTH_CHANNEL_BLH_0_NORTHREFCLK1",
                                "GTY_CHANNEL_BLH_1_NORTHREFCLK1",
                            ],
                        )
                        .extra_wire(
                            "SOUTHREFCLK0",
                            &[
                                "GTH_CHANNEL_BLH_0_SOUTHREFCLK0",
                                "GTY_CHANNEL_BLH_1_SOUTHREFCLK0",
                            ],
                        )
                        .extra_wire(
                            "SOUTHREFCLK1",
                            &[
                                "GTH_CHANNEL_BLH_0_SOUTHREFCLK1",
                                "GTY_CHANNEL_BLH_1_SOUTHREFCLK1",
                            ],
                        ),
                );
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

    for bslot in maker.builder.db.bel_slots.values_mut() {
        if bslot.tile_slot == tslots::CMT {
            bslot.tile_slot = tslots::BEL;
        }
    }

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
    maker.fill_tiles_hard();
    maker.fill_tiles_cfg();
    maker.fill_tiles_xiphy();
    maker.fill_tiles_hpio();
    maker.fill_tiles_hrio();
    maker.fill_tiles_rclk();
    maker.fill_tiles_gt();

    maker.builder.build()
}
