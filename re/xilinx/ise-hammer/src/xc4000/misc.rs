use prjcombine_interconnect::grid::{CellCoord, DieId, TileCoord};
use prjcombine_re_fpga_hammer::{FeatureId, FuzzerFeature, FuzzerProp, xlat_bit, xlat_enum};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_xc2000::{bels::xc4000 as bels, chip::ChipKind, tslots};
use unnamed_entity::EntityId;

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::DynProp,
    },
};

#[derive(Clone, Debug)]
struct ExtraTilesIoW;

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ExtraTilesIoW {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        for (tile_class, locs) in &backend.egrid.tile_index {
            let tile = backend.egrid.db.tile_classes.key(tile_class);
            if !tile.starts_with("IO.L") {
                continue;
            }
            for &tcrd in locs {
                let tile = &backend.egrid[tcrd];
                let tile = backend.egrid.db.tile_classes.key(tile.class);
                let fuzzer_id = fuzzer.info.features[0].id.clone();
                fuzzer.info.features.push(FuzzerFeature {
                    id: FeatureId {
                        tile: tile.into(),
                        ..fuzzer_id
                    },
                    tiles: vec![backend.edev.tile_bits(tcrd)[0]],
                });
            }
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
struct ExtraTilesAllIo;

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ExtraTilesAllIo {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        for (tile_class, locs) in &backend.egrid.tile_index {
            let tile = backend.egrid.db.tile_classes.key(tile_class);
            if !tile.starts_with("IO") {
                continue;
            }
            for &tcrd in locs {
                let tile = &backend.egrid[tcrd];
                let tile = backend.egrid.db.tile_classes.key(tile.class);
                let fuzzer_id = fuzzer.info.features[0].id.clone();
                fuzzer.info.features.push(FuzzerFeature {
                    id: FeatureId {
                        tile: tile.into(),
                        ..fuzzer_id
                    },
                    tiles: vec![backend.edev.tile_bits(tcrd)[0]],
                });
            }
        }
        Some((fuzzer, false))
    }
}

#[derive(Clone, Debug)]
pub struct ExtraTileSingle {
    pub tcrd: TileCoord,
}

impl ExtraTileSingle {
    pub fn new(tcrd: TileCoord) -> Self {
        Self { tcrd }
    }
}

impl<'b> FuzzerProp<'b, IseBackend<'b>> for ExtraTileSingle {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        _tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let tile = backend
            .egrid
            .db
            .tile_classes
            .key(backend.egrid[self.tcrd].class);
        let fuzzer_id = fuzzer.info.features[0].id.clone();
        fuzzer.info.features.push(FuzzerFeature {
            id: FeatureId {
                tile: tile.into(),
                ..fuzzer_id
            },
            tiles: vec![backend.edev.tile_bits(self.tcrd)[0]],
        });
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Xc2000(edev) = backend.edev else {
        unreachable!()
    };

    {
        let mut ctx = FuzzCtx::new(session, backend, "CNR.BL");
        for val in ["ENABLE", "DISABLE"] {
            ctx.test_manual("MISC", "READ_ABORT", val)
                .global("READABORT", val)
                .commit();
            ctx.test_manual("MISC", "READ_CAPTURE", val)
                .global("READCAPTURE", val)
                .commit();
        }
        for val in ["ON", "OFF"] {
            ctx.test_manual("MISC", "TM_BOT", val)
                .global("TMBOT", val)
                .commit();
        }
        if matches!(edev.chip.kind, ChipKind::Xc4000Xla | ChipKind::Xc4000Xv) {
            for val in ["ON", "OFF"] {
                ctx.test_manual("MISC", "5V_TOLERANT_IO", val)
                    .global("5V_TOLERANT_IO", val)
                    .commit();
            }
        }
        for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
            ctx.test_manual("MD0", "PULL", val)
                .global("M0PIN", val)
                .commit();
        }
        for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
            ctx.test_manual("MD1", "PULL", val)
                .global("M1PIN", val)
                .commit();
        }
        for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
            let opt = if edev.chip.kind == ChipKind::SpartanXl {
                "POWERDOWN"
            } else {
                "M2PIN"
            };
            ctx.test_manual("MD2", "PULL", val)
                .global(opt, val)
                .commit();
        }
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, "CNR.TL");
        for val in ["ON", "OFF"] {
            ctx.test_manual("MISC", "TM_LEFT", val)
                .global("TMLEFT", val)
                .commit();
        }
        for val in ["ON", "OFF"] {
            ctx.test_manual("MISC", "TM_TOP", val)
                .global("TMTOP", val)
                .commit();
        }
        for val in ["TTL", "CMOS"] {
            ctx.test_manual("MISC", "INPUT", val)
                .global("INPUT", val)
                .commit();
            ctx.test_manual("MISC", "OUTPUT", val)
                .global("OUTPUT", val)
                .commit();
        }
        if edev.chip.kind != ChipKind::Xc4000E {
            for val in ["ON", "OFF"] {
                ctx.test_manual("MISC", "3V", val)
                    .global("3V", val)
                    .commit();
            }
        }
        let mut bctx = ctx.bel(bels::BSCAN);
        let tcrd = CellCoord::new(DieId::from_idx(0), edev.chip.col_e(), edev.chip.row_n())
            .tile(tslots::MAIN);
        bctx.build()
            .extra_tile_fixed(tcrd, "BSCAN")
            .test_manual("ENABLE", "1")
            .mode("BSCAN")
            .attr("BSCAN", "USED")
            .commit();
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            for val in ["ENABLE", "DISABLE"] {
                bctx.test_manual("CONFIG", val)
                    .global("BSCAN_CONFIG", val)
                    .commit();
            }
        }
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, "CNR.BR");
        for val in ["ON", "OFF"] {
            ctx.test_manual("MISC", "TCTEST", val)
                .global("TCTEST", val)
                .commit();
        }
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv
        ) {
            for val in ["ON", "OFF"] {
                ctx.test_manual("MISC", "FIX_DISCHARGE", val)
                    .global("FIXDISCHARGE", val)
                    .commit();
            }
        }
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            for val in ["ON", "OFF"] {
                ctx.test_manual("OSC", "TM_OSC", val)
                    .global("TMOSC", val)
                    .commit();
            }
            for val in ["CCLK", "EXTCLK"] {
                ctx.test_manual("OSC", "OSC_CLK", val)
                    .global("OSCCLK", val)
                    .commit();
            }
        }
        let mut bctx = ctx.bel(bels::STARTUP);
        bctx.mode("STARTUP")
            .pin("GSR")
            .test_manual("INV.GSR", "1")
            .attr("GSRATTR", "NOT")
            .commit();
        bctx.mode("STARTUP")
            .pin("GTS")
            .test_manual("INV.GTS", "1")
            .attr("GTSATTR", "NOT")
            .commit();
        for val in ["ENABLE", "DISABLE"] {
            bctx.test_manual("CRC", val).global("CRC", val).commit();
        }
        for val in ["SLOW", "FAST"] {
            bctx.test_manual("CONFIG_RATE", val)
                .global("CONFIGRATE", val)
                .commit();
        }
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            for val in ["ENABLE", "DISABLE"] {
                bctx.build()
                    .global("CRC", "DISABLE")
                    .test_manual("EXPRESS_MODE", val)
                    .global("EXPRESSMODE", val)
                    .commit();
            }
        }
        for (val, phase) in [("CCLK", "C4"), ("USERCLK", "U3")] {
            bctx.mode("STARTUP")
                .pin("CLK")
                .global("SYNCTODONE", "NO")
                .global("DONEACTIVE", "C1")
                .test_manual("STARTUP_CLK", val)
                .global_diff("GSRINACTIVE", "C4", phase)
                .global_diff("OUTPUTSACTIVE", "C4", phase)
                .global_diff("STARTUPCLK", "CCLK", val)
                .commit();
        }
        for (val, phase) in [("NO", "C4"), ("YES", "DI_PLUS_1")] {
            bctx.build()
                .global("STARTUPCLK", "CCLK")
                .global("DONEACTIVE", "C1")
                .test_manual("SYNC_TO_DONE", val)
                .global_diff("GSRINACTIVE", "C4", phase)
                .global_diff("OUTPUTSACTIVE", "C4", phase)
                .global_diff("SYNCTODONE", "NO", val)
                .commit();
        }
        for val in ["C1", "C2", "C3", "C4"] {
            bctx.build()
                .global("SYNCTODONE", "NO")
                .global("STARTUPCLK", "CCLK")
                .test_manual("DONE_ACTIVE", val)
                .global_diff("DONEACTIVE", "C1", val)
                .commit();
        }
        for val in ["U2", "U3", "U4"] {
            bctx.mode("STARTUP")
                .pin("CLK")
                .global("SYNCTODONE", "NO")
                .global("STARTUPCLK", "USERCLK")
                .test_manual("DONE_ACTIVE", val)
                .global_diff("DONEACTIVE", "C1", val)
                .commit();
        }
        for (attr, opt) in [
            ("OUTPUTS_ACTIVE", "OUTPUTSACTIVE"),
            ("GSR_INACTIVE", "GSRINACTIVE"),
        ] {
            for val in ["C2", "C3", "C4"] {
                bctx.build()
                    .global("SYNCTODONE", "NO")
                    .global("STARTUPCLK", "CCLK")
                    .test_manual(attr, val)
                    .global_diff(opt, "C4", val)
                    .commit();
            }
            for val in ["U2", "U3", "U4"] {
                bctx.mode("STARTUP")
                    .pin("CLK")
                    .global("SYNCTODONE", "NO")
                    .global("STARTUPCLK", "USERCLK")
                    .test_manual(attr, val)
                    .global_diff(opt, "U3", val)
                    .commit();
            }
            for val in ["DI", "DI_PLUS_1", "DI_PLUS_2"] {
                bctx.mode("STARTUP")
                    .pin("CLK")
                    .global("SYNCTODONE", "YES")
                    .global("STARTUPCLK", "USERCLK")
                    .test_manual(attr, val)
                    .global_diff(opt, "DI_PLUS_1", val)
                    .commit();
            }
        }
        for val in ["PULLUP", "PULLNONE"] {
            ctx.test_manual("DONE", "PULL", val)
                .global("DONEPIN", val)
                .commit();
        }
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, "CNR.TR");
        let mut bctx = ctx.bel(bels::OSC);
        let cnr_br = CellCoord::new(DieId::from_idx(0), edev.chip.col_e(), edev.chip.row_s())
            .tile(tslots::MAIN);
        for (pin, opin) in [("OUT0", "OUT1"), ("OUT1", "OUT0")] {
            for spin in ["F15", "F490", "F16K", "F500K"] {
                bctx.build()
                    .mutex("MODE", "USE")
                    .mutex(format!("MUX.{pin}"), spin)
                    .mutex(format!("MUX.{opin}"), spin)
                    .pip(opin, spin)
                    .extra_tile_fixed(cnr_br, "OSC")
                    .null_bits()
                    .test_manual(format!("MUX.{pin}"), spin)
                    .pip(pin, spin)
                    .commit();
            }
        }
        bctx.build()
            .mutex("MODE", "TEST")
            .extra_tile_fixed(cnr_br, "OSC")
            .null_bits()
            .test_manual("ENABLE", "1")
            .pip("OUT0", "F15")
            .commit();
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, "CNR.TR");
        for val in ["ON", "OFF"] {
            ctx.test_manual("MISC", "TM_RIGHT", val)
                .global("TMRIGHT", val)
                .commit();
        }
        if edev.chip.kind != ChipKind::Xc4000E {
            for val in ["ON", "OFF"] {
                ctx.test_manual("MISC", "TAC", val)
                    .global("TAC", val)
                    .commit();
            }
            for val in ["18", "22"] {
                ctx.test_manual("MISC", "ADDRESS_LINES", val)
                    .global("ADDRESSLINES", val)
                    .commit();
            }
        }

        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            for val in ["ENABLE", "DISABLE"] {
                ctx.test_manual("BSCAN", "STATUS", val)
                    .global("BSCAN_STATUS", val)
                    .commit();
            }
        }

        for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
            ctx.test_manual("TDO", "PULL", val)
                .global("TDOPIN", val)
                .commit();
        }
    }

    {
        let mut ctx = FuzzCtx::new(session, backend, "CNR.BR");
        let mut bctx = ctx.bel(bels::READCLK);
        for val in ["CCLK", "RDBK"] {
            let tcrd = CellCoord::new(DieId::from_idx(0), edev.chip.col_e(), edev.chip.row_n())
                .tile(tslots::MAIN);
            bctx.mode("READCLK")
                .pin("I")
                .extra_tile_fixed(tcrd, "READCLK")
                .null_bits()
                .test_manual("READ_CLK", val)
                .global("READCLK", val)
                .commit();
        }
    }

    {
        let tile = if edev.chip.kind.is_xl() {
            "LLVC.IO.R"
        } else {
            "LLV.IO.R"
        };
        let mut ctx = FuzzCtx::new(session, backend, tile);
        for val in ["OFF", "ON"] {
            ctx.test_manual("MISC", "TLC", val)
                .global("TLC", val)
                .commit();
        }
    }

    if edev.chip.kind == ChipKind::SpartanXl {
        let mut ctx = FuzzCtx::new_null(session, backend);
        let cnr_bl = CellCoord::new(DieId::from_idx(0), edev.chip.col_w(), edev.chip.row_s())
            .tile(tslots::MAIN);
        let cnr_br = CellCoord::new(DieId::from_idx(0), edev.chip.col_e(), edev.chip.row_s())
            .tile(tslots::MAIN);
        let cnr_tr = CellCoord::new(DieId::from_idx(0), edev.chip.col_e(), edev.chip.row_n())
            .tile(tslots::MAIN);
        ctx.build()
            .prop(ExtraTileSingle::new(cnr_bl))
            .prop(ExtraTileSingle::new(cnr_br))
            .prop(ExtraTileSingle::new(cnr_tr))
            .prop(ExtraTilesAllIo)
            .test_manual("MISC", "5V_TOLERANT_IO", "OFF")
            .global("5V_TOLERANT_IO", "OFF")
            .commit();
    }
    if edev.chip.kind == ChipKind::Xc4000Ex {
        let mut ctx = FuzzCtx::new_null(session, backend);
        for val in ["EXTERNAL", "INTERNAL"] {
            ctx.build()
                .prop(ExtraTilesIoW)
                .test_manual("MISC", "PUMP", val)
                .global("PUMP", val)
                .commit();
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Xc2000(edev) = ctx.edev else {
        unreachable!()
    };

    {
        let tile = "CNR.BL";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "READ_ABORT", "DISABLE", "ENABLE");
        ctx.collect_enum_bool(tile, bel, "READ_CAPTURE", "DISABLE", "ENABLE");
        ctx.collect_enum_bool(tile, bel, "TM_BOT", "OFF", "ON");
        if matches!(edev.chip.kind, ChipKind::Xc4000Xla | ChipKind::Xc4000Xv) {
            ctx.collect_enum_bool(tile, bel, "5V_TOLERANT_IO", "OFF", "ON");
        }
        ctx.collect_enum(tile, "MD0", "PULL", &["PULLUP", "PULLDOWN", "PULLNONE"]);
        ctx.collect_enum(tile, "MD1", "PULL", &["PULLUP", "PULLDOWN", "PULLNONE"]);
        ctx.collect_enum(tile, "MD2", "PULL", &["PULLUP", "PULLDOWN", "PULLNONE"]);
        if edev.chip.kind == ChipKind::SpartanXl {
            let mut diff = ctx.state.get_diff(tile, bel, "5V_TOLERANT_IO", "OFF");
            let diff_m0 = diff.split_bits_by(|bit| bit.frame == 21);
            let diff_m1 = diff.split_bits_by(|bit| bit.frame == 22);
            let diff_m2 = diff.split_bits_by(|bit| bit.frame == 20);
            diff.assert_empty();
            ctx.tiledb
                .insert(tile, "MD0", "5V_TOLERANT_IO", xlat_bit(!diff_m0));
            ctx.tiledb
                .insert(tile, "MD1", "5V_TOLERANT_IO", xlat_bit(!diff_m1));
            ctx.tiledb
                .insert(tile, "MD2", "5V_TOLERANT_IO", xlat_bit(!diff_m2));
        }
    }

    {
        let tile = "CNR.TL";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "TM_LEFT", "OFF", "ON");
        ctx.collect_enum_bool(tile, bel, "TM_TOP", "OFF", "ON");
        if edev.chip.kind != ChipKind::Xc4000E {
            ctx.collect_enum_bool(tile, bel, "3V", "OFF", "ON");
        }
        ctx.collect_enum(tile, bel, "INPUT", &["CMOS", "TTL"]);
        ctx.collect_enum(tile, bel, "OUTPUT", &["CMOS", "TTL"]);
        let bel = "BSCAN";
        ctx.collect_bit(tile, bel, "ENABLE", "1");
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            ctx.collect_enum_bool(tile, bel, "CONFIG", "DISABLE", "ENABLE");
        }
    }

    {
        let tile = "CNR.BR";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "TCTEST", "OFF", "ON");
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv
        ) {
            ctx.collect_enum_bool(tile, bel, "FIX_DISCHARGE", "OFF", "ON");
        }
        if edev.chip.kind == ChipKind::SpartanXl {
            let mut diff = ctx.state.get_diff(tile, bel, "5V_TOLERANT_IO", "OFF");
            let diff_prog = diff.split_bits_by(|bit| bit.frame == 8);
            let diff_done = diff.split_bits_by(|bit| bit.frame == 3);
            diff.assert_empty();
            ctx.tiledb
                .insert(tile, "PROG", "5V_TOLERANT_IO", xlat_bit(!diff_prog));
            ctx.tiledb
                .insert(tile, "DONE", "5V_TOLERANT_IO", xlat_bit(!diff_done));
        }

        let bel = "STARTUP";
        ctx.collect_enum_bool(tile, bel, "CRC", "DISABLE", "ENABLE");
        ctx.collect_enum(tile, bel, "CONFIG_RATE", &["SLOW", "FAST"]);
        ctx.collect_bit(tile, bel, "INV.GSR", "1");
        ctx.collect_bit(tile, bel, "INV.GTS", "1");
        let item = xlat_enum(vec![
            ("Q0", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "C1")),
            ("Q2", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "C3")),
            ("Q3", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "C4")),
            ("Q1Q4", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "C2")),
            ("Q2", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "U2")),
            ("Q3", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "U3")),
            ("Q1Q4", ctx.state.get_diff(tile, bel, "DONE_ACTIVE", "U4")),
        ]);
        ctx.tiledb.insert(tile, bel, "DONE_ACTIVE", item);
        for attr in ["OUTPUTS_ACTIVE", "GSR_INACTIVE"] {
            let item = xlat_enum(vec![
                ("DONE_IN", ctx.state.get_diff(tile, bel, attr, "DI")),
                ("Q3", ctx.state.get_diff(tile, bel, attr, "DI_PLUS_1")),
                ("Q1Q4", ctx.state.get_diff(tile, bel, attr, "DI_PLUS_2")),
                ("Q2", ctx.state.get_diff(tile, bel, attr, "C3")),
                ("Q3", ctx.state.get_diff(tile, bel, attr, "C4")),
                ("Q1Q4", ctx.state.get_diff(tile, bel, attr, "C2")),
                ("Q2", ctx.state.get_diff(tile, bel, attr, "U2")),
                ("Q3", ctx.state.get_diff(tile, bel, attr, "U3")),
                ("Q1Q4", ctx.state.get_diff(tile, bel, attr, "U4")),
            ]);
            ctx.tiledb.insert(tile, bel, attr, item);
        }
        ctx.collect_enum(tile, bel, "STARTUP_CLK", &["CCLK", "USERCLK"]);
        ctx.collect_enum_bool(tile, bel, "SYNC_TO_DONE", "NO", "YES");
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Ex | ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            ctx.collect_enum_bool(tile, bel, "EXPRESS_MODE", "DISABLE", "ENABLE");
        }
        let bel = "OSC";
        ctx.collect_bit(tile, bel, "ENABLE", "1");
        ctx.collect_enum(tile, bel, "MUX.OUT0", &["F500K", "F16K", "F490", "F15"]);
        ctx.collect_enum(tile, bel, "MUX.OUT1", &["F500K", "F16K", "F490", "F15"]);
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            ctx.collect_enum_bool(tile, bel, "TM_OSC", "OFF", "ON");
            ctx.collect_enum(tile, bel, "OSC_CLK", &["CCLK", "EXTCLK"]);
        }
        ctx.collect_enum(tile, "DONE", "PULL", &["PULLUP", "PULLNONE"]);
    }
    {
        let tile = "CNR.TR";
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "TM_RIGHT", "OFF", "ON");
        ctx.collect_enum(tile, "TDO", "PULL", &["PULLUP", "PULLDOWN", "PULLNONE"]);
        if edev.chip.kind != ChipKind::Xc4000E {
            ctx.collect_enum_bool(tile, bel, "TAC", "OFF", "ON");
            ctx.collect_enum(tile, bel, "ADDRESS_LINES", &["18", "22"]);
        }
        if edev.chip.kind == ChipKind::SpartanXl {
            let mut diff = ctx.state.get_diff(tile, bel, "5V_TOLERANT_IO", "OFF");
            let diff_tdo = diff.split_bits_by(|bit| bit.frame == 12);
            let diff_cclk = diff.split_bits_by(|bit| bit.frame == 13);
            diff.assert_empty();
            ctx.tiledb
                .insert(tile, "TDO", "5V_TOLERANT_IO", xlat_bit(!diff_tdo));
            ctx.tiledb
                .insert(tile, "CCLK", "5V_TOLERANT_IO", xlat_bit(!diff_cclk));
        }
        let bel = "BSCAN";
        ctx.collect_bit(tile, bel, "ENABLE", "1");
        if matches!(
            edev.chip.kind,
            ChipKind::Xc4000Xla | ChipKind::Xc4000Xv | ChipKind::SpartanXl
        ) {
            ctx.collect_enum_bool(tile, bel, "STATUS", "DISABLE", "ENABLE");
        }
        let bel = "READCLK";
        ctx.collect_enum(tile, bel, "READ_CLK", &["CCLK", "RDBK"]);
    }
    {
        let tile = if edev.chip.kind.is_xl() {
            "LLVC.IO.R"
        } else {
            "LLV.IO.R"
        };
        let bel = "MISC";
        ctx.collect_enum_bool(tile, bel, "TLC", "OFF", "ON");
    }
    if edev.chip.kind == ChipKind::SpartanXl {
        for tile in edev.egrid.db.tile_classes.keys() {
            if !tile.starts_with("IO") {
                continue;
            }
            if !ctx.has_tile(tile) {
                continue;
            }
            let mut diff = ctx.state.get_diff(tile, "MISC", "5V_TOLERANT_IO", "OFF");
            let (f0, f1) = if tile.starts_with("IO.L") {
                (19, 20)
            } else if tile.starts_with("IO.R") {
                (6, 5)
            } else if tile.starts_with("IO.B") || tile.starts_with("IO.T") {
                (13, 12)
            } else {
                unreachable!()
            };
            let diff_iob0 = diff.split_bits_by(|bit| bit.frame == f0);
            let diff_iob1 = diff.split_bits_by(|bit| bit.frame == f1);
            diff.assert_empty();
            ctx.tiledb
                .insert(tile, "IO0", "5V_TOLERANT_IO", xlat_bit(!diff_iob0));
            ctx.tiledb
                .insert(tile, "IO1", "5V_TOLERANT_IO", xlat_bit(!diff_iob1));
        }
    }
    if edev.chip.kind == ChipKind::Xc4000Ex {
        for tile in edev.egrid.db.tile_classes.keys() {
            if !tile.starts_with("IO.L") {
                continue;
            }
            if !ctx.has_tile(tile) {
                continue;
            }
            ctx.collect_enum(tile, "MISC", "PUMP", &["EXTERNAL", "INTERNAL"]);
        }
    }
}
