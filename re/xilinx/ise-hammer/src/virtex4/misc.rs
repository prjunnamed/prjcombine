use prjcombine_entity::EntityId;
use prjcombine_interconnect::{
    dir::{DirH, DirV},
    grid::{CellCoord, DieId, TileCoord},
};
use prjcombine_re_fpga_hammer::{
    Diff, FeatureId, FuzzerFeature, FuzzerProp, OcdMode, xlat_bit, xlat_bitvec, xlat_enum_ocd,
};
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{
    bits,
    bsdata::{TileBit, TileItem, TileItemKind},
};
use prjcombine_virtex4::{bels, tslots};
use prjcombine_xilinx_bitstream::Reg;

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::{
            DynProp,
            relation::{FixedRelation, TileRelation},
        },
    },
};

#[derive(Clone, Copy, Debug)]
struct HclkDcm;

impl TileRelation for HclkDcm {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        Some(
            tcrd.cell
                .with_cr(edev.col_clk, edev.chips[tcrd.die].row_hclk(tcrd.row))
                .tile(tslots::HCLK_BEL),
        )
    }
}

#[derive(Clone, Debug)]
struct MgtRepeater(DirH, DirV, String, &'static str);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for MgtRepeater {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let chip = edev.chips[tcrd.die];
        let rrow = match self.1 {
            DirV::S => chip.row_bufg() - 8,
            DirV::N => chip.row_bufg() + 8,
        };
        for &col in &edev.chips[DieId::from_idx(0)].cols_vbrk {
            if (col < edev.col_cfg) == (self.0 == DirH::W) {
                let rcol = if self.0 == DirH::W { col } else { col - 1 };
                let ntcrd = tcrd.with_cr(rcol, rrow).tile(tslots::CLK);
                fuzzer.info.features.push(FuzzerFeature {
                    id: FeatureId {
                        tile: "HCLK_MGT_REPEATER".into(),
                        bel: "HCLK_MGT_REPEATER".into(),
                        attr: self.2.clone(),
                        val: self.3.into(),
                    },
                    rects: edev.tile_bits(ntcrd),
                });
            }
        }
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };

    {
        let mut ctx = FuzzCtx::new(session, backend, "CFG");
        for val in ["0", "1", "2", "3"] {
            ctx.test_manual("MISC", "PROBESEL", val)
                .global("PROBESEL", val)
                .commit();
        }
        for attr in ["CCLKPIN", "DONEPIN", "POWERDOWNPIN", "PROGPIN", "INITPIN"] {
            for val in ["PULLUP", "PULLNONE"] {
                ctx.test_manual("MISC", attr, val)
                    .global(attr, val)
                    .commit();
            }
        }
        for attr in [
            "HSWAPENPIN",
            "M0PIN",
            "M1PIN",
            "M2PIN",
            "CSPIN",
            "DINPIN",
            "BUSYPIN",
            "RDWRPIN",
            "TCKPIN",
            "TDIPIN",
            "TDOPIN",
            "TMSPIN",
        ] {
            for val in ["PULLUP", "PULLDOWN", "PULLNONE"] {
                ctx.test_manual("MISC", attr, val)
                    .global(attr, val)
                    .commit();
            }
        }

        for i in 0..32 {
            let mut bctx = ctx.bel(bels::BUFGCTRL[i]);
            let mode = "BUFGCTRL";
            bctx.test_manual("PRESENT", "1").mode(mode).commit();
            for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
                bctx.mode(mode).test_inv(pin);
            }
            bctx.mode(mode)
                .test_enum("PRESELECT_I0", &["FALSE", "TRUE"]);
            bctx.mode(mode)
                .test_enum("PRESELECT_I1", &["FALSE", "TRUE"]);
            bctx.mode(mode).test_enum("CREATE_EDGE", &["FALSE", "TRUE"]);
            bctx.mode(mode).test_enum("INIT_OUT", &["0", "1"]);

            for midx in 0..2 {
                let bus = format!("MUXBUS{midx}");
                let mux = format!("MUX.I{midx}");
                let opin = format!("I{midx}MUX");
                for val in ["CKINT0", "CKINT1"] {
                    bctx.build()
                        .mutex("IxMUX", &mux)
                        .mutex(&mux, val)
                        .test_manual(&mux, val)
                        .pip(&opin, val)
                        .commit();
                }
                let mb_idx = 2 * (i % 16) + midx;
                let mb_out = format!("MUXBUS_O{mb_idx}");
                let clk_iob = CellCoord::new(
                    DieId::from_idx(0),
                    edev.col_clk,
                    if i < 16 {
                        edev.row_dcmiob.unwrap()
                    } else {
                        edev.row_iobdcm.unwrap() - 16
                    },
                )
                .tile(tslots::CLK);
                bctx.build()
                    .mutex("IxMUX", &mux)
                    .mutex(&mux, "MUXBUS")
                    .global_mutex("CLK_IOB_MUXBUS", "USE")
                    .related_pip(
                        FixedRelation(clk_iob),
                        (bels::CLK_IOB, mb_out),
                        (bels::CLK_IOB, "PAD_BUF0"),
                    )
                    .test_manual(&mux, "MUXBUS")
                    .pip(&opin, bus)
                    .commit();
                for j in 0..16 {
                    let obel = bels::BUFGCTRL[if i < 16 { j } else { j + 16 }];
                    let val = format!("GFB{j}");
                    bctx.build()
                        .mutex("IxMUX", &mux)
                        .mutex(&mux, &val)
                        .test_manual(&mux, &val)
                        .pip(&opin, (obel, "GFB"))
                        .commit();
                }
                for val in ["MGT_L0", "MGT_L1", "MGT_R0", "MGT_R1"] {
                    let obel = if i < 16 {
                        bels::BUFG_MGTCLK_S
                    } else {
                        bels::BUFG_MGTCLK_N
                    };
                    let obel_bufg = bels::BUFGCTRL[i ^ 1];
                    bctx.build()
                        .mutex("IxMUX", &mux)
                        .mutex(&mux, val)
                        .global_mutex("BUFG_MGTCLK", "USE")
                        .bel_mutex(obel_bufg, "IxMUX", &mux)
                        .bel_mutex(obel_bufg, &mux, val)
                        .pip((obel_bufg, &opin), (obel, val))
                        .test_manual(&mux, val)
                        .pip(&opin, (obel, val))
                        .commit();
                }
            }
            bctx.mode(mode)
                .global_mutex("BUFGCTRL_OUT", "TEST")
                .test_manual("ENABLE", "1")
                .pin("O")
                .commit();
            bctx.mode(mode)
                .global_mutex("BUFGCTRL_OUT", "TEST")
                .pin("O")
                .test_manual("PIN_O_GFB", "1")
                .pip("GFB", "O")
                .commit();
            let mut builder = bctx
                .mode(mode)
                .global_mutex("BUFGCTRL_OUT", "TEST")
                .global_mutex("BUFGCTRL_O_GCLK", format!("BUFGCTRL{i}"))
                .pin("O");
            if !matches!(i, 19 | 30) {
                builder =
                    builder.extra_tiles_attr_by_kind("CLK_TERM", "CLK_TERM", "GCLK_ENABLE", "1")
            }
            builder
                .test_manual("PIN_O_GCLK", "1")
                .pip("GCLK", "O")
                .commit();
        }

        for i in 0..4 {
            let mut bctx = ctx.bel(bels::BSCAN[i]);
            bctx.test_manual("PRESENT", "1").mode("BSCAN").commit();
        }
        ctx.test_manual("BSCAN_COMMON", "USERID", "").multi_global(
            "USERID",
            MultiValue::HexPrefix,
            32,
        );

        for i in 0..2 {
            let mut bctx = ctx.bel(bels::ICAP[i]);
            let obel = bels::ICAP[i ^ 1];
            bctx.build()
                .bel_unused(obel)
                .test_manual("PRESENT", "1")
                .mode("ICAP")
                .commit();
            bctx.mode("ICAP").test_inv("CLK");
            bctx.mode("ICAP").test_inv("CE");
            bctx.mode("ICAP").test_inv("WRITE");
            bctx.mode("ICAP")
                .bel_unused(obel)
                .test_enum("ICAP_WIDTH", &["X8", "X32"]);
        }

        {
            let mut bctx = ctx.bel(bels::PMV0);
            bctx.test_manual("PRESENT", "1").mode("PMV").commit();
        }

        {
            let mut bctx = ctx.bel(bels::STARTUP);
            bctx.test_manual("PRESENT", "1").mode("STARTUP").commit();
            for pin in [
                "CLK",
                "GTS",
                "GSR",
                "USRCCLKTS",
                "USRCCLKO",
                "USRDONETS",
                "USRDONEO",
            ] {
                bctx.mode("STARTUP").test_inv(pin);
            }
            bctx.mode("STARTUP")
                .no_pin("GSR")
                .test_manual("PIN.GTS", "1")
                .pin("GTS")
                .commit();
            bctx.mode("STARTUP")
                .no_pin("GTS")
                .test_manual("PIN.GSR", "1")
                .pin("GSR")
                .commit();
            bctx.mode("STARTUP")
                .test_manual("PIN.USRCCLKO", "1")
                .pin("USRCCLKO")
                .commit();
            for attr in ["GSR_SYNC", "GWE_SYNC", "GTS_SYNC"] {
                for val in ["YES", "NO"] {
                    bctx.test_manual(attr, val).global(attr, val).commit();
                }
            }
            for val in ["CCLK", "USERCLK", "JTAGCLK"] {
                bctx.mode("STARTUP")
                    .pin("CLK")
                    .null_bits()
                    .extra_tile_reg(Reg::Cor0, "REG.COR", "STARTUP")
                    .test_manual("STARTUPCLK", val)
                    .global("STARTUPCLK", val)
                    .commit();
            }
        }

        {
            let mut bctx = ctx.bel(bels::JTAGPPC);
            bctx.test_manual("PRESENT", "1").mode("JTAGPPC").commit();
        }

        {
            let mut bctx = ctx.bel(bels::FRAME_ECC);
            bctx.test_manual("PRESENT", "1").mode("FRAME_ECC").commit();
        }

        {
            let mut bctx = ctx.bel(bels::DCIRESET);
            bctx.test_manual("PRESENT", "1").mode("DCIRESET").commit();
        }

        {
            let mut bctx = ctx.bel(bels::CAPTURE);
            bctx.test_manual("PRESENT", "1").mode("CAPTURE").commit();
            bctx.mode("CAPTURE").test_inv("CLK");
            bctx.mode("CAPTURE").test_inv("CAP");
            bctx.mode("CAPTURE")
                .null_bits()
                .extra_tile_reg(Reg::Cor0, "REG.COR", "CAPTURE")
                .test_enum("ONESHOT", &["FALSE", "TRUE"]);
        }

        {
            let mut bctx = ctx.bel(bels::USR_ACCESS);
            bctx.test_manual("PRESENT", "1").mode("USR_ACCESS").commit();
        }

        if edev.col_lgt.is_some() {
            for bel in [bels::BUFG_MGTCLK_S_HROW, bels::BUFG_MGTCLK_N_HROW] {
                let mut bctx = ctx.bel(bel);
                for (name, o, i) in [
                    ("BUF.MGT_L0", "MGT_L0_O", "MGT_L0_I"),
                    ("BUF.MGT_L1", "MGT_L1_O", "MGT_L1_I"),
                    ("BUF.MGT_R0", "MGT_R0_O", "MGT_R0_I"),
                    ("BUF.MGT_R1", "MGT_R1_O", "MGT_R1_I"),
                ] {
                    bctx.build()
                        .global_mutex("BUFG_MGTCLK", "TEST")
                        .test_manual(name, "1")
                        .pip(o, i)
                        .commit();
                }
            }
            for (bel, dir_row) in [
                (bels::BUFG_MGTCLK_S_HCLK, DirV::S),
                (bels::BUFG_MGTCLK_N_HCLK, DirV::N),
            ] {
                let mut bctx = ctx.bel(bel);
                for (name, o, i) in [
                    ("MGT_L0", "MGT_L0_O", "MGT_L0_I"),
                    ("MGT_L1", "MGT_L1_O", "MGT_L1_I"),
                    ("MGT_R0", "MGT_R0_O", "MGT_R0_I"),
                    ("MGT_R1", "MGT_R1_O", "MGT_R1_I"),
                ] {
                    let idx = if name.ends_with('1') { 1 } else { 0 };
                    bctx.build()
                        .global_mutex("MGT_OUT", "USE")
                        .null_bits()
                        .prop(MgtRepeater(
                            if name.starts_with("MGT_L") {
                                DirH::W
                            } else {
                                DirH::E
                            },
                            dir_row,
                            format!("BUF.MGT{idx}.CFG"),
                            "1",
                        ))
                        .test_manual(name, "1")
                        .pip(o, i)
                        .commit();
                }
            }
        }
    }

    {
        let mut ctx = FuzzCtx::new_null(session, backend);
        for val in ["1", "2", "3", "4", "5", "6", "DONE", "KEEP"] {
            ctx.test_reg(Reg::Cor0, "REG.COR", "STARTUP", "GWE_CYCLE", val)
                .global("GWE_CYCLE", val)
                .commit();
            ctx.test_reg(Reg::Cor0, "REG.COR", "STARTUP", "GTS_CYCLE", val)
                .global("GTS_CYCLE", val)
                .commit();
        }
        for val in ["1", "2", "3", "4", "5", "6", "KEEP"] {
            ctx.test_reg(Reg::Cor0, "REG.COR", "STARTUP", "DONE_CYCLE", val)
                .global("DONE_CYCLE", val)
                .commit();
        }
        for val in ["0", "1", "2", "3", "4", "5", "6", "NOWAIT"] {
            ctx.test_reg(Reg::Cor0, "REG.COR", "STARTUP", "LCK_CYCLE", val)
                .global("LCK_CYCLE", val)
                .commit();
            ctx.build()
                .global_mutex("GLOBAL_DCI", "NO")
                .test_reg(Reg::Cor0, "REG.COR", "STARTUP", "MATCH_CYCLE", val)
                .global("MATCH_CYCLE", val)
                .commit();
        }
        for val in ["NO", "YES"] {
            ctx.test_reg(Reg::Cor0, "REG.COR", "STARTUP", "DRIVE_DONE", val)
                .global("DRIVEDONE", val)
                .commit();
            ctx.test_reg(Reg::Cor0, "REG.COR", "STARTUP", "DONE_PIPE", val)
                .global("DONEPIPE", val)
                .commit();
        }
        for val in [
            "4", "5", "7", "8", "9", "10", "13", "15", "20", "26", "30", "34", "41", "51", "55",
            "60", "130",
        ] {
            ctx.test_reg(Reg::Cor0, "REG.COR", "STARTUP", "CONFIG_RATE", val)
                .global("CONFIGRATE", val)
                .commit();
        }
        for val in ["DISABLE", "ENABLE"] {
            ctx.test_reg(Reg::Cor0, "REG.COR", "STARTUP", "CRC", val)
                .global("CRC", val)
                .commit();
            ctx.test_reg(Reg::Cor0, "REG.COR", "STARTUP", "DCM_SHUTDOWN", val)
                .global("DCMSHUTDOWN", val)
                .commit();
            ctx.test_reg(Reg::Cor0, "REG.COR", "STARTUP", "POWERDOWN_STATUS", val)
                .global("POWERDOWNSTATUS", val)
                .commit();
        }
    }

    {
        let mut ctx = FuzzCtx::new_null(session, backend);
        for val in ["NO", "YES"] {
            ctx.test_reg(Reg::Ctl0, "REG.CTL", "MISC", "GTS_USR_B", val)
                .global("GTS_USR_B", val)
                .commit();
            ctx.test_reg(Reg::Ctl0, "REG.CTL", "MISC", "VGG_TEST", val)
                .global("VGG_TEST", val)
                .commit();
            ctx.test_reg(Reg::Ctl0, "REG.CTL", "MISC", "EN_VTEST", val)
                .global("EN_VTEST", val)
                .commit();
            ctx.test_reg(Reg::Ctl0, "REG.CTL", "MISC", "ENCRYPT", val)
                .global("ENCRYPT", val)
                .commit();
        }
        // persist not fuzzed — too much effort
        for val in ["NONE", "LEVEL1", "LEVEL2"] {
            ctx.test_reg(Reg::Ctl0, "REG.CTL", "MISC", "SECURITY", val)
                .global("SECURITY", val)
                .commit();
        }
    }

    {
        // TODO: more crap
        let mut ctx = FuzzCtx::new_null(session, backend);
        for val in ["NO", "YES"] {
            ctx.test_manual("NULL", "DISABLE_BANDGAP", val)
                .global("DISABLEBANDGAP", val)
                .commit();
        }
        for val in ["DISABLE", "ENABLE"] {
            ctx.test_manual("NULL", "DCI_SHUTDOWN", val)
                .global("DCISHUTDOWN", val)
                .commit();
        }
    }

    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, "SYSMON") {
        let mut bctx = ctx.bel(bels::SYSMON);
        let mode = "MONITOR";
        bctx.test_manual("PRESENT", "1").mode(mode).commit();
        for i in 0x40..0x70 {
            bctx.mode(mode)
                .test_multi_attr_hex(format!("INIT_{i:02X}"), 16);
        }
        bctx.mode(mode)
            .global_mutex("MONITOR_GLOBAL", "NONE")
            .test_enum("MONITOR_MODE", &["TEST", "MONITOR", "ADC"]);
        for pin in [
            "DEN",
            // DCLK?
            "DWE",
            "SCANTESTENA",
            "SCANTESTENB",
            // SCANMEMCLK?
            "SCANMEMWE",
            "ROMTESTENABLE",
            "RST",
            "CONVST",
            // SCLK[AB]?
            "SEA",
            "SEB",
        ] {
            bctx.mode(mode).test_inv(pin);
        }
        for (attr, len) in [
            ("DCLK_DIVID_2", 1),
            ("LW_DIVID_2_4", 1),
            ("MCCLK_DIVID", 8),
            ("OVER_TEMPERATURE", 10),
            ("OVER_TEMPERATURE_OFF", 1),
            ("OVER_TEMPERATURE_DELAY", 8),
            ("BLOCK_ENABLE", 5),
            ("DCLK_MISSING", 10),
            ("FEATURE_ENABLE", 8),
            ("PROM_DATA", 8),
        ] {
            bctx.mode(mode)
                .global_mutex_here("MONITOR_GLOBAL")
                .attr("MONITOR_MODE", "ADC")
                .test_manual(attr, "")
                .multi_global(format!("ADC_{attr}"), MultiValue::Bin, len);
        }
        for out in ["CONVST", "CONVST_TEST"] {
            bctx.build()
                .mutex("CONVST_OUT", out)
                .mutex("CONVST_IN", "INT_CLK")
                .test_manual(out, "INT_CLK")
                .pip(out, "CONVST_INT_CLK")
                .commit();
            bctx.build()
                .mutex("CONVST_OUT", out)
                .mutex("CONVST_IN", "INT_IMUX")
                .test_manual(out, "INT_IMUX")
                .pip(out, "CONVST_INT_IMUX")
                .commit();
            for i in 0..16 {
                bctx.build()
                    .mutex("CONVST_OUT", out)
                    .mutex("CONVST_IN", format!("GIOB{i}"))
                    .related_tile_mutex(HclkDcm, "HCLK_DCM", "USE")
                    .related_pip(
                        HclkDcm,
                        (bels::HCLK_DCM, format!("GIOB_O_D{i}")),
                        (bels::HCLK_DCM, format!("GIOB_I{i}")),
                    )
                    .related_pip(
                        HclkDcm,
                        (bels::HCLK_DCM, format!("GIOB_O_U{i}")),
                        (bels::HCLK_DCM, format!("GIOB_I{i}")),
                    )
                    .test_manual(out, format!("GIOB{i}"))
                    .pip(out, format!("GIOB{i}"))
                    .commit();
            }
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };
    let tile = "CFG";
    let bel = "MISC";
    ctx.collect_enum_default(tile, bel, "PROBESEL", &["0", "1", "2", "3"], "NONE");
    for attr in ["CCLKPIN", "DONEPIN", "POWERDOWNPIN", "PROGPIN", "INITPIN"] {
        ctx.collect_enum(tile, bel, attr, &["PULLUP", "PULLNONE"]);
    }
    for attr in [
        "HSWAPENPIN",
        "M0PIN",
        "M1PIN",
        "M2PIN",
        "CSPIN",
        "DINPIN",
        "BUSYPIN",
        "RDWRPIN",
        "TCKPIN",
        "TDIPIN",
        "TDOPIN",
        "TMSPIN",
    ] {
        ctx.collect_enum(tile, bel, attr, &["PULLUP", "PULLDOWN", "PULLNONE"]);
    }

    for i in 0..32 {
        let bel = format!("BUFGCTRL{i}");
        let bel = &bel;
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        for pin in ["CE0", "CE1", "S0", "S1", "IGNORE0", "IGNORE1"] {
            ctx.collect_inv(tile, bel, pin);
        }
        ctx.collect_enum_bool(tile, bel, "PRESELECT_I0", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "PRESELECT_I1", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "CREATE_EDGE", "FALSE", "TRUE");
        ctx.collect_enum_bool(tile, bel, "INIT_OUT", "0", "1");

        let (_, _, ien_diff) = Diff::split(
            ctx.state.peek_diff(tile, bel, "MUX.I0", "CKINT0").clone(),
            ctx.state.peek_diff(tile, bel, "MUX.I1", "CKINT0").clone(),
        );
        let ien_item = xlat_bit(ien_diff);
        for mux in ["MUX.I0", "MUX.I1"] {
            let mut vals = vec![("NONE", Diff::default())];
            for val in [
                "GFB0", "GFB1", "GFB2", "GFB3", "GFB4", "GFB5", "GFB6", "GFB7", "GFB8", "GFB9",
                "GFB10", "GFB11", "GFB12", "GFB13", "GFB14", "GFB15", "CKINT0", "CKINT1", "MGT_L0",
                "MGT_L1", "MGT_R0", "MGT_R1", "MUXBUS",
            ] {
                let mut diff = ctx.state.get_diff(tile, bel, mux, val);
                diff.apply_bit_diff(&ien_item, true, false);
                vals.push((val, diff));
            }
            ctx.tiledb
                .insert(tile, bel, mux, xlat_enum_ocd(vals, OcdMode::Mux));
        }
        ctx.tiledb.insert(tile, bel, "IMUX_ENABLE", ien_item);

        ctx.state
            .get_diff(tile, bel, "PIN_O_GFB", "1")
            .assert_empty();
        ctx.collect_bit_wide(tile, bel, "ENABLE", "1");
        ctx.state
            .get_diff(tile, bel, "PIN_O_GCLK", "1")
            .assert_empty();
    }
    {
        let tile = "CLK_TERM";
        let bel = "CLK_TERM";
        ctx.collect_bit(tile, bel, "GCLK_ENABLE", "1");
    }

    for bel in [
        "BSCAN0", "BSCAN1", "BSCAN2", "BSCAN3", "JTAGPPC", "DCIRESET", "ICAP0", "ICAP1",
    ] {
        let item = ctx.extract_bit(tile, bel, "PRESENT", "1");
        ctx.tiledb.insert(tile, bel, "ENABLE", item);
    }

    let bel = "BSCAN_COMMON";
    let item = xlat_bitvec(ctx.state.get_diffs(tile, bel, "USERID", ""));
    ctx.tiledb.insert(tile, bel, "USERID", item);

    let bel = "STARTUP";
    ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
    ctx.collect_enum_bool(tile, bel, "GSR_SYNC", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "GWE_SYNC", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "GTS_SYNC", "NO", "YES");
    for pin in [
        "CLK",
        "GSR",
        "USRDONETS",
        "USRDONEO",
        "USRCCLKTS",
        "USRCCLKO",
    ] {
        ctx.collect_int_inv(&["INT"; 16], tile, bel, pin, false);
    }
    ctx.collect_int_inv(&["INT"; 16], tile, bel, "GTS", true);
    let item0 = ctx.extract_bit(tile, bel, "PIN.GSR", "1");
    let item1 = ctx.extract_bit(tile, bel, "PIN.GTS", "1");
    assert_eq!(item0, item1);
    ctx.tiledb.insert(tile, "STARTUP", "GTS_GSR_ENABLE", item0);
    let item = ctx.extract_bit(tile, bel, "PIN.USRCCLKO", "1");
    ctx.tiledb.insert(tile, "STARTUP", "USRCCLK_ENABLE", item);

    let item0 = ctx.extract_enum(tile, "ICAP0", "ICAP_WIDTH", &["X8", "X32"]);
    let item1 = ctx.extract_enum(tile, "ICAP1", "ICAP_WIDTH", &["X8", "X32"]);
    assert_eq!(item0, item1);
    ctx.tiledb.insert(tile, "ICAP_COMMON", "ICAP_WIDTH", item0);
    for bel in ["ICAP0", "ICAP1"] {
        for pin in ["CLK", "CE", "WRITE"] {
            ctx.collect_int_inv(&["INT"; 16], tile, bel, pin, false);
        }
    }

    let bel = "CAPTURE";
    ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
    ctx.collect_int_inv(&["INT"; 16], tile, bel, "CLK", false);
    ctx.collect_int_inv(&["INT"; 16], tile, bel, "CAP", true);

    ctx.state
        .get_diff(tile, "PMV0", "PRESENT", "1")
        .assert_empty();
    ctx.state
        .get_diff(tile, "FRAME_ECC", "PRESENT", "1")
        .assert_empty();
    ctx.state
        .get_diff(tile, "USR_ACCESS", "PRESENT", "1")
        .assert_empty();

    if edev.col_lgt.is_some() {
        for bel in ["BUFG_MGTCLK_S", "BUFG_MGTCLK_N"] {
            for attr in ["BUF.MGT_L0", "BUF.MGT_L1", "BUF.MGT_R0", "BUF.MGT_R1"] {
                let item = ctx.extract_bit(tile, &format!("{bel}_HROW"), attr, "1");
                ctx.tiledb.insert(tile, bel, attr, item);
            }
        }
        if !edev.chips[DieId::from_idx(0)].cols_vbrk.is_empty() {
            let tile = "HCLK_MGT_REPEATER";
            let bel = "HCLK_MGT_REPEATER";
            let item = ctx.extract_bit(tile, bel, "BUF.MGT0.CFG", "1");
            ctx.tiledb.insert(tile, bel, "BUF.MGT0", item);
            let item = ctx.extract_bit(tile, bel, "BUF.MGT1.CFG", "1");
            ctx.tiledb.insert(tile, bel, "BUF.MGT1", item);
        }
    }

    // config regs

    let tile = "REG.COR";
    let bel = "STARTUP";
    ctx.collect_enum(
        tile,
        bel,
        "GWE_CYCLE",
        &["1", "2", "3", "4", "5", "6", "DONE", "KEEP"],
    );
    ctx.collect_enum(
        tile,
        bel,
        "GTS_CYCLE",
        &["1", "2", "3", "4", "5", "6", "DONE", "KEEP"],
    );
    ctx.collect_enum(
        tile,
        bel,
        "DONE_CYCLE",
        &["1", "2", "3", "4", "5", "6", "KEEP"],
    );
    ctx.collect_enum(
        tile,
        bel,
        "LCK_CYCLE",
        &["0", "1", "2", "3", "4", "5", "6", "NOWAIT"],
    );
    ctx.collect_enum(
        tile,
        bel,
        "MATCH_CYCLE",
        &["0", "1", "2", "3", "4", "5", "6", "NOWAIT"],
    );
    ctx.collect_enum(tile, bel, "STARTUPCLK", &["CCLK", "USERCLK", "JTAGCLK"]);
    ctx.collect_enum_ocd(
        tile,
        bel,
        "CONFIG_RATE",
        &[
            "4", "5", "7", "8", "9", "10", "13", "15", "20", "26", "30", "34", "41", "51", "55",
            "60", "130",
        ],
        OcdMode::BitOrder,
    );
    ctx.collect_enum_bool(tile, bel, "DRIVE_DONE", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "DONE_PIPE", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "DCM_SHUTDOWN", "DISABLE", "ENABLE");
    ctx.collect_enum_bool(tile, bel, "POWERDOWN_STATUS", "DISABLE", "ENABLE");
    ctx.collect_enum_bool(tile, bel, "CRC", "DISABLE", "ENABLE");
    let bel = "CAPTURE";
    ctx.collect_enum_bool(tile, bel, "ONESHOT", "FALSE", "TRUE");

    let tile = "REG.CTL";
    let bel = "MISC";
    ctx.collect_enum_bool(tile, bel, "GTS_USR_B", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "VGG_TEST", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "EN_VTEST", "NO", "YES");
    ctx.collect_enum_bool(tile, bel, "ENCRYPT", "NO", "YES");
    ctx.collect_enum(tile, bel, "SECURITY", &["NONE", "LEVEL1", "LEVEL2"]);
    // these are too much trouble to deal with the normal way.
    ctx.tiledb.insert(
        tile,
        bel,
        "PERSIST",
        TileItem {
            bits: vec![TileBit::new(0, 0, 3)],
            kind: TileItemKind::BitVec { invert: bits![0] },
        },
    );
    ctx.tiledb.insert(
        tile,
        bel,
        "GLUTMASK",
        TileItem {
            bits: vec![TileBit::new(0, 0, 8)],
            kind: TileItemKind::BitVec { invert: bits![1] },
        },
    );
    ctx.tiledb.insert(
        tile,
        bel,
        "ICAP_SELECT",
        TileItem {
            bits: vec![TileBit::new(0, 0, 30)],
            kind: TileItemKind::Enum {
                values: [
                    ("TOP".to_string(), bits![0]),
                    ("BOTTOM".to_string(), bits![1]),
                ]
                .into_iter()
                .collect(),
            },
        },
    );

    let sysmon = edev.db.get_tile_class("SYSMON");
    if !edev.tile_index[sysmon].is_empty() {
        let tile = "SYSMON";
        let bel = "SYSMON";
        ctx.collect_enum(tile, bel, "MONITOR_MODE", &["TEST", "MONITOR", "ADC"]);
        for i in 0x40..0x70 {
            ctx.collect_bitvec(tile, bel, &format!("INIT_{i:02X}"), "");
        }
        for pin in [
            "DEN",
            "DWE",
            "SCANTESTENA",
            "SCANTESTENB",
            "SCANMEMWE",
            "ROMTESTENABLE",
            "RST",
            "SEA",
            "SEB",
        ] {
            ctx.collect_int_inv(&["INT"; 8], tile, bel, pin, false);
        }
        ctx.collect_inv(tile, bel, "CONVST");
        let mut present = ctx.state.get_diff(tile, bel, "PRESENT", "1");
        for (attr, val) in [
            ("DCLK_DIVID_2", 0),
            ("LW_DIVID_2_4", 0),
            ("MCCLK_DIVID", 0xc8),
            ("OVER_TEMPERATURE", 0x31e),
            ("OVER_TEMPERATURE_OFF", 0),
            ("OVER_TEMPERATURE_DELAY", 0),
            ("BLOCK_ENABLE", 0x1e),
            ("DCLK_MISSING", 0x320),
            ("FEATURE_ENABLE", 0),
            ("PROM_DATA", 0),
        ] {
            ctx.collect_bitvec(tile, bel, attr, "");
            present.apply_bitvec_diff_int(ctx.tiledb.item(tile, bel, attr), val, 0);
        }
        present.assert_empty();

        let mut diffs = vec![];
        let diff = ctx.state.get_diff(tile, bel, "CONVST", "INT_IMUX");
        assert_eq!(
            diff,
            ctx.state.get_diff(tile, bel, "CONVST_TEST", "INT_IMUX")
        );
        diffs.push(("INT_IMUX".to_string(), diff));
        let mut diff = ctx.state.get_diff(tile, bel, "CONVST", "INT_CLK");
        assert_eq!(
            diff,
            ctx.state.get_diff(tile, bel, "CONVST_TEST", "INT_CLK")
        );
        let item = ctx.item_int_inv(&["INT"; 8], tile, bel, "CONVST_INT_CLK");
        diff.apply_bit_diff(&item, false, true);
        diffs.push(("INT_CLK".to_string(), diff));
        for i in 0..16 {
            let diff = ctx.state.get_diff(tile, bel, "CONVST", format!("GIOB{i}"));
            assert_eq!(
                diff,
                ctx.state
                    .get_diff(tile, bel, "CONVST_TEST", format!("GIOB{i}"))
            );
            diffs.push((format!("GIOB{i}"), diff));
        }
        ctx.tiledb
            .insert(tile, bel, "MUX.CONVST", xlat_enum_ocd(diffs, OcdMode::Mux));
    }
}
