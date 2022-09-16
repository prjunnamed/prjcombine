use prjcombine_entity::EntityId;
use prjcombine_int::grid::RowId;
use prjcombine_rawdump::Part;
use prjcombine_rdverify::{verify, BelContext, SitePinDir, Verifier};
use prjcombine_ultrascale::{
    ColumnKindLeft, DisabledPart, ExpandedDevice, GridKind, HardRowKind, HdioIobId,
};

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
    vrf.verify_bel(
        bel,
        "LAGUNA",
        &[
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
        ],
        &["RXOUT0", "RXOUT1", "RXOUT2", "RXOUT3", "RXOUT4", "RXOUT5"],
    );
    let bel_vcc = vrf.find_bel_sibling(bel, "VCC.LAGUNA");
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
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("TXOUT{i}")),
            bel.wire(&format!("TXD{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("TXOUT{i}")),
            bel.wire(&format!("TXQ{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("RXOUT{i}")),
            bel.wire(&format!("RXD{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("RXOUT{i}")),
            bel.wire(&format!("RXQ{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("RXD{i}")),
            bel.wire(&format!("TXOUT{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("RXD{i}")),
            bel.wire(&format!("UBUMP{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("UBUMP{i}")),
            bel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("UBUMP{i}")),
            bel.wire(&format!("TXOUT{i}")),
        );
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
        (0, 0, "O_DBG_L0_TXCLK"),
        (0, 1, "O_DBG_L0_RXCLK"),
        (0, 2, "O_DBG_L1_TXCLK"),
        (0, 3, "O_DBG_L1_RXCLK"),
        (0, 4, "O_DBG_L2_TXCLK"),
        (0, 5, "O_DBG_L2_RXCLK"),
        (0, 6, "O_DBG_L3_TXCLK"),
        (0, 7, "O_DBG_L3_RXCLK"),
        (1, 0, "APLL_TEST_CLK_OUT0"),
        (1, 1, "APLL_TEST_CLK_OUT1"),
        (1, 2, "DPLL_TEST_CLK_OUT0"),
        (1, 3, "DPLL_TEST_CLK_OUT1"),
        (1, 4, "VPLL_TEST_CLK_OUT0"),
        (1, 5, "VPLL_TEST_CLK_OUT1"),
        (1, 6, "DP_AUDIO_REF_CLK"),
        (1, 7, "DP_VIDEO_REF_CLK"),
        (1, 8, "DDR_DTO0"),
        (1, 9, "DDR_DTO1"),
        (2, 0, "PL_CLK0"),
        (2, 1, "PL_CLK1"),
        (2, 2, "PL_CLK2"),
        (2, 3, "PL_CLK3"),
        (2, 4, "IOPLL_TEST_CLK_OUT0"),
        (2, 5, "IOPLL_TEST_CLK_OUT1"),
        (2, 6, "RPLL_TEST_CLK_OUT0"),
        (2, 7, "RPLL_TEST_CLK_OUT1"),
        (2, 8, "FMIO_GEM0_FIFO_TX_CLK_TO_PL_BUFG"),
        (2, 9, "FMIO_GEM0_FIFO_RX_CLK_TO_PL_BUFG"),
        (2, 10, "FMIO_GEM1_FIFO_TX_CLK_TO_PL_BUFG"),
        (2, 11, "FMIO_GEM1_FIFO_RX_CLK_TO_PL_BUFG"),
        (2, 12, "FMIO_GEM2_FIFO_TX_CLK_TO_PL_BUFG"),
        (2, 13, "FMIO_GEM2_FIFO_RX_CLK_TO_PL_BUFG"),
        (2, 14, "FMIO_GEM3_FIFO_TX_CLK_TO_PL_BUFG"),
        (2, 15, "FMIO_GEM3_FIFO_RX_CLK_TO_PL_BUFG"),
        (2, 16, "FMIO_GEM_TSU_CLK_TO_PL_BUFG"),
        (2, 17, "PS_PL_SYSOSC_CLK"),
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
    let obels = [
        vrf.find_bel_delta(bel, 0, 30, "RCLK_PS").unwrap(),
        vrf.find_bel_delta(bel, 0, 90, "RCLK_PS").unwrap(),
        vrf.find_bel_delta(bel, 0, 150, "RCLK_PS").unwrap(),
    ];
    for (reg, idx, pin) in pins_clk {
        vrf.claim_node(&[
            bel.fwire(pin),
            obels[reg].fwire(&format!("PS_TO_PL_CLK{idx}")),
        ]);
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
    let pins_clk = [(0, "VCU_PLL_TEST_CLK_OUT0"), (1, "VCU_PLL_TEST_CLK_OUT1")];
    let obel = vrf.find_bel_delta(bel, 0, 30, "RCLK_PS").unwrap();
    let mut pins = vec![];
    for (idx, pin) in pins_clk {
        vrf.claim_node(&[bel.fwire(pin), obel.fwire(&format!("PS_TO_PL_CLK{idx}"))]);
        pins.push((pin, SitePinDir::Out));
    }
    vrf.verify_bel(bel, "VCU", &pins, &[]);
}

fn verify_sysmon(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let grid = edev.grids[bel.die];
    let vaux: Vec<_> = (0..16)
        .map(|i| (format!("VP_AUX{i}"), format!("VN_AUX{i}")))
        .collect();
    let mut pins = vec![];
    if grid.kind == GridKind::Ultrascale {
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
    let kind = match grid.kind {
        GridKind::Ultrascale => "SYSMONE1",
        GridKind::UltrascalePlus => "SYSMONE4",
    };
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    if grid.kind == GridKind::UltrascalePlus {
        for i in 0..16 {
            for pn in ['P', 'N'] {
                let pin = format!("V{pn}_AUX{i}");
                vrf.claim_node(&[bel.fwire_far(&pin)]);
                vrf.claim_pip(bel.crd(), bel.wire(&pin), bel.wire_far(&pin));
            }
        }
    }
}

fn verify_abus_switch(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = &[][..];
    if edev.grids[bel.die].kind == GridKind::UltrascalePlus
        && !bel.bel.pins.contains_key("TEST_ANALOGBUS_SEL_B")
    {
        pins = &[("TEST_ANALOGBUS_SEL_B", SitePinDir::In)];
    }
    vrf.verify_bel(bel, "ABUS_SWITCH", pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
}

fn verify_bufce_leaf_x16(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let clk_in: [_; 16] = core::array::from_fn(|i| format!("CLK_IN{i}"));
    let pins: Vec<_> = clk_in.iter().map(|x| (&x[..], SitePinDir::In)).collect();
    vrf.verify_bel(bel, "BUFCE_LEAF_X16", &pins, &[]);
    let obel = vrf.find_bel_sibling(bel, "RCLK_INT");
    for pin in &clk_in {
        vrf.claim_node(&[bel.fwire(pin)]);
        for j in 0..24 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire(&format!("HDISTR{j}")));
        }
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire("VCC"));
    }
}

fn verify_bufce_leaf(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = vec![
        ("CLK_CASC_OUT", SitePinDir::Out),
        ("CLK_IN", SitePinDir::In),
    ];
    if bel.key != "BUFCE_LEAF_D0" {
        pins.push(("CLK_CASC_IN", SitePinDir::In));
    }
    vrf.verify_bel(bel, "BUFCE_LEAF", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel = vrf.find_bel_sibling(bel, "RCLK_INT");
    for j in 0..24 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLK_IN"),
            obel.wire(&format!("HDISTR{j}")),
        );
    }
    vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("VCC"));
    if bel.key != "BUFCE_LEAF_D0" {
        let idx: usize = bel.key[12..].parse().unwrap();
        let okey = if bel.key == "BUFCE_LEAF_U0" {
            "BUFCE_LEAF_D15".to_string()
        } else {
            format!("{p}{ni}", p = &bel.key[..12], ni = idx - 1)
        };
        let obel = vrf.find_bel_sibling(bel, &okey);
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLK_CASC_IN"),
            obel.wire("CLK_CASC_OUT"),
        );
    }
}

fn verify_rclk_int(_edev: &ExpandedDevice, _vrf: &mut Verifier, _bel: &BelContext<'_>) {
    // XXX source HDISTR
}

fn verify_rclk_splitter(_edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel_vcc = vrf.find_bel_sibling(bel, "VCC.RCLK_SPLITTER");
    for i in 0..24 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L")),
            bel.wire(&format!("HROUTE{i}_R")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R")),
            bel.wire(&format!("HROUTE{i}_L")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_L")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_L")),
            bel.wire(&format!("HDISTR{i}_R")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_R")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_R")),
            bel.wire(&format!("HDISTR{i}_L")),
        );
    }
    // XXX source HROUTE, HDISTR
}

fn verify_rclk_hroute_splitter(_edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel_vcc = vrf.find_bel_sibling(bel, "VCC.RCLK_HROUTE_SPLITTER");
    for i in 0..24 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L")),
            bel.wire(&format!("HROUTE{i}_R")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R")),
            bel.wire(&format!("HROUTE{i}_L")),
        );
    }
    // XXX source HROUTE
}

fn verify_bufg_ps(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = vec![("CLK_OUT", SitePinDir::Out), ("CLK_IN", SitePinDir::In)];
    vrf.verify_bel(bel, "BUFG_PS", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel_vcc = vrf.find_bel_sibling(bel, "VCC.RCLK_PS");
    let obel = vrf.find_bel_sibling(bel, "RCLK_PS");
    for j in 0..18 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLK_IN"),
            obel.wire(&format!("PS_TO_PL_CLK{j}")),
        );
    }
    vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel_vcc.wire("VCC"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("CKINT"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire("CLK_IN_DUMMY"));
}

fn verify_rclk_ps(_edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel_vcc = vrf.find_bel_sibling(bel, "VCC.RCLK_PS");
    for i in 0..24 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}")),
            obel_vcc.wire("VCC"),
        );
        let obel = vrf.find_bel_sibling(bel, &format!("BUFG_PS{i}"));
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}")),
            obel.wire("CLK_OUT"),
        );
    }
    // XXX source HROUTE
}

fn verify_hdiob(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx: usize = bel.key[7..].parse().unwrap();
    let is_b = bel.row.to_idx() % 60 == 0;
    let hid = HdioIobId::from_idx(idx + if is_b { 0 } else { 6 });
    let grid = edev.grids[bel.die];
    let reg = grid.row_to_reg(bel.row);
    let is_m = bel.key.starts_with("HDIOB_M");
    let kind = if is_m { "HDIOB_M" } else { "HDIOB_S" };
    let pins = [
        ("OP", SitePinDir::In),
        ("TSP", SitePinDir::In),
        ("O_B", SitePinDir::Out),
        ("TSTATEB", SitePinDir::Out),
        ("OUTB_B", SitePinDir::Out),
        ("OUTB_B_IN", SitePinDir::In),
        ("TSTATE_OUT", SitePinDir::Out),
        ("TSTATE_IN", SitePinDir::In),
        ("LVDS_TRUE", SitePinDir::In),
        ("PAD_RES", SitePinDir::Out),
        ("I", SitePinDir::Out),
        ("IO", SitePinDir::In),
        ("SWITCH_OUT", SitePinDir::Out),
    ];
    if !edev
        .disabled
        .contains(&DisabledPart::HdioIob(bel.die, bel.col, reg, hid))
    {
        vrf.verify_bel_dummies(bel, kind, &pins, &[], &["IO"]);
    }
    for (pin, _) in pins {
        if pin != "IO" {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
    }
    let obel = vrf.find_bel_sibling(
        bel,
        &format!(
            "HDIOB_{ms}{i}",
            ms = if is_m { 'S' } else { 'M' },
            i = &bel.key[7..]
        ),
    );
    vrf.claim_pip(bel.crd(), bel.wire("OUTB_B_IN"), obel.wire("OUTB_B"));
    vrf.claim_pip(bel.crd(), bel.wire("TSTATE_IN"), obel.wire("TSTATE_OUT"));

    vrf.claim_pip(
        bel.crd(),
        bel.wire_far("SWITCH_OUT"),
        bel.wire("SWITCH_OUT"),
    );

    let hdio_kind = if let ColumnKindLeft::Hard(hi) = grid.columns[bel.col].l {
        grid.cols_hard[hi].regs[reg]
    } else {
        unreachable!()
    };

    let ams_idx = match (hdio_kind, is_b, idx) {
        (HardRowKind::HdioAms, true, _) => Some(11 - idx),
        (HardRowKind::HdioAms, false, _) => Some(5 - idx),
        (HardRowKind::Hdio, true, 0..=3) => Some(15 - idx),
        (HardRowKind::Hdio, false, 2..=5) => Some(11 - (idx - 2)),
        _ => None,
    };

    if let Some(ams_idx) = ams_idx {
        let scol = grid.col_cfg();
        let srow = grid.row_ams();
        let obel = vrf.find_bel(bel.die, (scol, srow), "SYSMON").unwrap();
        vrf.verify_node(&[
            bel.fwire_far("SWITCH_OUT"),
            obel.fwire_far(&format!(
                "V{pn}_AUX{ams_idx}",
                pn = if is_m { 'P' } else { 'N' }
            )),
        ]);
    }
}

fn verify_hdiobdiffinbuf(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx: usize = bel.key[14..].parse().unwrap();
    let is_b = bel.row.to_idx() % 60 == 0;
    let hid = HdioIobId::from_idx(idx + if is_b { 0 } else { 6 });
    let grid = edev.grids[bel.die];
    let reg = grid.row_to_reg(bel.row);
    let pins = [
        ("LVDS_TRUE", SitePinDir::Out),
        ("LVDS_COMP", SitePinDir::Out),
        ("PAD_RES_0", SitePinDir::In),
        ("PAD_RES_1", SitePinDir::In),
    ];
    if !edev
        .disabled
        .contains(&DisabledPart::HdioIob(bel.die, bel.col, reg, hid))
    {
        vrf.verify_bel(bel, "HDIOBDIFFINBUF", &pins, &[]);
    }
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel_m = vrf.find_bel_sibling(bel, &format!("HDIOB_M{idx}"));
    let obel_s = vrf.find_bel_sibling(bel, &format!("HDIOB_S{idx}"));
    vrf.claim_pip(bel.crd(), obel_m.wire("LVDS_TRUE"), bel.wire("LVDS_TRUE"));
    vrf.claim_pip(bel.crd(), obel_s.wire("LVDS_TRUE"), bel.wire("LVDS_COMP"));
    vrf.claim_pip(bel.crd(), bel.wire("PAD_RES_0"), obel_m.wire("PAD_RES"));
    vrf.claim_pip(bel.crd(), bel.wire("PAD_RES_1"), obel_s.wire("PAD_RES"));
}

fn verify_hdiologic(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let is_m = bel.key.starts_with("HDIOLOGIC_M");
    let kind = if is_m { "HDIOLOGIC_M" } else { "HDIOLOGIC_S" };
    let pins = if is_m {
        [
            ("IPFFM_D", SitePinDir::In),
            ("OPFFM_Q", SitePinDir::Out),
            ("TFFM_Q", SitePinDir::Out),
        ]
    } else {
        [
            ("IPFFS_D", SitePinDir::In),
            ("OPFFS_Q", SitePinDir::Out),
            ("TFFS_Q", SitePinDir::Out),
        ]
    };
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let okey = format!("HDIOB_{r}", r = &bel.key[10..]);
    let obel = vrf.find_bel_sibling(bel, &okey);
    vrf.claim_pip(
        bel.crd(),
        obel.wire("OP"),
        bel.wire(if is_m { "OPFFM_Q" } else { "OPFFS_Q" }),
    );
    vrf.claim_pip(
        bel.crd(),
        obel.wire("TSP"),
        bel.wire(if is_m { "TFFM_Q" } else { "TFFS_Q" }),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire(if is_m { "IPFFM_D" } else { "IPFFS_D" }),
        obel.wire("I"),
    );
}

fn verify_bufgce_hdio(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = [("CLK_OUT", SitePinDir::Out), ("CLK_IN", SitePinDir::In)];
    vrf.verify_bel(bel, "BUFGCE_HDIO", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel_rclk = vrf.find_bel_sibling(bel, "RCLK_HDIO");
    vrf.claim_node(&[bel.fwire("CLK_IN_MUX")]);
    vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire("CLK_IN_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel_rclk.wire("CKINT"));
    for i in 0..4 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLK_IN_MUX"),
            obel_rclk.wire(&format!("CCIO{i}")),
        );
    }
}

fn verify_rclk_hdio(_edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel_bufgce: [_; 4] =
        core::array::from_fn(|i| vrf.find_bel_sibling(bel, &format!("BUFGCE_HDIO{i}")));
    let obel_vcc = vrf.find_bel_sibling(bel, "VCC.RCLK_HDIO");
    for i in 0..24 {
        vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}")),
            bel.wire(&format!("HDISTR{i}_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}")),
            obel_vcc.wire("VCC"),
        );
        for obel in &obel_bufgce {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("HDISTR{i}_MUX")),
                obel.wire("CLK_OUT"),
            );
        }

        vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_L_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L")),
            bel.wire(&format!("HROUTE{i}_L_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L_MUX")),
            bel.wire(&format!("HROUTE{i}_R")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L_MUX")),
            bel.wire(&format!("HDISTR{i}_MUX")),
        );

        vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_R_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R")),
            bel.wire(&format!("HROUTE{i}_R_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R_MUX")),
            bel.wire(&format!("HROUTE{i}_L")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R_MUX")),
            bel.wire(&format!("HDISTR{i}_MUX")),
        );
    }
    for (i, dy, key) in [
        (0, 0, "HDIOB_M0"),
        (1, 0, "HDIOB_M1"),
        (2, -30, "HDIOB_M4"),
        (3, -30, "HDIOB_M5"),
    ] {
        let obel = vrf.find_bel_delta(bel, 0, dy, key).unwrap();
        vrf.verify_node(&[bel.fwire(&format!("CCIO{i}")), obel.fwire("I")]);
    }

    // XXX source HDISTR
    // XXX source/claim HROUTE
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
        _ if bel.key.starts_with("VCC") => verify_vcc(vrf, bel),

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
        | "BLI_HBM_AXI_INTF" | "HDLOGIC_CSSD" | "HDIO_VREF" | "HDIO_BIAS" => {
            vrf.verify_bel(bel, bel.key, &[], &[])
        }
        "CFG" => vrf.verify_bel(bel, "CONFIG_SITE", &[], &[]),
        "SYSMON" => verify_sysmon(edev, vrf, bel),
        "PS" => verify_ps(vrf, bel),
        "VCU" => verify_vcu(vrf, bel),
        _ if bel.key.starts_with("ABUS_SWITCH") => verify_abus_switch(edev, vrf, bel),

        _ if bel.key.starts_with("BUFCE_LEAF_X16") => verify_bufce_leaf_x16(vrf, bel),
        _ if bel.key.starts_with("BUFCE_LEAF") => verify_bufce_leaf(vrf, bel),
        "RCLK_INT" => verify_rclk_int(edev, vrf, bel),

        "RCLK_SPLITTER" => verify_rclk_splitter(edev, vrf, bel),
        "RCLK_HROUTE_SPLITTER" => verify_rclk_hroute_splitter(edev, vrf, bel),

        _ if bel.key.starts_with("BUFG_PS") => verify_bufg_ps(vrf, bel),
        "RCLK_PS" => verify_rclk_ps(edev, vrf, bel),

        _ if bel.key.starts_with("HDIOBDIFFINBUF") => verify_hdiobdiffinbuf(edev, vrf, bel),
        _ if bel.key.starts_with("HDIOB") => verify_hdiob(edev, vrf, bel),
        _ if bel.key.starts_with("HDIOLOGIC") => verify_hdiologic(vrf, bel),
        _ if bel.key.starts_with("BUFGCE_HDIO") => verify_bufgce_hdio(vrf, bel),
        "RCLK_HDIO" => verify_rclk_hdio(edev, vrf, bel),

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
