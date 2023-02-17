use prjcombine_entity::EntityId;
use prjcombine_sdf::Sdf;
use prjcombine_toolchain::Toolchain;
use prjcombine_vm6::{InputNodeKind, NodeKind};
use prjcombine_xilinx_cpld::{
    device::{Device, Package},
    timing::Timing,
    types::{FbId, PTermId},
};
use prjcombine_xilinx_recpld::{db::Part, tsim::run_tsim, vm6::prep_vm6};

use crate::{
    extract::{
        extract_and2, extract_buf, extract_ff, extract_latch, extract_tri_ctl, extract_tri_i,
        set_timing,
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
) -> Timing {
    let mut timing = Timing::default();
    test_comb(tc, part, device, package, spd, &mut timing);
    test_ff_pt(tc, part, device, package, spd, &mut timing);
    test_ff_ct(tc, part, device, package, spd, &mut timing);
    test_ff_fast(tc, part, device, package, spd, &mut timing);
    test_latch(tc, part, device, package, spd, &mut timing);
    test_iostd(tc, part, device, package, spd, &mut timing, "LVCMOS15");
    test_iostd(tc, part, device, package, spd, &mut timing, "LVCMOS18");
    test_iostd(tc, part, device, package, spd, &mut timing, "LVCMOS18_ANY");
    test_iostd(tc, part, device, package, spd, &mut timing, "LVCMOS25");
    test_iostd(tc, part, device, package, spd, &mut timing, "LVCMOS33");
    test_iostd(tc, part, device, package, spd, &mut timing, "LVTTL");
    if device.has_vref {
        test_iostd(tc, part, device, package, spd, &mut timing, "SSTL2_I");
        test_iostd(tc, part, device, package, spd, &mut timing, "SSTL3_I");
        test_iostd(tc, part, device, package, spd, &mut timing, "HSTL_I");
    }
    if device.dge_pad.is_some() {
        test_dge(tc, part, device, package, spd, &mut timing);
    }
    if device.cdr_pad.is_some() {
        let (s, h) = match (&*part.dev_name, spd) {
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
        timing.setup_cd_rst = Some(s);
        timing.hold_cd_rst = Some(h);
    }
    let (s, h) = match (&*part.dev_name, spd) {
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
    timing.setup_ce_clk = Some(s);
    timing.hold_ce_clk = Some(h);
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

    extract_buf(&sdf, "I1", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "I2", &mut timing.del_ibuf_imux);

    extract_and2(&sdf, "MC.D1", &mut timing.del_imux_pt);
    extract_and2(&sdf, "MC.D2", &mut timing.del_imux_or);

    extract_buf(&sdf, "MC.Q", &mut timing.del_d_q_comb);
    extract_buf(&sdf, "UMC.Q", &mut timing.del_d_q_comb);
    let iostd = timing.iostd.entry("LVCMOS18".into()).or_default();
    extract_buf(&sdf, "MC_PAD_6", &mut iostd.del_obuf_fast);
    extract_buf(&sdf, "UMC_PAD_8", &mut iostd.del_obuf_slow);

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

    extract_buf(&sdf, "D", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "I", &mut timing.del_ibuf_d);
    extract_buf(&sdf, "C", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "R", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "E", &mut timing.del_ibuf_imux);
    let iostd = timing.iostd.entry("LVCMOS18".into()).or_default();
    extract_tri_i(&sdf, "MC_PAD_15", &mut iostd.del_obuf_fast);
    extract_tri_ctl(&sdf, "MC_PAD_15", &mut timing.del_obuf_oe);
    extract_buf(&sdf, "IMC_PAD_17", &mut iostd.del_obuf_fast);

    extract_and2(&sdf, "MC.D1", &mut timing.del_imux_pt);
    extract_and2(&sdf, "MC.CLKF", &mut timing.del_imux_ct);
    extract_and2(&sdf, "MC.RSTF", &mut timing.del_imux_ct);
    extract_and2(&sdf, "MC.TRST", &mut timing.del_imux_ct);
    extract_and2(&sdf, "IMC.CLKF", &mut timing.del_imux_ct);

    let mut period_clk_pt = None;
    extract_ff(
        &sdf,
        "MC.REG",
        &mut timing.del_clk_q,
        &mut timing.del_sr_q,
        &mut timing.setup_d_clk_pt_pt,
        &mut timing.hold_d_clk_pt_pt,
        &mut None,
        &mut None,
        &mut period_clk_pt,
        &mut timing.width_sr,
    );

    extract_ff(
        &sdf,
        "IMC.REG",
        &mut timing.del_clk_q,
        &mut timing.del_sr_q,
        &mut timing.setup_d_clk_ibuf_pt,
        &mut timing.hold_d_clk_ibuf_pt,
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
    let node_i = insert_ibuf(&mut vm6, "I", NodeKind::IiReg, 0);
    let node_c = insert_ibuf(&mut vm6, "C", NodeKind::IiImux, 0);
    let node_r = insert_ibuf(&mut vm6, "R", NodeKind::IiImux, 0);
    let node_s = insert_ibuf(&mut vm6, "S", NodeKind::IiImux, 0);
    let node_e = insert_ibuf(&mut vm6, "E", NodeKind::IiImux, 0);
    let node_ce = insert_ibuf(&mut vm6, "CE", NodeKind::IiImux, 0);

    let node_sc = insert_ct(&mut vm6, FbId::from_idx(0), PTermId::from_idx(4), &[node_c]);
    let node_sr = insert_ct(&mut vm6, FbId::from_idx(0), PTermId::from_idx(5), &[node_r]);
    let node_ss = insert_ct(&mut vm6, FbId::from_idx(0), PTermId::from_idx(6), &[node_s]);
    let node_se = insert_ct(&mut vm6, FbId::from_idx(0), PTermId::from_idx(7), &[node_e]);

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

    extract_buf(&sdf, "D", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "I", &mut timing.del_ibuf_d);
    extract_buf(&sdf, "C", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "R", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "S", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "E", &mut timing.del_ibuf_imux);
    let iostd = timing.iostd.entry("LVCMOS18".into()).or_default();
    extract_tri_i(&sdf, "MC_PAD_19", &mut iostd.del_obuf_fast);
    extract_tri_ctl(&sdf, "MC_PAD_19", &mut timing.del_obuf_oe);
    extract_buf(&sdf, "IMC_PAD_21", &mut iostd.del_obuf_fast);

    extract_and2(&sdf, "MC.D1", &mut timing.del_imux_pt);
    extract_and2(&sdf, "MC.CE", &mut timing.del_imux_ct);
    extract_and2(&sdf, "FOOBAR1__ctinst/4", &mut timing.del_imux_ct);
    extract_and2(&sdf, "FOOBAR1__ctinst/5", &mut timing.del_imux_ct);
    extract_and2(&sdf, "FOOBAR1__ctinst/6", &mut timing.del_imux_ct);
    extract_and2(&sdf, "FOOBAR1__ctinst/7", &mut timing.del_imux_ct);

    let mut period_clk_pt = None;
    extract_ff(
        &sdf,
        "MC.REG",
        &mut timing.del_clk_q,
        &mut timing.del_sr_q,
        &mut timing.setup_d_clk_pt_pt,
        &mut timing.hold_d_clk_pt_pt,
        &mut None,
        &mut None,
        &mut period_clk_pt,
        &mut timing.width_sr,
    );

    extract_ff(
        &sdf,
        "IMC.REG",
        &mut timing.del_clk_q,
        &mut timing.del_sr_q,
        &mut timing.setup_d_clk_ibuf_pt,
        &mut timing.hold_d_clk_ibuf_pt,
        &mut None,
        &mut None,
        &mut period_clk_pt,
        &mut timing.width_sr,
    );
    set_timing(&mut timing.width_clk_pt, period_clk_pt.unwrap() / 2);
}

fn test_ff_fast(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    timing: &mut Timing,
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

    extract_buf(&sdf, "D", &mut timing.del_ibuf_imux);
    extract_buf(&sdf, "I", &mut timing.del_ibuf_d);
    extract_buf(&sdf, "C", &mut timing.del_ibuf_fclk);
    extract_buf(&sdf, "R", &mut timing.del_ibuf_fsr);
    extract_buf(&sdf, "E", &mut timing.del_ibuf_foe);
    let iostd = timing.iostd.entry("LVCMOS18".into()).or_default();
    extract_tri_i(&sdf, "MC_PAD_17", &mut iostd.del_obuf_fast);
    extract_tri_ctl(&sdf, "MC_PAD_17", &mut timing.del_obuf_oe);
    extract_buf(&sdf, "IMC_PAD_19", &mut iostd.del_obuf_fast);

    extract_and2(&sdf, "MC.D1", &mut timing.del_imux_pt);
    extract_and2(&sdf, "MC.CE", &mut timing.del_imux_ct);

    let mut period_clk = None;
    extract_ff(
        &sdf,
        "MC.REG",
        &mut timing.del_clk_q,
        &mut timing.del_sr_q,
        &mut timing.setup_d_clk_pt_fclk,
        &mut timing.hold_d_clk_pt_fclk,
        &mut None,
        &mut None,
        &mut period_clk,
        &mut timing.width_sr,
    );

    extract_ff(
        &sdf,
        "IMC.REG",
        &mut timing.del_clk_q,
        &mut timing.del_sr_q,
        &mut timing.setup_d_clk_ibuf_fclk,
        &mut timing.hold_d_clk_ibuf_fclk,
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

    let emcid = insert_mc(&mut vm6, "EMC", 0);
    insert_mc_si(&mut vm6, emcid, NodeKind::McSiD1, &[node_e]);
    insert_mc_si(&mut vm6, emcid, NodeKind::McSiD2, &[]);
    insert_srff(&mut vm6, emcid);
    let node_me = insert_mc_out(&mut vm6, emcid, NodeKind::McGlb);

    let mcid = insert_mc(&mut vm6, "MC", 0x4040);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD1, &[node_d]);
    insert_mc_si(&mut vm6, mcid, NodeKind::McSiD2, &[]);
    let node_sc = insert_ct(&mut vm6, FbId::from_idx(0), PTermId::from_idx(4), &[node_c]);
    let node_sr = insert_ct(&mut vm6, FbId::from_idx(0), PTermId::from_idx(5), &[node_r]);
    let node_ss = insert_ct(&mut vm6, FbId::from_idx(0), PTermId::from_idx(6), &[node_s]);
    insert_srff(&mut vm6, mcid);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffC, node_sc);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffR, node_sr);
    insert_srff_inp(&mut vm6, mcid, InputNodeKind::SrffS, node_ss);
    insert_bufoe(&mut vm6, mcid, node_me);
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

    extract_and2(&sdf, "EMC.D1", &mut timing.del_imux_pt);
    extract_buf(&sdf, "EMC.Q", &mut timing.del_d_q_comb);

    extract_and2(&sdf, "MC.D1", &mut timing.del_imux_pt);
    extract_and2(&sdf, "FOOBAR1__ctinst/4", &mut timing.del_imux_ct);
    extract_and2(&sdf, "FOOBAR1__ctinst/5", &mut timing.del_imux_ct);
    extract_and2(&sdf, "FOOBAR1__ctinst/6", &mut timing.del_imux_ct);
    extract_buf(&sdf, "MC.BUFOE.OUT", &mut timing.del_mc_foe);

    extract_latch(
        &sdf,
        "MC.REG",
        &mut timing.del_d_q_latch,
        &mut timing.del_clk_q,
        &mut timing.del_sr_q,
        &mut timing.setup_d_clk_pt_pt,
        &mut timing.hold_d_clk_pt_pt,
        &mut timing.width_clk_pt,
        &mut timing.width_sr,
    );
}

fn test_iostd(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    timing: &mut Timing,
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

    let iostd = timing.iostd.entry(iostd.into()).or_default();

    if has_plain {
        let mut del_ibuf_plain = None;
        extract_buf(&sdf, "IP", &mut del_ibuf_plain);
        set_timing(
            &mut iostd.del_ibuf_plain,
            del_ibuf_plain.unwrap() - timing.del_ibuf_imux.unwrap(),
        );
    }
    let mut del_ibuf_schmitt = None;
    extract_buf(&sdf, "IS", &mut del_ibuf_schmitt);
    set_timing(
        &mut iostd.del_ibuf_schmitt,
        del_ibuf_schmitt.unwrap() - timing.del_ibuf_imux.unwrap(),
    );

    extract_buf(&sdf, "FMC_PAD_6", &mut iostd.del_obuf_fast);
    extract_buf(&sdf, "SMC_PAD_8", &mut iostd.del_obuf_slow);
}

fn test_dge(
    tc: &Toolchain,
    part: &Part,
    device: &Device,
    package: &Package,
    spd: &str,
    timing: &mut Timing,
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

    let mut zero = Some(0);
    extract_buf(&sdf, "D", &mut zero);
    extract_buf(&sdf, "G", &mut timing.del_ibuf_imux);

    extract_latch(
        &sdf,
        "D_PAD_tsimcreated_dg_",
        &mut timing.del_ibuf_imux,
        &mut timing.del_ibuf_dge,
        &mut None,
        &mut timing.setup_ibuf_dge,
        &mut timing.hold_ibuf_dge,
        &mut timing.width_ibuf_dge,
        &mut None,
    );
}
