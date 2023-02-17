use prjcombine_sdf::Sdf;
use prjcombine_toolchain::Toolchain;
use prjcombine_vm6::{InputNodeKind, NodeKind};
use prjcombine_xilinx_cpld::device::{Device, DeviceKind, Package};
use prjcombine_xilinx_cpld::timing::Timing;
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
) -> Timing {
    let mut timing = Timing::default();
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
    timing.recovery_sr_clk = Some(rec);
    timing
}

fn test_comb(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    lp: bool,
    timing: &mut Timing,
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

    extract_buf(&sdf, "I", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "IX", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "IE", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "IEE", &mut timing.del_ibuf_imux);

    extract_and2(&sdf, "EMC.EXP_PT_1", &mut timing.del_exp_exp);
    let del_imux_d = if lp {
        &mut timing.del_imux_d_lp
    } else {
        &mut timing.del_imux_d_hp
    };
    extract_and2(&sdf, "MC.D1", del_imux_d);
    extract_and2(&sdf, "MC.D2_PT_0", del_imux_d);
    let del_imux_d = del_imux_d.unwrap();

    let mut del_imux_exp_d = None;
    extract_and2(&sdf, "EEMC.EXP", &mut del_imux_exp_d);
    extract_and2(&sdf, "EMC.EXP_PT_0", &mut del_imux_exp_d);
    set_timing(&mut timing.del_exp_d, del_imux_exp_d.unwrap() - del_imux_d);

    extract_buf(&sdf, "MC.Q", &mut timing.del_d_q_comb);
    extract_buf(&sdf, "UMC.Q", &mut timing.del_d_q_comb);
    extract_buf(&sdf, "MC_PAD_10", &mut timing.del_obuf_fast);
    extract_buf(&sdf, "UMC_PAD_12", &mut timing.del_obuf_fast);
    if device.has_fbk {
        extract_buf(&sdf, "FMC.Q", &mut timing.del_d_q_comb);
        extract_buf(&sdf, "FMC_PAD_14", &mut timing.del_obuf_fast);
    }

    let mut uim = None;
    extract_and2(&sdf, "UMC.D2", &mut uim);
    set_timing(&mut timing.del_uim_imux, uim.unwrap() - del_imux_d);
    if device.has_fbk {
        let mut fbk = None;
        extract_and2(&sdf, "FMC.D2", &mut fbk);
        set_timing(&mut timing.del_fbk_imux, fbk.unwrap() - del_imux_d);
    }
}

fn test_reg_pt(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    lp: bool,
    timing: &mut Timing,
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

    extract_buf(&sdf, "D", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "C", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "R", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "S", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "E", &mut timing.del_ibuf_imux);
    extract_tri_i(&sdf, "MC_PAD_12", &mut timing.del_obuf_slow);

    let del_imux_d = if lp {
        &mut timing.del_imux_d_lp
    } else {
        &mut timing.del_imux_d_hp
    };
    extract_and2(&sdf, "MC.D2", del_imux_d);
    if device.kind == DeviceKind::Xc9500 {
        extract_and2(&sdf, "MC.RSTF", del_imux_d);
        extract_and2(&sdf, "MC.SETF", del_imux_d);
    } else {
        extract_and2(&sdf, "MC.RSTF", &mut timing.del_imux_pt_sr);
        extract_and2(&sdf, "MC.SETF", &mut timing.del_imux_pt_sr);
    }
    extract_and2(&sdf, "MC.CLKF", &mut timing.del_imux_pt_clk);
    extract_and2(&sdf, "MC.TRST", &mut timing.del_imux_pt_oe);

    let mut period_clk_pt = None;
    extract_ff(
        &sdf,
        "MC.REG",
        &mut timing.del_clk_q,
        &mut timing.del_sr_q,
        &mut timing.setup_d_clk,
        &mut timing.hold_d_clk,
        &mut None,
        &mut None,
        &mut period_clk_pt,
        &mut timing.width_sr,
    );
    set_timing(&mut timing.width_clk_pt, period_clk_pt.unwrap() / 2);
}

fn test_reg_f(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    lp: bool,
    timing: &mut Timing,
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

    extract_buf(&sdf, "D", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "C", &mut timing.del_ibuf_fclk);
    extract_buf(&sdf, "R", &mut timing.del_ibuf_fsr);
    extract_buf(&sdf, "E", &mut timing.del_ibuf_foe);
    if device.kind != DeviceKind::Xc9500 {
        extract_tri_i(&sdf, "MC_PAD_12", &mut timing.del_obuf_fast);
        extract_buf(&sdf, "CE", &mut timing.del_ibuf_imux);
    } else {
        extract_tri_i(&sdf, "MC_PAD_10", &mut timing.del_obuf_fast);
    }

    let del_imux_d = if lp {
        &mut timing.del_imux_d_lp
    } else {
        &mut timing.del_imux_d_hp
    };
    extract_and2(&sdf, "MC.D2", del_imux_d);
    if device.kind != DeviceKind::Xc9500 {
        extract_and2(&sdf, "MC.CE", &mut timing.del_imux_pt_ce);
    }
    let mut dummy_a = None;
    let mut dummy_b = None;
    let mut period_clk = None;
    extract_ff(
        &sdf,
        "MC.REG",
        &mut timing.del_clk_q,
        &mut timing.del_sr_q,
        &mut timing.setup_d_clk,
        &mut timing.hold_d_clk,
        if device.kind != DeviceKind::Xc9500 {
            &mut timing.setup_ce_clk
        } else {
            &mut dummy_a
        },
        if device.kind != DeviceKind::Xc9500 {
            &mut timing.hold_ce_clk
        } else {
            &mut dummy_b
        },
        &mut period_clk,
        &mut timing.width_sr,
    );
    set_timing(&mut timing.width_clk, period_clk.unwrap() / 2);
}
