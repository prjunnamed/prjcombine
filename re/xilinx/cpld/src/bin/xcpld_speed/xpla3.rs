use prjcombine_re_sdf::Sdf;
use prjcombine_re_toolchain::Toolchain;
use prjcombine_re_xilinx_cpld::{
    db::Part,
    device::{Device, Package},
    tsim::run_tsim,
    types::{PTermId, Ut},
    vm6::{InputNodeKind, NodeKind},
    vm6_util::prep_vm6,
};
use prjcombine_types::{
    FbId,
    speed::{RecRem, SetupHold, Speed, SpeedVal, Time},
};
use unnamed_entity::EntityId;

use crate::{
    extract::{
        collect_and2, collect_and2_iopath, collect_buf, collect_ff, collect_latch, collect_tri_ctl,
        collect_tri_i, extract_and2, extract_buf, set_timing, set_timing_delay,
    },
    vm6_emit::{
        insert_bufoe, insert_ct, insert_fbn, insert_ibuf, insert_mc, insert_mc_out, insert_mc_si,
        insert_obuf, insert_srff, insert_srff_inp, insert_srff_ireg,
    },
};

pub fn test_xpla3(
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
    test_ff_ut(tc, part, device, package, spd, &mut speed);
    test_ff_fclk(tc, part, device, package, spd, &mut speed);
    test_latch(tc, part, device, package, spd, &mut speed);
    // sigh. CE setup/hold is wrong in tsim, recovery is missing; fill in from data sheets
    let (s, h, r) = match (&*part.dev_name, spd) {
        ("xcr3032xl", "-5") => (2000, 3000, 3500),
        ("xcr3064xl" | "xcr3128xl", "-6") => (2000, 3000, 4000),
        ("xcr3032xl" | "xcr3064xl" | "xcr3128xl", "-7") => (2500, 4500, 5000),
        ("xcr3032xl" | "xcr3064xl" | "xcr3128xl", "-10") => (3000, 5500, 6000),
        ("xcr3256xl" | "xcr3384xl" | "xcr3512xl", "-7") => (2000, 3000, 5000),
        ("xcr3256xl" | "xcr3384xl" | "xcr3512xl", "-10") => (2500, 4500, 7000),
        ("xcr3256xl" | "xcr3384xl" | "xcr3512xl", "-12") => (3000, 5500, 8000),
        (d, s) => panic!("missing data sheet timings for {d}{s}"),
    };
    speed.vals.insert(
        "SETUPHOLD_CE_CLK".into(),
        SpeedVal::SetupHold(SetupHold {
            setup: Time(s.into()),
            hold: Time(h.into()),
        }),
    );
    set_timing(
        &mut speed,
        "RECREM_SR_CLK",
        SpeedVal::RecRem(RecRem {
            recovery: Time(r.into()),
            removal: Time((0.0).into()),
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
    let node_if = insert_ibuf(&mut vm6, "IF", NodeKind::IiImux, 0);

    let node_f = insert_fbn(&mut vm6, "FBN", &[node_if]);

    let mcid = insert_mc(&mut vm6, "MC", 0);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD1, &[node_i1]);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD2, &[node_i2, node_f]);
    insert_srff(&mut vm6, mcid);
    insert_obuf(&mut vm6, mcid, 3);

    let mc_uim = insert_mc_out(&mut vm6, mcid, NodeKind::McUim);
    let umcid = insert_mc(&mut vm6, "UMC", 0);
    insert_mc_si(&mut vm6, umcid, NodeKind::McSiD1, &[mc_uim]);
    insert_mc_si(&mut vm6, umcid, NodeKind::McSiD2, &[]);
    insert_srff(&mut vm6, umcid);
    insert_obuf(&mut vm6, umcid, 2);

    let (_, sdf) = run_tsim(tc, &vm6).unwrap();
    let sdf = Sdf::parse(&sdf);
    assert_eq!(sdf.timescale, Some(3));

    collect_buf(&sdf, "I1", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "I2", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "IF", speed, "DEL_IBUF_IMUX");

    collect_and2(&sdf, "MC.D1", speed, "DEL_IMUX_PT");
    collect_and2(&sdf, "MC.D2_PT_0", speed, "DEL_IMUX_OR");
    collect_and2(&sdf, "MC.D2_PT_1", speed, "DEL_IMUX_OR");
    collect_and2_iopath(&sdf, "FBN", speed, "DEL_IMUX_FBN");

    assert_eq!(extract_buf(&sdf, "MC.Q"), Time::ZERO);
    assert_eq!(extract_buf(&sdf, "UMC.Q"), Time::ZERO);
    collect_buf(&sdf, "MC_PAD_8", speed, "DEL_OBUF_FAST");
    collect_buf(&sdf, "UMC_PAD_10", speed, "DEL_OBUF_SLOW");

    let SpeedVal::Delay(del_imux_pt) = speed.vals["DEL_IMUX_PT"] else {
        unreachable!()
    };
    let uim = extract_and2(&sdf, "UMC.D1");
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
    let node_c = insert_ibuf(&mut vm6, "C", NodeKind::IiImux, 0);
    let mcid = insert_mc(&mut vm6, "MC", 0);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD1, &[node_d]);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD2, &[]);
    let node_sc = insert_mc_si(&mut vm6, mcid, NodeKind::McSiClkf, &[node_c]);
    insert_srff(&mut vm6, mcid);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffC, node_sc);
    insert_obuf(&mut vm6, mcid, 3);

    let (_, sdf) = run_tsim(tc, &vm6).unwrap();
    let sdf = Sdf::parse(&sdf);
    assert_eq!(sdf.timescale, Some(3));

    collect_buf(&sdf, "D", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "C", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "MC_PAD_6", speed, "DEL_OBUF_FAST");

    collect_and2(&sdf, "MC.D1", speed, "DEL_IMUX_PT");
    collect_and2(&sdf, "MC.CLKF", speed, "DEL_IMUX_PT_CLK");

    collect_ff(
        &sdf,
        "MC.REG",
        speed,
        "DEL_CLK_Q",
        "DEL_SR_Q",
        "SETUPHOLD_D_CLK",
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
    let node_c = insert_ibuf(&mut vm6, "C", NodeKind::IiImux, 0);
    let node_r = insert_ibuf(&mut vm6, "R", NodeKind::IiImux, 0);
    let node_s = insert_ibuf(&mut vm6, "S", NodeKind::IiImux, 0);
    let node_e = insert_ibuf(&mut vm6, "E", NodeKind::IiImux, 0);
    let node_ce = insert_ibuf(&mut vm6, "CE", NodeKind::IiImux, 0);
    let mcid = insert_mc(&mut vm6, "MC", 0x4000);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD1, &[node_d]);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD2, &[]);
    let node_sc = insert_ct(&mut vm6, FbId::from_idx(0), PTermId::from_idx(5), &[node_c]);
    let node_sr = insert_ct(&mut vm6, FbId::from_idx(0), PTermId::from_idx(0), &[node_r]);
    let node_ss = insert_ct(&mut vm6, FbId::from_idx(0), PTermId::from_idx(1), &[node_s]);
    let node_se = insert_ct(&mut vm6, FbId::from_idx(0), PTermId::from_idx(2), &[node_e]);
    let node_sce = insert_ct(
        &mut vm6,
        FbId::from_idx(0),
        PTermId::from_idx(4),
        &[node_ce],
    );
    insert_srff(&mut vm6, mcid);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffC, node_sc);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffR, node_sr);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffS, node_ss);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffCe, node_sce);
    insert_bufoe(&mut vm6, mcid, node_se);
    insert_obuf(&mut vm6, mcid, 3);

    let (_, sdf) = run_tsim(tc, &vm6).unwrap();
    let sdf = Sdf::parse(&sdf);
    assert_eq!(sdf.timescale, Some(3));

    collect_buf(&sdf, "D", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "C", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "R", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "S", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "E", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "CE", speed, "DEL_IBUF_IMUX");
    collect_tri_i(&sdf, "MC_PAD_14", speed, "DEL_OBUF_FAST");
    collect_tri_ctl(&sdf, "MC_PAD_14", speed, "DEL_OBUF_OE");

    collect_and2(&sdf, "MC.D1", speed, "DEL_IMUX_PT");
    collect_and2(&sdf, "FOOBAR1__ctinst/0", speed, "DEL_IMUX_PT");
    collect_and2(&sdf, "FOOBAR1__ctinst/1", speed, "DEL_IMUX_PT");
    collect_and2(&sdf, "FOOBAR1__ctinst/2", speed, "DEL_IMUX_PT");
    collect_and2(&sdf, "FOOBAR1__ctinst/4", speed, "DEL_IMUX_PT_CLK"); // umm what?
    collect_and2(&sdf, "FOOBAR1__ctinst/5", speed, "DEL_IMUX_PT");

    collect_ff(
        &sdf,
        "MC.REG",
        speed,
        "DEL_CLK_Q",
        "DEL_SR_Q",
        "SETUPHOLD_D_CLK",
        Some("SETUPHOLD_CE_CLK"),
        "WIDTH_CLK_PT",
        "WIDTH_SR",
    );
}

fn test_ff_ut(
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
    let node_ce = insert_ibuf(&mut vm6, "CE", NodeKind::IiImux, 0);
    let mcid = insert_mc(&mut vm6, "MC", 0x4000);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD1, &[node_d]);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD2, &[]);
    let node_sc = insert_ct(&mut vm6, FbId::from_idx(0), PTermId::from_idx(6), &[node_c]);
    let node_se = insert_ct(&mut vm6, FbId::from_idx(0), PTermId::from_idx(7), &[node_e]);
    let node_sr = insert_ct(&mut vm6, FbId::from_idx(1), PTermId::from_idx(6), &[node_r]);
    let node_ss = insert_ct(&mut vm6, FbId::from_idx(1), PTermId::from_idx(7), &[node_s]);
    let node_sce = insert_mc_si(&mut vm6, mcid, NodeKind::McSiCe, &[node_ce]);
    insert_srff(&mut vm6, mcid);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffC, node_sc);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffR, node_sr);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffS, node_ss);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffCe, node_sce);
    insert_bufoe(&mut vm6, mcid, node_se);
    insert_obuf(&mut vm6, mcid, 3);
    vm6.utc[Ut::Clk] = Some(vm6.nodes[node_sc].name.clone());
    vm6.utc[Ut::Oe] = Some(vm6.nodes[node_se].name.clone());
    vm6.utc[Ut::Rst] = Some(vm6.nodes[node_sr].name.clone());
    vm6.utc[Ut::Set] = Some(vm6.nodes[node_ss].name.clone());

    let (_, sdf) = run_tsim(tc, &vm6).unwrap();
    let sdf = Sdf::parse(&sdf);
    assert_eq!(sdf.timescale, Some(3));

    collect_buf(&sdf, "D", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "C", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "R", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "S", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "E", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "CE", speed, "DEL_IBUF_IMUX");
    collect_tri_i(&sdf, "MC_PAD_14", speed, "DEL_OBUF_FAST");
    collect_tri_ctl(&sdf, "MC_PAD_14", speed, "DEL_OBUF_OE");

    collect_and2(&sdf, "MC.D1", speed, "DEL_IMUX_PT");
    let ut = extract_and2(&sdf, "FOOBAR1__ctinst/6");
    assert_eq!(ut, extract_and2(&sdf, "FOOBAR1__ctinst/7"));
    assert_eq!(ut, extract_and2(&sdf, "FOOBAR2__ctinst/6"));
    assert_eq!(ut, extract_and2(&sdf, "FOOBAR2__ctinst/7"));
    collect_and2(&sdf, "MC.CE", speed, "DEL_IMUX_PT");
    let SpeedVal::Delay(del_imux_pt) = speed.vals["DEL_IMUX_PT"] else {
        unreachable!()
    };
    set_timing_delay(speed, "DEL_PT_UT", ut - del_imux_pt);

    collect_ff(
        &sdf,
        "MC.REG",
        speed,
        "DEL_CLK_Q",
        "DEL_SR_Q",
        "SETUPHOLD_D_CLK",
        Some("SETUPHOLD_CE_CLK"),
        "WIDTH_CLK_PT",
        "WIDTH_SR",
    );
}

fn test_ff_fclk(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    speed: &mut Speed,
) {
    let mut vm6 = prep_vm6(part, device, package, spd);
    let node_d = insert_ibuf(&mut vm6, "D", NodeKind::IiReg, 0);
    let node_c = insert_ibuf(&mut vm6, "C", NodeKind::IiFclk, 0);
    let mcid = insert_mc(&mut vm6, "MC", 0);
    insert_srff_ireg(&mut vm6, mcid, node_d);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffC, node_c);
    insert_obuf(&mut vm6, mcid, 3);

    let (_, sdf) = run_tsim(tc, &vm6).unwrap();
    let sdf = Sdf::parse(&sdf);
    assert_eq!(sdf.timescale, Some(3));

    collect_buf(&sdf, "D", speed, "DEL_IBUF_D");
    collect_buf(&sdf, "C", speed, "DEL_IBUF_FCLK");
    collect_buf(&sdf, "MC_PAD_9", speed, "DEL_OBUF_FAST");

    collect_ff(
        &sdf,
        "MC.REG",
        speed,
        "DEL_CLK_Q",
        "DEL_SR_Q",
        "SETUPHOLD_D_CLK",
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
    let mcid = insert_mc(&mut vm6, "MC", 0x4040);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD1, &[node_d]);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD2, &[]);
    let node_sc = insert_ct(&mut vm6, FbId::from_idx(0), PTermId::from_idx(5), &[node_c]);
    let node_sr = insert_ct(&mut vm6, FbId::from_idx(0), PTermId::from_idx(0), &[node_r]);
    let node_ss = insert_ct(&mut vm6, FbId::from_idx(0), PTermId::from_idx(1), &[node_s]);
    let node_se = insert_ct(&mut vm6, FbId::from_idx(0), PTermId::from_idx(2), &[node_e]);
    insert_srff(&mut vm6, mcid);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffC, node_sc);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffR, node_sr);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffS, node_ss);
    insert_bufoe(&mut vm6, mcid, node_se);
    insert_obuf(&mut vm6, mcid, 3);

    let (_, sdf) = run_tsim(tc, &vm6).unwrap();
    let sdf = Sdf::parse(&sdf);
    assert_eq!(sdf.timescale, Some(3));

    collect_buf(&sdf, "D", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "C", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "R", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "S", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "E", speed, "DEL_IBUF_IMUX");
    collect_tri_i(&sdf, "MC_PAD_12", speed, "DEL_OBUF_FAST");
    collect_tri_ctl(&sdf, "MC_PAD_12", speed, "DEL_OBUF_OE");

    collect_and2(&sdf, "MC.D1", speed, "DEL_IMUX_PT");
    collect_and2(&sdf, "FOOBAR1__ctinst/0", speed, "DEL_IMUX_PT");
    collect_and2(&sdf, "FOOBAR1__ctinst/1", speed, "DEL_IMUX_PT");
    collect_and2(&sdf, "FOOBAR1__ctinst/2", speed, "DEL_IMUX_PT");
    collect_and2(&sdf, "FOOBAR1__ctinst/5", speed, "DEL_IMUX_PT");

    collect_latch(
        &sdf,
        "MC.REG",
        speed,
        "DEL_D_Q_LATCH",
        "DEL_CLK_Q",
        Some("DEL_SR_Q"),
        "SETUPHOLD_D_CLK",
        "WIDTH_CLK_PT",
        Some("WIDTH_SR"),
    );
}
