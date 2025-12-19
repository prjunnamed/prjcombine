use prjcombine_entity::EntityId;
use prjcombine_re_sdf::Sdf;
use prjcombine_re_toolchain::Toolchain;
use prjcombine_re_xilinx_cpld::{
    db::Part,
    device::{Device, Package},
    tsim::run_tsim,
    vm6::{InputNodeKind, NodeKind},
    vm6_util::prep_vm6,
};
use prjcombine_types::{
    cpld::{BlockId, ProductTermId},
    speed::{SetupHold, Speed, SpeedVal},
    units::Time,
};

use crate::{
    extract::{
        collect_and2, collect_buf, collect_ff, collect_latch, collect_tri_ctl, collect_tri_i,
        extract_and2, extract_buf, set_timing, set_timing_delay,
    },
    vm6_emit::{
        insert_bufoe, insert_ct, insert_ibuf, insert_mc, insert_mc_out, insert_mc_si, insert_obuf,
        insert_srff, insert_srff_inp, insert_srff_ireg,
    },
};

pub fn test_xbr(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
) -> Speed {
    let mut speed = Speed::new();
    test_comb(tc, part, device, package, spd, &mut speed);
    test_ff_pt(tc, part, device, package, spd, &mut speed);
    test_ff_ct(tc, part, device, package, spd, &mut speed);
    test_ff_fast(tc, part, device, package, spd, &mut speed);
    test_latch(tc, part, device, package, spd, &mut speed);
    test_iostd(tc, part, device, package, spd, &mut speed, "LVCMOS15");
    test_iostd(tc, part, device, package, spd, &mut speed, "LVCMOS18");
    test_iostd(tc, part, device, package, spd, &mut speed, "LVCMOS18_ANY");
    test_iostd(tc, part, device, package, spd, &mut speed, "LVCMOS25");
    test_iostd(tc, part, device, package, spd, &mut speed, "LVCMOS33");
    test_iostd(tc, part, device, package, spd, &mut speed, "LVTTL");
    if device.has_vref {
        test_iostd(tc, part, device, package, spd, &mut speed, "SSTL2_I");
        test_iostd(tc, part, device, package, spd, &mut speed, "SSTL3_I");
        test_iostd(tc, part, device, package, spd, &mut speed, "HSTL_I");
    }
    if device.dge_pad.is_some() {
        test_dge(tc, part, device, package, spd, &mut speed);
    }
    if device.cdr_pad.is_some() {
        let (setup, hold) = match (&*part.dev_name, spd) {
            ("xc2c128", "-6") => (1300, 0),
            ("xc2c128", "-7") => (2000, 0),
            ("xa2c128", "-7") => (2000, 0),
            ("xa2c128", "-8") => (2000, 0),
            ("xc2c256", "-6") => (1600, 0),
            ("xc2c256", "-7") => (2000, 0),
            ("xa2c256", "-7") => (2000, 0),
            ("xa2c256", "-8") => (2000, 0),
            ("xc2c384", "-7") => (1700, 0),
            ("xc2c384", "-10") => (2500, 0),
            ("xa2c384", "-10") => (2500, 0),
            ("xa2c384", "-11") => (2500, 200),
            ("xc2c512", "-7") => (1700, 0),
            ("xc2c512", "-10") => (2500, 0),
            (d, s) => panic!("missing data sheet timings for {d}{s}"),
        };
        set_timing(
            &mut speed,
            "SETUPHOLD_CD_RST",
            SpeedVal::SetupHold(SetupHold {
                setup: Time(setup.into()),
                hold: Time(hold.into()),
            }),
        );
    }
    let (setup, hold) = match (&*part.dev_name, spd) {
        ("xc2c32", "-3") => (900, 0),
        ("xc2c32", "-4") => (700, 0),
        ("xc2c32", "-6") => (1700, 0),
        ("xc2c32a", "-4") => (700, 0),
        ("xc2c32a", "-6") => (1700, 0),
        ("xa2c32a", "-6") => (1700, 0),
        ("xa2c32a", "-7") => (1700, 0),
        ("xc2c64", "-5") => (900, 0),
        ("xc2c64", "-7") => (1300, 0),
        ("xc2c64a", "-5") => (900, 0),
        ("xc2c64a", "-7") => (1300, 0),
        ("xa2c64a", "-7") => (1300, 0),
        ("xa2c64a", "-8") => (1300, 0),
        ("xc2c128", "-6") => (1400, 0),
        ("xc2c128", "-7") => (1600, 0),
        ("xa2c128", "-7") => (1600, 0),
        ("xa2c128", "-8") => (1600, 0),
        ("xc2c256", "-6") => (800, 0),
        ("xc2c256", "-7") => (1800, 0),
        ("xa2c256", "-7") => (1800, 0),
        ("xa2c256", "-8") => (1100, 0),
        ("xc2c384", "-7") => (1500, 0),
        ("xc2c384", "-10") => (2000, 0),
        ("xa2c384", "-10") => (2000, 0),
        ("xa2c384", "-11") => (2600, 1700),
        ("xc2c512", "-7") => (1300, 0),
        ("xc2c512", "-10") => (1800, 0),
        (d, s) => panic!("missing data sheet timings for {d}{s}"),
    };
    set_timing(
        &mut speed,
        "SETUPHOLD_CE_CLK",
        SpeedVal::SetupHold(SetupHold {
            setup: Time(setup.into()),
            hold: Time(hold.into()),
        }),
    );

    speed
}

fn test_comb(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    speed: &mut Speed,
) {
    let mut vm6 = prep_vm6(part, device, package, spd);
    let node_i1 = insert_ibuf(&mut vm6, "I1", NodeKind::IiImux, 0);
    let node_i2 = insert_ibuf(&mut vm6, "I2", NodeKind::IiImux, 0);

    let mcid = insert_mc(&mut vm6, "MC", 0);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD1, &[node_i1]);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD2, &[node_i2]);
    insert_srff(&mut vm6, mcid);
    insert_obuf(&mut vm6, mcid, 3);

    let mc_uim = insert_mc_out(&mut vm6, mcid, NodeKind::McUim);
    let umcid = insert_mc(&mut vm6, "UMC", 0);
    insert_mc_si(&mut vm6, umcid, NodeKind::McSiD1, &[mc_uim]);
    insert_mc_si(&mut vm6, umcid, NodeKind::McSiD2, &[]);
    insert_srff(&mut vm6, umcid);
    insert_obuf(&mut vm6, umcid, 2);

    vm6.iostd_default = Some("LVCMOS18".into());
    vm6.iostd.insert("I1".into(), "LVCMOS18".into());
    vm6.iostd.insert("I2".into(), "LVCMOS18".into());
    vm6.iostd.insert("MC_PAD".into(), "LVCMOS18".into());
    vm6.iostd.insert("UMC_PAD".into(), "LVCMOS18".into());

    let (_, sdf) = run_tsim(tc, &vm6).unwrap();
    let sdf = Sdf::parse(&sdf);
    assert_eq!(sdf.timescale, Some(3));

    collect_buf(&sdf, "I1", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "I2", speed, "DEL_IBUF_IMUX");

    collect_and2(&sdf, "MC.D1", speed, "DEL_IMUX_PT");
    collect_and2(&sdf, "MC.D2", speed, "DEL_IMUX_OR");

    collect_buf(&sdf, "MC.Q", speed, "DEL_D_Q_COMB");
    collect_buf(&sdf, "UMC.Q", speed, "DEL_D_Q_COMB");
    collect_buf(&sdf, "MC_PAD_6", speed, "DEL_OBUF_FAST.LVCMOS18");
    collect_buf(&sdf, "UMC_PAD_8", speed, "DEL_OBUF_SLOW.LVCMOS18");

    let uim = extract_and2(&sdf, "UMC.D1");
    let SpeedVal::Delay(del_imux_pt) = speed.vals["DEL_IMUX_PT"] else {
        unreachable!()
    };
    set_timing_delay(speed, "DEL_UIM_IMUX", uim - del_imux_pt);
}

fn test_ff_pt(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    speed: &mut Speed,
) {
    let mut vm6 = prep_vm6(part, device, package, spd);
    let node_d = insert_ibuf(&mut vm6, "D", NodeKind::IiImux, 0);
    let node_i = insert_ibuf(&mut vm6, "I", NodeKind::IiReg, 0);
    let node_c = insert_ibuf(&mut vm6, "C", NodeKind::IiImux, 0);
    let node_r = insert_ibuf(&mut vm6, "R", NodeKind::IiImux, 0);
    let node_e = insert_ibuf(&mut vm6, "E", NodeKind::IiImux, 0);

    let mcid = insert_mc(&mut vm6, "MC", 0x4000);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD1, &[node_d]);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD2, &[]);
    let node_sc = insert_mc_si(&mut vm6, mcid, NodeKind::McSiClkf, &[node_c]);
    let node_sr = insert_mc_si(&mut vm6, mcid, NodeKind::McSiRstf, &[node_r]);
    let node_se = insert_mc_si(&mut vm6, mcid, NodeKind::McSiTrst, &[node_e]);
    insert_srff(&mut vm6, mcid);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffC, node_sc);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffR, node_sr);
    insert_bufoe(&mut vm6, mcid, node_se);
    insert_obuf(&mut vm6, mcid, 3);

    let imcid = insert_mc(&mut vm6, "IMC", 0);
    let node_sc = insert_mc_si(&mut vm6, imcid, NodeKind::McSiClkf, &[node_c]);
    insert_srff_ireg(&mut vm6, imcid, node_i);
    insert_srff_inp(&mut vm6, imcid, InputNodeKind::SrffC, node_sc);
    insert_obuf(&mut vm6, imcid, 3);

    let (_, sdf) = run_tsim(tc, &vm6).unwrap();
    let sdf = Sdf::parse(&sdf);
    assert_eq!(sdf.timescale, Some(3));

    collect_buf(&sdf, "D", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "I", speed, "DEL_IBUF_D");
    collect_buf(&sdf, "C", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "R", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "E", speed, "DEL_IBUF_IMUX");
    collect_tri_i(&sdf, "MC_PAD_15", speed, "DEL_OBUF_FAST.LVCMOS18");
    collect_tri_ctl(&sdf, "MC_PAD_15", speed, "DEL_OBUF_OE");
    collect_buf(&sdf, "IMC_PAD_17", speed, "DEL_OBUF_FAST.LVCMOS18");

    collect_and2(&sdf, "MC.D1", speed, "DEL_IMUX_PT");
    collect_and2(&sdf, "MC.CLKF", speed, "DEL_IMUX_CT");
    collect_and2(&sdf, "MC.RSTF", speed, "DEL_IMUX_CT");
    collect_and2(&sdf, "MC.TRST", speed, "DEL_IMUX_CT");
    collect_and2(&sdf, "IMC.CLKF", speed, "DEL_IMUX_CT");

    collect_ff(
        &sdf,
        "MC.REG",
        speed,
        "DEL_CLK_Q",
        "DEL_SR_Q",
        "SETUPHOLD_D_CLK_PT_PT",
        None,
        "WIDTH_CLK_PT",
        "WIDTH_SR",
    );

    collect_ff(
        &sdf,
        "IMC.REG",
        speed,
        "DEL_CLK_Q",
        "DEL_SR_Q",
        "SETUPHOLD_D_CLK_IBUF_PT",
        None,
        "WIDTH_CLK_PT",
        "WIDTH_SR",
    );
}

fn test_ff_ct(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    speed: &mut Speed,
) {
    let mut vm6 = prep_vm6(part, device, package, spd);
    let node_d = insert_ibuf(&mut vm6, "D", NodeKind::IiImux, 0);
    let node_i = insert_ibuf(&mut vm6, "I", NodeKind::IiReg, 0);
    let node_c = insert_ibuf(&mut vm6, "C", NodeKind::IiImux, 0);
    let node_r = insert_ibuf(&mut vm6, "R", NodeKind::IiImux, 0);
    let node_s = insert_ibuf(&mut vm6, "S", NodeKind::IiImux, 0);
    let node_e = insert_ibuf(&mut vm6, "E", NodeKind::IiImux, 0);
    let node_ce = insert_ibuf(&mut vm6, "CE", NodeKind::IiImux, 0);

    let node_sc = insert_ct(
        &mut vm6,
        BlockId::from_idx(0),
        ProductTermId::from_idx(4),
        &[node_c],
    );
    let node_sr = insert_ct(
        &mut vm6,
        BlockId::from_idx(0),
        ProductTermId::from_idx(5),
        &[node_r],
    );
    let node_ss = insert_ct(
        &mut vm6,
        BlockId::from_idx(0),
        ProductTermId::from_idx(6),
        &[node_s],
    );
    let node_se = insert_ct(
        &mut vm6,
        BlockId::from_idx(0),
        ProductTermId::from_idx(7),
        &[node_e],
    );

    let mcid = insert_mc(&mut vm6, "MC", 0x4000);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD1, &[node_d]);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD2, &[]);
    let node_sce = insert_mc_si(&mut vm6, mcid, NodeKind::McSiCe, &[node_ce]);
    insert_srff(&mut vm6, mcid);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffC, node_sc);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffR, node_sr);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffS, node_ss);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffCe, node_sce);
    insert_bufoe(&mut vm6, mcid, node_se);
    insert_obuf(&mut vm6, mcid, 3);

    let imcid = insert_mc(&mut vm6, "IMC", 0);
    insert_srff_ireg(&mut vm6, imcid, node_i);
    insert_srff_inp(&mut vm6, imcid, InputNodeKind::SrffC, node_sc);
    insert_obuf(&mut vm6, imcid, 3);

    let (_, sdf) = run_tsim(tc, &vm6).unwrap();
    let sdf = Sdf::parse(&sdf);
    assert_eq!(sdf.timescale, Some(3));

    collect_buf(&sdf, "D", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "I", speed, "DEL_IBUF_D");
    collect_buf(&sdf, "C", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "R", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "S", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "E", speed, "DEL_IBUF_IMUX");
    collect_tri_i(&sdf, "MC_PAD_19", speed, "DEL_OBUF_FAST.LVCMOS18");
    collect_tri_ctl(&sdf, "MC_PAD_19", speed, "DEL_OBUF_OE");
    collect_buf(&sdf, "IMC_PAD_21", speed, "DEL_OBUF_FAST.LVCMOS18");

    collect_and2(&sdf, "MC.D1", speed, "DEL_IMUX_PT");
    collect_and2(&sdf, "MC.CE", speed, "DEL_IMUX_CT");
    collect_and2(&sdf, "FOOBAR1__ctinst/4", speed, "DEL_IMUX_CT");
    collect_and2(&sdf, "FOOBAR1__ctinst/5", speed, "DEL_IMUX_CT");
    collect_and2(&sdf, "FOOBAR1__ctinst/6", speed, "DEL_IMUX_CT");
    collect_and2(&sdf, "FOOBAR1__ctinst/7", speed, "DEL_IMUX_CT");

    collect_ff(
        &sdf,
        "MC.REG",
        speed,
        "DEL_CLK_Q",
        "DEL_SR_Q",
        "SETUPHOLD_D_CLK_PT_PT",
        None,
        "WIDTH_CLK_PT",
        "WIDTH_SR",
    );

    collect_ff(
        &sdf,
        "IMC.REG",
        speed,
        "DEL_CLK_Q",
        "DEL_SR_Q",
        "SETUPHOLD_D_CLK_IBUF_PT",
        None,
        "WIDTH_CLK_PT",
        "WIDTH_SR",
    );
}

fn test_ff_fast(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    speed: &mut Speed,
) {
    let mut vm6 = prep_vm6(part, device, package, spd);
    let node_d = insert_ibuf(&mut vm6, "D", NodeKind::IiImux, 0);
    let node_i = insert_ibuf(&mut vm6, "I", NodeKind::IiReg, 0);
    let node_c = insert_ibuf(&mut vm6, "C", NodeKind::IiFclk, 0);
    let node_r = insert_ibuf(&mut vm6, "R", NodeKind::IiFsr, 0);
    let node_e = insert_ibuf(&mut vm6, "E", NodeKind::IiFoe, 0);
    let node_ce = insert_ibuf(&mut vm6, "CE", NodeKind::IiImux, 0);

    let mcid = insert_mc(&mut vm6, "MC", 0x4000);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD1, &[node_d]);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD2, &[]);
    let node_sce = insert_mc_si(&mut vm6, mcid, NodeKind::McSiCe, &[node_ce]);
    insert_srff(&mut vm6, mcid);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffC, node_c);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffR, node_r);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffCe, node_sce);
    insert_bufoe(&mut vm6, mcid, node_e);
    insert_obuf(&mut vm6, mcid, 3);

    let imcid = insert_mc(&mut vm6, "IMC", 0);
    insert_srff_ireg(&mut vm6, imcid, node_i);
    insert_srff_inp(&mut vm6, imcid, InputNodeKind::SrffC, node_c);
    insert_obuf(&mut vm6, imcid, 3);

    let (_, sdf) = run_tsim(tc, &vm6).unwrap();
    let sdf = Sdf::parse(&sdf);
    assert_eq!(sdf.timescale, Some(3));

    collect_buf(&sdf, "D", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "I", speed, "DEL_IBUF_D");
    collect_buf(&sdf, "C", speed, "DEL_IBUF_FCLK");
    collect_buf(&sdf, "R", speed, "DEL_IBUF_FSR");
    collect_buf(&sdf, "E", speed, "DEL_IBUF_FOE");
    collect_tri_i(&sdf, "MC_PAD_17", speed, "DEL_OBUF_FAST.LVCMOS18");
    collect_tri_ctl(&sdf, "MC_PAD_17", speed, "DEL_OBUF_OE");
    collect_buf(&sdf, "IMC_PAD_19", speed, "DEL_OBUF_FAST.LVCMOS18");

    collect_and2(&sdf, "MC.D1", speed, "DEL_IMUX_PT");
    collect_and2(&sdf, "MC.CE", speed, "DEL_IMUX_CT");

    collect_ff(
        &sdf,
        "MC.REG",
        speed,
        "DEL_CLK_Q",
        "DEL_SR_Q",
        "SETUPHOLD_D_CLK_PT_FCLK",
        None,
        "WIDTH_CLK",
        "WIDTH_SR",
    );

    collect_ff(
        &sdf,
        "IMC.REG",
        speed,
        "DEL_CLK_Q",
        "DEL_SR_Q",
        "SETUPHOLD_D_CLK_IBUF_FCLK",
        None,
        "WIDTH_CLK",
        "WIDTH_SR",
    );
}

fn test_latch(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    speed: &mut Speed,
) {
    let mut vm6 = prep_vm6(part, device, package, spd);
    let node_d = insert_ibuf(&mut vm6, "D", NodeKind::IiImux, 0);
    let node_c = insert_ibuf(&mut vm6, "C", NodeKind::IiImux, 0);
    let node_r = insert_ibuf(&mut vm6, "R", NodeKind::IiImux, 0);
    let node_s = insert_ibuf(&mut vm6, "S", NodeKind::IiImux, 0);
    let node_e = insert_ibuf(&mut vm6, "E", NodeKind::IiImux, 0);

    let emcid = insert_mc(&mut vm6, "EMC", 0);
    insert_mc_si(&mut vm6, emcid, NodeKind::McSiD1, &[node_e]);
    insert_mc_si(&mut vm6, emcid, NodeKind::McSiD2, &[]);
    insert_srff(&mut vm6, emcid);
    let node_me = insert_mc_out(&mut vm6, emcid, NodeKind::McGlb);

    let mcid = insert_mc(&mut vm6, "MC", 0x4040);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD1, &[node_d]);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD2, &[]);
    let node_sc = insert_ct(
        &mut vm6,
        BlockId::from_idx(0),
        ProductTermId::from_idx(4),
        &[node_c],
    );
    let node_sr = insert_ct(
        &mut vm6,
        BlockId::from_idx(0),
        ProductTermId::from_idx(5),
        &[node_r],
    );
    let node_ss = insert_ct(
        &mut vm6,
        BlockId::from_idx(0),
        ProductTermId::from_idx(6),
        &[node_s],
    );
    insert_srff(&mut vm6, mcid);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffC, node_sc);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffR, node_sr);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffS, node_ss);
    insert_bufoe(&mut vm6, mcid, node_me);
    insert_obuf(&mut vm6, mcid, 3);

    let (_, sdf) = run_tsim(tc, &vm6).unwrap();
    let sdf = Sdf::parse(&sdf);
    assert_eq!(sdf.timescale, Some(3));

    collect_buf(&sdf, "D", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "C", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "R", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "S", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "E", speed, "DEL_IBUF_IMUX");
    collect_tri_i(&sdf, "MC_PAD_12", speed, "DEL_OBUF_FAST.LVCMOS18");
    collect_tri_ctl(&sdf, "MC_PAD_12", speed, "DEL_OBUF_OE");

    collect_and2(&sdf, "EMC.D1", speed, "DEL_IMUX_PT");
    collect_buf(&sdf, "EMC.Q", speed, "DEL_D_Q_COMB");

    collect_and2(&sdf, "MC.D1", speed, "DEL_IMUX_PT");
    collect_and2(&sdf, "FOOBAR1__ctinst/4", speed, "DEL_IMUX_CT");
    collect_and2(&sdf, "FOOBAR1__ctinst/5", speed, "DEL_IMUX_CT");
    collect_and2(&sdf, "FOOBAR1__ctinst/6", speed, "DEL_IMUX_CT");
    collect_buf(&sdf, "MC.BUFOE.OUT", speed, "DEL_MC_FOE");

    collect_latch(
        &sdf,
        "MC.REG",
        speed,
        "DEL_D_Q_LATCH",
        "DEL_CLK_Q",
        Some("DEL_SR_Q"),
        "SETUPHOLD_D_CLK_PT_PT",
        "WIDTH_CLK_PT",
        Some("WIDTH_SR"),
    );
}

fn test_iostd(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    speed: &mut Speed,
    iostd: &str,
) {
    let mut vm6 = prep_vm6(part, device, package, spd);
    let has_plain = iostd != "LVCMOS15";
    let node_ip = insert_ibuf(
        &mut vm6,
        "IP",
        NodeKind::IiImux,
        if has_plain { 0 } else { 4 },
    );
    let node_is = insert_ibuf(&mut vm6, "IS", NodeKind::IiImux, 4);

    let fmcid = insert_mc(&mut vm6, "FMC", 0);
    insert_mc_si(&mut vm6, fmcid, NodeKind::McSiD1, &[node_ip]);
    insert_mc_si(&mut vm6, fmcid, NodeKind::McSiD2, &[node_is]);
    insert_srff(&mut vm6, fmcid);
    insert_obuf(&mut vm6, fmcid, 3);

    let smcid = insert_mc(&mut vm6, "SMC", 0);
    insert_mc_si(&mut vm6, smcid, NodeKind::McSiD1, &[node_ip]);
    insert_mc_si(&mut vm6, smcid, NodeKind::McSiD2, &[node_is]);
    insert_srff(&mut vm6, smcid);
    insert_obuf(&mut vm6, smcid, 2);

    vm6.iostd_default = Some(iostd.into());
    vm6.iostd.insert("IP_PAD".into(), iostd.into());
    vm6.iostd.insert("IS_PAD".into(), iostd.into());
    vm6.iostd.insert("FMC_PAD".into(), iostd.into());
    vm6.iostd.insert("SMC_PAD".into(), iostd.into());

    let (_, sdf) = run_tsim(tc, &vm6).unwrap();
    let sdf = Sdf::parse(&sdf);
    assert_eq!(sdf.timescale, Some(3));

    let SpeedVal::Delay(del_ibuf_imux) = speed.vals["DEL_IBUF_IMUX"] else {
        unreachable!()
    };
    if has_plain {
        let del_ibuf_plain = extract_buf(&sdf, "IP");
        set_timing_delay(
            speed,
            &format!("DEL_IBUF_PLAIN.{iostd}"),
            del_ibuf_plain - del_ibuf_imux,
        );
    }
    let del_ibuf_shcmitt = extract_buf(&sdf, "IS");
    set_timing_delay(
        speed,
        &format!("DEL_IBUF_SCHMITT.{iostd}"),
        del_ibuf_shcmitt - del_ibuf_imux,
    );

    collect_buf(&sdf, "FMC_PAD_6", speed, &format!("DEL_OBUF_FAST.{iostd}"));
    collect_buf(&sdf, "SMC_PAD_8", speed, &format!("DEL_OBUF_SLOW.{iostd}"));
}

fn test_dge(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    speed: &mut Speed,
) {
    let mut vm6 = prep_vm6(part, device, package, spd);
    let node_d = insert_ibuf(&mut vm6, "D", NodeKind::IiImux, 0x420);
    insert_ibuf(&mut vm6, "G", NodeKind::IiImux, 0x440);

    vm6.dge = Some("G_PAD".into());

    let mcid = insert_mc(&mut vm6, "MC", 0);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD1, &[node_d]);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD2, &[]);
    insert_srff(&mut vm6, mcid);
    insert_obuf(&mut vm6, mcid, 3);

    let (_, sdf) = run_tsim(tc, &vm6).unwrap();
    let sdf = Sdf::parse(&sdf);
    assert_eq!(sdf.timescale, Some(3));

    assert_eq!(extract_buf(&sdf, "D"), Time::ZERO);
    collect_buf(&sdf, "G", speed, "DEL_IBUF_IMUX");

    collect_latch(
        &sdf,
        "D_PAD_tsimcreated_dg_",
        speed,
        "DEL_IBUF_IMUX",
        "DEL_IBUF_DGE",
        None,
        "SETUPHOLD_IBUF_DGE",
        "WIDTH_IBUF_DGE",
        None,
    );
}
