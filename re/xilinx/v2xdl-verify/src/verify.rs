use crate::types::{Test, TgtConfigVal, TgtPinDir};
use prjcombine_re_xilinx_xdl::{parse_lut, Design, NetType};
use std::collections::{HashMap, HashSet};

fn recog_lut(
    cfg_expect: &mut HashMap<String, (String, TgtConfigVal)>,
    c: &[String],
    family: &str,
) -> Option<(String, String)> {
    let (l, sz) = match &c[0][..] {
        "F" => ("F", 4),
        "G" => ("G", 4),
        "A5LUT" => ("A", 5),
        "B5LUT" => ("B", 5),
        "C5LUT" => ("C", 5),
        "D5LUT" => ("D", 5),
        "A6LUT" => ("A", 6),
        "B6LUT" => ("B", 6),
        "C6LUT" => ("C", 6),
        "D6LUT" => ("D", 6),
        _ => return None,
    };
    match sz {
        4 => {
            let v = parse_lut(4, &c[3])?;
            let inum = match v {
                0xaaaa => 1,
                0xcccc => 2,
                0xf0f0 => 3,
                0xff00 => 4,
                _ => return None,
            };
            let x = if l == "F" { "X" } else { "Y" };
            cfg_expect.insert(c[0].clone(), (c[1].clone(), TgtConfigVal::Lut(4, v)));
            if family != "virtex4" {
                if l == "F" {
                    cfg_expect.insert(
                        "FXMUX".to_string(),
                        ("".to_string(), TgtConfigVal::Plain("F".to_string())),
                    );
                } else {
                    cfg_expect.insert(
                        "GYMUX".to_string(),
                        ("".to_string(), TgtConfigVal::Plain("G".to_string())),
                    );
                }
            }
            cfg_expect.insert(
                format!("{x}USED"),
                ("".to_string(), TgtConfigVal::Plain("0".to_string())),
            );
            Some((x.to_string(), format!("{il}{inum}", il = c[0])))
        }
        5 => {
            let v = parse_lut(5, &c[3])?;
            let inum = match v {
                0xaaaaaaaa => 1,
                0xcccccccc => 2,
                0xf0f0f0f0 => 3,
                0xff00ff00 => 4,
                0xffff0000 => 5,
                _ => return None,
            };
            cfg_expect.insert(c[0].clone(), (c[1].clone(), TgtConfigVal::Lut(5, v)));
            cfg_expect.insert(
                format!("{l}OUTMUX"),
                ("".to_string(), TgtConfigVal::Plain("O5".to_string())),
            );
            Some((format!("{l}MUX"), format!("{l}{inum}")))
        }
        6 => {
            let v = parse_lut(6, &c[3])?;
            let inum = match v {
                0xaaaaaaaaaaaaaaaa => 1,
                0xcccccccccccccccc => 2,
                0xf0f0f0f0f0f0f0f0 => 3,
                0xff00ff00ff00ff00 => 4,
                0xffff0000ffff0000 => 5,
                0xffffffff00000000 => 6,
                _ => return None,
            };
            cfg_expect.insert(c[0].clone(), (c[1].clone(), TgtConfigVal::Lut(6, v)));
            cfg_expect.insert(
                format!("{l}USED"),
                ("".to_string(), TgtConfigVal::Plain("0".to_string())),
            );
            Some((l.to_string(), format!("{l}{inum}")))
        }
        _ => unreachable!(),
    }
}

pub fn verify(test: &Test, design: &Design, family: &str) -> bool {
    let mut ok = true;
    let mut in_nets_pending: HashSet<_> = test.src_ins.iter().cloned().collect();
    let mut in2_nets_pending: HashSet<_> = test.src_ins.iter().cloned().collect();
    let mut out_nets_pending: HashSet<_> = test.src_outs.iter().cloned().collect();
    let mut wire_map: HashMap<(String, String), (String, TgtPinDir)> = HashMap::new();
    let mut wire_tie: HashMap<(String, String), bool> = HashMap::new();
    let mut dummy_out_wires = HashSet::new();
    let mut exp_bels = HashMap::new();
    for ti in test.tgt_insts.iter() {
        let mut bel = None;
        for c in ti.config.iter() {
            if !c.1.is_empty() && c.1 != "DUMMY" {
                bel = Some(c.1.to_string());
                break;
            }
        }
        exp_bels.insert(bel.unwrap(), ti);
    }
    for inst in design.instances.iter() {
        let mut cfg_expect = HashMap::new();
        for c in inst.cfg.iter() {
            if let Some(ti) = exp_bels.remove(&c[1]) {
                if !ti.kind.contains(&inst.kind) {
                    println!("unexpected inst kind {} {:?}", inst.kind, ti.kind);
                    ok = false;
                }
                for (k, b, v, ki) in ti.config.iter() {
                    if let Some(exp_ki) = ki {
                        if exp_ki != &inst.kind {
                            continue;
                        }
                    }
                    cfg_expect.insert(k.clone(), (b.clone(), v.clone()));
                }
                for (pin, net, dir) in ti.pins.iter() {
                    wire_map.insert((inst.name.clone(), pin.clone()), (net.clone(), *dir));
                }
                for (pin, val) in ti.pin_ties.iter() {
                    wire_tie.insert((inst.name.clone(), pin.clone()), *val);
                }
                for pin in ti.pin_dumout.iter() {
                    dummy_out_wires.insert((inst.name.clone(), pin.clone()));
                }
            } else if c[1].starts_with("_ibuf2_") {
                let name = &c[1][7..];
                let lut = recog_lut(&mut cfg_expect, c, family);
                if lut.is_none() {
                    continue;
                }
                let (opin, ipin) = lut.unwrap();
                if !in2_nets_pending.remove(name) {
                    println!("funny ibuf2 found: {c:?}");
                    ok = false;
                    continue;
                }
                wire_map.insert(
                    (inst.name.clone(), opin),
                    (format!("_in_{name}"), TgtPinDir::Output),
                );
                wire_tie.insert((inst.name.clone(), ipin), false);
            } else if c[1].starts_with("_ibuf_") {
                let name = &c[1][6..];
                let lut = recog_lut(&mut cfg_expect, c, family);
                if lut.is_none() {
                    continue;
                }
                let (opin, ipin) = lut.unwrap();
                if !in_nets_pending.remove(name) {
                    println!("funny ibuf found: {c:?}");
                    ok = false;
                    continue;
                }
                wire_map.insert(
                    (inst.name.clone(), opin),
                    (name.to_string(), TgtPinDir::Output),
                );
                wire_map.insert(
                    (inst.name.clone(), ipin),
                    (format!("_in_{name}"), TgtPinDir::Input),
                );
            } else if c[1].starts_with("_obuf_") {
                let name = &c[1][6..];
                let lut = recog_lut(&mut cfg_expect, c, family);
                if lut.is_none() {
                    continue;
                }
                let (opin, ipin) = lut.unwrap();
                if !out_nets_pending.remove(name) {
                    println!("funny obuf found: {c:?}");
                    ok = false;
                    continue;
                }
                wire_map.insert(
                    (inst.name.clone(), ipin),
                    (name.to_string(), TgtPinDir::Input),
                );
                dummy_out_wires.insert((inst.name.clone(), opin));
            } else if c[1] == "XIL_ML_PMV" {
                // Virtex 4 special.
                cfg_expect.insert(
                    "PMV".to_string(),
                    (c[1].clone(), TgtConfigVal::Plain(String::new())),
                );
                for (p, v) in [
                    ("A0", false),
                    ("A1", false),
                    ("A2", true),
                    ("A3", false),
                    ("A4", false),
                    ("A5", false),
                    ("EN", false),
                ] {
                    wire_tie.insert((inst.name.clone(), p.to_string()), v);
                }
                wire_map.insert(
                    (inst.name.clone(), "ODIV4".to_string()),
                    ("PMV_ODIV4".to_string(), TgtPinDir::Output),
                );
            } else if c[1].starts_with("XIL_ML_UNUSED_DCM_") {
                // Virtex 4 special.
                cfg_expect.insert(
                    "DCM_ADV".to_string(),
                    (c[1].clone(), TgtConfigVal::Plain(String::new())),
                );
                for (p, v) in [
                    ("BGM_CONFIG_REF_SEL", "CLKIN"),
                    ("BGM_DIVIDE", "16"),
                    ("BGM_LDLY", "5"),
                    ("BGM_MODE", "BG_SNAPSHOT"),
                    ("BGM_MULTIPLY", "16"),
                    ("BGM_SAMPLE_LEN", "2"),
                    ("BGM_SDLY", "3"),
                    ("BGM_VADJ", "5"),
                    ("BGM_VLDLY", "7"),
                    ("BGM_VSDLY", "0"),
                    ("CLKDV_DIVIDE", "2.0"),
                    ("CLKFX_DIVIDE", "1"),
                    ("CLKFX_MULTIPLY", "4"),
                    ("CLKIN_DIVIDE_BY_2", "TRUE"),
                    ("CLKOUT_PHASE_SHIFT", "FIXED"),
                    ("CLK_FEEDBACK", "1X"),
                    ("CTLMODEINV", "CTLMODE"),
                    ("DCM_CLKDV_CLKFX_ALIGNMENT", "TRUE"),
                    ("DCM_EXT_FB_EN", "FALSE"),
                    ("DCM_LOCK_HIGH", "FALSE"),
                    ("DCM_PERFORMANCE_MODE", "MAX_SPEED"),
                    ("DCM_UNUSED_TAPS_POWERDOWN", "FALSE"),
                    ("DCM_VREF_SOURCE", "VBG_DLL"),
                    ("DCM_VREG_ENABLE", "FALSE"),
                    ("DESKEW_ADJUST", "20"),
                    ("DFS_AVE_FREQ_ADJ_INTERVAL", "3"),
                    ("DFS_AVE_FREQ_GAIN", "2.0"),
                    ("DFS_AVE_FREQ_SAMPLE_INTERVAL", "2"),
                    ("DFS_COARSE_SEL", "LEGACY"),
                    ("DFS_EARLY_LOCK", "FALSE"),
                    ("DFS_EN_RELRST", "TRUE"),
                    ("DFS_EXTEND_FLUSH_TIME", "FALSE"),
                    ("DFS_EXTEND_HALT_TIME", "FALSE"),
                    ("DFS_EXTEND_RUN_TIME", "FALSE"),
                    ("DFS_FINE_SEL", "LEGACY"),
                    ("DFS_FREQUENCY_MODE", "LOW"),
                    ("DFS_NON_STOP", "FALSE"),
                    ("DFS_OSCILLATOR_MODE", "PHASE_FREQ_LOCK"),
                    ("DFS_SKIP_FINE", "FALSE"),
                    ("DFS_TP_SEL", "LEVEL"),
                    ("DFS_TRACKMODE", "1"),
                    ("DLL_CONTROL_CLOCK_SPEED", "HALF"),
                    ("DLL_CTL_SEL_CLKIN_DIV2", "FALSE"),
                    ("DLL_DESKEW_LOCK_BY1", "FALSE"),
                    ("DLL_FREQUENCY_MODE", "LOW"),
                    ("DLL_PD_DLY_SEL", "0"),
                    ("DLL_PERIOD_LOCK_BY1", "FALSE"),
                    ("DLL_PHASE_DETECTOR_AUTO_RESET", "TRUE"),
                    ("DLL_PHASE_DETECTOR_MODE", "ENHANCED"),
                    ("DLL_PHASE_SHIFT_CALIBRATION", "AUTO_DPS"),
                    ("DLL_PHASE_SHIFT_LOCK_BY1", "FALSE"),
                    ("DUTY_CYCLE_CORRECTION", "TRUE"),
                    ("PMCD_SYNC", "FALSE"),
                    ("PSENINV", "PSEN_B"),
                    ("STARTUP_WAIT", "FALSE"),
                    ("CLKIN_PERIOD", "10.0"),
                    ("DCM_PULSE_WIDTH_CORRECTION_HIGH", "11111"),
                    ("DCM_PULSE_WIDTH_CORRECTION_LOW", "11111"),
                    ("DCM_VBG_PD", "00"),
                    ("DCM_VBG_SEL", "0000"),
                    ("DCM_VREG_PHASE_MARGIN", "010"),
                    ("DFS_COIN_WINDOW", "00"),
                    ("DFS_HARDSYNC", "00"),
                    ("DFS_SPARE", "0000000000000000"),
                    ("DLL_DEAD_TIME", "10"),
                    ("DLL_DESKEW_MAXTAP", "210"),
                    ("DLL_DESKEW_MINTAP", "42"),
                    ("DLL_LIVE_TIME", "5"),
                    ("DLL_PHASE_SHIFT_HFC", "206"),
                    ("DLL_PHASE_SHIFT_LFC", "413"),
                    ("DLL_SETTLE_TIME", "10"),
                    ("DLL_SPARE", "0000000000000000"),
                    ("DLL_TEST_MUX_SEL", "00"),
                    ("FACTORY_JF", "F0F0"),
                    ("PHASE_SHIFT", "0"),
                ] {
                    cfg_expect.insert(
                        p.to_string(),
                        ("".to_string(), TgtConfigVal::Plain(v.to_string())),
                    );
                }
                for (p, v) in [("PSEN", true), ("CTLMODE", true)] {
                    wire_tie.insert((inst.name.clone(), p.to_string()), v);
                }
                wire_map.insert(
                    (inst.name.clone(), "CLKFB".to_string()),
                    (format!("{}_CLKFB", c[1]), TgtPinDir::Input),
                );
                wire_map.insert(
                    (inst.name.clone(), "CLK0".to_string()),
                    (format!("{}_CLKFB", c[1]), TgtPinDir::Output),
                );
                wire_map.insert(
                    (inst.name.clone(), "CLKIN".to_string()),
                    ("PMV_ODIV4".to_string(), TgtPinDir::Input),
                );
            } else if c[1] == "STARTUP_V6_PWRUP_GTXE1_ML_INSERTED" {
                // Virtex 6 special.
                cfg_expect.insert(
                    "STARTUP".to_string(),
                    (c[1].clone(), TgtConfigVal::Plain(String::new())),
                );
                cfg_expect.insert(
                    "PROG_USR".to_string(),
                    ("".to_string(), TgtConfigVal::Plain("FALSE".to_string())),
                );
                for (p, v) in [
                    ("CLK", false),
                    ("GSR", false),
                    ("GTS", false),
                    ("PACK", false),
                    ("USRCCLKO", false),
                    ("KEYCLEARB", true),
                    ("USRCCLKTS", true),
                    ("USRDONEO", true),
                    ("USRDONETS", true),
                ] {
                    wire_tie.insert((inst.name.clone(), p.to_string()), v);
                }
                wire_map.insert(
                    (inst.name.clone(), "CFGMCLK".to_string()),
                    ("STARTUP_CFGMCLK".to_string(), TgtPinDir::Output),
                );
            } else if c[1].starts_with("GTXE1_ML_REPLICATED_") {
                cfg_expect.insert(
                    "GTXE1".to_string(),
                    (c[1].clone(), TgtConfigVal::Plain(String::new())),
                );
                for (p, v) in [
                    ("AC_CAP_DIS", "TRUE"),
                    ("ALIGN_COMMA_WORD", "1"),
                    ("CHAN_BOND_1_MAX_SKEW", "7"),
                    ("CHAN_BOND_2_MAX_SKEW", "1"),
                    ("CHAN_BOND_KEEP_ALIGN", "FALSE"),
                    ("CHAN_BOND_SEQ_2_USE", "FALSE"),
                    ("CHAN_BOND_SEQ_LEN", "1"),
                    ("CLK_CORRECT_USE", "TRUE"),
                    ("CLK_COR_ADJ_LEN", "1"),
                    ("CLK_COR_DET_LEN", "1"),
                    ("CLK_COR_INSERT_IDLE_FLAG", "FALSE"),
                    ("CLK_COR_KEEP_IDLE", "FALSE"),
                    ("CLK_COR_MAX_LAT", "20"),
                    ("CLK_COR_MIN_LAT", "18"),
                    ("CLK_COR_PRECEDENCE", "TRUE"),
                    ("CLK_COR_REPEAT_WAIT", "0"),
                    ("CLK_COR_SEQ_2_USE", "FALSE"),
                    ("COMMA_DOUBLE", "FALSE"),
                    ("DCLKINV", "DCLK_B"),
                    ("DEC_MCOMMA_DETECT", "TRUE"),
                    ("DEC_PCOMMA_DETECT", "TRUE"),
                    ("DEC_VALID_COMMA_ONLY", "TRUE"),
                    ("DFE_DRP_EN", "FALSE"),
                    ("GEN_RXUSRCLK", "TRUE"),
                    ("GEN_TXUSRCLK", "TRUE"),
                    ("GREFCLKRXINV", "#OFF"),
                    ("GREFCLKTXINV", "#OFF"),
                    ("GTX_CFG_PWRUP", "FALSE"),
                    ("LOOPBACK_DRP_EN", "FALSE"),
                    ("MASTER_DRP_EN", "FALSE"),
                    ("MCOMMA_DETECT", "TRUE"),
                    ("PCI_EXPRESS_MODE", "FALSE"),
                    ("PCOMMA_DETECT", "TRUE"),
                    ("PDELIDLE_DRP_EN", "FALSE"),
                    ("PHASEALIGN_DRP_EN", "FALSE"),
                    ("PLL_DRP_EN", "FALSE"),
                    ("PMA_CAS_CLK_EN", "FALSE"),
                    ("POLARITY_DRP_EN", "FALSE"),
                    ("PRBS_DRP_EN", "FALSE"),
                    ("RCV_TERM_GND", "FALSE"),
                    ("RCV_TERM_VTTRX", "FALSE"),
                    ("RESET_DRP_EN", "FALSE"),
                    ("RXBUF_OVFL_THRESH", "61"),
                    ("RXBUF_OVRD_THRESH", "FALSE"),
                    ("RXBUF_UDFL_THRESH", "4"),
                    ("RXGEARBOX_USE", "FALSE"),
                    ("RXPLL_DIVSEL45_FB", "5"),
                    ("RXPLL_DIVSEL_FB", "2"),
                    ("RXPLL_DIVSEL_OUT", "1"),
                    ("RXPLL_DIVSEL_REF", "1"),
                    ("RXPLL_STARTUP_EN", "TRUE"),
                    ("RXRECCLK_CTRL", "CLKTESTSIG1"),
                    ("RXUSRCLK2INV", "RXUSRCLK2_B"),
                    ("RXUSRCLKINV", "RXUSRCLK_B"),
                    ("RX_BUFFER_USE", "TRUE"),
                    ("RX_CDR_FORCE_ROTATE", "FALSE"),
                    ("RX_CLK25_DIVIDER", "6"),
                    ("RX_DATA_WIDTH", "20"),
                    ("RX_DECODE_SEQ_MATCH", "TRUE"),
                    ("RX_EN_IDLE_HOLD_CDR", "FALSE"),
                    ("RX_EN_IDLE_HOLD_DFE", "TRUE"),
                    ("RX_EN_IDLE_RESET_BUF", "TRUE"),
                    ("RX_EN_IDLE_RESET_FR", "TRUE"),
                    ("RX_EN_IDLE_RESET_PH", "TRUE"),
                    ("RX_EN_MODE_RESET_BUF", "TRUE"),
                    ("RX_EN_RATE_RESET_BUF", "TRUE"),
                    ("RX_EN_REALIGN_RESET_BUF", "FALSE"),
                    ("RX_EN_REALIGN_RESET_BUF2", "FALSE"),
                    ("RX_FIFO_ADDR_MODE", "FULL"),
                    ("RX_LOSS_OF_SYNC_FSM", "FALSE"),
                    ("RX_LOS_INVALID_INCR", "1"),
                    ("RX_LOS_THRESHOLD", "4"),
                    ("RX_OVERSAMPLE_MODE", "FALSE"),
                    ("RX_SLIDE_AUTO_WAIT", "5"),
                    ("RX_SLIDE_MODE", "OFF"),
                    ("RX_XCLK_SEL", "RXREC"),
                    ("SAS_MAX_COMSAS", "52"),
                    ("SAS_MIN_COMSAS", "40"),
                    ("SATA_MAX_BURST", "7"),
                    ("SATA_MAX_INIT", "22"),
                    ("SATA_MAX_WAKE", "7"),
                    ("SATA_MIN_BURST", "4"),
                    ("SATA_MIN_INIT", "12"),
                    ("SATA_MIN_WAKE", "4"),
                    ("SCANCLKINV", "#OFF"),
                    ("SHOW_REALIGN_COMMA", "TRUE"),
                    ("TERMINATION_OVRD", "FALSE"),
                    ("TSTCLK0INV", "TSTCLK0_B"),
                    ("TSTCLK1INV", "TSTCLK1_B"),
                    ("TXDRIVE_DRP_EN", "FALSE"),
                    ("TXDRIVE_LOOPBACK_HIZ", "FALSE"),
                    ("TXDRIVE_LOOPBACK_PD", "FALSE"),
                    ("TXGEARBOX_USE", "FALSE"),
                    ("TXOUTCLKPCS_SEL", "0"),
                    ("TXOUTCLK_CTRL", "CLKTESTSIG0"),
                    ("TXPLL_DIVSEL45_FB", "5"),
                    ("TXPLL_DIVSEL_FB", "2"),
                    ("TXPLL_DIVSEL_OUT", "1"),
                    ("TXPLL_DIVSEL_REF", "1"),
                    ("TXPLL_STARTUP_EN", "TRUE"),
                    ("TXUSRCLK2INV", "TXUSRCLK2_B"),
                    ("TXUSRCLKINV", "TXUSRCLK_B"),
                    ("TX_BUFFER_USE", "TRUE"),
                    ("TX_CLK25_DIVIDER", "6"),
                    ("TX_CLK_SOURCE", "RXPLL"),
                    ("TX_DATA_WIDTH", "20"),
                    ("TX_DRIVE_MODE", "DIRECT"),
                    ("TX_EN_RATE_RESET_BUF", "TRUE"),
                    ("TX_OVERSAMPLE_MODE", "FALSE"),
                    ("TX_XCLK_SEL", "TXUSR"),
                    ("A_DFECLKDLYADJ", "000000"),
                    ("A_DFEDLYOVRD", "0"),
                    ("A_DFETAP1", "00000"),
                    ("A_DFETAP2", "00000"),
                    ("A_DFETAP3", "0000"),
                    ("A_DFETAP4", "0000"),
                    ("A_DFETAPOVRD", "0"),
                    ("A_GTXRXRESET", "0"),
                    ("A_GTXTXRESET", "0"),
                    ("A_LOOPBACK", "000"),
                    ("A_PLLCLKRXRESET", "0"),
                    ("A_PLLCLKTXRESET", "0"),
                    ("A_PLLRXRESET", "0"),
                    ("A_PLLTXRESET", "0"),
                    ("A_PRBSCNTRESET", "0"),
                    ("A_RXBUFRESET", "0"),
                    ("A_RXCDRFREQRESET", "0"),
                    ("A_RXCDRHOLD", "0"),
                    ("A_RXCDRPHASERESET", "0"),
                    ("A_RXCDRRESET", "0"),
                    ("A_RXDFERESET", "0"),
                    ("A_RXENPMAPHASEALIGN", "0"),
                    ("A_RXENPRBSTST", "000"),
                    ("A_RXENSAMPLEALIGN", "0"),
                    ("A_RXEQMIX", "0000000000"),
                    ("A_RXPLLLKDETEN", "0"),
                    ("A_RXPLLPOWERDOWN", "0"),
                    ("A_RXPMASETPHASE", "0"),
                    ("A_RXPOLARITY", "0"),
                    ("A_RXPOWERDOWN", "00"),
                    ("A_RXRESET", "0"),
                    ("A_TXBUFDIFFCTRL", "010"),
                    ("A_TXDEEMPH", "0"),
                    ("A_TXDIFFCTRL", "0000"),
                    ("A_TXELECIDLE", "0"),
                    ("A_TXENPMAPHASEALIGN", "0"),
                    ("A_TXENPRBSTST", "000"),
                    ("A_TXMARGIN", "010"),
                    ("A_TXPLLLKDETEN", "0"),
                    ("A_TXPLLPOWERDOWN", "0"),
                    ("A_TXPMASETPHASE", "0"),
                    ("A_TXPOLARITY", "0"),
                    ("A_TXPOSTEMPHASIS", "00000"),
                    ("A_TXPOWERDOWN", "00"),
                    ("A_TXPRBSFORCEERR", "0"),
                    ("A_TXPREEMPHASIS", "0000"),
                    ("A_TXRESET", "0"),
                    ("A_TXSWING", "0"),
                    ("BGTEST_CFG", "00"),
                    ("BIAS_CFG", "00000"),
                    ("CDR_PH_ADJ_TIME", "10100"),
                    ("CHAN_BOND_SEQ_1_1", "0101111100"),
                    ("CHAN_BOND_SEQ_1_2", "0001001010"),
                    ("CHAN_BOND_SEQ_1_3", "0001001010"),
                    ("CHAN_BOND_SEQ_1_4", "0110111100"),
                    ("CHAN_BOND_SEQ_1_ENABLE", "1111"),
                    ("CHAN_BOND_SEQ_2_1", "0100111100"),
                    ("CHAN_BOND_SEQ_2_2", "0100111100"),
                    ("CHAN_BOND_SEQ_2_3", "0110111100"),
                    ("CHAN_BOND_SEQ_2_4", "0100111100"),
                    ("CHAN_BOND_SEQ_2_CFG", "00000"),
                    ("CHAN_BOND_SEQ_2_ENABLE", "1111"),
                    ("CLK_COR_SEQ_1_1", "0100011100"),
                    ("CLK_COR_SEQ_1_2", "0000000000"),
                    ("CLK_COR_SEQ_1_3", "0000000000"),
                    ("CLK_COR_SEQ_1_4", "0000000000"),
                    ("CLK_COR_SEQ_1_ENABLE", "1111"),
                    ("CLK_COR_SEQ_2_1", "0000000000"),
                    ("CLK_COR_SEQ_2_2", "0000000000"),
                    ("CLK_COR_SEQ_2_3", "0000000000"),
                    ("CLK_COR_SEQ_2_4", "0000000000"),
                    ("CLK_COR_SEQ_2_ENABLE", "1111"),
                    ("CM_TRIM", "01"),
                    ("COMMA_10B_ENABLE", "1111111111"),
                    ("COM_BURST_VAL", "1111"),
                    ("DFE_CAL_TIME", "01100"),
                    ("DFE_CFG", "00011011"),
                    ("GEARBOX_ENDEC", "000"),
                    ("MCOMMA_10B_VALUE", "1010000011"),
                    ("OOBDETECT_THRESHOLD", "011"),
                    ("PCOMMA_10B_VALUE", "0101111100"),
                    ("PMA_CDR_SCAN", "640404c"),
                    ("PMA_CFG", "0040000040000000003"),
                    ("PMA_RXSYNC_CFG", "00"),
                    ("PMA_RX_CFG", "05ce048"),
                    ("PMA_TX_CFG", "00082"),
                    ("POWER_SAVE", "0000110100"),
                    ("RXPLL_COM_CFG", "21680a"),
                    ("RXPLL_CP_CFG", "00"),
                    ("RXPLL_LKDET_CFG", "111"),
                    ("RXPRBSERR_LOOPBACK", "0"),
                    ("RXRECCLK_DLY", "0000000000"),
                    ("RXUSRCLK_DLY", "0000"),
                    ("RX_DLYALIGN_CTRINC", "0100"),
                    ("RX_DLYALIGN_EDGESET", "00110"),
                    ("RX_DLYALIGN_LPFINC", "0111"),
                    ("RX_DLYALIGN_MONSEL", "000"),
                    ("RX_DLYALIGN_OVRDSETTING", "00000000"),
                    ("RX_EYE_OFFSET", "4c"),
                    ("RX_EYE_SCANMODE", "00"),
                    ("RX_IDLE_HI_CNT", "1000"),
                    ("RX_IDLE_LO_CNT", "0000"),
                    ("SATA_BURST_VAL", "100"),
                    ("SATA_IDLE_VAL", "100"),
                    ("TERMINATION_CTRL", "10100"),
                    ("TRANS_TIME_FROM_P2", "03c"),
                    ("TRANS_TIME_NON_P2", "19"),
                    ("TRANS_TIME_RATE", "0e"),
                    ("TRANS_TIME_TO_P2", "064"),
                    ("TST_ATTR", "00000000"),
                    ("TXOUTCLK_DLY", "0000000000"),
                    ("TXPLL_COM_CFG", "21680a"),
                    ("TXPLL_CP_CFG", "00"),
                    ("TXPLL_LKDET_CFG", "111"),
                    ("TXPLL_SATA", "00"),
                    ("TX_BYTECLK_CFG", "00"),
                    ("TX_DEEMPH_0", "11010"),
                    ("TX_DEEMPH_1", "10000"),
                    ("TX_DETECT_RX_CFG", "1832"),
                    ("TX_DLYALIGN_CTRINC", "0100"),
                    ("TX_DLYALIGN_LPFINC", "0110"),
                    ("TX_DLYALIGN_MONSEL", "000"),
                    ("TX_DLYALIGN_OVRDSETTING", "10000000"),
                    ("TX_IDLE_ASSERT_DELAY", "100"),
                    ("TX_IDLE_DEASSERT_DELAY", "010"),
                    ("TX_MARGIN_FULL_0", "1001110"),
                    ("TX_MARGIN_FULL_1", "1001001"),
                    ("TX_MARGIN_FULL_2", "1000101"),
                    ("TX_MARGIN_FULL_3", "1000010"),
                    ("TX_MARGIN_FULL_4", "1000000"),
                    ("TX_MARGIN_LOW_0", "1000110"),
                    ("TX_MARGIN_LOW_1", "1000100"),
                    ("TX_MARGIN_LOW_2", "1000010"),
                    ("TX_MARGIN_LOW_3", "1000000"),
                    ("TX_MARGIN_LOW_4", "1000000"),
                    ("TX_PMADATA_OPT", "0"),
                    ("TX_TDCC_CFG", "11"),
                    ("TX_USRCLK_CFG", "00"),
                    ("USR_CODE_ERR_CLR", "0"),
                ] {
                    cfg_expect.insert(
                        p.to_string(),
                        ("".to_string(), TgtConfigVal::Plain(v.to_string())),
                    );
                }
                for (p, v) in [
                    ("DADDR0", false),
                    ("DADDR1", false),
                    ("DADDR2", false),
                    ("DADDR3", false),
                    ("DADDR4", false),
                    ("DADDR5", false),
                    ("DADDR6", false),
                    ("DADDR7", false),
                    ("DEN", false),
                    ("DFECLKDLYADJ0", false),
                    ("DFECLKDLYADJ1", false),
                    ("DFECLKDLYADJ2", false),
                    ("DFECLKDLYADJ3", false),
                    ("DFECLKDLYADJ4", false),
                    ("DFECLKDLYADJ5", false),
                    ("DFETAP10", false),
                    ("DFETAP11", false),
                    ("DFETAP12", false),
                    ("DFETAP13", false),
                    ("DFETAP14", false),
                    ("DFETAP20", false),
                    ("DFETAP21", false),
                    ("DFETAP22", false),
                    ("DFETAP23", false),
                    ("DFETAP24", false),
                    ("DFETAP30", false),
                    ("DFETAP31", false),
                    ("DFETAP32", false),
                    ("DFETAP33", false),
                    ("DFETAP40", false),
                    ("DFETAP41", false),
                    ("DFETAP42", false),
                    ("DFETAP43", false),
                    ("DI0", false),
                    ("DI1", false),
                    ("DI10", false),
                    ("DI11", false),
                    ("DI12", false),
                    ("DI13", false),
                    ("DI14", false),
                    ("DI15", false),
                    ("DI2", false),
                    ("DI3", false),
                    ("DI4", false),
                    ("DI5", false),
                    ("DI6", false),
                    ("DI7", false),
                    ("DI8", false),
                    ("DI9", false),
                    ("DWE", false),
                    ("GATERXELECIDLE", false),
                    ("GTXRXRESET", false),
                    ("GTXTEST0", false),
                    ("GTXTEST1", false),
                    ("GTXTEST10", false),
                    ("GTXTEST11", false),
                    ("GTXTEST2", false),
                    ("GTXTEST3", false),
                    ("GTXTEST4", false),
                    ("GTXTEST5", false),
                    ("GTXTEST6", false),
                    ("GTXTEST7", false),
                    ("GTXTEST8", false),
                    ("GTXTEST9", false),
                    ("GTXTXRESET", false),
                    ("IGNORESIGDET", false),
                    ("LOOPBACK0", false),
                    ("LOOPBACK1", false),
                    ("LOOPBACK2", false),
                    ("PLLRXRESET", false),
                    ("PLLTXRESET", false),
                    ("PRBSCNTRESET", false),
                    ("RXBUFRESET", false),
                    ("RXCDRRESET", false),
                    ("RXCHBONDI0", false),
                    ("RXCHBONDI1", false),
                    ("RXCHBONDI2", false),
                    ("RXCHBONDI3", false),
                    ("RXCHBONDLEVEL0", false),
                    ("RXCHBONDLEVEL1", false),
                    ("RXCHBONDLEVEL2", false),
                    ("RXCHBONDMASTER", false),
                    ("RXCHBONDSLAVE", false),
                    ("RXCOMMADETUSE", false),
                    ("RXDEC8B10BUSE", false),
                    ("RXDLYALIGNOVERRIDE", false),
                    ("RXDLYALIGNUPDSW", false),
                    ("RXENCHANSYNC", false),
                    ("RXENMCOMMAALIGN", false),
                    ("RXENPCOMMAALIGN", false),
                    ("RXENPMAPHASEALIGN", false),
                    ("RXENPRBSTST0", false),
                    ("RXENPRBSTST1", false),
                    ("RXENPRBSTST2", false),
                    ("RXENSAMPLEALIGN", false),
                    ("RXEQMIX2", false),
                    ("RXEQMIX3", false),
                    ("RXEQMIX4", false),
                    ("RXEQMIX5", false),
                    ("RXEQMIX6", false),
                    ("RXEQMIX9", false),
                    ("RXGEARBOXSLIP", false),
                    ("RXPLLLKDETEN", false),
                    ("RXPLLPOWERDOWN", false),
                    ("RXPLLREFSELDY0", false),
                    ("RXPLLREFSELDY1", false),
                    ("RXPLLREFSELDY2", false),
                    ("RXPMASETPHASE", false),
                    ("RXPOLARITY", false),
                    ("RXPOWERDOWN0", false),
                    ("RXPOWERDOWN1", false),
                    ("RXRATE0", false),
                    ("RXRATE1", false),
                    ("RXRESET", false),
                    ("RXSLIDE", false),
                    ("TSTIN0", false),
                    ("TSTIN1", false),
                    ("TSTIN10", false),
                    ("TSTIN11", false),
                    ("TSTIN12", false),
                    ("TSTIN13", false),
                    ("TSTIN14", false),
                    ("TSTIN15", false),
                    ("TSTIN17", false),
                    ("TSTIN18", false),
                    ("TSTIN19", false),
                    ("TSTIN2", false),
                    ("TSTIN3", false),
                    ("TSTIN4", false),
                    ("TSTIN5", false),
                    ("TSTIN6", false),
                    ("TSTIN7", false),
                    ("TSTIN8", false),
                    ("TSTIN9", false),
                    ("TXBUFDIFFCTRL0", false),
                    ("TXBUFDIFFCTRL1", false),
                    ("TXBUFDIFFCTRL2", false),
                    ("TXBYPASS8B10B0", false),
                    ("TXBYPASS8B10B1", false),
                    ("TXBYPASS8B10B2", false),
                    ("TXBYPASS8B10B3", false),
                    ("TXCHARDISPMODE0", false),
                    ("TXCHARDISPMODE1", false),
                    ("TXCHARDISPMODE2", false),
                    ("TXCHARDISPMODE3", false),
                    ("TXCHARDISPVAL0", false),
                    ("TXCHARDISPVAL1", false),
                    ("TXCHARDISPVAL2", false),
                    ("TXCHARDISPVAL3", false),
                    ("TXCHARISK0", false),
                    ("TXCHARISK1", false),
                    ("TXCHARISK2", false),
                    ("TXCHARISK3", false),
                    ("TXCOMINIT", false),
                    ("TXCOMSAS", false),
                    ("TXCOMWAKE", false),
                    ("TXDATA0", false),
                    ("TXDATA1", false),
                    ("TXDATA10", false),
                    ("TXDATA11", false),
                    ("TXDATA12", false),
                    ("TXDATA13", false),
                    ("TXDATA14", false),
                    ("TXDATA15", false),
                    ("TXDATA16", false),
                    ("TXDATA17", false),
                    ("TXDATA18", false),
                    ("TXDATA19", false),
                    ("TXDATA2", false),
                    ("TXDATA20", false),
                    ("TXDATA21", false),
                    ("TXDATA22", false),
                    ("TXDATA23", false),
                    ("TXDATA24", false),
                    ("TXDATA25", false),
                    ("TXDATA26", false),
                    ("TXDATA27", false),
                    ("TXDATA28", false),
                    ("TXDATA29", false),
                    ("TXDATA3", false),
                    ("TXDATA30", false),
                    ("TXDATA31", false),
                    ("TXDATA4", false),
                    ("TXDATA5", false),
                    ("TXDATA6", false),
                    ("TXDATA7", false),
                    ("TXDATA8", false),
                    ("TXDATA9", false),
                    ("TXDEEMPH", false),
                    ("TXDETECTRX", false),
                    ("TXDIFFCTRL0", false),
                    ("TXDIFFCTRL1", false),
                    ("TXDIFFCTRL2", false),
                    ("TXDIFFCTRL3", false),
                    ("TXDLYALIGNOVERRIDE", false),
                    ("TXELECIDLE", false),
                    ("TXENC8B10BUSE", false),
                    ("TXENPMAPHASEALIGN", false),
                    ("TXENPRBSTST0", false),
                    ("TXENPRBSTST1", false),
                    ("TXENPRBSTST2", false),
                    ("TXHEADER0", false),
                    ("TXHEADER1", false),
                    ("TXHEADER2", false),
                    ("TXINHIBIT", false),
                    ("TXMARGIN0", false),
                    ("TXMARGIN1", false),
                    ("TXMARGIN2", false),
                    ("TXPDOWNASYNCH", false),
                    ("TXPLLLKDETEN", false),
                    ("TXPLLREFSELDY0", false),
                    ("TXPLLREFSELDY1", false),
                    ("TXPLLREFSELDY2", false),
                    ("TXPMASETPHASE", false),
                    ("TXPOLARITY", false),
                    ("TXPOSTEMPHASIS0", false),
                    ("TXPOSTEMPHASIS1", false),
                    ("TXPOSTEMPHASIS2", false),
                    ("TXPOSTEMPHASIS3", false),
                    ("TXPOSTEMPHASIS4", false),
                    ("TXPRBSFORCEERR", false),
                    ("TXPREEMPHASIS0", false),
                    ("TXPREEMPHASIS1", false),
                    ("TXPREEMPHASIS2", false),
                    ("TXPREEMPHASIS3", false),
                    ("TXRATE0", false),
                    ("TXRATE1", false),
                    ("TXRESET", false),
                    ("TXSEQUENCE0", false),
                    ("TXSEQUENCE1", false),
                    ("TXSEQUENCE2", false),
                    ("TXSEQUENCE3", false),
                    ("TXSEQUENCE4", false),
                    ("TXSEQUENCE5", false),
                    ("TXSEQUENCE6", false),
                    ("TXSTARTSEQ", false),
                    ("TXSWING", false),
                    ("USRCODEERR", false),
                    ("DCLK", true),
                    ("DFEDLYOVRD", true),
                    ("DFETAPOVRD", true),
                    ("GTXTEST12", true),
                    ("RXDLYALIGNDISABLE", true),
                    ("RXDLYALIGNMONENB", true),
                    ("RXDLYALIGNRESET", true),
                    ("RXDLYALIGNSWPPRECURB", true),
                    ("RXEQMIX0", true),
                    ("RXEQMIX1", true),
                    ("RXEQMIX7", true),
                    ("RXEQMIX8", true),
                    ("RXUSRCLK", true),
                    ("RXUSRCLK2", true),
                    ("TSTCLK0", true),
                    ("TSTCLK1", true),
                    ("TSTIN16", true),
                    ("TXDLYALIGNDISABLE", true),
                    ("TXDLYALIGNMONENB", true),
                    ("TXDLYALIGNRESET", true),
                    ("TXDLYALIGNUPDSW", true),
                    ("TXPLLPOWERDOWN", true),
                    ("TXPOWERDOWN0", true),
                    ("TXPOWERDOWN1", true),
                    ("TXUSRCLK", true),
                    ("TXUSRCLK2", true),
                ] {
                    wire_tie.insert((inst.name.clone(), p.to_string()), v);
                }
                for p in ["CLKTESTSIG0", "CLKTESTSIG1"] {
                    wire_map.insert(
                        (inst.name.clone(), p.to_string()),
                        ("STARTUP_CFGMCLK".to_string(), TgtPinDir::Input),
                    );
                }
            }
        }
        for c in inst.cfg.iter() {
            if let Some((bel, val)) = cfg_expect.remove(&c[0]) {
                if c[1] != bel && bel != "DUMMY" {
                    println!("mismatched bel {ina} {c:?} {bel}", ina = inst.name);
                    ok = false;
                }
                match val {
                    TgtConfigVal::Plain(v) => {
                        if c[2] != v || c.len() != 3 {
                            println!("mismatched val {ina} {c:?} {v}", ina = inst.name);
                            ok = false;
                        }
                    }
                    TgtConfigVal::Lut(n, v) => {
                        if c[2] != "#LUT" || c.len() != 4 || parse_lut(n, &c[3]) != Some(v) {
                            println!("mismatched val {c:?} {n} LUT {v:x}");
                            ok = false;
                            continue;
                        }
                    }
                    TgtConfigVal::Rom(n, v) => {
                        if !matches!(&c[2][..], "#ROM" | "#LUT")
                            || c.len() != 4
                            || parse_lut(n, &c[3]) != Some(v)
                        {
                            println!("mismatched val {c:?} {n} ROM {v:x}");
                            ok = false;
                            continue;
                        }
                    }
                    TgtConfigVal::Ram(n, v) => {
                        if c[2] != "#RAM" || c.len() != 4 || parse_lut(n, &c[3]) != Some(v) {
                            println!("mismatched val {c:?} {n} RAM {v:x}");
                            ok = false;
                            continue;
                        }
                    }
                }
            } else if c[0] == "_INST_PROP" || c[0] == "_BEL_PROP" || c[2] == "#OFF" {
                // skip
            } else {
                println!("unexpected cfg {iname} {c:?}", iname = inst.name);
                ok = false;
            }
        }
        for x in cfg_expect.iter() {
            println!("expected cfg {iname} {x:?} not found", iname = inst.name);
            ok = false;
        }
    }
    for n in in_nets_pending {
        println!("ibuf not found: {n}");
        ok = false;
    }
    for n in in2_nets_pending {
        println!("ibuf2 not found: {n}");
        ok = false;
    }
    for n in out_nets_pending {
        println!("obuf not found: {n}");
        ok = false;
    }
    for n in exp_bels {
        println!("bel not found: {n:?}");
        ok = false;
    }
    let mut nets_found = HashSet::new();
    for net in design.nets.iter() {
        if net.outpins.len() == 1 && net.typ == NetType::Plain {
            let key = (net.outpins[0].inst_name.clone(), net.outpins[0].pin.clone());
            if net.inpins.is_empty() && dummy_out_wires.remove(&key) {
                continue;
            }
        }
        if net.typ == NetType::Plain {
            let mut nn = None;
            for (dd, pins) in [
                (TgtPinDir::Output, &net.outpins),
                (TgtPinDir::Input, &net.inpins),
            ] {
                for pin in pins {
                    let key = (pin.inst_name.clone(), pin.pin.clone());
                    if let Some((nnn, dir)) = wire_map.remove(&key) {
                        match &nn {
                            None => {
                                nn = Some(nnn);
                            }
                            Some(cnn) => {
                                if &nnn != cnn {
                                    println!("mixed net {} {} {}", net.name, cnn, nnn);
                                    ok = false;
                                }
                            }
                        }
                        if dir != dd {
                            println!("wrong pin dir {key:?}");
                            ok = false;
                        }
                    } else {
                        println!("unknown pin {key:?}");
                        ok = false;
                    }
                }
            }
            if let Some(cnn) = nn {
                if !nets_found.insert(cnn.clone()) {
                    println!("duplicate net {cnn}");
                    ok = false;
                }
            }
        } else {
            let tval = net.typ == NetType::Vcc;
            if !net.outpins.is_empty() {
                println!("out on tie net {}", net.name);
                ok = false;
            }
            for pin in net.inpins.iter() {
                let key = (pin.inst_name.clone(), pin.pin.clone());
                if let Some(pval) = wire_tie.remove(&key) {
                    if tval != pval {
                        println!("wrong tie val {key:?} has {tval} expected {pval}");
                        ok = false;
                    }
                } else {
                    println!("unknown tie {key:?} {tval}");
                    ok = false;
                }
            }
        }
    }
    for k in dummy_out_wires {
        println!("missing obuf out net {k:?}");
        ok = false;
    }
    for (k, v) in wire_map {
        println!("missing pin {k:?} {v:?}");
    }
    for (k, v) in wire_tie {
        println!("missing tie pin {k:?} {v:?}");
    }
    ok
}
