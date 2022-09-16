use prjcombine_entity::EntityId;
use prjcombine_int::grid::RowId;
use prjcombine_rawdump::Part;
use prjcombine_rdverify::{verify, BelContext, SitePinDir, Verifier};
use prjcombine_ultrascale::{ExpandedDevice, GridKind};

fn verify_slice(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let kind = if bel.node_kind == "CLEM" {
        "SLICEM"
    } else {
        "SLICEL"
    };
    if bel.name.is_some() {
        vrf.verify_bel(
            bel,
            kind,
            &[("CIN", SitePinDir::In), ("COUT", SitePinDir::Out)],
            &[],
        );
    }
    vrf.claim_pip(bel.crd(), bel.wire("CIN"), bel.wire_far("CIN"));
    vrf.claim_node(&[bel.fwire("CIN")]);
    if let Some(obel) = vrf.find_bel_delta(bel, 0, -1, bel.key) {
        vrf.verify_node(&[bel.fwire_far("CIN"), obel.fwire("COUT")]);
    }
    vrf.claim_node(&[bel.fwire("COUT")]);
}

fn verify_dsp(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pairs = vec![];
    pairs.push(("MULTSIGNIN".to_string(), "MULTSIGNOUT".to_string()));
    pairs.push(("CARRYCASCIN".to_string(), "CARRYCASCOUT".to_string()));
    for i in 0..30 {
        pairs.push((format!("ACIN_B{i}"), format!("ACOUT_B{i}")));
    }
    for i in 0..18 {
        pairs.push((format!("BCIN_B{i}"), format!("BCOUT_B{i}")));
    }
    for i in 0..48 {
        pairs.push((format!("PCIN{i}"), format!("PCOUT{i}")));
    }
    let mut pins = vec![];
    for (ipin, opin) in &pairs {
        pins.push((&ipin[..], SitePinDir::In));
        pins.push((&opin[..], SitePinDir::Out));
        vrf.claim_node(&[bel.fwire(opin)]);
        vrf.claim_node(&[bel.fwire(ipin)]);
        if bel.key == "DSP0" {
            vrf.claim_pip(bel.crd(), bel.wire(ipin), bel.wire_far(ipin));
            if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, "DSP1") {
                vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire(opin)]);
            }
        } else {
            let obel = vrf.find_bel_sibling(bel, "DSP0");
            vrf.claim_pip(bel.crd(), bel.wire(ipin), obel.wire(opin));
        }
    }
    if bel.name.is_some() {
        vrf.verify_bel(bel, "DSP48E2", &pins, &[]);
    }
}

fn verify_bram_f(vrf: &mut Verifier, bel: &BelContext<'_>) {
    #[derive(Copy, Clone, Eq, PartialEq, Debug)]
    enum Mode {
        Up,
        DownHalfReg,
        UpBuf,
        DownBuf,
    }
    let mut pairs = vec![
        (
            "ENABLE_BIST".to_string(),
            "START_RSR_NEXT".to_string(),
            Mode::DownHalfReg,
        ),
        (
            "CASINSBITERR".to_string(),
            "CASOUTSBITERR".to_string(),
            Mode::Up,
        ),
        (
            "CASINDBITERR".to_string(),
            "CASOUTDBITERR".to_string(),
            Mode::Up,
        ),
        (
            "CASPRVEMPTY".to_string(),
            "CASNXTEMPTY".to_string(),
            Mode::UpBuf,
        ),
        (
            "CASNXTRDEN".to_string(),
            "CASPRVRDEN".to_string(),
            Mode::DownBuf,
        ),
    ];
    for ab in ['A', 'B'] {
        for ul in ['U', 'L'] {
            for i in 0..16 {
                pairs.push((
                    format!("CASDI{ab}{ul}{i}"),
                    format!("CASDO{ab}{ul}{i}"),
                    Mode::UpBuf,
                ));
            }
            for i in 0..2 {
                pairs.push((
                    format!("CASDIP{ab}{ul}{i}"),
                    format!("CASDOP{ab}{ul}{i}"),
                    Mode::UpBuf,
                ));
            }
        }
    }
    let mut pins = vec![("CASMBIST12OUT", SitePinDir::Out)];
    vrf.claim_node(&[bel.fwire("CASMBIST12OUT")]);
    for (ipin, opin, mode) in &pairs {
        pins.push((&ipin[..], SitePinDir::In));
        pins.push((&opin[..], SitePinDir::Out));
        vrf.claim_node(&[bel.fwire(opin)]);
        vrf.claim_node(&[bel.fwire(ipin)]);
        vrf.claim_pip(bel.crd(), bel.wire(ipin), bel.wire_far(ipin));
        match mode {
            Mode::UpBuf => {
                vrf.claim_node(&[bel.fwire_far(opin)]);
                vrf.claim_pip(bel.crd(), bel.wire_far(opin), bel.wire(opin));
                if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, "BRAM_F") {
                    vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire_far(opin)]);
                } else {
                    vrf.claim_node(&[bel.fwire_far(ipin)]);
                }
            }
            Mode::Up => {
                if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, "BRAM_F") {
                    vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire(opin)]);
                } else {
                    vrf.claim_node(&[bel.fwire_far(ipin)]);
                }
            }
            Mode::DownBuf => {
                vrf.claim_node(&[bel.fwire_far(opin)]);
                vrf.claim_pip(bel.crd(), bel.wire_far(opin), bel.wire(opin));
                if let Some(obel) = vrf.find_bel_delta(bel, 0, 5, "BRAM_F") {
                    vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire_far(opin)]);
                } else {
                    vrf.claim_node(&[bel.fwire_far(ipin)]);
                }
            }
            Mode::DownHalfReg => {
                if bel.row.to_idx() % 30 != 25 {
                    if let Some(obel) = vrf.find_bel_delta(bel, 0, 5, "BRAM_F") {
                        vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire(opin)]);
                    } else {
                        vrf.claim_node(&[bel.fwire_far(ipin)]);
                    }
                } else {
                    vrf.claim_node(&[bel.fwire_far(ipin)]);
                }
            }
        }
    }
    vrf.verify_bel(bel, "RAMBFIFO36", &pins, &[]);
}

fn verify_bram_h(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let (kind, ul) = match bel.key {
        "BRAM_H0" => ("RAMBFIFO18", 'L'),
        "BRAM_H1" => ("RAMB181", 'U'),
        _ => unreachable!(),
    };
    let mut pins = vec![];
    if ul == 'L' {
        pins.extend([
            ("CASPRVEMPTY".to_string(), SitePinDir::In),
            ("CASNXTEMPTY".to_string(), SitePinDir::Out),
            ("CASPRVRDEN".to_string(), SitePinDir::Out),
            ("CASNXTRDEN".to_string(), SitePinDir::In),
        ]);
    }
    for ab in ['A', 'B'] {
        for i in 0..16 {
            pins.push((format!("CASDI{ab}{ul}{i}"), SitePinDir::In));
            pins.push((format!("CASDO{ab}{ul}{i}"), SitePinDir::Out));
        }
        for i in 0..2 {
            pins.push((format!("CASDIP{ab}{ul}{i}"), SitePinDir::In));
            pins.push((format!("CASDOP{ab}{ul}{i}"), SitePinDir::Out));
        }
    }
    let pin_refs: Vec<_> = pins.iter().map(|&(ref pin, dir)| (&pin[..], dir)).collect();
    vrf.verify_bel(bel, kind, &pin_refs, &[]);
    let obel = vrf.find_bel_sibling(bel, "BRAM_F");
    for (pin, dir) in pin_refs {
        vrf.claim_node(&[bel.fwire(pin)]);
        match dir {
            SitePinDir::In => vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire_far(pin)),
            SitePinDir::Out => vrf.claim_pip(bel.crd(), obel.wire_far(pin), bel.wire(pin)),
            _ => unreachable!(),
        }
    }
}

fn verify_uram(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pairs = vec![];
    for ab in ['A', 'B'] {
        for i in 0..23 {
            pairs.push((
                format!("CAS_IN_ADDR_{ab}{i}"),
                format!("CAS_OUT_ADDR_{ab}{i}"),
            ));
        }
        for i in 0..9 {
            pairs.push((
                format!("CAS_IN_BWE_{ab}{i}"),
                format!("CAS_OUT_BWE_{ab}{i}"),
            ));
        }
        for i in 0..72 {
            pairs.push((
                format!("CAS_IN_DIN_{ab}{i}"),
                format!("CAS_OUT_DIN_{ab}{i}"),
            ));
            pairs.push((
                format!("CAS_IN_DOUT_{ab}{i}"),
                format!("CAS_OUT_DOUT_{ab}{i}"),
            ));
        }
        for pin in ["EN", "RDACCESS", "RDB_WR", "DBITERR", "SBITERR"] {
            pairs.push((format!("CAS_IN_{pin}_{ab}"), format!("CAS_OUT_{pin}_{ab}")));
        }
    }
    let mut pins = vec![];
    for (ipin, opin) in &pairs {
        pins.push((&ipin[..], SitePinDir::In));
        pins.push((&opin[..], SitePinDir::Out));
        vrf.claim_node(&[bel.fwire(opin)]);
        vrf.claim_node(&[bel.fwire(ipin)]);
        if bel.key == "URAM0" {
            vrf.claim_pip(bel.crd(), bel.wire(ipin), bel.wire_far(ipin));
            if let Some(obel) = vrf.find_bel_delta(bel, 0, -15, "URAM3") {
                vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire(opin)]);
            }
        } else {
            let okey = match bel.key {
                "URAM1" => "URAM0",
                "URAM2" => "URAM1",
                "URAM3" => "URAM2",
                _ => unreachable!(),
            };
            let obel = vrf.find_bel_sibling(bel, okey);
            vrf.claim_pip(bel.crd(), bel.wire(ipin), obel.wire(opin));
        }
    }
    vrf.verify_bel(bel, "URAM288", &pins, &[]);
}

fn verify_laguna(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(bel, "LAGUNA", &[
        ("RXD0", SitePinDir::In),
        ("RXD1", SitePinDir::In),
        ("RXD2", SitePinDir::In),
        ("RXD3", SitePinDir::In),
        ("RXD4", SitePinDir::In),
        ("RXD5", SitePinDir::In),
        ("RXQ0", SitePinDir::Out),
        ("RXQ1", SitePinDir::Out),
        ("RXQ2", SitePinDir::Out),
        ("RXQ3", SitePinDir::Out),
        ("RXQ4", SitePinDir::Out),
        ("RXQ5", SitePinDir::Out),
        ("TXQ0", SitePinDir::Out),
        ("TXQ1", SitePinDir::Out),
        ("TXQ2", SitePinDir::Out),
        ("TXQ3", SitePinDir::Out),
        ("TXQ4", SitePinDir::Out),
        ("TXQ5", SitePinDir::Out),
    ], &[
        "RXOUT0",
        "RXOUT1",
        "RXOUT2",
        "RXOUT3",
        "RXOUT4",
        "RXOUT5",
    ]);
    let bel_vcc = vrf.find_bel_sibling(bel, "VCC");
    let mut obel = None;
    if bel.row.to_idx() < 60 {
        let odie = bel.die - 1;
        let orow = RowId::from_idx(edev.egrid.die(odie).rows().len() - 60 + bel.row.to_idx());
        obel = vrf.find_bel(odie, (bel.col, orow), bel.key);
        assert!(obel.is_some());
    }
    for i in 0..6 {
        vrf.claim_node(&[bel.fwire(&format!("TXQ{i}"))]);
        vrf.claim_node(&[bel.fwire(&format!("TXOUT{i}"))]);
        vrf.claim_node(&[bel.fwire(&format!("RXD{i}"))]);
        vrf.claim_node(&[bel.fwire(&format!("RXQ{i}"))]);
        vrf.claim_pip(bel.crd(), bel.wire(&format!("TXOUT{i}")), bel.wire(&format!("TXD{i}")));
        vrf.claim_pip(bel.crd(), bel.wire(&format!("TXOUT{i}")), bel.wire(&format!("TXQ{i}")));
        vrf.claim_pip(bel.crd(), bel.wire(&format!("RXOUT{i}")), bel.wire(&format!("RXD{i}")));
        vrf.claim_pip(bel.crd(), bel.wire(&format!("RXOUT{i}")), bel.wire(&format!("RXQ{i}")));
        vrf.claim_pip(bel.crd(), bel.wire(&format!("RXD{i}")), bel.wire(&format!("TXOUT{i}")));
        vrf.claim_pip(bel.crd(), bel.wire(&format!("RXD{i}")), bel.wire(&format!("UBUMP{i}")));
        vrf.claim_pip(bel.crd(), bel.wire(&format!("UBUMP{i}")), bel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), bel.wire(&format!("UBUMP{i}")), bel.wire(&format!("TXOUT{i}")));
        if let Some(ref obel) = obel {
            vrf.claim_node(&[
                bel.fwire(&format!("UBUMP{i}")),
                obel.fwire(&format!("UBUMP{i}")),
            ]);
        }
    }
}

fn verify_vcc(vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.claim_node(&[bel.fwire("VCC")]);
}

fn verify_pcie(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let kind = match bel.key {
        "PCIE" => "PCIE_3_1",
        "PCIE4" => "PCIE40E4",
        "PCIE4C" => "PCIE4CE4",
        _ => unreachable!(),
    };
    vrf.verify_bel_dummies(
        bel,
        kind,
        &[
            ("MCAP_PERST0_B", SitePinDir::In),
            ("MCAP_PERST1_B", SitePinDir::In),
        ],
        &[],
        &["MCAP_PERST0_B", "MCAP_PERST1_B"],
    );
}

fn verify_ps(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins_clk = [
        "O_DBG_L0_TXCLK",                   // 0.0
        "O_DBG_L0_RXCLK",                   // 0.1
        "O_DBG_L1_TXCLK",                   // 0.2
        "O_DBG_L1_RXCLK",                   // 0.3
        "O_DBG_L2_TXCLK",                   // 0.4
        "O_DBG_L2_RXCLK",                   // 0.5
        "O_DBG_L3_TXCLK",                   // 0.6
        "O_DBG_L3_RXCLK",                   // 0.7
        "APLL_TEST_CLK_OUT0",               // 1.0
        "APLL_TEST_CLK_OUT1",               // 1.1
        "DPLL_TEST_CLK_OUT0",               // 1.2
        "DPLL_TEST_CLK_OUT1",               // 1.3
        "VPLL_TEST_CLK_OUT0",               // 1.4
        "VPLL_TEST_CLK_OUT1",               // 1.5
        "DP_AUDIO_REF_CLK",                 // 1.6
        "DP_VIDEO_REF_CLK",                 // 1.7
        "DDR_DTO0",                         // 1.8
        "DDR_DTO1",                         // 1.9
        "PL_CLK0",                          // 2.0
        "PL_CLK1",                          // 2.1
        "PL_CLK2",                          // 2.2
        "PL_CLK3",                          // 2.3
        "IOPLL_TEST_CLK_OUT0",              // 2.4
        "IOPLL_TEST_CLK_OUT1",              // 2.5
        "RPLL_TEST_CLK_OUT0",               // 2.6
        "RPLL_TEST_CLK_OUT1",               // 2.7
        "FMIO_GEM0_FIFO_TX_CLK_TO_PL_BUFG", // 2.8
        "FMIO_GEM0_FIFO_RX_CLK_TO_PL_BUFG", // 2.9
        "FMIO_GEM1_FIFO_TX_CLK_TO_PL_BUFG", // 2.10
        "FMIO_GEM1_FIFO_RX_CLK_TO_PL_BUFG", // 2.11
        "FMIO_GEM2_FIFO_TX_CLK_TO_PL_BUFG", // 2.12
        "FMIO_GEM2_FIFO_RX_CLK_TO_PL_BUFG", // 2.13
        "FMIO_GEM3_FIFO_TX_CLK_TO_PL_BUFG", // 2.14
        "FMIO_GEM3_FIFO_RX_CLK_TO_PL_BUFG", // 2.15
        "FMIO_GEM_TSU_CLK_TO_PL_BUFG",      // 2.16
        "PS_PL_SYSOSC_CLK",                 // 2.17
    ];
    let pins_cfg_in = [
        "BSCAN_RESET_TAP_B",
        "BSCAN_CLOCKDR",
        "BSCAN_SHIFTDR",
        "BSCAN_UPDATEDR",
        "BSCAN_INTEST",
        "BSCAN_EXTEST",
        "BSCAN_INIT_MEMORY",
        "BSCAN_AC_TEST",
        "BSCAN_AC_MODE",
        "BSCAN_MISR_JTAG_LOAD",
        "PSS_CFG_RESET_B",
        "PSS_FST_CFG_B",
        "PSS_GTS_CFG_B",
        "PSS_GTS_USR_B",
        "PSS_GHIGH_B",
        "PSS_GPWRDWN_B",
        "PCFG_POR_B",
    ];
    let mut pins_dummy_in = vec![
        "IDCODE15",
        "IDCODE17",
        "IDCODE18",
        "IDCODE20",
        "IDCODE21",
        "IDCODE28",
        "IDCODE29",
        "IDCODE30",
        "IDCODE31",
        "PS_VERSION_0",
        "PS_VERSION_2",
        "PS_VERSION_3",
    ];
    let tk = vrf.rd.tile_kinds.get("PSS_ALTO").unwrap().1;
    let site = tk.sites.values().next().unwrap();
    if site.pins.contains_key("IDCODE16") {
        pins_dummy_in.push("IDCODE16");
    }
    let mut pins = vec![];
    for pin in pins_clk {
        vrf.claim_node(&[bel.fwire(pin)]);
        pins.push((pin, SitePinDir::Out));
    }
    for pin in pins_cfg_in {
        vrf.claim_node(&[bel.fwire(pin)]);
        pins.push((pin, SitePinDir::In));
    }
    for &pin in &pins_dummy_in {
        pins.push((pin, SitePinDir::In));
    }
    vrf.verify_bel_dummies(bel, "PS8", &pins, &[], &pins_dummy_in);
}

fn verify_vcu(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [
        ("VCU_PLL_TEST_CLK_OUT0", SitePinDir::Out), // 0
        ("VCU_PLL_TEST_CLK_OUT1", SitePinDir::Out), // 1
    ];
    vrf.verify_bel(bel, "VCU", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
}

fn verify_sysmon(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let vaux: Vec<_> = (0..16)
        .map(|i| (format!("VP_AUX{i}"), format!("VN_AUX{i}")))
        .collect();
    let mut pins = vec![];
    if edev.grids[bel.die].kind == GridKind::Ultrascale {
        pins.extend([
            ("I2C_SCLK_IN", SitePinDir::In),
            ("I2C_SCLK_TS", SitePinDir::Out),
            ("I2C_SDA_IN", SitePinDir::In),
            ("I2C_SDA_TS", SitePinDir::Out),
        ]);
    }
    for (vp, vn) in &vaux {
        pins.extend([(&vp[..], SitePinDir::In), (&vn[..], SitePinDir::In)]);
    }
    let kind = match edev.grids[bel.die].kind {
        GridKind::Ultrascale => "SYSMONE1",
        GridKind::UltrascalePlus => "SYSMONE4",
    };
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    // XXX source VAUX?
}

fn verify_abus_switch(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = &[][..];
    if edev.grids[bel.die].kind == GridKind::UltrascalePlus {
        pins = &[("TEST_ANALOGBUS_SEL_B", SitePinDir::In)];
    }
    vrf.verify_bel(bel, "ABUS_SWITCH", pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
}

fn verify_bel(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.key {
        "SLICE_L" | "SLICE_R" => verify_slice(vrf, bel),
        "DSP0" | "DSP1" => verify_dsp(vrf, bel),
        "BRAM_F" => verify_bram_f(vrf, bel),
        "BRAM_H0" | "BRAM_H1" => verify_bram_h(vrf, bel),
        _ if bel.key.starts_with("HARD_SYNC") => vrf.verify_bel(bel, "HARD_SYNC", &[], &[]),
        _ if bel.key.starts_with("URAM") => verify_uram(vrf, bel),
        "LAGUNA0" | "LAGUNA1" | "LAGUNA2" | "LAGUNA3" => verify_laguna(edev, vrf, bel),
        "VCC" => verify_vcc(vrf, bel),
        "PCIE" | "PCIE4" | "PCIE4C" => verify_pcie(vrf, bel),
        "CMAC" => vrf.verify_bel(
            bel,
            if edev.grids[bel.die].kind == GridKind::Ultrascale {
                "CMAC_SITE"
            } else {
                "CMACE4"
            },
            &[],
            &[],
        ),
        "ILKN" => vrf.verify_bel(
            bel,
            if edev.grids[bel.die].kind == GridKind::Ultrascale {
                "ILKN_SITE"
            } else {
                "ILKNE4"
            },
            &[],
            &[],
        ),
        "PMV" | "PMV2" | "PMVIOB" | "MTBF3" | "CFGIO_SITE" | "DFE_A" | "DFE_B" | "DFE_C"
        | "DFE_D" | "DFE_E" | "DFE_F" | "DFE_G" | "FE" | "BLI_HBM_APB_INTF"
        | "BLI_HBM_AXI_INTF" => vrf.verify_bel(bel, bel.key, &[], &[]),
        "CFG" => vrf.verify_bel(bel, "CONFIG_SITE", &[], &[]),
        "SYSMON" => verify_sysmon(edev, vrf, bel),
        "PS" => verify_ps(vrf, bel),
        "VCU" => verify_vcu(vrf, bel),
        _ if bel.key.starts_with("ABUS_SWITCH") => verify_abus_switch(edev, vrf, bel),
        _ => println!("MEOW {} {:?}", bel.key, bel.name),
    }
}

fn verify_extra(_edev: &ExpandedDevice, vrf: &mut Verifier) {
    // XXX
    vrf.skip_residual();
}

pub fn verify_device(edev: &ExpandedDevice, rd: &Part) {
    verify(
        rd,
        &edev.egrid,
        |vrf, bel| verify_bel(edev, vrf, bel),
        |vrf| verify_extra(edev, vrf),
    );
}
