use prjcombine_re_sdf::Sdf;
use prjcombine_re_toolchain::Toolchain;
use prjcombine_re_xilinx_cpld::device::{Device, DeviceKind, Package};
use prjcombine_re_xilinx_cpld::vm6::{InputNodeKind, NodeKind};
use prjcombine_re_xilinx_cpld::{db::Part, tsim::run_tsim, vm6_util::prep_vm6};
use prjcombine_types::speed::{RecRem, Speed, SpeedVal, Time};

use crate::extract::{extract_and2, set_timing_delay};
use crate::{
    extract::{collect_and2, collect_buf, collect_ff, collect_tri_i, set_timing},
    vm6_emit::{
        insert_bufoe, insert_ibuf, insert_mc, insert_mc_out, insert_mc_si, insert_obuf,
        insert_srff, insert_srff_inp,
    },
};

pub fn test_xc9500(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
) -> Speed {
    let mut speed = Speed::new();
    for lp in [false, true] {
        test_comb(tc, part, device, package, spd, lp, &mut speed);
        test_reg_pt(tc, part, device, package, spd, lp, &mut speed);
        test_reg_f(tc, part, device, package, spd, lp, &mut speed);
    }
    // obtained from data sheets
    let rec = match spd {
        "-5" => 5000,
        "-6" => 6000,
        "-7" => 7500,
        "-10" => 10000,
        "-15" => 10000,
        "-20" => 10000,
        _ => panic!("missing recovery data for {spd}"),
    };
    set_timing(
        &mut speed,
        "RECREM_SR_CLK",
        SpeedVal::RecRem(RecRem {
            recovery: Time(rec.into()),
            removal: Time::ZERO,
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
    lp: bool,
    speed: &mut Speed,
) {
    let mut vm6 = prep_vm6(part, device, package, spd);
    let node_i = insert_ibuf(&mut vm6, "I", NodeKind::IiImux, 0);
    let node_ix = insert_ibuf(&mut vm6, "IX", NodeKind::IiImux, 0);
    let node_ie = insert_ibuf(&mut vm6, "IE", NodeKind::IiImux, 0);
    let node_iee = insert_ibuf(&mut vm6, "IEE", NodeKind::IiImux, 0);

    let eemcid = insert_mc(&mut vm6, "EEMC", if lp { 1 } else { 0 });
    insert_mc_si(&mut vm6, eemcid, NodeKind::McSiExport, &[node_iee]);
    let node_ee = insert_mc_out(&mut vm6, eemcid, NodeKind::McExport);

    let emcid = insert_mc(&mut vm6, "EMC", if lp { 1 } else { 0 });
    insert_mc_si(&mut vm6, emcid, NodeKind::McSiExport, &[node_ie, node_ee]);
    let node_e = insert_mc_out(&mut vm6, emcid, NodeKind::McExport);

    let mcid = insert_mc(&mut vm6, "MC", if lp { 1 } else { 0 });
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD1, &[node_ix]);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD2, &[node_i, node_e]);
    insert_srff(&mut vm6, mcid);
    insert_obuf(&mut vm6, mcid, 3);

    let mc_uim = insert_mc_out(&mut vm6, mcid, NodeKind::McUim);
    let umcid = insert_mc(&mut vm6, "UMC", if lp { 1 } else { 0 });
    insert_mc_si(&mut vm6, umcid, NodeKind::McSiD1, &[]);
    insert_mc_si(&mut vm6, umcid, NodeKind::McSiD2, &[mc_uim]);
    insert_srff(&mut vm6, umcid);
    insert_obuf(&mut vm6, umcid, 3);

    if device.has_fbk {
        let mc_fbk = insert_mc_out(&mut vm6, mcid, NodeKind::McFbk);
        let fmcid = insert_mc(&mut vm6, "FMC", if lp { 1 } else { 0 });
        insert_mc_si(&mut vm6, fmcid, NodeKind::McSiD1, &[]);
        insert_mc_si(&mut vm6, fmcid, NodeKind::McSiD2, &[mc_fbk]);
        insert_srff(&mut vm6, fmcid);
        insert_obuf(&mut vm6, fmcid, 3);
    }

    let (_, sdf) = run_tsim(tc, &vm6).unwrap();
    let sdf = Sdf::parse(&sdf);
    assert_eq!(sdf.timescale, Some(3));

    collect_buf(&sdf, "I", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "IX", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "IE", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "IEE", speed, "DEL_IBUF_IMUX");

    collect_and2(&sdf, "EMC.EXP_PT_1", speed, "DEL_EXP_EXP");
    let del_imux_d = if lp { "DEL_IMUX_D_LP" } else { "DEL_IMUX_D_HP" };
    collect_and2(&sdf, "MC.D1", speed, del_imux_d);
    collect_and2(&sdf, "MC.D2_PT_0", speed, del_imux_d);
    let SpeedVal::Delay(del_imux_d) = speed.vals[del_imux_d] else {
        unreachable!()
    };

    let del_imux_exp_d = extract_and2(&sdf, "EEMC.EXP");
    assert_eq!(del_imux_exp_d, extract_and2(&sdf, "EMC.EXP_PT_0"));
    set_timing_delay(speed, "DEL_EXP_D", del_imux_exp_d - del_imux_d);

    collect_buf(&sdf, "MC.Q", speed, "DEL_D_Q_COMB");
    collect_buf(&sdf, "UMC.Q", speed, "DEL_D_Q_COMB");
    collect_buf(&sdf, "MC_PAD_10", speed, "DEL_OBUF_FAST");
    collect_buf(&sdf, "UMC_PAD_12", speed, "DEL_OBUF_FAST");
    if device.has_fbk {
        collect_buf(&sdf, "FMC.Q", speed, "DEL_D_Q_COMB");
        collect_buf(&sdf, "FMC_PAD_14", speed, "DEL_OBUF_FAST");
    }

    let uim = extract_and2(&sdf, "UMC.D2");
    set_timing_delay(speed, "DEL_UIM_IMUX", uim - del_imux_d);
    if device.has_fbk {
        let fbk = extract_and2(&sdf, "FMC.D2");
        set_timing_delay(speed, "DEL_FBK_IMUX", fbk - del_imux_d);
    }
}

fn test_reg_pt(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    lp: bool,
    speed: &mut Speed,
) {
    let mut vm6 = prep_vm6(part, device, package, spd);

    let node_d = insert_ibuf(&mut vm6, "D", NodeKind::IiImux, 0);
    let node_c = insert_ibuf(&mut vm6, "C", NodeKind::IiImux, 0);
    let node_r = insert_ibuf(&mut vm6, "R", NodeKind::IiImux, 0);
    let node_s = insert_ibuf(&mut vm6, "S", NodeKind::IiImux, 0);
    let node_e = insert_ibuf(&mut vm6, "E", NodeKind::IiImux, 0);

    let mcid = insert_mc(&mut vm6, "MC", if lp { 1 } else { 0 });
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD1, &[]);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD2, &[node_d]);
    let node_sc = insert_mc_si(&mut vm6, mcid, NodeKind::McSiClkf, &[node_c]);
    let node_sr = insert_mc_si(&mut vm6, mcid, NodeKind::McSiRstf, &[node_r]);
    let node_ss = insert_mc_si(&mut vm6, mcid, NodeKind::McSiSetf, &[node_s]);
    let node_se = insert_mc_si(&mut vm6, mcid, NodeKind::McSiTrst, &[node_e]);
    insert_srff(&mut vm6, mcid);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffC, node_sc);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffR, node_sr);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffS, node_ss);
    insert_bufoe(&mut vm6, mcid, node_se);
    insert_obuf(&mut vm6, mcid, 2);

    let (_, sdf) = run_tsim(tc, &vm6).unwrap();
    let sdf = Sdf::parse(&sdf);
    assert_eq!(sdf.timescale, Some(3));

    collect_buf(&sdf, "D", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "C", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "R", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "S", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "E", speed, "DEL_IBUF_IMUX");
    collect_tri_i(&sdf, "MC_PAD_12", speed, "DEL_OBUF_SLOW");

    let del_imux_d = if lp { "DEL_IMUX_D_LP" } else { "DEL_IMUX_D_HP" };
    collect_and2(&sdf, "MC.D2", speed, del_imux_d);
    if device.kind == DeviceKind::Xc9500 {
        collect_and2(&sdf, "MC.RSTF", speed, del_imux_d);
        collect_and2(&sdf, "MC.SETF", speed, del_imux_d);
    } else {
        collect_and2(&sdf, "MC.RSTF", speed, "DEL_IMUX_PT_SR");
        collect_and2(&sdf, "MC.SETF", speed, "DEL_IMUX_PT_SR");
    }
    collect_and2(&sdf, "MC.CLKF", speed, "DEL_IMUX_PT_CLK");
    collect_and2(&sdf, "MC.TRST", speed, "DEL_IMUX_PT_OE");

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

fn test_reg_f(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    lp: bool,
    speed: &mut Speed,
) {
    let mut vm6 = prep_vm6(part, device, package, spd);

    let node_d = insert_ibuf(&mut vm6, "D", NodeKind::IiImux, 0);
    let node_c = insert_ibuf(&mut vm6, "C", NodeKind::IiFclk, 0);
    let node_r = insert_ibuf(&mut vm6, "R", NodeKind::IiFsr, 0);
    let node_e = insert_ibuf(&mut vm6, "E", NodeKind::IiFoe, 0);

    let mcid = insert_mc(&mut vm6, "MC", if lp { 1 } else { 0 });
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD1, &[]);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD2, &[node_d]);
    insert_srff(&mut vm6, mcid);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffC, node_c);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffR, node_r);
    if device.kind != DeviceKind::Xc9500 {
        let node_ce = insert_ibuf(&mut vm6, "CE", NodeKind::IiImux, 0);
        let node_sce = insert_mc_si(&mut vm6, mcid, NodeKind::McSiCe, &[node_ce]);
        insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffCe, node_sce);
    }
    insert_bufoe(&mut vm6, mcid, node_e);
    insert_obuf(&mut vm6, mcid, 3);

    let (_, sdf) = run_tsim(tc, &vm6).unwrap();
    let sdf = Sdf::parse(&sdf);
    assert_eq!(sdf.timescale, Some(3));

    collect_buf(&sdf, "D", speed, "DEL_IBUF_IMUX");
    collect_buf(&sdf, "C", speed, "DEL_IBUF_FCLK");
    collect_buf(&sdf, "R", speed, "DEL_IBUF_FSR");
    collect_buf(&sdf, "E", speed, "DEL_IBUF_FOE");
    if device.kind != DeviceKind::Xc9500 {
        collect_tri_i(&sdf, "MC_PAD_12", speed, "DEL_OBUF_FAST");
        collect_buf(&sdf, "CE", speed, "DEL_IBUF_IMUX");
    } else {
        collect_tri_i(&sdf, "MC_PAD_10", speed, "DEL_OBUF_FAST");
    }

    let del_imux_d = if lp { "DEL_IMUX_D_LP" } else { "DEL_IMUX_D_HP" };
    collect_and2(&sdf, "MC.D2", speed, del_imux_d);
    if device.kind != DeviceKind::Xc9500 {
        collect_and2(&sdf, "MC.CE", speed, "DEL_IMUX_PT_CE");
    }
    collect_ff(
        &sdf,
        "MC.REG",
        speed,
        "DEL_CLK_Q",
        "DEL_SR_Q",
        "SETUPHOLD_D_CLK",
        if device.kind != DeviceKind::Xc9500 {
            Some("SETUPHOLD_CE_CLK")
        } else {
            None
        },
        "WIDTH_CLK",
        "WIDTH_SR",
    );
}
