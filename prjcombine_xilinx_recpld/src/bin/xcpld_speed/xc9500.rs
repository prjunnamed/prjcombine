use std::collections::BTreeMap;

use prjcombine_sdf::Sdf;
use prjcombine_toolchain::Toolchain;
use prjcombine_vm6::{InputNodeKind, NodeKind};
use prjcombine_xilinx_cpld::device::{Device, DeviceKind, Package};
use prjcombine_xilinx_recpld::{db::Part, tsim::run_tsim, vm6::prep_vm6};

use crate::{
    extract::{extract_and2, extract_buf, extract_ff, extract_tri_i, set_timing},
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
) -> BTreeMap<String, i64> {
    let mut timing = BTreeMap::new();
    for lp in [false, true] {
        test_comb(tc, part, device, package, spd, lp, &mut timing);
        test_reg_pt(tc, part, device, package, spd, lp, &mut timing);
        test_reg_f(tc, part, device, package, spd, lp, &mut timing);
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
    timing.insert("RECOVERY_SR_CLK".into(), rec);
    timing
}

fn test_comb(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    lp: bool,
    timing: &mut BTreeMap<String, i64>,
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

    extract_buf(&sdf, "I", timing, "DEL_IBUF_IMUX");
    extract_buf(&sdf, "IX", timing, "DEL_IBUF_IMUX");
    extract_buf(&sdf, "IE", timing, "DEL_IBUF_IMUX");
    extract_buf(&sdf, "IEE", timing, "DEL_IBUF_IMUX");

    extract_and2(&sdf, "EMC.EXP_PT_1", timing, "DEL_EXP_EXP");
    let del_imux_d = if lp { "DEL_IMUX_D_LP" } else { "DEL_IMUX_D_HP" };
    extract_and2(&sdf, "MC.D1", timing, del_imux_d);
    extract_and2(&sdf, "MC.D2_PT_0", timing, del_imux_d);

    let mut tmp = BTreeMap::new();
    extract_and2(&sdf, "EEMC.EXP", &mut tmp, "DEL_IMUX_EXP_D");
    extract_and2(&sdf, "EMC.EXP_PT_0", &mut tmp, "DEL_IMUX_EXP_D");
    set_timing(
        timing,
        "DEL_EXP_D",
        tmp["DEL_IMUX_EXP_D"] - timing[del_imux_d],
    );

    extract_buf(&sdf, "MC.Q", timing, "DEL_D_Q_COMB");
    extract_buf(&sdf, "UMC.Q", timing, "DEL_D_Q_COMB");
    extract_buf(&sdf, "MC_PAD_10", timing, "DEL_OBUF_FAST");
    extract_buf(&sdf, "UMC_PAD_12", timing, "DEL_OBUF_FAST");
    if device.has_fbk {
        extract_buf(&sdf, "FMC.Q", timing, "DEL_D_Q_COMB");
        extract_buf(&sdf, "FMC_PAD_14", timing, "DEL_OBUF_FAST");
    }

    let mut tmp = BTreeMap::new();
    extract_and2(&sdf, "UMC.D2", &mut tmp, "UIM");
    set_timing(timing, "DEL_UIM_IMUX", tmp["UIM"] - timing[del_imux_d]);
    if device.has_fbk {
        let mut tmp = BTreeMap::new();
        extract_and2(&sdf, "FMC.D2", &mut tmp, "FBK");
        set_timing(timing, "DEL_FBK_IMUX", tmp["FBK"] - timing[del_imux_d]);
    }
}

fn test_reg_pt(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    lp: bool,
    timing: &mut BTreeMap<String, i64>,
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

    extract_buf(&sdf, "D", timing, "DEL_IBUF_IMUX");
    extract_buf(&sdf, "C", timing, "DEL_IBUF_IMUX");
    extract_buf(&sdf, "R", timing, "DEL_IBUF_IMUX");
    extract_buf(&sdf, "S", timing, "DEL_IBUF_IMUX");
    extract_buf(&sdf, "E", timing, "DEL_IBUF_IMUX");
    extract_tri_i(&sdf, "MC_PAD_12", timing, "DEL_OBUF_SLOW");

    let del_imux_d = if lp { "DEL_IMUX_D_LP" } else { "DEL_IMUX_D_HP" };
    extract_and2(&sdf, "MC.D2", timing, del_imux_d);
    if device.kind == DeviceKind::Xc9500 {
        extract_and2(&sdf, "MC.RSTF", timing, del_imux_d);
        extract_and2(&sdf, "MC.SETF", timing, del_imux_d);
    } else {
        extract_and2(&sdf, "MC.RSTF", timing, "DEL_IMUX_PT_SR");
        extract_and2(&sdf, "MC.SETF", timing, "DEL_IMUX_PT_SR");
    }
    extract_and2(&sdf, "MC.CLKF", timing, "DEL_IMUX_PT_CLK");
    extract_and2(&sdf, "MC.TRST", timing, "DEL_IMUX_PT_OE");

    extract_ff(
        &sdf,
        "MC.REG",
        timing,
        "DEL_CLK_Q",
        "DEL_SR_Q",
        "SETUP_D_CLK",
        "HOLD_D_CLK",
        None,
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
    timing: &mut BTreeMap<String, i64>,
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

    extract_buf(&sdf, "D", timing, "DEL_IBUF_IMUX");
    extract_buf(&sdf, "C", timing, "DEL_IBUF_FCLK");
    extract_buf(&sdf, "R", timing, "DEL_IBUF_FSR");
    extract_buf(&sdf, "E", timing, "DEL_IBUF_FOE");
    if device.kind != DeviceKind::Xc9500 {
        extract_tri_i(&sdf, "MC_PAD_12", timing, "DEL_OBUF_FAST");
        extract_buf(&sdf, "CE", timing, "DEL_IBUF_IMUX");
    } else {
        extract_tri_i(&sdf, "MC_PAD_10", timing, "DEL_OBUF_FAST");
    }

    let del_imux_d = if lp { "DEL_IMUX_D_LP" } else { "DEL_IMUX_D_HP" };
    extract_and2(&sdf, "MC.D2", timing, del_imux_d);
    if device.kind != DeviceKind::Xc9500 {
        extract_and2(&sdf, "MC.CE", timing, "DEL_IMUX_PT_CE");
    }
    extract_ff(
        &sdf,
        "MC.REG",
        timing,
        "DEL_CLK_Q",
        "DEL_SR_Q",
        "SETUP_D_CLK",
        "HOLD_D_CLK",
        if device.kind != DeviceKind::Xc9500 {
            Some("SETUP_CE_CLK")
        } else {
            None
        },
        if device.kind != DeviceKind::Xc9500 {
            Some("HOLD_CE_CLK")
        } else {
            None
        },
        "WIDTH_CLK",
        "WIDTH_SR",
    );
}
