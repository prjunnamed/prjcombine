use prjcombine_sdf::Sdf;
use prjcombine_toolchain::Toolchain;
use prjcombine_vm6::{InputNodeKind, NodeKind};
use prjcombine_xilinx_cpld::{
    device::{Device, Package},
    timing::Timing,
    types::{FbId, PTermId, Ut},
};
use prjcombine_xilinx_recpld::{db::Part, tsim::run_tsim, vm6::prep_vm6};
use unnamed_entity::EntityId;

use crate::{
    extract::{
        extract_and2, extract_and2_iopath, extract_buf, extract_ff, extract_latch, extract_tri_ctl,
        extract_tri_i, set_timing,
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
) -> Timing {
    let mut timing = Timing::default();
    test_comb(tc, part, device, package, spd, &mut timing);
    test_ff_pt(tc, part, device, package, spd, &mut timing);
    test_ff_ct(tc, part, device, package, spd, &mut timing);
    test_ff_ut(tc, part, device, package, spd, &mut timing);
    test_ff_fclk(tc, part, device, package, spd, &mut timing);
    test_latch(tc, part, device, package, spd, &mut timing);
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
    timing.setup_ce_clk = Some(s);
    timing.hold_ce_clk = Some(h);
    timing.recovery_sr_clk = Some(r);
    timing
}

fn test_comb(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    timing: &mut Timing,
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

    extract_buf(&sdf, "I1", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "I2", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "IF", &mut timing.del_ibuf_imux);

    extract_and2(&sdf, "MC.D1", &mut timing.del_imux_pt);
    extract_and2(&sdf, "MC.D2_PT_0", &mut timing.del_imux_or);
    extract_and2(&sdf, "MC.D2_PT_1", &mut timing.del_imux_or);
    extract_and2_iopath(&sdf, "FBN", &mut timing.del_imux_fbn);

    let mut zero = Some(0);
    extract_buf(&sdf, "MC.Q", &mut zero);
    extract_buf(&sdf, "UMC.Q", &mut zero);
    extract_buf(&sdf, "MC_PAD_8", &mut timing.del_obuf_fast);
    extract_buf(&sdf, "UMC_PAD_10", &mut timing.del_obuf_slow);

    let mut uim = None;
    extract_and2(&sdf, "UMC.D1", &mut uim);
    let del_imux_pt = timing.del_imux_pt.unwrap();
    set_timing(&mut timing.del_uim_imux, uim.unwrap() - del_imux_pt);
}

fn test_ff_pt(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    timing: &mut Timing,
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

    extract_buf(&sdf, "D", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "C", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "MC_PAD_6", &mut timing.del_obuf_fast);

    extract_and2(&sdf, "MC.D1", &mut timing.del_imux_pt);
    extract_and2(&sdf, "MC.CLKF", &mut timing.del_imux_pt_clk);

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

fn test_ff_ct(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    timing: &mut Timing,
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

    extract_buf(&sdf, "D", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "C", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "R", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "S", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "E", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "CE", &mut timing.del_ibuf_imux);
    extract_tri_i(&sdf, "MC_PAD_14", &mut timing.del_obuf_fast);
    extract_tri_ctl(&sdf, "MC_PAD_14", &mut timing.del_obuf_oe);

    extract_and2(&sdf, "MC.D1", &mut timing.del_imux_pt);
    extract_and2(&sdf, "FOOBAR1__ctinst/0", &mut timing.del_imux_pt);
    extract_and2(&sdf, "FOOBAR1__ctinst/1", &mut timing.del_imux_pt);
    extract_and2(&sdf, "FOOBAR1__ctinst/2", &mut timing.del_imux_pt);
    extract_and2(&sdf, "FOOBAR1__ctinst/4", &mut timing.del_imux_pt_clk); // umm what?
    extract_and2(&sdf, "FOOBAR1__ctinst/5", &mut timing.del_imux_pt);

    let mut period_clk_pt = None;
    extract_ff(
        &sdf,
        "MC.REG",
        &mut timing.del_clk_q,
        &mut timing.del_sr_q,
        &mut timing.setup_d_clk,
        &mut timing.hold_d_clk,
        &mut timing.setup_ce_clk,
        &mut timing.hold_ce_clk,
        &mut period_clk_pt,
        &mut timing.width_sr,
    );
    set_timing(&mut timing.width_clk_pt, period_clk_pt.unwrap() / 2);
}

fn test_ff_ut(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    timing: &mut Timing,
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

    extract_buf(&sdf, "D", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "C", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "R", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "S", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "E", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "CE", &mut timing.del_ibuf_imux);
    extract_tri_i(&sdf, "MC_PAD_14", &mut timing.del_obuf_fast);
    extract_tri_ctl(&sdf, "MC_PAD_14", &mut timing.del_obuf_oe);

    extract_and2(&sdf, "MC.D1", &mut timing.del_imux_pt);
    let mut ut = None;
    extract_and2(&sdf, "FOOBAR1__ctinst/6", &mut ut);
    extract_and2(&sdf, "FOOBAR1__ctinst/7", &mut ut);
    extract_and2(&sdf, "FOOBAR2__ctinst/6", &mut ut);
    extract_and2(&sdf, "FOOBAR2__ctinst/7", &mut ut);
    extract_and2(&sdf, "MC.CE", &mut timing.del_imux_pt);
    set_timing(
        &mut timing.del_pt_ut,
        ut.unwrap() - timing.del_imux_pt.unwrap(),
    );

    let mut period_clk_pt = None;
    extract_ff(
        &sdf,
        "MC.REG",
        &mut timing.del_clk_q,
        &mut timing.del_sr_q,
        &mut timing.setup_d_clk,
        &mut timing.hold_d_clk,
        &mut timing.setup_ce_clk,
        &mut timing.hold_ce_clk,
        &mut period_clk_pt,
        &mut timing.width_sr,
    );
    set_timing(&mut timing.width_clk_pt, period_clk_pt.unwrap() / 2);
}

fn test_ff_fclk(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    timing: &mut Timing,
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

    extract_buf(&sdf, "D", &mut timing.del_ibuf_d);
    extract_buf(&sdf, "C", &mut timing.del_ibuf_fclk);
    extract_buf(&sdf, "MC_PAD_9", &mut timing.del_obuf_fast);

    let mut period_clk = None;
    extract_ff(
        &sdf,
        "MC.REG",
        &mut timing.del_clk_q,
        &mut timing.del_sr_q,
        &mut timing.setup_d_clk,
        &mut timing.hold_d_clk,
        &mut None,
        &mut None,
        &mut period_clk,
        &mut timing.width_sr,
    );
    set_timing(&mut timing.width_clk, period_clk.unwrap() / 2);
}

fn test_latch(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    timing: &mut Timing,
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

    extract_buf(&sdf, "D", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "C", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "R", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "S", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "E", &mut timing.del_ibuf_imux);
    extract_tri_i(&sdf, "MC_PAD_12", &mut timing.del_obuf_fast);
    extract_tri_ctl(&sdf, "MC_PAD_12", &mut timing.del_obuf_oe);

    extract_and2(&sdf, "MC.D1", &mut timing.del_imux_pt);
    extract_and2(&sdf, "FOOBAR1__ctinst/0", &mut timing.del_imux_pt);
    extract_and2(&sdf, "FOOBAR1__ctinst/1", &mut timing.del_imux_pt);
    extract_and2(&sdf, "FOOBAR1__ctinst/2", &mut timing.del_imux_pt);
    extract_and2(&sdf, "FOOBAR1__ctinst/5", &mut timing.del_imux_pt);

    extract_latch(
        &sdf,
        "MC.REG",
        &mut timing.del_d_q_latch,
        &mut timing.del_clk_q,
        &mut timing.del_sr_q,
        &mut timing.setup_d_clk,
        &mut timing.hold_d_clk,
        &mut timing.width_clk_pt,
        &mut timing.width_sr,
    );
}
