use prjcombine_interconnect::{
    db::{BelAttributeEnum, BelInfo, BelInputId, BelSlotId},
    dir::{DirH, DirHV},
    grid::TileCoord,
};
use prjcombine_re_collector::diff::{xlat_bit, xlat_bit_wide_bi};
use prjcombine_re_fpga_hammer::FuzzerProp;
use prjcombine_re_hammer::{Fuzzer, Session};
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::{bits, bitvec::BitVec, bsdata::TileBit};
use prjcombine_virtex::{
    chip::ChipKind,
    defs::{
        bcls::{self, DLL, GLOBAL},
        bslots, enums, tcls,
    },
};

use crate::{
    backend::{IseBackend, Key},
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::DynProp,
    },
    virtex::specials,
};

#[derive(Copy, Clone, Debug)]
struct DeviceSide(DirH);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for DeviceSide {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let ExpandedDevice::Virtex(edev) = backend.edev else {
            unreachable!()
        };
        match self.0 {
            DirH::W => {
                if tcrd.col >= edev.chip.col_clk() {
                    return None;
                }
            }
            DirH::E => {
                if tcrd.col < edev.chip.col_clk() {
                    return None;
                }
            }
        }
        Some((fuzzer, false))
    }
}

#[derive(Copy, Clone, Debug)]
struct PinWireMutexShared(BelSlotId, BelInputId);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for PinWireMutexShared {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let tile = &backend.edev[tcrd];
        let tcls = &backend.edev.db[tile.class];
        let bel_data = &tcls.bels[self.0];
        let BelInfo::Bel(bel_data) = bel_data else {
            unreachable!()
        };
        let wire = bel_data.inputs[self.1].wire();
        let wire = backend
            .edev
            .resolve_wire(backend.edev.tile_wire(tcrd, wire))?;
        fuzzer = fuzzer.base(Key::WireMutex(wire), "SHARED");
        Some((fuzzer, false))
    }
}

#[derive(Copy, Clone, Debug)]
struct FuzzGlobalDll(BelSlotId, &'static str, &'static str);

impl<'b> FuzzerProp<'b, IseBackend<'b>> for FuzzGlobalDll {
    fn dyn_clone(&self) -> Box<DynProp<'b>> {
        Box::new(Clone::clone(self))
    }

    fn apply<'a>(
        &self,
        backend: &IseBackend<'a>,
        tcrd: TileCoord,
        mut fuzzer: Fuzzer<IseBackend<'a>>,
    ) -> Option<(Fuzzer<IseBackend<'a>>, bool)> {
        let site = backend.ngrid.get_bel_name(tcrd.bel(self.0)).unwrap();
        let opt = self.1;
        let ExpandedDevice::Virtex(edev) = backend.edev else {
            unreachable!()
        };
        let opt = if opt == "TESTZD2OSC*" && site.len() == 4 && edev.chip.kind.is_virtexe() {
            opt.replace('*', &format!("{}S", &site[3..]))
        } else {
            opt.replace('*', &site[3..])
        };
        fuzzer = fuzzer.fuzz(Key::GlobalOpt(opt), None, self.2);
        Some((fuzzer, false))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Virtex(edev) = backend.edev else {
        unreachable!()
    };
    for tcid in [
        tcls::DLL_S,
        tcls::DLL_N,
        tcls::DLLP_S,
        tcls::DLLP_N,
        tcls::DLLS_S,
        tcls::DLLS_N,
    ] {
        let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcid) else {
            continue;
        };
        let mut bctx = ctx.bel(bslots::DLL);
        let cnr_nw = edev.chip.corner(DirHV::NW);
        bctx.build()
            .extra_fixed_bel_attr_bits(cnr_nw, bslots::MISC_NW, bcls::MISC_NW::DLL_ENABLE)
            .global_mutex_here("DLL")
            .test_bel_special(specials::PRESENT)
            .mode("DLL")
            .commit();
        for (val, vname) in [(false, "1"), (true, "0"), (false, "RST"), (true, "RST_B")] {
            bctx.mode("DLL")
                .global_mutex("DLL", "USE")
                .pin("RST")
                .test_bel_input_inv(DLL::RST, val)
                .attr("RSTMUX", vname)
                .commit();
        }
        bctx.mode("DLL")
            .global_mutex("DLL", "USE")
            .test_bel_attr_bits(DLL::HIGH_FREQUENCY)
            .attr("HIGH_FREQ_ATTR", "HIGH_FREQUENCY")
            .commit();
        bctx.mode("DLL")
            .global_mutex("DLL", "USE")
            .test_bel_attr_bool_rename("DUTY_ATTR", DLL::DUTY_CYCLE_CORRECTION, "FALSE", "TRUE");
        for (attr, aname) in [
            (DLL::FACTORY_JF1, "JF_ZD1_ATTR"),
            (DLL::FACTORY_JF2, "JF_ZD2_ATTR"),
        ] {
            for (val, vname) in [
                (0x80, "0X80"),
                (0xc0, "0XC0"),
                (0xe0, "0XE0"),
                (0xf0, "0XF0"),
                (0xf8, "0XF8"),
                (0xfc, "0XFC"),
                (0xfe, "0XFE"),
                (0xff, "0XFF"),
            ] {
                bctx.mode("DLL")
                    .global_mutex("DLL", "USE")
                    .test_bel_attr_u32(attr, val)
                    .attr(aname, vname)
                    .commit();
            }
        }
        for i in 2..=16 {
            bctx.mode("DLL")
                .global_mutex("DLL", "USE")
                .test_bel_special_u32(specials::DLL_DIVIDE_INT, i)
                .attr("DIVIDE_ATTR", i.to_string())
                .commit();
        }
        for i in 1..8 {
            bctx.mode("DLL")
                .global_mutex("DLL", "USE")
                .attr("HIGH_FREQ_ATTR", "")
                .test_bel_special_u32(specials::DLL_DIVIDE_HALF_LOW, i)
                .attr("DIVIDE_ATTR", format!("{i}_5"))
                .commit();
            bctx.mode("DLL")
                .global_mutex("DLL", "USE")
                .attr("HIGH_FREQ_ATTR", "HIGH_FREQUENCY")
                .test_bel_special_u32(specials::DLL_DIVIDE_HALF_HIGH, i)
                .attr("DIVIDE_ATTR", format!("{i}_5"))
                .commit();
        }
        for (attr, opt) in [
            (DLL::CLK_FEEDBACK_2X, "IDLL*FB2X"),
            (DLL::CFG_O_14, "IDLL*CFG_O_14"),
            (DLL::LVL1_MUX_20, "IDLL*_ILVL1_MUX_20"),
            (DLL::LVL1_MUX_21, "IDLL*_ILVL1_MUX_21"),
            (DLL::LVL1_MUX_22, "IDLL*_ILVL1_MUX_22"),
            (DLL::LVL1_MUX_23, "IDLL*_ILVL1_MUX_23"),
            (DLL::LVL1_MUX_24, "IDLL*_ILVL1_MUX_24"),
        ] {
            for (val, vname) in [(false, "0"), (true, "1")] {
                // value "0" is apparently buggy and affects other DLLs than the one we're
                // aiming for, sometimes.
                //
                // have I mentioned I hate ISE?
                if attr == DLL::LVL1_MUX_21 && !val {
                    continue;
                }
                bctx.mode("DLL")
                    .global_mutex("DLL", "USE")
                    .prop(PinWireMutexShared(bslots::DLL, DLL::CLKIN))
                    .prop(PinWireMutexShared(bslots::DLL, DLL::CLKFB))
                    .test_bel_attr_bits_bi(attr, val)
                    .prop(FuzzGlobalDll(bslots::DLL, opt, vname))
                    .commit();
            }
        }
        for (attr, opt) in [(DLL::TESTDLL, "TESTDLL*"), (DLL::TESTZD2OSC, "TESTZD2OSC*")] {
            for (val, vname) in [(false, "NO"), (true, "YES")] {
                bctx.mode("DLL")
                    .global_mutex("DLL", "USE")
                    .test_bel_attr_bits_bi(attr, val)
                    .prop(FuzzGlobalDll(bslots::DLL, opt, vname))
                    .commit();
            }
        }

        if matches!(tcid, tcls::DLL_S | tcls::DLLP_S)
            || (tcid == tcls::DLLS_S && !backend.device.name.contains('v'))
        {
            bctx.mode("DLL")
                .global_mutex_here("DLL")
                .prop(DeviceSide(DirH::W))
                .extra_tiles_by_bel_attr_bits(bslots::GLOBAL, GLOBAL::LOCK_WAIT_SW)
                .null_bits()
                .test_bel_special(specials::DLL_STARTUP_WAIT)
                .attr("STARTUP_ATTR", "STARTUP_WAIT")
                .commit();

            bctx.mode("DLL")
                .global_mutex_here("DLL")
                .prop(DeviceSide(DirH::E))
                .extra_tiles_by_bel_attr_bits(bslots::GLOBAL, GLOBAL::LOCK_WAIT_SE)
                .null_bits()
                .test_bel_special(specials::DLL_STARTUP_WAIT)
                .attr("STARTUP_ATTR", "STARTUP_WAIT")
                .commit();
        } else if matches!(tcid, tcls::DLL_N | tcls::DLLP_N)
            || (tcid == tcls::DLLS_N && !backend.device.name.contains('v'))
        {
            bctx.mode("DLL")
                .global_mutex_here("DLL")
                .prop(DeviceSide(DirH::W))
                .extra_tiles_by_bel_attr_bits(bslots::GLOBAL, GLOBAL::LOCK_WAIT_NW)
                .null_bits()
                .test_bel_special(specials::DLL_STARTUP_WAIT)
                .attr("STARTUP_ATTR", "STARTUP_WAIT")
                .commit();
            bctx.mode("DLL")
                .global_mutex_here("DLL")
                .prop(DeviceSide(DirH::E))
                .extra_tiles_by_bel_attr_bits(bslots::GLOBAL, GLOBAL::LOCK_WAIT_NE)
                .null_bits()
                .test_bel_special(specials::DLL_STARTUP_WAIT)
                .attr("STARTUP_ATTR", "STARTUP_WAIT")
                .commit();
        }
    }
    let mut ctx = FuzzCtx::new_null(session, backend);
    for (val, vname) in [
        (enums::DLL_TEST_OSC::_90, "90"),
        (enums::DLL_TEST_OSC::_180, "180"),
        (enums::DLL_TEST_OSC::_270, "270"),
        (enums::DLL_TEST_OSC::_360, "360"),
    ] {
        ctx.build()
            .extra_tiles_by_bel_attr_val(bslots::DLL, DLL::TEST_OSC, val)
            .test_global_special(specials::DLL_TEST_OSC)
            .global("TESTOSC", vname)
            .commit();
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex(edev) = ctx.edev else {
        unreachable!()
    };
    for tcid in [
        tcls::DLL_S,
        tcls::DLL_N,
        tcls::DLLP_S,
        tcls::DLLP_N,
        tcls::DLLS_S,
        tcls::DLLS_N,
    ] {
        if !ctx.has_tcls(tcid) {
            continue;
        }
        let bslot = bslots::DLL;

        let mut present = ctx.get_diff_bel_special(tcid, bslot, specials::PRESENT);

        let item = xlat_bit_wide_bi(
            ctx.get_diff_attr_bool_bi(tcid, bslot, DLL::DUTY_CYCLE_CORRECTION, false),
            ctx.get_diff_attr_bool_bi(tcid, bslot, DLL::DUTY_CYCLE_CORRECTION, true),
        );
        present.apply_bitvec_diff(&item, &BitVec::repeat(true, 4), &BitVec::repeat(false, 4));
        ctx.insert_bel_attr_bitvec(tcid, bslot, DLL::DUTY_CYCLE_CORRECTION, item);

        ctx.collect_bel_attr(tcid, bslot, DLL::HIGH_FREQUENCY);

        ctx.collect_bel_input_inv_bi(tcid, bslot, DLL::RST);

        let item_jf2 = Vec::from_iter((0..8).map(|bit| TileBit::new(0, 17, bit).pos()));
        let item_jf1 = Vec::from_iter((8..16).map(|bit| TileBit::new(0, 17, bit).pos()));
        for (attr, item, base) in [
            (DLL::FACTORY_JF2, &item_jf2, 0x80),
            (DLL::FACTORY_JF1, &item_jf1, 0xc0),
        ] {
            for val in [0x80, 0xc0, 0xe0, 0xf0, 0xf8, 0xfc, 0xfe, 0xff] {
                let mut diff = ctx.get_diff_attr_u32(tcid, bslot, attr, val as u32);
                diff.apply_bitvec_diff_int(item, val, base);
                diff.assert_empty();
            }
            present.apply_bitvec_diff_int(item, base, 0xf0);
        }
        ctx.insert_bel_attr_bitvec(tcid, bslot, DLL::FACTORY_JF2, item_jf2);
        ctx.insert_bel_attr_bitvec(tcid, bslot, DLL::FACTORY_JF1, item_jf1);

        let clkdv_count_max = Vec::from_iter((4..8).map(|bit| TileBit::new(0, 18, bit).pos()));
        let clkdv_count_fall = Vec::from_iter((8..12).map(|bit| TileBit::new(0, 18, bit).pos()));
        let clkdv_count_fall_2 = Vec::from_iter((12..16).map(|bit| TileBit::new(0, 18, bit).pos()));
        let clkdv_phase_rise = Vec::from_iter((1..3).map(|bit| TileBit::new(0, 16, bit).pos()));
        let clkdv_phase_fall = Vec::from_iter((3..5).map(|bit| TileBit::new(0, 16, bit).pos()));
        let clkdv_mode = BelAttributeEnum {
            bits: vec![TileBit::new(0, 16, 15)],
            values: [
                (enums::DLL_CLKDV_MODE::HALF, bits![0]),
                (enums::DLL_CLKDV_MODE::INT, bits![1]),
            ]
            .into_iter()
            .collect(),
        };
        for i in 2..=16 {
            let mut diff =
                ctx.get_diff_bel_special_u32(tcid, bslot, specials::DLL_DIVIDE_INT, i as u32);
            diff.apply_bitvec_diff_int(&clkdv_count_max, i - 1, 1);
            diff.apply_bitvec_diff_int(&clkdv_count_fall, (i - 1) / 2, 0);
            diff.apply_bitvec_diff_int(&clkdv_phase_fall, (i % 2) * 2, 0);
            diff.assert_empty();
        }
        for i in 1..=7 {
            let mut diff =
                ctx.get_diff_bel_special_u32(tcid, bslot, specials::DLL_DIVIDE_HALF_LOW, i as u32);
            diff.apply_enum_diff(
                &clkdv_mode,
                enums::DLL_CLKDV_MODE::HALF,
                enums::DLL_CLKDV_MODE::INT,
            );
            diff.apply_bitvec_diff_int(&clkdv_count_max, 2 * i, 1);
            diff.apply_bitvec_diff_int(&clkdv_count_fall, i / 2, 0);
            diff.apply_bitvec_diff_int(&clkdv_count_fall_2, 3 * i / 2 + 1, 0);
            diff.apply_bitvec_diff_int(&clkdv_phase_fall, (i % 2) * 2 + 1, 0);
            diff.assert_empty();
            let mut diff =
                ctx.get_diff_bel_special_u32(tcid, bslot, specials::DLL_DIVIDE_HALF_HIGH, i as u32);
            diff.apply_enum_diff(
                &clkdv_mode,
                enums::DLL_CLKDV_MODE::HALF,
                enums::DLL_CLKDV_MODE::INT,
            );
            diff.apply_bitvec_diff_int(&clkdv_count_max, 2 * i, 1);
            diff.apply_bitvec_diff_int(&clkdv_count_fall, (i - 1) / 2, 0);
            diff.apply_bitvec_diff_int(&clkdv_count_fall_2, (3 * i).div_ceil(2), 0);
            diff.apply_bitvec_diff_int(&clkdv_phase_fall, (i % 2) * 2, 0);
            diff.assert_empty();
        }
        present.apply_bitvec_diff_int(&clkdv_count_max, 1, 0);
        present.apply_enum_diff(
            &clkdv_mode,
            enums::DLL_CLKDV_MODE::INT,
            enums::DLL_CLKDV_MODE::HALF,
        );
        ctx.insert_bel_attr_bitvec(tcid, bslot, DLL::CLKDV_COUNT_MAX, clkdv_count_max);
        ctx.insert_bel_attr_bitvec(tcid, bslot, DLL::CLKDV_COUNT_FALL, clkdv_count_fall);
        ctx.insert_bel_attr_bitvec(tcid, bslot, DLL::CLKDV_COUNT_FALL_2, clkdv_count_fall_2);
        ctx.insert_bel_attr_bitvec(tcid, bslot, DLL::CLKDV_PHASE_RISE, clkdv_phase_rise);
        ctx.insert_bel_attr_bitvec(tcid, bslot, DLL::CLKDV_PHASE_FALL, clkdv_phase_fall);
        ctx.insert_bel_attr_enum(tcid, bslot, DLL::CLKDV_MODE, clkdv_mode);

        ctx.collect_bel_attr_bi(tcid, bslot, DLL::CFG_O_14);
        ctx.collect_bel_attr_bi(tcid, bslot, DLL::LVL1_MUX_20);
        ctx.collect_bel_attr(tcid, bslot, DLL::LVL1_MUX_21);
        ctx.collect_bel_attr_bi(tcid, bslot, DLL::LVL1_MUX_22);
        ctx.collect_bel_attr_bi(tcid, bslot, DLL::LVL1_MUX_23);
        ctx.collect_bel_attr_bi(tcid, bslot, DLL::LVL1_MUX_24);
        ctx.collect_bel_attr_bi(tcid, bslot, DLL::TESTZD2OSC);
        let item = xlat_bit_wide_bi(
            ctx.get_diff_attr_bool_bi(tcid, bslot, DLL::TESTDLL, false),
            ctx.get_diff_attr_bool_bi(tcid, bslot, DLL::TESTDLL, true),
        );
        ctx.insert_bel_attr_bitvec(tcid, bslot, DLL::TESTDLL, item);
        ctx.collect_bel_attr_bi(tcid, bslot, DLL::CLK_FEEDBACK_2X);

        present.apply_bit_diff(ctx.bel_attr_bit(tcid, bslot, DLL::CFG_O_14), true, false);
        if ctx.device.name.ends_with('e') {
            ctx.insert_bel_attr_bool(tcid, bslot, DLL::ENABLE, xlat_bit(present));
        } else {
            present.assert_empty();
        }
        ctx.collect_bel_attr(tcid, bslot, DLL::TEST_OSC);
    }
    let cnr_nw = if edev.chip.kind == ChipKind::Spartan2 {
        tcls::CNR_NW_S2
    } else {
        tcls::CNR_NW
    };
    ctx.collect_bel_attr(cnr_nw, bslots::MISC_NW, bcls::MISC_NW::DLL_ENABLE);

    let tcid = tcls::GLOBAL;
    let bslot = bslots::GLOBAL;
    ctx.collect_bel_attr(tcid, bslot, GLOBAL::LOCK_WAIT_SW);
    ctx.collect_bel_attr(tcid, bslot, GLOBAL::LOCK_WAIT_SE);
    ctx.collect_bel_attr(tcid, bslot, GLOBAL::LOCK_WAIT_NW);
    ctx.collect_bel_attr(tcid, bslot, GLOBAL::LOCK_WAIT_NE);
}
