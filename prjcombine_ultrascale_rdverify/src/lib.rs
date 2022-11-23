#![allow(clippy::bool_to_int_with_if)]
#![allow(clippy::collapsible_else_if)]
use prjcombine_entity::EntityId;
use prjcombine_int::grid::{ColId, DieId, RowId};
use prjcombine_rawdump::Part;
use prjcombine_rdverify::{verify, BelContext, SitePinDir, Verifier};
use prjcombine_ultrascale::{
    ClkSrc, ColSide, ColumnKindLeft, DisabledPart, ExpandedDevice, GridKind, HardRowKind,
    HdioIobId, HpioIobId,
};

fn is_cut_d(edev: &ExpandedDevice, die: DieId, row: RowId) -> bool {
    let reg = edev.grids[die].row_to_reg(row);
    if reg.to_idx() == 0 {
        false
    } else {
        edev.disabled.contains(&DisabledPart::Region(die, reg - 1))
    }
}

fn is_cut_u(edev: &ExpandedDevice, die: DieId, row: RowId) -> bool {
    let reg = edev.grids[die].row_to_reg(row);
    edev.disabled.contains(&DisabledPart::Region(die, reg + 1))
}

fn verify_slice(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
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
    } else if !is_cut_d(edev, bel.die, bel.row) {
        vrf.claim_node(&[bel.fwire_far("CIN")]);
    }
    vrf.claim_node(&[bel.fwire("COUT")]);
}

fn verify_dsp(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
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
            } else if !is_cut_d(edev, bel.die, bel.row) {
                vrf.claim_node(&[bel.fwire_far(ipin)]);
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

fn verify_bram_f(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
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
                if !edev.is_cut || vrf.find_bel_delta(bel, 0, 5, "BRAM_F").is_some() {
                    vrf.claim_node(&[bel.fwire_far(opin)]);
                    vrf.claim_pip(bel.crd(), bel.wire_far(opin), bel.wire(opin));
                }
                if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, "BRAM_F") {
                    vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire_far(opin)]);
                } else if !is_cut_d(edev, bel.die, bel.row) {
                    vrf.claim_node(&[bel.fwire_far(ipin)]);
                }
            }
            Mode::Up => {
                if let Some(obel) = vrf.find_bel_delta(bel, 0, -5, "BRAM_F") {
                    vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire(opin)]);
                } else if !is_cut_d(edev, bel.die, bel.row) {
                    vrf.claim_node(&[bel.fwire_far(ipin)]);
                }
            }
            Mode::DownBuf => {
                if !edev.is_cut || vrf.find_bel_delta(bel, 0, -5, "BRAM_F").is_some() {
                    vrf.claim_node(&[bel.fwire_far(opin)]);
                    vrf.claim_pip(bel.crd(), bel.wire_far(opin), bel.wire(opin));
                }
                if let Some(obel) = vrf.find_bel_delta(bel, 0, 5, "BRAM_F") {
                    vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire_far(opin)]);
                } else if !is_cut_u(edev, bel.die, bel.row) {
                    vrf.claim_node(&[bel.fwire_far(ipin)]);
                }
            }
            Mode::DownHalfReg => match bel.row.to_idx() % 60 {
                25 => (),
                55 => vrf.claim_node(&[bel.fwire_far(ipin)]),
                _ => {
                    if let Some(obel) = vrf.find_bel_delta(bel, 0, 5, "BRAM_F") {
                        vrf.verify_node(&[bel.fwire_far(ipin), obel.fwire(opin)]);
                    } else {
                        vrf.claim_node(&[bel.fwire_far(ipin)]);
                    }
                }
            },
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
    let skip = if bel.row.to_idx() < 60 {
        bel.die.to_idx() == 0
    } else {
        bel.die.to_idx() == edev.grids.len() - 1
    };
    if !skip {
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
    }
    let bel_vcc = vrf.find_bel_sibling(bel, "VCC.LAGUNA");
    let mut obel = None;

    if bel.row.to_idx() < 60 && !skip {
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
        } else if skip {
            vrf.claim_node(&[bel.fwire(&format!("UBUMP{i}"))]);
        }
    }
}

fn verify_laguna_extra(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let skip = if bel.row.to_idx() < 60 {
        bel.die.to_idx() == 0
    } else {
        bel.die.to_idx() == edev.grids.len() - 1
    };
    let bel_vcc = vrf.find_bel_sibling(bel, "VCC.LAGUNA");
    let mut obel = None;

    if bel.row.to_idx() < 60 && !skip {
        let odie = bel.die - 1;
        let orow = RowId::from_idx(edev.egrid.die(odie).rows().len() - 60 + bel.row.to_idx());
        obel = vrf.find_bel(odie, (bel.col, orow), bel.key);
        assert!(obel.is_some());
    }

    if !edev.is_cut {
        vrf.claim_node(&[bel.fwire("RXD")]);
        vrf.claim_pip(bel.crd(), bel.wire("RXD"), bel.wire("TXOUT"));
        vrf.claim_pip(bel.crd(), bel.wire("RXD"), bel.wire("UBUMP"));
    }
    vrf.claim_node(&[bel.fwire("TXOUT")]);
    vrf.claim_pip(bel.crd(), bel.wire("UBUMP"), bel_vcc.wire("VCC"));
    vrf.claim_pip(bel.crd(), bel.wire("UBUMP"), bel.wire("TXOUT"));
    if let Some(ref obel) = obel {
        vrf.claim_node(&[bel.fwire("UBUMP"), obel.fwire("UBUMP")]);
    } else if skip {
        vrf.claim_node(&[bel.fwire("UBUMP")]);
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

fn verify_cmac(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        if edev.kind == GridKind::Ultrascale {
            "CMAC_SITE"
        } else {
            "CMACE4"
        },
        &[],
        &[],
    );
}
fn verify_ilkn(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    vrf.verify_bel(
        bel,
        if edev.kind == GridKind::Ultrascale {
            "ILKN_SITE"
        } else {
            "ILKNE4"
        },
        &[],
        &[],
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
    let vaux: Vec<_> = (0..16)
        .map(|i| (format!("VP_AUX{i}"), format!("VN_AUX{i}")))
        .collect();
    let mut pins = vec![];
    if edev.kind == GridKind::Ultrascale {
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
    let kind = match edev.kind {
        GridKind::Ultrascale => "SYSMONE1",
        GridKind::UltrascalePlus => "SYSMONE4",
    };
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    if edev.kind == GridKind::UltrascalePlus {
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
    let grid = edev.grids[bel.die];
    let mut pins = &[][..];
    if edev.kind == GridKind::UltrascalePlus && !bel.bel.pins.contains_key("TEST_ANALOGBUS_SEL_B") {
        pins = &[("TEST_ANALOGBUS_SEL_B", SitePinDir::In)];
    }
    let mut skip = false;
    if bel.node_kind.starts_with("GTM") {
        let reg = grid.row_to_reg(bel.row);
        skip = edev
            .disabled
            .contains(&DisabledPart::Gt(bel.die, bel.col, reg));
    }
    if !skip {
        vrf.verify_bel(bel, "ABUS_SWITCH", pins, &[]);
    }
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

fn find_hdistr_src<'a>(
    edev: &ExpandedDevice,
    vrf: &mut Verifier<'a>,
    die: DieId,
    col: ColId,
    row: RowId,
    side: ColSide,
) -> BelContext<'a> {
    let src = edev.hdistr_src[col][side];
    match src {
        ClkSrc::Gt(scol) => vrf
            .find_bel(die, (scol, row), "RCLK_GT_R")
            .or_else(|| vrf.find_bel(die, (scol, row), "RCLK_XIPHY"))
            .or_else(|| vrf.find_bel(die, (scol, row), "CMT"))
            .unwrap(),
        ClkSrc::DspSplitter(scol) => vrf.find_bel(die, (scol, row), "RCLK_SPLITTER").unwrap(),
        ClkSrc::Cmt(scol) => vrf
            .find_bel(die, (scol, row), "RCLK_XIPHY")
            .or_else(|| vrf.find_bel(die, (scol, row), "CMT"))
            .unwrap(),
        ClkSrc::RouteSplitter(_) => unreachable!(),
    }
}

fn find_hroute_src<'a>(
    edev: &ExpandedDevice,
    vrf: &mut Verifier<'a>,
    die: DieId,
    col: ColId,
    row: RowId,
    side: ColSide,
) -> BelContext<'a> {
    let src = edev.hroute_src[col][side];
    match src {
        ClkSrc::Gt(scol) => vrf
            .find_bel(die, (scol, row), "RCLK_GT_R")
            .or_else(|| vrf.find_bel(die, (scol, row), "CMT"))
            .unwrap(),
        ClkSrc::DspSplitter(scol) => vrf.find_bel(die, (scol, row), "RCLK_SPLITTER").unwrap(),
        ClkSrc::Cmt(scol) => vrf.find_bel(die, (scol, row), "CMT").unwrap(),
        ClkSrc::RouteSplitter(scol) => vrf
            .find_bel(die, (scol, row), "RCLK_HROUTE_SPLITTER")
            .or_else(|| vrf.find_bel(die, (scol, row), "RCLK_HDIO"))
            .unwrap(),
    }
}

fn verify_rclk_int(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel = find_hdistr_src(edev, vrf, bel.die, bel.col, bel.row, ColSide::Left);
    for i in 0..24 {
        vrf.verify_node(&[
            bel.fwire(&format!("HDISTR{i}")),
            obel.fwire(&format!("HDISTR{i}_L")),
        ]);
    }
}

fn verify_rclk_splitter(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
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
    let obel_hd = find_hdistr_src(edev, vrf, bel.die, bel.col, bel.row, ColSide::Right);
    let obel_hr = find_hroute_src(edev, vrf, bel.die, bel.col, bel.row, ColSide::Right);
    for i in 0..24 {
        vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_L"))]);
        vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_L"))]);
        vrf.verify_node(&[
            bel.fwire(&format!("HDISTR{i}_R")),
            obel_hd.fwire(&format!("HDISTR{i}_L")),
        ]);
        vrf.verify_node(&[
            bel.fwire(&format!("HROUTE{i}_R")),
            obel_hr.fwire(&format!("HROUTE{i}_L")),
        ]);
    }
}

fn verify_rclk_hroute_splitter(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
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
    let side = if bel.node_kind == "RCLK_HROUTE_SPLITTER_L" {
        ColSide::Left
    } else {
        ColSide::Right
    };
    let obel_hr = find_hroute_src(edev, vrf, bel.die, bel.col, bel.row, side);
    for i in 0..24 {
        vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_L"))]);
        vrf.verify_node(&[
            bel.fwire(&format!("HROUTE{i}_R")),
            obel_hr.fwire(&format!("HROUTE{i}_L")),
        ]);
    }
}

fn verify_bufce_row(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let grid = edev.grids[bel.die];
    let pins = vec![
        ("CLK_IN", SitePinDir::In),
        ("CLK_OUT", SitePinDir::Out),
        ("CLK_OUT_OPT_DLY", SitePinDir::Out),
    ];
    let kind = if edev.kind == GridKind::UltrascalePlus {
        "BUFCE_ROW_FSR"
    } else {
        "BUFCE_ROW"
    };
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let is_l = bel.key.starts_with("BUFCE_ROW_L");
    let idx: usize = bel.key[11..].parse().unwrap();
    let hidx = if is_l {
        grid.columns[bel.col].clk_l[idx]
    } else {
        grid.columns[bel.col].clk_r[idx]
    };

    let obel_gtb = vrf.find_bel_sibling(bel, &format!("GCLK_TEST_BUF_{k}", k = &bel.key[10..]));
    if let Some(hidx) = hidx {
        let obel_hd = find_hdistr_src(edev, vrf, bel.die, bel.col, bel.row, ColSide::Left);
        let obel_hr = find_hroute_src(edev, vrf, bel.die, bel.col, bel.row, ColSide::Left);
        vrf.verify_node(&[
            obel_gtb.fwire_far("CLK_IN"),
            obel_hd.fwire(&format!("HDISTR{hidx}_L")),
        ]);
        vrf.verify_node(&[
            bel.fwire("HROUTE"),
            obel_hr.fwire(&format!("HROUTE{hidx}_L")),
        ]);
    } else {
        vrf.claim_node(&[obel_gtb.fwire_far("CLK_IN")]);
        vrf.claim_node(&[bel.fwire("HROUTE")]);
    }

    vrf.claim_node(&[bel.fwire("VROUTE_T")]);
    vrf.claim_node(&[bel.fwire("VDISTR_T")]);
    let obel_s = vrf.find_bel_delta(bel, 0, -60, bel.key).or_else(|| {
        if bel.die.to_idx() == 0 || bel.row.to_idx() != 30 || hidx.is_none() {
            return None;
        }
        let odie = bel.die - 1;
        let ogrid = edev.grids[odie];
        vrf.find_bel(
            odie,
            (
                bel.col,
                ogrid.row_reg_rclk(ogrid.regs().next_back().unwrap()),
            ),
            bel.key,
        )
    });
    if let Some(obel) = obel_s {
        vrf.verify_node(&[bel.fwire("VROUTE_B"), obel.fwire("VROUTE_T")]);
        vrf.verify_node(&[bel.fwire("VDISTR_B"), obel.fwire("VDISTR_T")]);
    } else {
        vrf.claim_node(&[bel.fwire("VROUTE_B")]);
        vrf.claim_node(&[bel.fwire("VDISTR_B")]);
    }

    let okey_vcc = if is_l { "VCC.RCLK_V_L" } else { "VCC.RCLK_V_R" };
    let obel_vcc = vrf.find_bel_sibling(bel, okey_vcc);
    vrf.claim_pip(bel.crd(), bel.wire("VDISTR_B"), bel.wire("VDISTR_B_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("VDISTR_B"), obel_vcc.wire("VCC"));
    vrf.claim_pip(bel.crd(), bel.wire("VDISTR_T"), bel.wire("VDISTR_T_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("VDISTR_T"), obel_vcc.wire("VCC"));
    vrf.claim_pip(bel.crd(), bel.wire("VROUTE_B"), bel.wire("VROUTE_B_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("VROUTE_B"), obel_vcc.wire("VCC"));
    vrf.claim_pip(bel.crd(), bel.wire("VROUTE_T"), bel.wire("VROUTE_T_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("VROUTE_T"), obel_vcc.wire("VCC"));
    vrf.claim_pip(bel.crd(), bel.wire("HROUTE"), bel.wire("HROUTE_MUX"));
    vrf.claim_pip(bel.crd(), bel.wire("HROUTE"), obel_vcc.wire("VCC"));

    vrf.claim_node(&[bel.fwire("HROUTE_MUX")]);
    vrf.claim_node(&[bel.fwire("VROUTE_B_MUX")]);
    vrf.claim_node(&[bel.fwire("VROUTE_T_MUX")]);
    vrf.claim_node(&[bel.fwire("VDISTR_B_MUX")]);
    vrf.claim_node(&[bel.fwire("VDISTR_T_MUX")]);

    if edev.kind == GridKind::Ultrascale {
        vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire("VDISTR_B"));
        vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire("VDISTR_T"));

        vrf.claim_pip(bel.crd(), bel.wire("HROUTE_MUX"), bel.wire("VROUTE_B"));
        vrf.claim_pip(bel.crd(), bel.wire("HROUTE_MUX"), bel.wire("VROUTE_T"));

        vrf.claim_pip(bel.crd(), bel.wire("VDISTR_B_MUX"), bel.wire("VDISTR_T"));
        vrf.claim_pip(bel.crd(), bel.wire("VDISTR_B_MUX"), bel.wire("VROUTE_T"));
        vrf.claim_pip(bel.crd(), bel.wire("VDISTR_B_MUX"), bel.wire("HROUTE"));
        vrf.claim_pip(bel.crd(), bel.wire("VDISTR_T_MUX"), bel.wire("VDISTR_B"));
        vrf.claim_pip(bel.crd(), bel.wire("VDISTR_T_MUX"), bel.wire("VROUTE_B"));
        vrf.claim_pip(bel.crd(), bel.wire("VDISTR_T_MUX"), bel.wire("HROUTE"));

        vrf.claim_pip(bel.crd(), bel.wire("VROUTE_B_MUX"), bel.wire("VROUTE_T"));
        vrf.claim_pip(bel.crd(), bel.wire("VROUTE_B_MUX"), bel.wire("HROUTE"));
        vrf.claim_pip(
            bel.crd(),
            bel.wire("VROUTE_B_MUX"),
            obel_gtb.wire("CLK_OUT"),
        );
        vrf.claim_pip(bel.crd(), bel.wire("VROUTE_T_MUX"), bel.wire("VROUTE_B"));
        vrf.claim_pip(bel.crd(), bel.wire("VROUTE_T_MUX"), bel.wire("HROUTE"));
        vrf.claim_pip(
            bel.crd(),
            bel.wire("VROUTE_T_MUX"),
            obel_gtb.wire("CLK_OUT"),
        );
    } else {
        vrf.claim_node(&[bel.fwire("VDISTR_B_BUF")]);
        vrf.claim_node(&[bel.fwire("VDISTR_T_BUF")]);
        vrf.claim_node(&[bel.fwire("VROUTE_B_BUF")]);
        vrf.claim_node(&[bel.fwire("VROUTE_T_BUF")]);
        vrf.claim_pip(bel.crd(), bel.wire("VDISTR_B_BUF"), bel.wire("VDISTR_B"));
        vrf.claim_pip(bel.crd(), bel.wire("VDISTR_T_BUF"), bel.wire("VDISTR_T"));
        vrf.claim_pip(bel.crd(), bel.wire("VROUTE_B_BUF"), bel.wire("VROUTE_B"));
        vrf.claim_pip(bel.crd(), bel.wire("VROUTE_T_BUF"), bel.wire("VROUTE_T"));

        vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire("VDISTR_B_BUF"));
        vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire("VDISTR_T_BUF"));

        vrf.claim_pip(bel.crd(), bel.wire("HROUTE_MUX"), bel.wire("VROUTE_B_BUF"));
        vrf.claim_pip(bel.crd(), bel.wire("HROUTE_MUX"), bel.wire("VROUTE_T_BUF"));

        vrf.claim_pip(
            bel.crd(),
            bel.wire("VDISTR_B_MUX"),
            bel.wire("VDISTR_T_BUF"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("VDISTR_B_MUX"),
            bel.wire("VROUTE_T_BUF"),
        );
        vrf.claim_pip(bel.crd(), bel.wire("VDISTR_B_MUX"), bel.wire("HROUTE"));
        vrf.claim_pip(
            bel.crd(),
            bel.wire("VDISTR_T_MUX"),
            bel.wire("VDISTR_B_BUF"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("VDISTR_T_MUX"),
            bel.wire("VROUTE_B_BUF"),
        );
        vrf.claim_pip(bel.crd(), bel.wire("VDISTR_T_MUX"), bel.wire("HROUTE"));

        vrf.claim_pip(
            bel.crd(),
            bel.wire("VROUTE_B_MUX"),
            bel.wire("VROUTE_T_BUF"),
        );
        vrf.claim_pip(bel.crd(), bel.wire("VROUTE_B_MUX"), bel.wire("HROUTE"));
        vrf.claim_pip(
            bel.crd(),
            bel.wire("VROUTE_B_MUX"),
            obel_gtb.wire("CLK_OUT"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("VROUTE_T_MUX"),
            bel.wire("VROUTE_B_BUF"),
        );
        vrf.claim_pip(bel.crd(), bel.wire("VROUTE_T_MUX"), bel.wire("HROUTE"));
        vrf.claim_pip(
            bel.crd(),
            bel.wire("VROUTE_T_MUX"),
            obel_gtb.wire("CLK_OUT"),
        );
    }

    vrf.claim_pip(bel.crd(), obel_gtb.wire_far("CLK_IN"), obel_vcc.wire("VCC"));
    vrf.claim_pip(bel.crd(), obel_gtb.wire_far("CLK_IN"), bel.wire("CLK_OUT"));
}

fn verify_gclk_test_buf(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = vec![("CLK_IN", SitePinDir::In), ("CLK_OUT", SitePinDir::Out)];
    vrf.verify_bel(bel, "GCLK_TEST_BUFE3", &pins, &[]);
    if !bel.naming.pins["CLK_IN"].pips.is_empty() {
        vrf.claim_node(&[bel.fwire("CLK_IN")]);
        vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire_far("CLK_IN"));
    }
    vrf.claim_node(&[bel.fwire("CLK_OUT")]);
    // other stuff dealt with in BUFCE_ROW
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

    vrf.claim_node(&[bel.fwire("CLK_IN_DUMMY")]);
}

fn verify_rclk_ps(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
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
    let obel_hr = find_hroute_src(edev, vrf, bel.die, bel.col, bel.row, ColSide::Left);
    for i in 0..24 {
        vrf.verify_node(&[
            bel.fwire(&format!("HROUTE{i}")),
            obel_hr.fwire(&format!("HROUTE{i}_L")),
        ]);
    }
}

fn verify_hdiob(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx: usize = bel.key[7..].parse().unwrap();
    let is_b = bel.row.to_idx() % 60 == 0;
    let is_m = bel.key.starts_with("HDIOB_M");
    let hid = HdioIobId::from_idx(2 * idx + if is_b { 0 } else { 12 } + if is_m { 0 } else { 1 });
    let grid = edev.grids[bel.die];
    let reg = grid.row_to_reg(bel.row);
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

fn verify_hdiodiffin(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx: usize = bel.key[10..].parse().unwrap();
    let is_b = bel.row.to_idx() % 60 == 0;
    let hid = HdioIobId::from_idx(2 * idx + if is_b { 0 } else { 12 });
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

fn verify_rclk_hdio(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
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

    let obel_hd = find_hdistr_src(edev, vrf, bel.die, bel.col, bel.row, ColSide::Left);
    let obel_hr = find_hroute_src(edev, vrf, bel.die, bel.col, bel.row, ColSide::Left);
    for i in 0..24 {
        vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_L"))]);
        vrf.verify_node(&[
            bel.fwire(&format!("HDISTR{i}")),
            obel_hd.fwire(&format!("HDISTR{i}_L")),
        ]);
        vrf.verify_node(&[
            bel.fwire(&format!("HROUTE{i}_R")),
            obel_hr.fwire(&format!("HROUTE{i}_L")),
        ]);
    }
}

fn verify_bufce_row_io(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = vec![
        ("CLK_IN", SitePinDir::In),
        ("CLK_OUT", SitePinDir::Out),
        ("CLK_OUT_OPT_DLY", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "BUFCE_ROW", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let idx: usize = bel.key[12..].parse().unwrap();
    let obel_cmt = vrf.find_bel_sibling(bel, "CMT");
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_IN"),
        obel_cmt.wire(&format!("VDISTR{idx}_B")),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_IN"),
        obel_cmt.wire(&format!("VDISTR{idx}_T")),
    );
}

fn verify_bufgce(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let is_l = bel.node_kind != "CMT_R";
    let hr_lr = if is_l { 'L' } else { 'R' };
    let pins = vec![("CLK_IN", SitePinDir::In), ("CLK_OUT", SitePinDir::Out)];
    vrf.verify_bel(bel, "BUFGCE", &pins, &["CLK_IN_CKINT"]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    if !bel.naming.pins["CLK_IN"].pips.is_empty() {
        vrf.claim_node(&[bel.fwire_far("CLK_IN")]);
        vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire_far("CLK_IN"));
    }

    let idx: usize = bel.key[6..].parse().unwrap();
    let obel_mmcm = vrf.find_bel_sibling(bel, "MMCM");
    let obel_pll0 = vrf.find_bel_sibling(bel, "PLL0");
    let obel_pll1 = vrf.find_bel_sibling(bel, "PLL1");
    let obel_cmt = vrf.find_bel_sibling(bel, "CMT");
    let obel_vcc = vrf.find_bel_sibling(bel, "VCC.CMT");
    let obel_gtb = vrf.find_bel_sibling(bel, &format!("GCLK_TEST_BUF_IO{idx}"));
    for pin in [
        "CLKOUT0",
        "CLKOUT0B",
        "CLKOUT1",
        "CLKOUT1B",
        "CLKOUT2",
        "CLKOUT2B",
        "CLKOUT3",
        "CLKOUT3B",
        "CLKOUT4",
        "CLKOUT5",
        "CLKOUT6",
        "CLKFBOUT",
        "CLKFBOUTB",
    ] {
        vrf.claim_pip(bel.crd(), bel.wire_far("CLK_IN"), obel_mmcm.wire(pin));
    }
    for pin in ["CLKOUT0", "CLKOUT0B", "CLKOUT1", "CLKOUT1B"] {
        vrf.claim_pip(bel.crd(), bel.wire_far("CLK_IN"), obel_pll0.wire(pin));
        vrf.claim_pip(bel.crd(), bel.wire_far("CLK_IN"), obel_pll1.wire(pin));
    }
    vrf.claim_pip(bel.crd(), bel.wire_far("CLK_IN"), obel_vcc.wire("VCC"));
    vrf.claim_pip(
        bel.crd(),
        bel.wire_far("CLK_IN"),
        bel.wire_far("CLK_IN_MUX_HROUTE"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire_far("CLK_IN"),
        bel.wire_far("CLK_IN_MUX_PLL_CKINT"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire_far("CLK_IN"),
        bel.wire_far("CLK_IN_MUX_TEST"),
    );
    for i in 0..4 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire_far("CLK_IN"),
            obel_cmt.wire(&format!("CCIO{i}")),
        );
    }
    if edev.kind == GridKind::Ultrascale {
        for i in 0..8 {
            let ii = [0, 6, 13, 19, 26, 32, 39, 45][i];
            let obel = vrf.find_bel_sibling(bel, &format!("BITSLICE_RX_TX{ii}"));
            vrf.claim_pip(
                bel.crd(),
                bel.wire_far("CLK_IN"),
                obel.wire("PHY2CLB_FIFO_WRCLK"),
            );
        }
    } else {
        for i in 0..8 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire_far("CLK_IN"),
                obel_cmt.wire(&format!("FIFO_WRCLK{i}")),
            );
        }
    }
    vrf.claim_node(&[bel.fwire("CLK_IN_MUX_HROUTE")]);
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_IN_MUX_HROUTE"),
        obel_vcc.wire("VCC"),
    );
    for i in 0..24 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLK_IN_MUX_HROUTE"),
            obel_cmt.wire(&format!("HROUTE{i}_{hr_lr}")),
        );
    }
    vrf.claim_node(&[bel.fwire("CLK_IN_MUX_PLL_CKINT")]);
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_IN_MUX_PLL_CKINT"),
        bel.wire("CLK_IN_CKINT"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_IN_MUX_PLL_CKINT"),
        obel_pll0.wire("CLKFBOUT"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_IN_MUX_PLL_CKINT"),
        obel_pll1.wire("CLKFBOUT"),
    );
    vrf.claim_node(&[bel.fwire("CLK_IN_MUX_TEST")]);
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_IN_MUX_TEST"),
        obel_mmcm.wire("TMUXOUT"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_IN_MUX_TEST"),
        obel_pll0.wire("TMUXOUT"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_IN_MUX_TEST"),
        obel_pll1.wire("TMUXOUT"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLK_IN_MUX_TEST"),
        obel_gtb.wire("CLK_OUT"),
    );
}

fn verify_bufgctrl(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = vec![
        ("CLK_I0", SitePinDir::In),
        ("CLK_I1", SitePinDir::In),
        ("CLK_OUT", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "BUFGCTRL", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let idx: usize = bel.key[8..].parse().unwrap();
    let obel0 = vrf.find_bel_sibling(bel, &format!("BUFGCE{ii}", ii = idx * 3));
    let obel1 = vrf.find_bel_sibling(bel, &format!("BUFGCE{ii}", ii = idx * 3 + 1));
    let obel_p = vrf.find_bel_sibling(bel, &format!("BUFGCTRL{ii}", ii = (idx + 7) % 8));
    let obel_n = vrf.find_bel_sibling(bel, &format!("BUFGCTRL{ii}", ii = (idx + 1) % 8));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_I0"), obel0.wire_far("CLK_IN"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_I0"), obel_p.wire_far("CLK_OUT"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_I0"), obel_n.wire_far("CLK_OUT"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_I1"), obel1.wire_far("CLK_IN"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_I1"), obel_p.wire_far("CLK_OUT"));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_I1"), obel_n.wire_far("CLK_OUT"));
}

fn verify_bufgce_div(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = vec![("CLK_IN", SitePinDir::In), ("CLK_OUT", SitePinDir::Out)];
    vrf.verify_bel(bel, "BUFGCE_DIV", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let idx: usize = bel.key[10..].parse().unwrap();
    let obel = vrf.find_bel_sibling(bel, &format!("BUFGCE{ii}", ii = idx * 6 + 5));
    vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire_far("CLK_IN"));
}

fn verify_mmcm(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let is_l = bel.node_kind != "CMT_R";
    let hr_lr = if is_l { 'L' } else { 'R' };
    let kind = match edev.kind {
        GridKind::Ultrascale => "MMCME3_ADV",
        GridKind::UltrascalePlus => "MMCM",
    };
    let pins = vec![
        ("CLKIN1", SitePinDir::In),
        ("CLKIN2", SitePinDir::In),
        ("CLKFBIN", SitePinDir::In),
        ("CLKOUT0", SitePinDir::Out),
        ("CLKOUT0B", SitePinDir::Out),
        ("CLKOUT1", SitePinDir::Out),
        ("CLKOUT1B", SitePinDir::Out),
        ("CLKOUT2", SitePinDir::Out),
        ("CLKOUT2B", SitePinDir::Out),
        ("CLKOUT3", SitePinDir::Out),
        ("CLKOUT3B", SitePinDir::Out),
        ("CLKOUT4", SitePinDir::Out),
        ("CLKOUT5", SitePinDir::Out),
        ("CLKOUT6", SitePinDir::Out),
        ("CLKFBOUT", SitePinDir::Out),
        ("CLKFBOUTB", SitePinDir::Out),
        ("TMUXOUT", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_cmt = vrf.find_bel_sibling(bel, "CMT");
    let obel_vcc = vrf.find_bel_sibling(bel, "VCC.CMT");

    for pin in [
        "CLKIN1_MUX_HDISTR",
        "CLKIN2_MUX_HDISTR",
        "CLKFBIN_MUX_HDISTR",
    ] {
        vrf.claim_node(&[bel.fwire(pin)]);
        for i in 0..24 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(pin),
                obel_cmt.wire(&format!("HDISTR{i}_L")),
            );
        }
    }
    for pin in ["CLKIN1_MUX_HROUTE", "CLKIN2_MUX_HROUTE"] {
        vrf.claim_node(&[bel.fwire(pin)]);
        for i in 0..24 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(pin),
                obel_cmt.wire(&format!("HROUTE{i}_{hr_lr}")),
            );
        }
    }
    for pin in [
        "CLKIN1_MUX_BUFCE_ROW_DLY",
        "CLKIN2_MUX_BUFCE_ROW_DLY",
        "CLKFBIN_MUX_BUFCE_ROW_DLY",
    ] {
        vrf.claim_node(&[bel.fwire(pin)]);
        for i in 0..24 {
            let obel_bufce_row = vrf.find_bel_sibling(bel, &format!("BUFCE_ROW_IO{i}"));
            vrf.claim_pip(
                bel.crd(),
                bel.wire(pin),
                obel_bufce_row.wire("CLK_OUT_OPT_DLY"),
            );
        }
    }

    vrf.claim_node(&[bel.fwire("CLKIN1_MUX_DUMMY0")]);
    vrf.claim_node(&[bel.fwire("CLKIN2_MUX_DUMMY0")]);
    vrf.claim_node(&[bel.fwire("CLKFBIN_MUX_DUMMY0")]);
    vrf.claim_node(&[bel.fwire("CLKFBIN_MUX_DUMMY1")]);

    vrf.claim_pip(bel.crd(), bel.wire("CLKIN1"), bel.wire("CLKIN1_MUX_HDISTR"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN1"), bel.wire("CLKIN1_MUX_HROUTE"));
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKIN1"),
        bel.wire("CLKIN1_MUX_BUFCE_ROW_DLY"),
    );
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN1"), bel.wire("CLKIN1_MUX_DUMMY0"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN2"), bel.wire("CLKIN2_MUX_HDISTR"));
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN2"), bel.wire("CLKIN2_MUX_HROUTE"));
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKIN2"),
        bel.wire("CLKIN2_MUX_BUFCE_ROW_DLY"),
    );
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN2"), bel.wire("CLKIN2_MUX_DUMMY0"));
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKFBIN"),
        bel.wire("CLKFBIN_MUX_HDISTR"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKFBIN"),
        bel.wire("CLKFBIN_MUX_BUFCE_ROW_DLY"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKFBIN"),
        bel.wire("CLKFBIN_MUX_DUMMY0"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKFBIN"),
        bel.wire("CLKFBIN_MUX_DUMMY1"),
    );
    for pin in ["CLKIN1", "CLKIN2", "CLKFBIN"] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel_vcc.wire("VCC"));
        for i in 0..4 {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_cmt.wire(&format!("CCIO{i}")));
        }
        if edev.kind == GridKind::Ultrascale {
            for i in 0..8 {
                let ii = [0, 6, 13, 19, 26, 32, 39, 45][i];
                let obel = vrf.find_bel_sibling(bel, &format!("BITSLICE_RX_TX{ii}"));
                vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire("PHY2CLB_FIFO_WRCLK"));
            }
        } else {
            for i in 0..8 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(pin),
                    obel_cmt.wire(&format!("FIFO_WRCLK{i}")),
                );
            }
        }
    }
    for pin in [
        "CLKIN1_MUX_HDISTR",
        "CLKIN1_MUX_HROUTE",
        "CLKIN1_MUX_BUFCE_ROW_DLY",
        "CLKIN2_MUX_HDISTR",
        "CLKIN2_MUX_HROUTE",
        "CLKIN2_MUX_BUFCE_ROW_DLY",
        "CLKFBIN_MUX_HDISTR",
        "CLKFBIN_MUX_BUFCE_ROW_DLY",
    ] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel_vcc.wire("VCC"));
    }
}

fn verify_pll(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let is_l = bel.node_kind != "CMT_R";
    let hr_lr = if is_l { 'L' } else { 'R' };
    let kind = match edev.kind {
        GridKind::Ultrascale => "PLLE3_ADV",
        GridKind::UltrascalePlus => "PLL",
    };
    let pins = vec![
        ("CLKIN", SitePinDir::In),
        ("CLKFBIN", SitePinDir::In),
        ("CLKOUT0", SitePinDir::Out),
        ("CLKOUT0B", SitePinDir::Out),
        ("CLKOUT1", SitePinDir::Out),
        ("CLKOUT1B", SitePinDir::Out),
        ("CLKFBOUT", SitePinDir::Out),
        ("TMUXOUT", SitePinDir::Out),
        ("CLKOUTPHY_P", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, kind, &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_cmt = vrf.find_bel_sibling(bel, "CMT");
    let obel_mmcm = vrf.find_bel_sibling(bel, "MMCM");
    let obel_vcc = vrf.find_bel_sibling(bel, "VCC.CMT");
    let has_hbm = vrf.find_bel_delta(bel, 0, 0, "HBM_REF_CLK0").is_some();

    for pin in ["CLKIN_MUX_HDISTR", "CLKFBIN_MUX_HDISTR"] {
        if has_hbm && pin == "CLKFBIN_MUX_HDISTR" {
            continue;
        }
        vrf.claim_node(&[bel.fwire(pin)]);
        for i in 0..24 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(pin),
                obel_cmt.wire(&format!("HDISTR{i}_L")),
            );
        }
    }
    vrf.claim_node(&[bel.fwire("CLKIN_MUX_HROUTE")]);
    for i in 0..24 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKIN_MUX_HROUTE"),
            obel_cmt.wire(&format!("HROUTE{i}_{hr_lr}")),
        );
    }
    for pin in ["CLKIN_MUX_BUFCE_ROW_DLY", "CLKFBIN_MUX_BUFCE_ROW_DLY"] {
        if has_hbm && pin == "CLKFBIN_MUX_BUFCE_ROW_DLY" {
            continue;
        }
        vrf.claim_node(&[bel.fwire(pin)]);
        for i in 0..24 {
            let obel_bufce_row = vrf.find_bel_sibling(bel, &format!("BUFCE_ROW_IO{i}"));
            vrf.claim_pip(
                bel.crd(),
                bel.wire(pin),
                obel_bufce_row.wire("CLK_OUT_OPT_DLY"),
            );
        }
    }
    vrf.claim_node(&[bel.fwire("CLKIN_MUX_MMCM")]);
    for i in 0..4 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKIN_MUX_MMCM"),
            obel_mmcm.wire(&format!("CLKOUT{i}")),
        );
    }

    for pin in [
        "CLKIN_MUX_HDISTR",
        "CLKIN_MUX_HROUTE",
        "CLKIN_MUX_BUFCE_ROW_DLY",
        "CLKIN_MUX_MMCM",
    ] {
        vrf.claim_pip(bel.crd(), bel.wire("CLKIN"), bel.wire(pin));
    }
    vrf.claim_pip(bel.crd(), bel.wire("CLKIN"), obel_vcc.wire("VCC"));
    for i in 0..4 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKIN"),
            obel_cmt.wire(&format!("CCIO{i}")),
        );
    }
    if edev.kind == GridKind::Ultrascale {
        for i in 0..8 {
            let ii = [0, 6, 13, 19, 26, 32, 39, 45][i];
            let obel = vrf.find_bel_sibling(bel, &format!("BITSLICE_RX_TX{ii}"));
            vrf.claim_pip(
                bel.crd(),
                bel.wire("CLKIN"),
                obel.wire("PHY2CLB_FIFO_WRCLK"),
            );
        }
    } else {
        for i in 0..8 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire("CLKIN"),
                obel_cmt.wire(&format!("FIFO_WRCLK{i}")),
            );
        }
    }

    for pin in [
        "CLKIN_MUX_HDISTR",
        "CLKIN_MUX_HROUTE",
        "CLKIN_MUX_BUFCE_ROW_DLY",
    ] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel_vcc.wire("VCC"));
    }
    if has_hbm {
        vrf.claim_node(&[bel.fwire_far("CLKFBIN")]);
        vrf.claim_pip(bel.crd(), bel.wire("CLKFBIN"), bel.wire_far("CLKFBIN"));
    } else {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKFBIN"),
            bel.wire("CLKFBIN_MUX_HDISTR"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLKFBIN"),
            bel.wire("CLKFBIN_MUX_BUFCE_ROW_DLY"),
        );
        for pin in ["CLKFBIN_MUX_HDISTR", "CLKFBIN_MUX_BUFCE_ROW_DLY"] {
            vrf.claim_pip(bel.crd(), bel.wire(pin), obel_vcc.wire("VCC"));
        }
    }
}

fn verify_hbm_ref_clk(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = vec![("REF_CLK", SitePinDir::In)];
    vrf.verify_bel(bel, "HBM_REF_CLK", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_cmt = vrf.find_bel_sibling(bel, "CMT");

    vrf.claim_node(&[bel.fwire("REF_CLK_MUX_HDISTR")]);
    for i in 0..24 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire("REF_CLK_MUX_HDISTR"),
            obel_cmt.wire(&format!("HDISTR{i}_L")),
        );
    }
    vrf.claim_node(&[bel.fwire("REF_CLK_MUX_BUFCE_ROW_DLY")]);
    for i in 0..24 {
        let obel_bufce_row = vrf.find_bel_sibling(bel, &format!("BUFCE_ROW_IO{i}"));
        vrf.claim_pip(
            bel.crd(),
            bel.wire("REF_CLK_MUX_BUFCE_ROW_DLY"),
            obel_bufce_row.wire("CLK_OUT_OPT_DLY"),
        );
    }
    vrf.claim_pip(
        bel.crd(),
        bel.wire("REF_CLK"),
        bel.wire("REF_CLK_MUX_HDISTR"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("REF_CLK"),
        bel.wire("REF_CLK_MUX_BUFCE_ROW_DLY"),
    );
}

fn verify_cmt(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let is_l = bel.node_kind != "CMT_R";
    let obel_vcc = vrf.find_bel_sibling(bel, "VCC.CMT");

    let obel_s = vrf.find_bel_delta(bel, 0, -60, "CMT");

    for i in 0..24 {
        vrf.claim_node(&[bel.fwire(&format!("VDISTR{i}_T"))]);
        if let Some(ref obel_s) = obel_s {
            vrf.verify_node(&[
                bel.fwire(&format!("VDISTR{i}_B")),
                obel_s.fwire(&format!("VDISTR{i}_T")),
            ]);
        } else {
            vrf.claim_node(&[bel.fwire(&format!("VDISTR{i}_B"))]);
        }
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("VDISTR{i}_B")),
            bel.wire(&format!("VDISTR{i}_B_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("VDISTR{i}_B")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("VDISTR{i}_T")),
            bel.wire(&format!("VDISTR{i}_T_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("VDISTR{i}_T")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_node(&[bel.fwire(&format!("VDISTR{i}_B_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("VDISTR{i}_B_MUX")),
            bel.wire(&format!("VDISTR{i}_T")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("VDISTR{i}_B_MUX")),
            bel.wire(&format!("OUT_MUX{i}")),
        );
        vrf.claim_node(&[bel.fwire(&format!("VDISTR{i}_T_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("VDISTR{i}_T_MUX")),
            bel.wire(&format!("VDISTR{i}_B")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("VDISTR{i}_T_MUX")),
            bel.wire(&format!("OUT_MUX{i}")),
        );
    }

    let obel_hr = find_hroute_src(edev, vrf, bel.die, bel.col, bel.row, ColSide::Right);
    for i in 0..24 {
        vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_L"))]);
        if is_l {
            vrf.verify_node(&[
                bel.fwire(&format!("HROUTE{i}_R")),
                obel_hr.fwire(&format!("HROUTE{i}_L")),
            ]);
        } else {
            vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_R"))]);
        }
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
            bel.wire(&format!("HROUTE{i}_R")),
            bel.wire(&format!("HROUTE{i}_R_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_L_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L_MUX")),
            bel.wire(&format!("HROUTE{i}_R")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_L_MUX")),
            bel.wire(&format!("OUT_MUX{i}")),
        );
        vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_R_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R_MUX")),
            bel.wire(&format!("HROUTE{i}_L")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HROUTE{i}_R_MUX")),
            bel.wire(&format!("OUT_MUX{i}")),
        );
    }

    if edev.kind == GridKind::Ultrascale {
        let obel_hd = find_hdistr_src(edev, vrf, bel.die, bel.col, bel.row, ColSide::Right);
        for i in 0..24 {
            vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_L"))]);
            vrf.verify_node(&[
                bel.fwire(&format!("HDISTR{i}_R")),
                obel_hd.fwire(&format!("HDISTR{i}_L")),
            ]);
        }
    } else {
        let obel_hd = find_hdistr_src(edev, vrf, bel.die, bel.col, bel.row, ColSide::Right);
        for i in 0..24 {
            vrf.verify_node(&[
                bel.fwire(&format!("HDISTR{i}_L")),
                obel_hd.fwire(&format!("HDISTR{i}_L")),
            ]);
        }
        // R is a lie and goes nowhere.
        for i in 0..24 {
            vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_R"))]);
        }
    }
    for i in 0..24 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_L")),
            bel.wire(&format!("HDISTR{i}_L_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_L")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_R")),
            bel.wire(&format!("HDISTR{i}_R_MUX")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_R")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_L_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_L_MUX")),
            bel.wire(&format!("HDISTR{i}_R")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_L_MUX")),
            bel.wire(&format!("HDISTR{i}_OUT_MUX")),
        );
        vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_R_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_R_MUX")),
            bel.wire(&format!("HDISTR{i}_L")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_R_MUX")),
            bel.wire(&format!("HDISTR{i}_OUT_MUX")),
        );
        vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_OUT_MUX"))]);
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_OUT_MUX")),
            bel.wire(&format!("OUT_MUX{i}")),
        );
        let obel = vrf.find_bel_sibling(bel, &format!("BUFCE_ROW_IO{i}"));
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_OUT_MUX")),
            obel.wire("CLK_OUT"),
        );
    }

    for i in 0..24 {
        let pin = format!("OUT_MUX{i}");
        vrf.claim_node(&[bel.fwire(&pin)]);
        vrf.claim_pip(bel.crd(), bel.wire(&pin), obel_vcc.wire("VCC"));
        for j in 0..3 {
            vrf.claim_node(&[bel.fwire(&format!("OUT_MUX{i}_DUMMY{j}"))]);
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&pin),
                bel.wire(&format!("OUT_MUX{i}_DUMMY{j}")),
            );
        }
        let obel = vrf.find_bel_sibling(bel, &format!("BUFGCE{i}"));
        vrf.claim_pip(bel.crd(), bel.wire(&pin), obel.wire("CLK_OUT"));
        for j in 0..8 {
            let obel = vrf.find_bel_sibling(bel, &format!("BUFGCTRL{j}"));
            vrf.claim_pip(bel.crd(), bel.wire(&pin), obel.wire("CLK_OUT"));
        }
        for j in 0..4 {
            let obel = vrf.find_bel_sibling(bel, &format!("BUFGCE_DIV{j}"));
            vrf.claim_pip(bel.crd(), bel.wire(&pin), obel.wire("CLK_OUT"));
        }
    }

    for i in 0..24 {
        let obel = vrf.find_bel_sibling(bel, &format!("GCLK_TEST_BUF_IO{i}"));
        vrf.claim_node(&[obel.fwire("CLK_IN")]);
        vrf.claim_pip(bel.crd(), obel.wire("CLK_IN"), obel_vcc.wire("VCC"));
        for j in 0..24 {
            vrf.claim_pip(
                bel.crd(),
                obel.wire("CLK_IN"),
                bel.wire(&format!("HDISTR{j}_L")),
            );
        }
    }

    for (i, dy, hpio_key, hrio_key) in [
        (0, 0, "HPIOB0", "HRIOB0"),
        (1, 0, "HPIOB2", "HRIOB2"),
        (2, -30, "HPIOB21", "HRIOB21"),
        (3, -30, "HPIOB23", "HRIOB23"),
    ] {
        let obel = vrf
            .find_bel_delta(bel, 0, dy, hpio_key)
            .or_else(|| vrf.find_bel_delta(bel, 0, dy, hrio_key))
            .unwrap();
        vrf.verify_node(&[
            bel.fwire(&format!("CCIO{i}")),
            obel.fwire(if obel.key.starts_with("HRIO") {
                "DOUT"
            } else {
                "I"
            }),
        ]);
    }

    if edev.kind == GridKind::UltrascalePlus {
        for i in 0..8 {
            let ii = match i % 2 {
                0 => 0,
                1 => 6,
                _ => unreachable!(),
            };
            let obel = vrf
                .find_bel_delta(bel, 0, -30 + 15 * (i / 2), &format!("BITSLICE_RX_TX{ii}"))
                .unwrap();
            vrf.verify_node(&[
                bel.fwire(&format!("FIFO_WRCLK{i}")),
                obel.fwire("PHY2CLB_FIFO_WRCLK"),
            ]);
        }
    }

    if edev.kind == GridKind::Ultrascale {
        for i in 0..6 {
            for bt in ['B', 'T'] {
                let pin = format!("XIPHY_CLK{i}_{bt}");
                vrf.claim_node(&[bel.fwire(&pin)]);
                vrf.claim_pip(bel.crd(), bel.wire(&pin), obel_vcc.wire("VCC"));
                for j in 0..24 {
                    vrf.claim_pip(bel.crd(), bel.wire(&pin), bel.wire(&format!("HDISTR{j}_L")));
                }
            }
        }
    }
}

fn verify_bitslice_rx_tx(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx: usize = bel.key[14..].parse().unwrap();
    let bidx = idx / 13;
    let bsidx = idx % 13;
    let nidx = usize::from(bsidx >= 6);
    let sidx = bsidx - nidx * 6;
    let obel_bsctl =
        vrf.find_bel_sibling(bel, &format!("BITSLICE_CONTROL{ii}", ii = bidx * 2 + nidx));
    let obel_feed = vrf.find_bel_sibling(bel, &format!("XIPHY_FEEDTHROUGH{bidx}"));
    let obel_bstx = vrf.find_bel_sibling(bel, &format!("BITSLICE_TX{ii}", ii = bidx * 2 + nidx));
    let mut pins = vec![
        // mux
        ("RX_CLK", SitePinDir::In),
        ("RX_CLK_C", SitePinDir::In),
        ("RX_CLK_C_B", SitePinDir::In),
        ("TX_OCLK", SitePinDir::In),
        ("RX_CLKDIV", SitePinDir::In),  // alias of RX_CLK
        ("TX_CLK", SitePinDir::In),     // alias of RX_CLK
        ("TX_OCLKDIV", SitePinDir::In), // alias of RX_CLK
        // to IOB
        ("TX_Q", SitePinDir::Out),
        ("RX_D", SitePinDir::In),
        // to BSCTL
        ("BS2CTL_TX_DDR_PHASE_SEL", SitePinDir::Out),
        ("TX_VTC_READY", SitePinDir::Out),
        ("RX_VTC_READY", SitePinDir::Out),
        ("BS2CTL_IDELAY_DELAY_FORMAT", SitePinDir::Out),
        ("BS2CTL_ODELAY_DELAY_FORMAT", SitePinDir::Out),
        ("RX_DQS_OUT", SitePinDir::Out),
        ("BS2CTL_RX_DDR_EN_DQS", SitePinDir::Out),
        ("BS2CTL_RX_P0_DQ_OUT", SitePinDir::Out),
        ("BS2CTL_RX_N0_DQ_OUT", SitePinDir::Out),
        // to RCLK
        ("PHY2CLB_FIFO_WRCLK", SitePinDir::Out),
        // cascade stuff
        ("RX2TX_CASC_RETURN_IN", SitePinDir::In),
        ("TX2RX_CASC_IN", SitePinDir::In),
        ("TX2RX_CASC_OUT", SitePinDir::Out),
    ];

    for (pin, opin) in [
        ("TX_CTRL_CLK", format!("ODELAY_CTRL_CLK{sidx}")),
        ("TX_CTRL_CE", format!("ODELAY_CE_OUT{sidx}")),
        ("TX_CTRL_INC", format!("ODELAY_INC_OUT{sidx}")),
        ("TX_CTRL_LD", format!("ODELAY_LD_OUT{sidx}")),
        ("TX_DIV2_CLK", format!("DIV2_CLK_OUT{sidx}")),
        ("TX_DIV4_CLK", format!("DIV_CLK_OUT{sidx}")),
        ("TX_DDR_CLK", format!("DDR_CLK_OUT{sidx}")),
        ("CTL2BS_DYNAMIC_MODE_EN", format!("DYNAMIC_MODE_EN{sidx}")),
        ("CTL2BS_TX_DDR_PHASE_SEL", format!("TX_DATA_PHASE{sidx}")),
        ("TX_TOGGLE_DIV2_SEL", format!("TOGGLE_DIV2_SEL{sidx}")),
        ("TX_MUX_360_P_SEL", format!("PH02_DIV2_360_{sidx}")),
        ("TX_MUX_360_N_SEL", format!("PH13_DIV2_360_{sidx}")),
        ("TX_MUX_720_P0_SEL", format!("PH0_DIV_720_{sidx}")),
        ("TX_MUX_720_P1_SEL", format!("PH1_DIV_720_{sidx}")),
        ("TX_MUX_720_P2_SEL", format!("PH2_DIV_720_{sidx}")),
        ("TX_MUX_720_P3_SEL", format!("PH3_DIV_720_{sidx}")),
        ("TX_WL_TRAIN", format!("WL_TRAIN{sidx}")),
        ("TX_BS_RESET", format!("TX_BS_RESET{sidx}")),
        ("RX_CLK_P", format!("PDQS_OUT{sidx}")),
        ("RX_CLK_N", format!("NDQS_OUT{sidx}")),
        ("RX_CTRL_CLK", format!("IDELAY_CTRL_CLK{sidx}")),
        ("RX_CTRL_CE", format!("IDELAY_CE_OUT{sidx}")),
        ("RX_CTRL_INC", format!("IDELAY_INC_OUT{sidx}")),
        ("RX_CTRL_LD", format!("IDELAY_LD_OUT{sidx}")),
        ("RX_DCC0", format!("RX_DCC{sidx:02}_0")),
        ("RX_DCC1", format!("RX_DCC{sidx:02}_1")),
        ("RX_DCC2", format!("RX_DCC{sidx:02}_2")),
        ("RX_DCC3", format!("RX_DCC{sidx:02}_3")),
        ("RX_BS_RESET", format!("RX_BS_RESET{sidx}")),
        ("CTL2BS_FIFO_BYPASS", format!("IFIFO_BYPASS{sidx}")),
    ] {
        pins.push((pin, SitePinDir::In));
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel_bsctl.wire(&opin));
    }

    let ul = if nidx == 1 { "UPP" } else { "LOW" };
    let mut pins_feed = vec![
        (
            "CTL2BS_RX_RECALIBRATE_EN",
            format!("CTL2BS_REFCLK_EN_{ul}_SMX{sidx}"),
        ),
        ("CLB2PHY_FIFO_CLK", format!("CLB2PHY_FIFO_CLK_SMX{bsidx}")),
    ];

    if edev.kind == GridKind::Ultrascale {
        pins_feed.extend([
            ("RX_RESET_B", format!("CLB2PHY_IDELAY_RST_B_SMX{bsidx}")),
            ("TX_REGRST_B", format!("CLB2PHY_ODELAY_RST_B_SMX{bsidx}")),
            ("RX_RST_B", format!("CLB2PHY_RXBIT_RST_B_SMX{bsidx}")),
            ("TX_RST_B", format!("CLB2PHY_TXBIT_RST_B_SMX{bsidx}")),
        ]);
    } else {
        pins_feed.extend([
            ("RX_RESET", format!("CLB2PHY_IDELAY_RST_SMX{bsidx}")),
            ("TX_REGRST", format!("CLB2PHY_ODELAY_RST_SMX{bsidx}")),
            ("RX_RST", format!("CLB2PHY_RXBIT_RST_SMX{bsidx}")),
            ("TX_RST", format!("CLB2PHY_TXBIT_RST_SMX{bsidx}")),
        ]);
    }

    for (pin, opin) in pins_feed {
        pins.push((pin, SitePinDir::In));
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel_feed.wire(&opin));
    }

    let rx_ctrl_dly: [_; 9] = core::array::from_fn(|i| format!("RX_CTRL_DLY{i}"));
    let tx_ctrl_dly: [_; 9] = core::array::from_fn(|i| format!("TX_CTRL_DLY{i}"));
    for (i, pin) in rx_ctrl_dly.iter().enumerate() {
        pins.push((pin, SitePinDir::In));
        vrf.claim_pip(
            bel.crd(),
            bel.wire(pin),
            obel_bsctl.wire(&format!("IDELAY{sidx:02}_OUT{i}")),
        );
    }
    for (i, pin) in tx_ctrl_dly.iter().enumerate() {
        pins.push((pin, SitePinDir::In));
        vrf.claim_pip(
            bel.crd(),
            bel.wire(pin),
            obel_bsctl.wire(&format!("ODELAY{sidx:02}_OUT{i}")),
        );
    }
    // to BSCTL
    let rx_cntvalueout: [_; 9] = core::array::from_fn(|i| format!("BS2CTL_RX_CNTVALUEOUT{i}"));
    for pin in &rx_cntvalueout {
        pins.push((pin, SitePinDir::Out));
    }
    let tx_cntvalueout: [_; 9] = core::array::from_fn(|i| format!("BS2CTL_TX_CNTVALUEOUT{i}"));
    for pin in &tx_cntvalueout {
        pins.push((pin, SitePinDir::Out));
    }
    let idelay_fixed_dly_ratio: [_; 18] =
        core::array::from_fn(|i| format!("BS2CTL_IDELAY_FIXED_DLY_RATIO{i}"));
    for pin in &idelay_fixed_dly_ratio {
        pins.push((pin, SitePinDir::Out));
    }
    let odelay_fixed_dly_ratio: [_; 18] =
        core::array::from_fn(|i| format!("BS2CTL_ODELAY_FIXED_DLY_RATIO{i}"));
    for pin in &odelay_fixed_dly_ratio {
        pins.push((pin, SitePinDir::Out));
    }

    pins.push(("TX_TBYTE_IN", SitePinDir::In));
    vrf.claim_pip(bel.crd(), bel.wire("TX_TBYTE_IN"), obel_bstx.wire("Q"));

    vrf.verify_bel(bel, "BITSLICE_RX_TX", &pins, &["DYN_DCI_OUT_INT"]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    vrf.claim_node(&[bel.fwire_far("RX_CLK")]);

    if edev.kind == GridKind::Ultrascale {
        let obel = vrf.find_bel_sibling(bel, "CMT");
        let bt = if bidx < 2 { 'B' } else { 'T' };
        for i in 0..6 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire_far("RX_CLK"),
                obel.wire(&format!("XIPHY_CLK{i}_{bt}")),
            );
            for pin in ["RX_CLK_C", "RX_CLK_C_B", "TX_OCLK"] {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(pin),
                    obel.wire(&format!("XIPHY_CLK{i}_{bt}")),
                );
            }
        }
    } else {
        let obel = vrf.find_bel_sibling(bel, "XIPHY_BYTE");
        for i in 0..6 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire_far("RX_CLK"),
                obel.wire(&format!("XIPHY_CLK{i}")),
            );
            for pin in ["RX_CLK_C", "RX_CLK_C_B", "TX_OCLK"] {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(pin),
                    obel.wire(&format!("XIPHY_CLK{i}")),
                );
            }
        }
    };
    for pin in ["RX_CLK", "RX_CLKDIV", "TX_CLK", "TX_OCLKDIV"] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire_far("RX_CLK"));
    }

    vrf.claim_pip(bel.crd(), bel.wire("RX_D"), bel.wire_far("RX_D"));

    if bsidx != 12 {
        let obel_bs = vrf.find_bel_sibling(bel, &format!("BITSLICE_RX_TX{ii}", ii = idx + 1));
        vrf.claim_pip(
            bel.crd(),
            bel.wire("TX2RX_CASC_IN"),
            obel_bs.wire("TX2RX_CASC_OUT"),
        );
    }
    if bsidx != 0 {
        let obel_bs = vrf.find_bel_sibling(bel, &format!("BITSLICE_RX_TX{ii}", ii = idx - 1));
        vrf.claim_pip(
            bel.crd(),
            bel.wire("RX2TX_CASC_RETURN_IN"),
            obel_bs.wire("RX_Q5"),
        );
    }

    vrf.claim_pip(
        bel.crd(),
        bel.wire("DYN_DCI_OUT"),
        bel.wire("DYN_DCI_OUT_INT"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("DYN_DCI_OUT"),
        obel_bsctl.wire(&format!("DYN_DCI_OUT{sidx}")),
    );
}

fn verify_bitslice_tx(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx: usize = bel.key[11..].parse().unwrap();
    let obel_bsctl = vrf.find_bel_sibling(bel, &format!("BITSLICE_CONTROL{idx}"));
    let obel_feed = vrf.find_bel_sibling(bel, &format!("XIPHY_FEEDTHROUGH{ii}", ii = idx / 2));
    let mut pins = vec![
        // mux
        ("CLK", SitePinDir::In),
        // to BITSLICE_RX_TX.TBYTE_IN [all of them]
        ("Q", SitePinDir::Out),
        // to BSCTL
        ("BS2CTL_TX_DDR_PHASE_SEL", SitePinDir::Out),
        ("VTC_READY", SitePinDir::Out),
        // dummy stuff?
        ("CDATAIN0", SitePinDir::In),
        ("CDATAIN1", SitePinDir::In),
        ("CDATAOUT", SitePinDir::Out),
    ];

    for (pin, opin) in [
        ("CTRL_CE", "TRISTATE_ODELAY_CE_OUT"),
        ("CTRL_INC", "TRISTATE_ODELAY_INC_OUT"),
        ("CTRL_LD", "TRISTATE_ODELAY_LD_OUT"),
        ("CTRL_CLK", "ODELAY_CTRL_CLK7"),
        ("DIV2_CLK", "DIV2_CLK_OUT7"),
        ("DIV4_CLK", "DIV_CLK_OUT7"),
        ("DDR_CLK", "DDR_CLK_OUT7"),
        ("CTL2BS_DYNAMIC_MODE_EN", "DYNAMIC_MODE_EN7"),
        ("CTL2BS_TX_DDR_PHASE_SEL", "TX_DATA_PHASE7"),
        ("FORCE_OE_B", "FORCE_OE_B"),
        ("TOGGLE_DIV2_SEL", "TOGGLE_DIV2_SEL7"),
        ("TX_MUX_360_P_SEL", "PH02_DIV2_360_7"),
        ("TX_MUX_360_N_SEL", "PH13_DIV2_360_7"),
        ("TX_MUX_720_P0_SEL", "PH0_DIV_720_7"),
        ("TX_MUX_720_P1_SEL", "PH1_DIV_720_7"),
        ("TX_MUX_720_P2_SEL", "PH2_DIV_720_7"),
        ("TX_MUX_720_P3_SEL", "PH3_DIV_720_7"),
        ("BS_RESET", "BS_RESET_TRI"),
    ] {
        pins.push((pin, SitePinDir::In));
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel_bsctl.wire(opin));
    }
    let feed_pins = if edev.kind == GridKind::Ultrascale {
        [
            (
                "REGRST_B",
                "CLB2PHY_TRISTATE_ODELAY_RST_B_SMX0",
                "CLB2PHY_TRISTATE_ODELAY_RST_B_SMX1",
            ),
            (
                "RST_B",
                "CLB2PHY_TXBIT_TRI_RST_B_SMX0",
                "CLB2PHY_TXBIT_TRI_RST_B_SMX1",
            ),
        ]
    } else {
        [
            (
                "REGRST",
                "CLB2PHY_TRISTATE_ODELAY_RST_SMX0",
                "CLB2PHY_TRISTATE_ODELAY_RST_SMX1",
            ),
            (
                "RST",
                "CLB2PHY_TXBIT_TRI_RST_SMX0",
                "CLB2PHY_TXBIT_TRI_RST_SMX1",
            ),
        ]
    };
    for (pin, opin_l, opin_u) in feed_pins {
        pins.push((pin, SitePinDir::In));
        vrf.claim_pip(
            bel.crd(),
            bel.wire(pin),
            obel_feed.wire(if idx % 2 == 0 { opin_l } else { opin_u }),
        );
    }
    let ctrl_dly: [_; 9] = core::array::from_fn(|i| format!("CTRL_DLY{i}"));
    for (i, pin) in ctrl_dly.iter().enumerate() {
        pins.push((pin, SitePinDir::In));
        vrf.claim_pip(
            bel.crd(),
            bel.wire(pin),
            obel_bsctl.wire(&format!("TRISTATE_ODELAY_OUT{i}")),
        );
    }
    let d: [_; 8] = core::array::from_fn(|i| format!("D{i}"));
    for (i, pin) in d.iter().enumerate() {
        pins.push((pin, SitePinDir::In));
        vrf.claim_pip(
            bel.crd(),
            bel.wire(pin),
            obel_bsctl.wire(&format!("EN_DIV_DLY_OE{i}")),
        );
    }
    // to BSCTL
    let cntvalueout: [_; 9] = core::array::from_fn(|i| format!("BS2CTL_CNTVALUEOUT{i}"));
    for pin in &cntvalueout {
        pins.push((pin, SitePinDir::Out));
    }

    vrf.verify_bel(bel, "BITSLICE_TX", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    if edev.kind == GridKind::Ultrascale {
        let obel = vrf.find_bel_sibling(bel, "CMT");
        let bt = if idx < 4 { 'B' } else { 'T' };
        for i in 0..6 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire("CLK"),
                obel.wire(&format!("XIPHY_CLK{i}_{bt}")),
            );
        }
    } else {
        let obel = vrf.find_bel_sibling(bel, "XIPHY_BYTE");
        for i in 0..6 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire("CLK"),
                obel.wire(&format!("XIPHY_CLK{i}")),
            );
        }
    };
}

fn verify_bitslice_control(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let grid = edev.grids[bel.die];
    let idx: usize = bel.key[16..].parse().unwrap();
    let obel_pll_sel = vrf.find_bel_sibling(bel, &format!("PLL_SELECT{idx}"));
    let obel_bstx = vrf.find_bel_sibling(bel, &format!("BITSLICE_TX{idx}"));
    let obel_feed = vrf.find_bel_sibling(bel, &format!("XIPHY_FEEDTHROUGH{ii}", ii = idx / 2));

    let mut opins = vec![];
    // to PLL_SELECT
    for pin in ["REFCLK_DFD", "PLL_CLK_EN"] {
        opins.push(pin.to_string());
    }
    // to RIU
    opins.push("RIU2CLB_VALID".to_string());
    for i in 0..16 {
        opins.push(format!("RIU2CLB_RD_DATA{i}"));
    }
    // to BITSLICE_TX
    for pin in [
        "TRISTATE_ODELAY_CE_OUT",
        "TRISTATE_ODELAY_INC_OUT",
        "TRISTATE_ODELAY_LD_OUT",
        "FORCE_OE_B",
        "BS_RESET_TRI",
    ] {
        opins.push(pin.to_string());
    }
    for i in 0..8 {
        opins.push(format!("EN_DIV_DLY_OE{i}"));
    }
    for i in 0..9 {
        opins.push(format!("TRISTATE_ODELAY_OUT{i}"));
    }
    // to BITSLICE_TX and BITSLICE_RX_TX
    for i in 0..8 {
        for p in [
            format!("ODELAY_CTRL_CLK{i}"),
            format!("DYNAMIC_MODE_EN{i}"),
            format!("TOGGLE_DIV2_SEL{i}"),
            format!("TX_DATA_PHASE{i}"),
            format!("DIV2_CLK_OUT{i}"),
            format!("DIV_CLK_OUT{i}"),
            format!("DDR_CLK_OUT{i}"),
            format!("PH02_DIV2_360_{i}"),
            format!("PH13_DIV2_360_{i}"),
            format!("PH0_DIV_720_{i}"),
            format!("PH1_DIV_720_{i}"),
            format!("PH2_DIV_720_{i}"),
            format!("PH3_DIV_720_{i}"),
        ] {
            opins.push(p);
        }
    }
    // to BITSLICE_RX_TX
    for i in 0..7 {
        for p in [
            format!("IDELAY_CTRL_CLK{i}"),
            format!("IDELAY_CE_OUT{i}"),
            format!("IDELAY_INC_OUT{i}"),
            format!("IDELAY_LD_OUT{i}"),
            format!("ODELAY_CE_OUT{i}"),
            format!("ODELAY_INC_OUT{i}"),
            format!("ODELAY_LD_OUT{i}"),
            format!("WL_TRAIN{i}"),
            format!("RX_BS_RESET{i}"),
            format!("TX_BS_RESET{i}"),
            format!("PDQS_OUT{i}"),
            format!("NDQS_OUT{i}"),
            format!("RX_DCC{i:02}_0"),
            format!("RX_DCC{i:02}_1"),
            format!("RX_DCC{i:02}_2"),
            format!("RX_DCC{i:02}_3"),
            format!("IFIFO_BYPASS{i}"),
        ] {
            opins.push(p);
        }
        for j in 0..9 {
            for p in [
                format!("IDELAY{i:02}_OUT{j}"),
                format!("ODELAY{i:02}_OUT{j}"),
            ] {
                opins.push(p);
            }
        }
    }
    // to IOB, via a mux associated with BITSLICE_RX_TX
    for i in 0..7 {
        opins.push(format!("DYN_DCI_OUT{i}"));
    }
    // to XIPHY_FEEDTHROUGH
    for i in 0..7 {
        opins.push(format!("REFCLK_EN{i}"));
    }
    for pin in [
        // to other BSCTL
        "CLK_TO_EXT_SOUTH",
        "CLK_TO_EXT_NORTH",
        "PDQS_GT_OUT",
        "NDQS_GT_OUT",
        // to XIPHY_FEEDTHROUGH
        "LOCAL_DIV_CLK",
    ] {
        opins.push(pin.to_string());
    }

    let mut ipins = vec![];
    // from PLL_SELECT
    ipins.push("PLL_CLK".to_string());
    vrf.claim_pip(bel.crd(), bel.wire("PLL_CLK"), obel_pll_sel.wire("Z"));
    // from BITSLICE_TX
    ipins.push("BS2CTL_RIU_TX_DATA_PHASE7".to_string());
    vrf.claim_pip(
        bel.crd(),
        bel.wire("BS2CTL_RIU_TX_DATA_PHASE7"),
        obel_bstx.wire("BS2CTL_TX_DDR_PHASE_SEL"),
    );
    ipins.push("TRISTATE_VTC_READY".to_string());
    vrf.claim_pip(
        bel.crd(),
        bel.wire("TRISTATE_VTC_READY"),
        obel_bstx.wire("VTC_READY"),
    );
    for i in 0..9 {
        let pin = format!("TRISTATE_ODELAY_IN{i}");
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&pin),
            obel_bstx.wire(&format!("BS2CTL_CNTVALUEOUT{i}")),
        );
        ipins.push(pin);
    }

    // from BITSLICE_RX_TX
    for i in 0..7 {
        let obel = if idx % 2 == 0 && i == 6 {
            None
        } else {
            let ii = idx / 2 * 13 + idx % 2 * 6 + i;
            Some(vrf.find_bel_sibling(bel, &format!("BITSLICE_RX_TX{ii}")))
        };
        for (pin, opin) in [
            (
                format!("BS2CTL_RIU_TX_DATA_PHASE{i}"),
                "BS2CTL_TX_DDR_PHASE_SEL",
            ),
            (format!("RX_PDQ{i}_IN"), "BS2CTL_RX_P0_DQ_OUT"),
            (format!("RX_NDQ{i}_IN"), "BS2CTL_RX_N0_DQ_OUT"),
            (format!("FIXED_IDELAY{i:02}"), "BS2CTL_IDELAY_DELAY_FORMAT"),
            (format!("FIXED_ODELAY{i:02}"), "BS2CTL_ODELAY_DELAY_FORMAT"),
            (format!("VTC_READY_IDELAY{i:02}"), "RX_VTC_READY"),
            (format!("VTC_READY_ODELAY{i:02}"), "TX_VTC_READY"),
            (format!("DQS_IN{i}"), "RX_DQS_OUT"),
            (format!("BS2CTL_RIU_BS_DQS_EN{i}"), "BS2CTL_RX_DDR_EN_DQS"),
        ] {
            if let Some(ref obel) = obel {
                vrf.claim_pip(bel.crd(), bel.wire(&pin), obel.wire(opin));
            }
            ipins.push(pin);
        }
        for j in 0..9 {
            for (pin, opin) in [
                (
                    format!("IDELAY{i:02}_IN{j}"),
                    format!("BS2CTL_RX_CNTVALUEOUT{j}"),
                ),
                (
                    format!("ODELAY{i:02}_IN{j}"),
                    format!("BS2CTL_TX_CNTVALUEOUT{j}"),
                ),
            ] {
                if let Some(ref obel) = obel {
                    vrf.claim_pip(bel.crd(), bel.wire(&pin), obel.wire(&opin));
                }
                ipins.push(pin);
            }
        }
        for j in 0..18 {
            for (pin, opin) in [
                (
                    format!("FIXDLYRATIO_IDELAY{i:02}_{j}"),
                    format!("BS2CTL_IDELAY_FIXED_DLY_RATIO{j}"),
                ),
                (
                    format!("FIXDLYRATIO_ODELAY{i:02}_{j}"),
                    format!("BS2CTL_ODELAY_FIXED_DLY_RATIO{j}"),
                ),
            ] {
                if let Some(ref obel) = obel {
                    vrf.claim_pip(bel.crd(), bel.wire(&pin), obel.wire(&opin));
                }
                ipins.push(pin);
            }
        }
    }

    for (pin, opin) in [
        (
            "CLK_STOP",
            match idx % 2 {
                0 => "XIPHY_CLK_STOP_CTRL_LOW",
                1 => "XIPHY_CLK_STOP_CTRL_UPP",
                _ => unreachable!(),
            },
        ),
        (
            "SCAN_INT",
            match idx % 2 {
                0 => "SCAN_INT_LOWER",
                1 => "SCAN_INT_UPPER",
                _ => unreachable!(),
            },
        ),
        if edev.kind == GridKind::Ultrascale {
            (
                "CLB2PHY_CTRL_RST_B",
                match idx % 2 {
                    0 => "CLB2PHY_CTRL_RST_B_LOW_SMX",
                    1 => "CLB2PHY_CTRL_RST_B_UPP_SMX",
                    _ => unreachable!(),
                },
            )
        } else {
            (
                "CLB2PHY_CTRL_RST",
                match idx % 2 {
                    0 => "CLB2PHY_CTRL_RST_LOW_SMX",
                    1 => "CLB2PHY_CTRL_RST_UPP_SMX",
                    _ => unreachable!(),
                },
            )
        },
    ] {
        ipins.push(pin.to_string());
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel_feed.wire(opin));
    }

    for pin in [
        // from other BSCTL in byte
        "PDQS_GT_IN",
        "NDQS_GT_IN",
        // from BSCTL in another byte
        "CLK_FROM_EXT",
    ] {
        ipins.push(pin.to_string());
    }
    let obel_bsctl_on = vrf.find_bel_sibling(bel, &format!("BITSLICE_CONTROL{ii}", ii = idx ^ 1));
    vrf.claim_pip(
        bel.crd(),
        bel.wire("PDQS_GT_IN"),
        obel_bsctl_on.wire("PDQS_GT_OUT"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("NDQS_GT_IN"),
        obel_bsctl_on.wire("NDQS_GT_OUT"),
    );
    if edev.kind == GridKind::Ultrascale {
        let is_from_n = idx < 4;
        let obel_bsctl_ob = vrf.find_bel_sibling(
            bel,
            &format!(
                "BITSLICE_CONTROL{ii}",
                ii = if is_from_n { idx + 2 } else { idx - 2 }
            ),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLK_FROM_EXT"),
            obel_bsctl_ob.wire(if is_from_n {
                "CLK_TO_EXT_SOUTH"
            } else {
                "CLK_TO_EXT_NORTH"
            }),
        );
    } else {
        let is_from_n = bel.row < grid.row_rclk(bel.row);
        let obel_bsctl_ob = vrf
            .find_bel_delta(bel, 0, if is_from_n { 15 } else { -15 }, bel.key)
            .unwrap();
        vrf.claim_pip(
            bel.crd(),
            bel.wire("CLK_FROM_EXT"),
            bel.wire_far("CLK_FROM_EXT"),
        );
        vrf.verify_node(&[
            bel.fwire_far("CLK_FROM_EXT"),
            obel_bsctl_ob.fwire(if is_from_n {
                "CLK_TO_EXT_SOUTH"
            } else {
                "CLK_TO_EXT_NORTH"
            }),
        ]);
    }

    let mut pins = vec![];
    for pin in &opins {
        pins.push((&pin[..], SitePinDir::Out));
    }
    for pin in &ipins {
        pins.push((&pin[..], SitePinDir::In));
    }
    vrf.verify_bel(bel, "BITSLICE_CONTROL", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
}

fn verify_pll_select(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let grid = edev.grids[bel.die];
    let idx: usize = bel.key[10..].parse().unwrap();
    let pins = vec![
        ("D0", SitePinDir::In),
        ("D1", SitePinDir::In),
        ("REFCLK_DFD", SitePinDir::In),
        ("PLL_CLK_EN", SitePinDir::In),
        ("Z", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "PLL_SELECT_SITE", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel_pll0 = vrf
        .find_bel(bel.die, (bel.col, grid.row_rclk(bel.row)), "PLL0")
        .unwrap();
    let obel_pll1 = vrf
        .find_bel(bel.die, (bel.col, grid.row_rclk(bel.row)), "PLL1")
        .unwrap();
    vrf.claim_pip(bel.crd(), bel.wire("D0"), bel.wire_far("D0"));
    vrf.claim_pip(bel.crd(), bel.wire("D1"), bel.wire_far("D1"));
    vrf.verify_node(&[bel.fwire_far("D0"), obel_pll0.fwire("CLKOUTPHY_P")]);
    vrf.verify_node(&[bel.fwire_far("D1"), obel_pll1.fwire("CLKOUTPHY_P")]);

    let obel_bsctl = vrf.find_bel_sibling(bel, &format!("BITSLICE_CONTROL{idx}"));
    vrf.claim_pip(
        bel.crd(),
        bel.wire("REFCLK_DFD"),
        obel_bsctl.wire("REFCLK_DFD"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("PLL_CLK_EN"),
        obel_bsctl.wire("PLL_CLK_EN"),
    );
}

fn verify_riu_or(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx: usize = bel.key[6..].parse().unwrap();
    let obel_l = vrf.find_bel_sibling(bel, &format!("BITSLICE_CONTROL{ii}", ii = idx * 2));
    let obel_u = vrf.find_bel_sibling(bel, &format!("BITSLICE_CONTROL{ii}", ii = idx * 2 + 1));
    let mut ipins = vec![];
    for (obel, ul) in [(obel_l, "LOW"), (obel_u, "UPP")] {
        let pin = format!("RIU_RD_VALID_{ul}");
        vrf.claim_pip(bel.crd(), bel.wire(&pin), obel.wire("RIU2CLB_VALID"));
        ipins.push(pin);
        for i in 0..16 {
            let pin = format!("RIU_RD_DATA_{ul}{i}");
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&pin),
                obel.wire(&format!("RIU2CLB_RD_DATA{i}")),
            );
            ipins.push(pin);
        }
    }

    let pins: Vec<_> = ipins.iter().map(|x| (&x[..], SitePinDir::In)).collect();
    vrf.verify_bel(bel, "RIU_OR", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
}

fn verify_xiphy_feedthrough(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx: usize = bel.key[17..].parse().unwrap();
    let obel_bsctl_l = vrf.find_bel_sibling(bel, &format!("BITSLICE_CONTROL{ii}", ii = idx * 2));
    let obel_bsctl_u =
        vrf.find_bel_sibling(bel, &format!("BITSLICE_CONTROL{ii}", ii = idx * 2 + 1));
    let mut pins = vec![
        // to BSCTL
        ("SCAN_INT_LOWER", SitePinDir::Out),
        ("SCAN_INT_UPPER", SitePinDir::Out),
        ("XIPHY_CLK_STOP_CTRL_LOW", SitePinDir::Out),
        ("XIPHY_CLK_STOP_CTRL_UPP", SitePinDir::Out),
        // from BSCTL
        ("DIV_CLK_OUT_LOW", SitePinDir::In),
        ("DIV_CLK_OUT_UPP", SitePinDir::In),
        // dummy ins
        ("RCLK2PHY_CLKDR", SitePinDir::In),
        ("RCLK2PHY_SHIFTDR", SitePinDir::In),
    ];
    if edev.kind == GridKind::Ultrascale {
        pins.extend([
            // to BSCTL
            ("CLB2PHY_CTRL_RST_B_LOW_SMX", SitePinDir::Out),
            ("CLB2PHY_CTRL_RST_B_UPP_SMX", SitePinDir::Out),
            // to BITSLICE_TX
            ("CLB2PHY_TRISTATE_ODELAY_RST_B_SMX0", SitePinDir::Out),
            ("CLB2PHY_TRISTATE_ODELAY_RST_B_SMX1", SitePinDir::Out),
            ("CLB2PHY_TXBIT_TRI_RST_B_SMX0", SitePinDir::Out),
            ("CLB2PHY_TXBIT_TRI_RST_B_SMX1", SitePinDir::Out),
        ]);
    } else {
        pins.extend([
            // to BSCTL
            ("CLB2PHY_CTRL_RST_LOW_SMX", SitePinDir::Out),
            ("CLB2PHY_CTRL_RST_UPP_SMX", SitePinDir::Out),
            // to BITSLICE_TX
            ("CLB2PHY_TRISTATE_ODELAY_RST_SMX0", SitePinDir::Out),
            ("CLB2PHY_TRISTATE_ODELAY_RST_SMX1", SitePinDir::Out),
            ("CLB2PHY_TXBIT_TRI_RST_SMX0", SitePinDir::Out),
            ("CLB2PHY_TXBIT_TRI_RST_SMX1", SitePinDir::Out),
        ]);
    }
    vrf.claim_pip(
        bel.crd(),
        bel.wire("DIV_CLK_OUT_LOW"),
        obel_bsctl_l.wire("LOCAL_DIV_CLK"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("DIV_CLK_OUT_UPP"),
        obel_bsctl_u.wire("LOCAL_DIV_CLK"),
    );
    let mut opins = vec![];
    // to BITSLICE_RX_TX
    for i in 0..13 {
        opins.push(format!("CLB2PHY_FIFO_CLK_SMX{i}"));
        if edev.kind == GridKind::Ultrascale {
            opins.push(format!("CLB2PHY_TXBIT_RST_B_SMX{i}"));
            opins.push(format!("CLB2PHY_RXBIT_RST_B_SMX{i}"));
            opins.push(format!("CLB2PHY_IDELAY_RST_B_SMX{i}"));
            opins.push(format!("CLB2PHY_ODELAY_RST_B_SMX{i}"));
        } else {
            opins.push(format!("CLB2PHY_TXBIT_RST_SMX{i}"));
            opins.push(format!("CLB2PHY_RXBIT_RST_SMX{i}"));
            opins.push(format!("CLB2PHY_IDELAY_RST_SMX{i}"));
            opins.push(format!("CLB2PHY_ODELAY_RST_SMX{i}"));
        }
    }
    for i in 0..6 {
        opins.push(format!("CTL2BS_REFCLK_EN_LOW_SMX{i}"));
    }
    for i in 0..7 {
        opins.push(format!("CTL2BS_REFCLK_EN_UPP_SMX{i}"));
    }
    let mut ipins = vec![];
    // from BSCTL
    for i in 0..7 {
        let pin_l = format!("CTL2BS_REFCLK_EN_LOW{i}");
        let pin_u = format!("CTL2BS_REFCLK_EN_UPP{i}");
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&pin_l),
            obel_bsctl_l.wire(&format!("REFCLK_EN{i}")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&pin_u),
            obel_bsctl_u.wire(&format!("REFCLK_EN{i}")),
        );
        ipins.push(pin_l);
        ipins.push(pin_u);
    }
    for pin in &ipins {
        pins.push((pin, SitePinDir::In));
    }
    for pin in &opins {
        pins.push((pin, SitePinDir::Out));
    }
    vrf.verify_bel(bel, "XIPHY_FEEDTHROUGH", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
}

fn verify_xiphy_byte(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let grid = edev.grids[bel.die];
    let srow = grid.row_rclk(bel.row);
    let obel = vrf
        .find_bel(bel.die, (bel.col, srow), "RCLK_XIPHY")
        .unwrap();
    let bt = if bel.row < srow { 'B' } else { 'T' };
    for i in 0..6 {
        vrf.verify_node(&[
            bel.fwire(&format!("XIPHY_CLK{i}")),
            obel.fwire(&format!("XIPHY_CLK{i}_{bt}")),
        ]);
    }
}

fn verify_rclk_xiphy(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let obel_vcc = vrf.find_bel_sibling(bel, "VCC.RCLK_XIPHY");
    let hd_lr = match bel.node_kind {
        "RCLK_XIPHY_L" => 'R',
        "RCLK_XIPHY_R" => 'L',
        _ => unreachable!(),
    };
    for i in 0..6 {
        for bt in ['B', 'T'] {
            let pin = format!("XIPHY_CLK{i}_{bt}");
            vrf.claim_node(&[bel.fwire(&pin)]);
            vrf.claim_pip(bel.crd(), bel.wire(&pin), obel_vcc.wire("VCC"));
            for j in 0..24 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire(&pin),
                    bel.wire(&format!("HDISTR{j}_{hd_lr}")),
                );
            }
        }
    }
    for i in 0..24 {
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_L")),
            bel.wire(&format!("HDISTR{i}_R")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_L")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_R")),
            bel.wire(&format!("HDISTR{i}_L")),
        );
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("HDISTR{i}_R")),
            obel_vcc.wire("VCC"),
        );
        vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_L"))]);
    }
    if bel.node_kind == "RCLK_XIPHY_L" {
        let obel_hd = find_hdistr_src(edev, vrf, bel.die, bel.col, bel.row, ColSide::Left);
        for i in 0..24 {
            vrf.verify_node(&[
                bel.fwire(&format!("HDISTR{i}_R")),
                obel_hd.fwire(&format!("HDISTR{i}_L")),
            ]);
        }
    } else {
        for i in 0..24 {
            vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_R"))]);
        }
    }
}

fn verify_hpiob(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let grid = edev.grids[bel.die];
    let idx: usize = bel.key[5..].parse().unwrap();
    let pidx = if matches!(idx, 12 | 25) {
        None
    } else if idx < 12 {
        Some(idx ^ 1)
    } else {
        Some(((idx - 1) ^ 1) + 1)
    };
    let is_single = pidx.is_none();
    let is_m = !is_single && if idx < 13 { idx % 2 == 0 } else { idx % 2 == 1 };
    let kind = if edev.kind == GridKind::Ultrascale {
        "HPIOB"
    } else if is_single {
        "HPIOB_SNGL"
    } else if is_m {
        "HPIOB_M"
    } else {
        "HPIOB_S"
    };
    let fidx = if bel.row.to_idx() % 60 == 0 {
        idx
    } else {
        idx + 26
    };
    let hid = HpioIobId::from_idx(fidx);
    let reg = grid.row_to_reg(bel.row);

    let mut pins = vec![
        // to/from PHY
        ("I", SitePinDir::Out),
        ("OP", SitePinDir::In),
        ("TSP", SitePinDir::In),
        ("DYNAMIC_DCI_TS", SitePinDir::In),
        // to AMS
        ("SWITCH_OUT", SitePinDir::Out),
        // to/from paired IOB
        ("OUTB_B_IN", SitePinDir::In),
        ("OUTB_B", SitePinDir::Out),
        ("TSTATE_IN", SitePinDir::In),
        ("TSTATE_OUT", SitePinDir::Out),
        // to/from differential out
        ("O_B", SitePinDir::Out),
        ("TSTATEB", SitePinDir::Out),
        ("IO", SitePinDir::In),
        // from differential in
        ("LVDS_TRUE", SitePinDir::In),
        // from VREF
        ("VREF", SitePinDir::In),
        // dummies
        ("DOUT", SitePinDir::Out),
        ("TSDI", SitePinDir::In),
    ];

    // to differential in
    if edev.kind == GridKind::Ultrascale {
        pins.push(("CTLE_IN", SitePinDir::Out));
    }
    if edev.kind == GridKind::Ultrascale || !matches!(idx, 12 | 25) {
        pins.push(("PAD_RES", SitePinDir::Out));
    }

    let mut dummies = vec!["TSDI"];
    if is_single {
        dummies.push("IO");
        if edev.kind == GridKind::UltrascalePlus {
            dummies.push("LVDS_TRUE");
            dummies.push("OUTB_B_IN");
            dummies.push("TSTATE_IN");
        }
    }
    if !edev
        .disabled
        .contains(&DisabledPart::HpioIob(bel.die, bel.col, reg, hid))
    {
        vrf.verify_bel_dummies(bel, kind, &pins, &[], &dummies);
    }
    for (pin, _) in pins {
        if !dummies.contains(&pin) {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
    }

    if let Some(pidx) = pidx {
        let obel = vrf.find_bel_sibling(bel, &format!("HPIOB{pidx}"));
        vrf.claim_pip(bel.crd(), bel.wire("OUTB_B_IN"), obel.wire("OUTB_B"));
        vrf.claim_pip(bel.crd(), bel.wire("TSTATE_IN"), obel.wire("TSTATE_OUT"));
    } else if edev.kind == GridKind::Ultrascale {
        vrf.claim_pip(bel.crd(), bel.wire("OUTB_B_IN"), bel.wire("OUTB_B"));
    }

    let obel_bs = if edev.kind == GridKind::Ultrascale {
        let srow = grid.row_rclk(bel.row);
        vrf.find_bel(
            bel.die,
            (bel.col, srow),
            &format!(
                "BITSLICE_RX_TX{ii}",
                ii = if bel.row < srow { idx } else { idx + 26 }
            ),
        )
        .unwrap()
    } else {
        let srow = if idx < 13 { bel.row } else { bel.row + 15 };
        vrf.find_bel(
            bel.die,
            (bel.col, srow),
            &format!("BITSLICE_RX_TX{ii}", ii = idx % 13),
        )
        .unwrap()
    };

    vrf.claim_pip(bel.crd(), bel.wire("OP"), bel.wire_far("OP"));
    vrf.claim_pip(bel.crd(), bel.wire("TSP"), bel.wire_far("TSP"));
    vrf.verify_node(&[bel.fwire("DYNAMIC_DCI_TS"), obel_bs.fwire("DYN_DCI_OUT")]);
    vrf.verify_node(&[bel.fwire("I"), obel_bs.fwire_far("RX_D")]);
    vrf.verify_node(&[bel.fwire_far("OP"), obel_bs.fwire("TX_Q")]);
    vrf.verify_node(&[bel.fwire_far("TSP"), obel_bs.fwire("TX_T_OUT")]);

    if !bel.naming.pins["SWITCH_OUT"].pips.is_empty() {
        vrf.claim_pip(
            bel.crd(),
            bel.wire_far("SWITCH_OUT"),
            bel.wire("SWITCH_OUT"),
        );

        let ams_idx = match fidx {
            4 | 5 => Some(15),
            6 | 7 => Some(7),
            8 | 9 => Some(14),
            10 | 11 => Some(6),
            13 | 14 => Some(13),
            15 | 16 => Some(5),
            17 | 18 => Some(12),
            19 | 20 => Some(4),
            30 | 31 => Some(11),
            32 | 33 => Some(3),
            34 | 35 => Some(10),
            36 | 37 => Some(2),
            39 | 40 => Some(9),
            41 | 42 => Some(1),
            43 | 44 => Some(8),
            45 | 46 => Some(0),
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

    let obel_vref = vrf.find_bel_sibling(bel, "HPIO_VREF");
    vrf.claim_pip(
        bel.crd(),
        bel.wire("VREF"),
        obel_vref.wire(if idx < 13 { "VREF1" } else { "VREF2" }),
    );
}

fn verify_hpiodiffin(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let grid = edev.grids[bel.die];
    let idx: usize = bel.key[10..].parse().unwrap();
    let (pidx, nidx) = if idx < 6 {
        (idx * 2, idx * 2 + 1)
    } else {
        (idx * 2 + 1, idx * 2 + 2)
    };
    let reg = grid.row_to_reg(bel.row);
    let mut disabled = false;
    for sidx in [pidx, nidx] {
        let fidx = if bel.row.to_idx() % 60 == 0 {
            sidx
        } else {
            sidx + 26
        };
        let hid = HpioIobId::from_idx(fidx);
        disabled |= edev
            .disabled
            .contains(&DisabledPart::HpioIob(bel.die, bel.col, reg, hid));
    }
    let mut pins = vec![
        ("PAD_RES_0", SitePinDir::In),
        ("PAD_RES_1", SitePinDir::In),
        ("LVDS_TRUE", SitePinDir::Out),
        ("LVDS_COMP", SitePinDir::Out),
        ("VREF", SitePinDir::In),
    ];
    if edev.kind == GridKind::Ultrascale {
        pins.push(("CTLE_IN_1", SitePinDir::In));
    }
    if !disabled {
        vrf.verify_bel(bel, "HPIOBDIFFINBUF", &pins, &[]);
    }
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_p = vrf.find_bel_sibling(bel, &format!("HPIOB{pidx}"));
    let obel_n = vrf.find_bel_sibling(bel, &format!("HPIOB{nidx}"));
    vrf.claim_pip(bel.crd(), bel.wire("PAD_RES_0"), obel_p.wire("PAD_RES"));
    vrf.claim_pip(bel.crd(), bel.wire("PAD_RES_1"), obel_n.wire("PAD_RES"));
    vrf.claim_pip(bel.crd(), obel_p.wire("LVDS_TRUE"), bel.wire("LVDS_TRUE"));
    vrf.claim_pip(bel.crd(), obel_n.wire("LVDS_TRUE"), bel.wire("LVDS_COMP"));
    if edev.kind == GridKind::Ultrascale {
        vrf.claim_pip(bel.crd(), bel.wire("CTLE_IN_1"), obel_n.wire("CTLE_IN"));
    }

    let obel_vref = vrf.find_bel_sibling(bel, "HPIO_VREF");
    vrf.claim_pip(
        bel.crd(),
        bel.wire("VREF"),
        obel_vref.wire(if idx < 6 { "VREF1" } else { "VREF2" }),
    );
}

fn verify_hpiodiffout(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let grid = edev.grids[bel.die];
    let idx: usize = bel.key[11..].parse().unwrap();
    let (pidx, nidx) = if idx < 6 {
        (idx * 2, idx * 2 + 1)
    } else {
        (idx * 2 + 1, idx * 2 + 2)
    };
    let reg = grid.row_to_reg(bel.row);
    let mut disabled = false;
    for sidx in [pidx, nidx] {
        let fidx = if bel.row.to_idx() % 60 == 0 {
            sidx
        } else {
            sidx + 26
        };
        let hid = HpioIobId::from_idx(fidx);
        disabled |= edev
            .disabled
            .contains(&DisabledPart::HpioIob(bel.die, bel.col, reg, hid));
    }
    let pins = vec![
        ("O_B", SitePinDir::In),
        ("TSTATEB", SitePinDir::In),
        ("AOUT", SitePinDir::Out),
        ("BOUT", SitePinDir::Out),
    ];
    if !disabled {
        vrf.verify_bel(bel, "HPIOBDIFFOUTBUF", &pins, &[]);
    }
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_p = vrf.find_bel_sibling(bel, &format!("HPIOB{pidx}"));
    let obel_n = vrf.find_bel_sibling(bel, &format!("HPIOB{nidx}"));
    vrf.claim_pip(bel.crd(), bel.wire("O_B"), obel_p.wire("O_B"));
    vrf.claim_pip(bel.crd(), bel.wire("TSTATEB"), obel_p.wire("TSTATEB"));
    vrf.claim_pip(bel.crd(), obel_p.wire("IO"), bel.wire("AOUT"));
    vrf.claim_pip(bel.crd(), obel_n.wire("IO"), bel.wire("BOUT"));
}

fn verify_hpio_vref(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let pins = vec![("VREF1", SitePinDir::Out), ("VREF2", SitePinDir::Out)];
    vrf.verify_bel(bel, "HPIO_VREF_SITE", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
}

fn verify_hpio_dci(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let grid = edev.grids[bel.die];
    let reg = grid.row_to_reg(bel.row);
    if !edev
        .disabled
        .contains(&DisabledPart::HpioDci(bel.die, bel.col, reg))
    {
        vrf.verify_bel(bel, "HPIOB_DCI_SNGL", &[], &[]);
    }
}

fn verify_hriob(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let grid = edev.grids[bel.die];
    let idx: usize = bel.key[5..].parse().unwrap();
    let pidx = if matches!(idx, 12 | 25) {
        None
    } else if idx < 12 {
        Some(idx ^ 1)
    } else {
        Some(((idx - 1) ^ 1) + 1)
    };
    let is_single = pidx.is_none();
    let is_m = !is_single && if idx < 13 { idx % 2 == 0 } else { idx % 2 == 1 };
    let pins = vec![
        // to/from PHY
        ("DOUT", SitePinDir::Out),
        ("OP", SitePinDir::In),
        ("TSP", SitePinDir::In),
        ("DYNAMIC_DCI_TS", SitePinDir::In),
        // to AMS
        ("SWITCH_OUT", SitePinDir::Out),
        // to/from pair
        ("OUTB_B_IN", SitePinDir::In),
        ("OUTB_B", SitePinDir::Out),
        ("TSTATEIN", SitePinDir::In),
        ("TSTATEOUT", SitePinDir::Out),
        // to/from differential out
        ("O_B", SitePinDir::Out),
        ("TSTATEB", SitePinDir::Out),
        ("IO", SitePinDir::In),
        // to/from differential in
        ("TMDS_IBUF_OUT", SitePinDir::In),
        ("DRIVER_BOT_IBUF", SitePinDir::Out),
        // dummies
        ("TSDI", SitePinDir::In),
    ];
    let idx: usize = bel.key[5..].parse().unwrap();
    let mut dummies = vec![];
    let has_tsdi = false;
    if !has_tsdi {
        dummies.push("TSDI");
    }
    if is_single {
        dummies.push("IO");
    }
    vrf.verify_bel_dummies(bel, "HRIO", &pins, &[], &dummies);
    for (pin, _) in pins {
        if !dummies.contains(&pin) {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
    }

    if let Some(pidx) = pidx {
        let obel = vrf.find_bel_sibling(bel, &format!("HRIOB{pidx}"));
        vrf.claim_pip(bel.crd(), bel.wire("OUTB_B_IN"), obel.wire("OUTB_B"));
        vrf.claim_pip(bel.crd(), bel.wire("TSTATEIN"), obel.wire("TSTATEOUT"));
    } else {
        vrf.claim_pip(bel.crd(), bel.wire("OUTB_B_IN"), bel.wire("OUTB_B"));
    }

    let srow = grid.row_rclk(bel.row);
    let obel_bs = vrf
        .find_bel(
            bel.die,
            (bel.col, srow),
            &format!(
                "BITSLICE_RX_TX{ii}",
                ii = if bel.row < srow { idx } else { idx + 26 }
            ),
        )
        .unwrap();

    vrf.claim_pip(bel.crd(), bel.wire("OP"), bel.wire_far("OP"));
    vrf.claim_pip(bel.crd(), bel.wire("TSP"), bel.wire_far("TSP"));
    vrf.verify_node(&[bel.fwire("DYNAMIC_DCI_TS"), obel_bs.fwire("DYN_DCI_OUT")]);
    vrf.verify_node(&[bel.fwire("DOUT"), obel_bs.fwire_far("RX_D")]);
    vrf.verify_node(&[bel.fwire_far("OP"), obel_bs.fwire("TX_Q")]);
    vrf.verify_node(&[bel.fwire_far("TSP"), obel_bs.fwire("TX_T_OUT")]);
    if has_tsdi {
        vrf.claim_pip(bel.crd(), bel.wire("TSDI"), bel.wire_far("TSDI"));
    }

    if !bel.naming.pins["SWITCH_OUT"].pips.is_empty() {
        vrf.claim_pip(
            bel.crd(),
            bel.wire_far("SWITCH_OUT"),
            bel.wire("SWITCH_OUT"),
        );
        let fidx = if bel.row.to_idx() % 60 == 0 {
            idx
        } else {
            idx + 26
        };

        let ams_idx = match fidx {
            4 | 5 => Some(15),
            6 | 7 => Some(7),
            8 | 9 => Some(14),
            10 | 11 => Some(6),
            13 | 14 => Some(13),
            15 | 16 => Some(5),
            17 | 18 => Some(12),
            19 | 20 => Some(4),
            30 | 31 => Some(11),
            32 | 33 => Some(3),
            34 | 35 => Some(10),
            36 | 37 => Some(2),
            39 | 40 => Some(9),
            41 | 42 => Some(1),
            43 | 44 => Some(8),
            45 | 46 => Some(0),
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
}

fn verify_hriodiffin(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx: usize = bel.key[10..].parse().unwrap();
    let (pidx, nidx) = if idx < 6 {
        (idx * 2, idx * 2 + 1)
    } else {
        (idx * 2 + 1, idx * 2 + 2)
    };
    let pins = vec![
        ("LVDS_IN_P", SitePinDir::In),
        ("LVDS_IN_N", SitePinDir::In),
        ("LVDS_IBUF_OUT", SitePinDir::Out),
        ("LVDS_IBUF_OUT_B", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "HRIODIFFINBUF", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_p = vrf.find_bel_sibling(bel, &format!("HRIOB{pidx}"));
    let obel_n = vrf.find_bel_sibling(bel, &format!("HRIOB{nidx}"));
    vrf.claim_pip(
        bel.crd(),
        bel.wire("LVDS_IN_P"),
        obel_p.wire("DRIVER_BOT_IBUF"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("LVDS_IN_N"),
        obel_n.wire("DRIVER_BOT_IBUF"),
    );
    vrf.claim_pip(
        bel.crd(),
        obel_p.wire("TMDS_IBUF_OUT"),
        bel.wire("LVDS_IBUF_OUT"),
    );
    vrf.claim_pip(
        bel.crd(),
        obel_n.wire("TMDS_IBUF_OUT"),
        bel.wire("LVDS_IBUF_OUT_B"),
    );
}

fn verify_hriodiffout(vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx: usize = bel.key[11..].parse().unwrap();
    let (pidx, nidx) = if idx < 6 {
        (idx * 2, idx * 2 + 1)
    } else {
        (idx * 2 + 1, idx * 2 + 2)
    };
    let pins = vec![
        ("O_B", SitePinDir::In),
        ("TSTATEB", SitePinDir::In),
        ("AOUT", SitePinDir::Out),
        ("BOUT", SitePinDir::Out),
    ];
    vrf.verify_bel(bel, "HRIODIFFOUTBUF", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_p = vrf.find_bel_sibling(bel, &format!("HRIOB{pidx}"));
    let obel_n = vrf.find_bel_sibling(bel, &format!("HRIOB{nidx}"));
    vrf.claim_pip(bel.crd(), bel.wire("O_B"), obel_p.wire("O_B"));
    vrf.claim_pip(bel.crd(), bel.wire("TSTATEB"), obel_p.wire("TSTATEB"));
    vrf.claim_pip(bel.crd(), obel_p.wire("IO"), bel.wire("AOUT"));
    vrf.claim_pip(bel.crd(), obel_n.wire("IO"), bel.wire("BOUT"));
}

fn verify_bufg_gt(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let mut pins = vec![
        ("CLK_IN", SitePinDir::In),
        ("CE", SitePinDir::In),
        ("RST_PRE_OPTINV", SitePinDir::In),
        ("CLK_OUT", SitePinDir::Out),
    ];
    if !bel.bel.pins.contains_key("DIV0") {
        pins.extend([
            ("DIV0", SitePinDir::In),
            ("DIV1", SitePinDir::In),
            ("DIV2", SitePinDir::In),
        ]);
    }
    vrf.verify_bel(bel, "BUFG_GT", &pins, &[]);
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel_vcc = vrf.find_bel_sibling(bel, "VCC.GT");
    if edev.kind == GridKind::Ultrascale {
        let gtk = &bel.node_kind[..3];
        for (key, pin) in [
            ("COMMON", "REFCLK2HROW0"),
            ("CHANNEL0", "TXOUTCLK_INT"),
            ("CHANNEL1", "TXOUTCLK_INT"),
            ("CHANNEL0", "RXRECCLK_INT"),
            ("CHANNEL1", "RXRECCLK_INT"),
            ("COMMON", "REFCLK2HROW1"),
            ("CHANNEL2", "TXOUTCLK_INT"),
            ("CHANNEL3", "TXOUTCLK_INT"),
            ("CHANNEL2", "RXRECCLK_INT"),
            ("CHANNEL3", "RXRECCLK_INT"),
        ] {
            let obel = vrf.find_bel_sibling(bel, &format!("{gtk}_{key}"));
            vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire(pin));
        }
        let obel = vrf.find_bel_sibling(bel, "BUFG_GT_SYNC10");
        vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("CLK_IN"));
        for i in 0..11 {
            let obel = vrf.find_bel_sibling(bel, &format!("BUFG_GT_SYNC{i}"));
            vrf.claim_pip(bel.crd(), bel.wire("CE"), obel.wire("CE_OUT"));
            vrf.claim_pip(bel.crd(), bel.wire("RST_PRE_OPTINV"), obel.wire("RST_OUT"));
        }
        vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), bel.wire("CE"), obel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), bel.wire("RST_PRE_OPTINV"), obel_vcc.wire("VCC"));
        for i in 0..5 {
            vrf.claim_pip(
                bel.crd(),
                bel.wire("CLK_IN"),
                bel.wire(&format!("CLK_IN_MUX_DUMMY{i}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire("CE"),
                bel.wire(&format!("CE_MUX_DUMMY{i}")),
            );
            vrf.claim_pip(
                bel.crd(),
                bel.wire("RST_PRE_OPTINV"),
                bel.wire(&format!("RST_MUX_DUMMY{i}")),
            );
            vrf.claim_node(&[bel.fwire(&format!("CLK_IN_MUX_DUMMY{i}"))]);
            vrf.claim_node(&[bel.fwire(&format!("CE_MUX_DUMMY{i}"))]);
            vrf.claim_node(&[bel.fwire(&format!("RST_MUX_DUMMY{i}"))]);
        }
    } else {
        if bel.node_kind.starts_with("GTM") {
            let obel = vrf.find_bel_sibling(bel, "GTM_DUAL");
            for i in 0..6 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire("CLK_IN"),
                    obel.wire(&format!("CLK_BUFGT_CLK_IN_BOT{i}")),
                );
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire("CLK_IN"),
                    obel.wire(&format!("CLK_BUFGT_CLK_IN_TOP{i}")),
                );
            }
            for key in ["BUFG_GT_SYNC6", "BUFG_GT_SYNC13"] {
                let obel = vrf.find_bel_sibling(bel, key);
                vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("CLK_IN"));
            }
            vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire("CLK_IN_MUX_DUMMY0"));
            vrf.claim_pip(bel.crd(), bel.wire("CE"), bel.wire("CE_MUX_DUMMY0"));
            vrf.claim_pip(
                bel.crd(),
                bel.wire("RST_PRE_OPTINV"),
                bel.wire("RST_MUX_DUMMY0"),
            );
        } else if bel.node_kind.starts_with("GT") {
            let gtk = &bel.node_kind[..3];
            for (key, pin) in [
                ("COMMON", "REFCLK2HROW0"),
                ("CHANNEL0", "TXOUTCLK_INT"),
                ("CHANNEL1", "TXOUTCLK_INT"),
                ("CHANNEL0", "RXRECCLK_INT"),
                ("CHANNEL1", "RXRECCLK_INT"),
                ("CHANNEL0", "DMONOUTCLK_INT"),
                ("CHANNEL1", "DMONOUTCLK_INT"),
                ("COMMON", "REFCLK2HROW1"),
                ("CHANNEL2", "TXOUTCLK_INT"),
                ("CHANNEL3", "TXOUTCLK_INT"),
                ("CHANNEL2", "RXRECCLK_INT"),
                ("CHANNEL3", "RXRECCLK_INT"),
                ("CHANNEL2", "DMONOUTCLK_INT"),
                ("CHANNEL3", "DMONOUTCLK_INT"),
            ] {
                let obel = vrf.find_bel_sibling(bel, &format!("{gtk}_{key}"));
                vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire(pin));
            }
            vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), bel.wire("CLK_IN_MUX_DUMMY0"));
            vrf.claim_pip(bel.crd(), bel.wire("CE"), bel.wire("CE_MUX_DUMMY0"));
            vrf.claim_pip(
                bel.crd(),
                bel.wire("RST_PRE_OPTINV"),
                bel.wire("RST_MUX_DUMMY0"),
            );
            vrf.claim_node(&[bel.fwire("CLK_IN_MUX_DUMMY0")]);
            vrf.claim_node(&[bel.fwire("CE_MUX_DUMMY0")]);
            vrf.claim_node(&[bel.fwire("RST_MUX_DUMMY0")]);
        } else {
            let obel = vrf.find_bel_sibling(bel, &bel.node_kind[..5]);
            if obel.key.ends_with("ADC") {
                vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("CLK_ADC"));
                vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("CLK_ADC_SPARE"));
            } else {
                vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("CLK_DAC"));
                vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("CLK_DAC_SPARE"));
            }
            vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("PLL_DMON_OUT"));
            vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("PLL_REFCLK_OUT"));
            for i in 0..11 {
                vrf.claim_pip(
                    bel.crd(),
                    bel.wire("CLK_IN"),
                    bel.wire(&format!("CLK_IN_MUX_DUMMY{i}")),
                );
            }
            vrf.claim_pip(bel.crd(), bel.wire("CE"), bel.wire("CE_MUX_DUMMY0"));
            vrf.claim_pip(
                bel.crd(),
                bel.wire("RST_PRE_OPTINV"),
                bel.wire("RST_MUX_DUMMY0"),
            );
        }
        let obel = vrf.find_bel_sibling(bel, "BUFG_GT_SYNC14");
        vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire("CLK_IN"));
        for i in 0..15 {
            let obel = vrf.find_bel_sibling(bel, &format!("BUFG_GT_SYNC{i}"));
            vrf.claim_pip(bel.crd(), bel.wire("CE"), obel.wire("CE_OUT"));
            vrf.claim_pip(bel.crd(), bel.wire("RST_PRE_OPTINV"), obel.wire("RST_OUT"));
        }
        vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), bel.wire("CE"), obel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), bel.wire("RST_PRE_OPTINV"), obel_vcc.wire("VCC"));
    }
}

fn verify_bufg_gt_sync(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let grid = edev.grids[bel.die];
    let idx: usize = bel.key[12..].parse().unwrap();
    let mut pins = vec![("CE_OUT", SitePinDir::Out), ("RST_OUT", SitePinDir::Out)];
    let mut dummies = vec![];
    let mut is_int = false;
    if edev.kind == GridKind::Ultrascale {
        if idx == 10 {
            is_int = true;
        } else {
            let nk = &bel.node_kind[..3];
            let (okey, pin) = match idx {
                0 => ("COMMON", "REFCLK2HROW0"),
                1 => ("CHANNEL0", "TXOUTCLK_INT"),
                2 => ("CHANNEL1", "TXOUTCLK_INT"),
                3 => ("CHANNEL0", "RXRECCLK_INT"),
                4 => ("CHANNEL1", "RXRECCLK_INT"),
                5 => ("COMMON", "REFCLK2HROW1"),
                6 => ("CHANNEL2", "TXOUTCLK_INT"),
                7 => ("CHANNEL3", "TXOUTCLK_INT"),
                8 => ("CHANNEL2", "RXRECCLK_INT"),
                9 => ("CHANNEL3", "RXRECCLK_INT"),
                _ => unreachable!(),
            };
            let obel = vrf.find_bel_sibling(bel, &format!("{nk}_{okey}"));
            vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire(pin));
        }
    } else {
        if idx == 14 {
            is_int = true;
        } else {
            if bel.node_kind.starts_with("GTM") {
                if matches!(idx, 6 | 13) {
                    dummies.push("CLK_IN");
                } else {
                    let obel = vrf.find_bel_sibling(bel, "GTM_DUAL");
                    let pin = if idx < 6 {
                        format!("CLK_BUFGT_CLK_IN_BOT{idx}")
                    } else {
                        format!("CLK_BUFGT_CLK_IN_TOP{ii}", ii = idx - 7)
                    };
                    vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire(&pin));
                }
            } else if bel.node_kind.starts_with("GT") {
                let nk = &bel.node_kind[..3];
                let (okey, pin) = match idx {
                    0 => ("COMMON", "REFCLK2HROW0"),
                    1 => ("CHANNEL0", "TXOUTCLK_INT"),
                    2 => ("CHANNEL1", "TXOUTCLK_INT"),
                    3 => ("CHANNEL0", "RXRECCLK_INT"),
                    4 => ("CHANNEL1", "RXRECCLK_INT"),
                    5 => ("CHANNEL0", "DMONOUTCLK_INT"),
                    6 => ("CHANNEL1", "DMONOUTCLK_INT"),
                    7 => ("COMMON", "REFCLK2HROW1"),
                    8 => ("CHANNEL2", "TXOUTCLK_INT"),
                    9 => ("CHANNEL3", "TXOUTCLK_INT"),
                    10 => ("CHANNEL2", "RXRECCLK_INT"),
                    11 => ("CHANNEL3", "RXRECCLK_INT"),
                    12 => ("CHANNEL2", "DMONOUTCLK_INT"),
                    13 => ("CHANNEL3", "DMONOUTCLK_INT"),
                    _ => unreachable!(),
                };
                let obel = vrf.find_bel_sibling(bel, &format!("{nk}_{okey}"));
                vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire(pin));
            } else {
                if idx < 4 {
                    let is_adc = bel.node_kind.contains("ADC");
                    let pin = match (idx, is_adc) {
                        (0, true) => "CLK_ADC",
                        (0, false) => "CLK_DAC",
                        (1, _) => "PLL_DMON_OUT",
                        (2, _) => "PLL_REFCLK_OUT",
                        (3, true) => "CLK_ADC_SPARE",
                        (3, false) => "CLK_DAC_SPARE",
                        _ => unreachable!(),
                    };
                    let obel = vrf.find_bel_sibling(bel, &bel.node_kind[..5]);
                    vrf.claim_pip(bel.crd(), bel.wire("CLK_IN"), obel.wire(pin));
                }
            }
        }
        if !bel.bel.pins.contains_key("CE_IN") {
            pins.extend([("CE_IN", SitePinDir::In), ("RST_IN", SitePinDir::In)]);
        }
    }
    if !is_int {
        pins.push(("CLK_IN", SitePinDir::In));
    }

    let reg = grid.row_to_reg(bel.row);
    let skip = edev
        .disabled
        .contains(&DisabledPart::GtmSpareBufs(bel.die, bel.col, reg))
        && matches!(idx, 6 | 13);
    if !skip {
        vrf.verify_bel_dummies(bel, "BUFG_GT_SYNC", &pins, &[], &dummies);
    }
    for (pin, _) in pins {
        if !dummies.contains(&pin) {
            vrf.claim_node(&[bel.fwire(pin)]);
        }
    }
}

fn verify_gt_channel(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let idx: usize = bel.key[11..].parse().unwrap();
    let grid = edev.grids[bel.die];
    let kind = if bel.key.starts_with("GTH") {
        match edev.kind {
            GridKind::Ultrascale => "GTHE3_CHANNEL",
            GridKind::UltrascalePlus => "GTHE4_CHANNEL",
        }
    } else if bel.key.starts_with("GTY") {
        match edev.kind {
            GridKind::Ultrascale => "GTYE3_CHANNEL",
            GridKind::UltrascalePlus => "GTYE4_CHANNEL",
        }
    } else {
        "GTF_CHANNEL"
    };
    let mut pins = vec![
        // from COMMON
        ("MGTREFCLK0", SitePinDir::In),
        ("MGTREFCLK1", SitePinDir::In),
        ("NORTHREFCLK0", SitePinDir::In),
        ("NORTHREFCLK1", SitePinDir::In),
        ("SOUTHREFCLK0", SitePinDir::In),
        ("SOUTHREFCLK1", SitePinDir::In),
        ("QDCMREFCLK0_INT", SitePinDir::In),
        ("QDCMREFCLK1_INT", SitePinDir::In),
        ("QDPLL0CLK0P_INT", SitePinDir::In),
        ("QDPLL1CLK0P_INT", SitePinDir::In),
        ("RING_OSC_CLK_INT", SitePinDir::In),
        // to COMMON
        ("RXRECCLKOUT", SitePinDir::Out),
        // to BUFG_*
        ("RXRECCLK_INT", SitePinDir::Out),
        ("TXOUTCLK_INT", SitePinDir::Out),
    ];
    if edev.kind == GridKind::UltrascalePlus {
        // to BUFG_*
        pins.push(("DMONOUTCLK_INT", SitePinDir::Out));
    }
    let reg = grid.row_to_reg(bel.row);
    if !edev
        .disabled
        .contains(&DisabledPart::Gt(bel.die, bel.col, reg))
    {
        vrf.verify_bel(bel, kind, &pins, &[]);
    }
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    let obel = vrf.find_bel_sibling(bel, &format!("{pref}_COMMON", pref = &bel.key[..3]));
    let cross_qdpll = edev.kind == GridKind::Ultrascale && bel.key.starts_with("GTH");
    for (pin, opin) in [
        ("MGTREFCLK0", "MGTREFCLK0"),
        ("MGTREFCLK1", "MGTREFCLK1"),
        ("NORTHREFCLK0", "NORTHREFCLK0"),
        ("NORTHREFCLK1", "NORTHREFCLK1"),
        ("SOUTHREFCLK0", "SOUTHREFCLK0"),
        ("SOUTHREFCLK1", "SOUTHREFCLK1"),
        ("QDCMREFCLK0_INT", "QDCMREFCLK_INT_0"),
        ("QDCMREFCLK1_INT", "QDCMREFCLK_INT_1"),
        (
            "QDPLL0CLK0P_INT",
            if cross_qdpll {
                "QDPLLCLK0P_1"
            } else {
                "QDPLLCLK0P_0"
            },
        ),
        (
            "QDPLL1CLK0P_INT",
            if cross_qdpll {
                "QDPLLCLK0P_0"
            } else {
                "QDPLLCLK0P_1"
            },
        ),
    ] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire(opin));
    }
    vrf.claim_pip(
        bel.crd(),
        bel.wire("RING_OSC_CLK_INT"),
        obel.wire(&format!("SARC_CLK{idx}")),
    );
}

fn verify_gt_common(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let grid = edev.grids[bel.die];
    let kind = if bel.key.starts_with("GTH") {
        match edev.kind {
            GridKind::Ultrascale => "GTHE3_COMMON",
            GridKind::UltrascalePlus => "GTHE4_COMMON",
        }
    } else if bel.key.starts_with("GTY") {
        match edev.kind {
            GridKind::Ultrascale => "GTYE3_COMMON",
            GridKind::UltrascalePlus => "GTYE4_COMMON",
        }
    } else {
        "GTF_COMMON"
    };
    let pins = [
        // from CHANNEL
        ("RXRECCLK0", SitePinDir::In),
        ("RXRECCLK1", SitePinDir::In),
        ("RXRECCLK2", SitePinDir::In),
        ("RXRECCLK3", SitePinDir::In),
        // to CHANNEL, broadcast
        ("QDCMREFCLK_INT_0", SitePinDir::Out),
        ("QDCMREFCLK_INT_1", SitePinDir::Out),
        ("QDPLLCLK0P_0", SitePinDir::Out),
        ("QDPLLCLK0P_1", SitePinDir::Out),
        ("MGTREFCLK0", SitePinDir::Out),
        ("MGTREFCLK1", SitePinDir::Out),
        // to CHANNEL, specific
        ("SARC_CLK0", SitePinDir::Out),
        ("SARC_CLK1", SitePinDir::Out),
        ("SARC_CLK2", SitePinDir::Out),
        ("SARC_CLK3", SitePinDir::Out),
        // to BUFG
        ("REFCLK2HROW0", SitePinDir::Out),
        ("REFCLK2HROW1", SitePinDir::Out),
        // from self and up/down
        ("COM0_REFCLKOUT0", SitePinDir::In),
        ("COM0_REFCLKOUT1", SitePinDir::In),
        ("COM0_REFCLKOUT2", SitePinDir::In),
        ("COM0_REFCLKOUT3", SitePinDir::In),
        ("COM0_REFCLKOUT4", SitePinDir::In),
        ("COM0_REFCLKOUT5", SitePinDir::In),
        ("COM2_REFCLKOUT0", SitePinDir::In),
        ("COM2_REFCLKOUT1", SitePinDir::In),
        ("COM2_REFCLKOUT2", SitePinDir::In),
        ("COM2_REFCLKOUT3", SitePinDir::In),
        ("COM2_REFCLKOUT4", SitePinDir::In),
        ("COM2_REFCLKOUT5", SitePinDir::In),
    ];
    let reg = grid.row_to_reg(bel.row);
    if !edev
        .disabled
        .contains(&DisabledPart::Gt(bel.die, bel.col, reg))
    {
        vrf.verify_bel(bel, kind, &pins, &[]);
    }
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }

    for i in 0..4 {
        let obel = vrf.find_bel_sibling(bel, &format!("{pref}_CHANNEL{i}", pref = &bel.key[..3]));
        vrf.claim_pip(
            bel.crd(),
            bel.wire(&format!("RXRECCLK{i}")),
            obel.wire("RXRECCLKOUT"),
        );
    }

    for (i, pin) in [
        (0, "MGTREFCLK0"),
        (1, "MGTREFCLK1"),
        (2, "NORTHREFCLK0"),
        (3, "NORTHREFCLK1"),
        (4, "SOUTHREFCLK0"),
        (5, "SOUTHREFCLK1"),
    ] {
        for j in [0, 2] {
            vrf.claim_pip(
                bel.crd(),
                bel.wire(&format!("COM{j}_REFCLKOUT{i}")),
                bel.wire(pin),
            );
        }
    }

    let obel_vcc = vrf.find_bel_sibling(bel, "VCC.GT");
    for pin in [
        "CLKOUT_NORTH0",
        "CLKOUT_NORTH1",
        "CLKOUT_SOUTH0",
        "CLKOUT_SOUTH1",
    ] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("MGTREFCLK0"));
        vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire("MGTREFCLK1"));
    }
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKOUT_NORTH0"),
        bel.wire("NORTHREFCLK0"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKOUT_NORTH1"),
        bel.wire("NORTHREFCLK1"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKOUT_SOUTH0"),
        bel.wire("SOUTHREFCLK0"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("CLKOUT_SOUTH1"),
        bel.wire("SOUTHREFCLK1"),
    );

    if let Some(obel_n) = vrf.find_bel_delta(bel, 0, 60, bel.key) {
        vrf.verify_node(&[bel.fwire("SOUTHREFCLK0"), obel_n.fwire("CLKOUT_SOUTH0")]);
        vrf.verify_node(&[bel.fwire("SOUTHREFCLK1"), obel_n.fwire("CLKOUT_SOUTH1")]);
        vrf.claim_node(&[bel.fwire("CLKOUT_NORTH0")]);
        vrf.claim_node(&[bel.fwire("CLKOUT_NORTH1")]);
    } else {
        vrf.claim_dummy_in(bel.fwire("SOUTHREFCLK0"));
        vrf.claim_dummy_in(bel.fwire("SOUTHREFCLK1"));
        vrf.claim_dummy_out(bel.fwire("CLKOUT_NORTH0"));
        vrf.claim_dummy_out(bel.fwire("CLKOUT_NORTH1"));
    }
    if let Some(obel_s) = vrf.find_bel_delta(bel, 0, -60, bel.key) {
        vrf.verify_node(&[bel.fwire("NORTHREFCLK0"), obel_s.fwire("CLKOUT_NORTH0")]);
        vrf.verify_node(&[bel.fwire("NORTHREFCLK1"), obel_s.fwire("CLKOUT_NORTH1")]);
        vrf.claim_node(&[bel.fwire("CLKOUT_SOUTH0")]);
        vrf.claim_node(&[bel.fwire("CLKOUT_SOUTH1")]);
    } else {
        vrf.claim_dummy_in(bel.fwire("NORTHREFCLK0"));
        vrf.claim_dummy_in(bel.fwire("NORTHREFCLK1"));
        vrf.claim_dummy_out(bel.fwire("CLKOUT_SOUTH0"));
        vrf.claim_dummy_out(bel.fwire("CLKOUT_SOUTH1"));
    }
}

fn verify_gtm_dual(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let grid = edev.grids[bel.die];
    let pins = [
        // BUFG_*
        ("CLK_BUFGT_CLK_IN_BOT0", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_BOT1", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_BOT2", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_BOT3", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_BOT4", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_BOT5", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_TOP0", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_TOP1", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_TOP2", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_TOP3", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_TOP4", SitePinDir::Out),
        ("CLK_BUFGT_CLK_IN_TOP5", SitePinDir::Out),
        // to/from GTM_REFCLK
        ("HROW_TEST_CK_SA", SitePinDir::Out),
        ("REFCLKPDB_SA", SitePinDir::Out),
        ("RXRECCLK0_INT", SitePinDir::Out),
        ("RXRECCLK1_INT", SitePinDir::Out),
        ("MGTREFCLK_CLEAN", SitePinDir::In),
        ("REFCLK2HROW", SitePinDir::In),
        // from s/n
        ("REFCLK_DIST2PLL0", SitePinDir::In),
        ("REFCLK_DIST2PLL1", SitePinDir::In),
        // dummy ins
        ("RCALSEL0", SitePinDir::In),
        ("RCALSEL1", SitePinDir::In),
    ];
    let reg = grid.row_to_reg(bel.row);
    if !edev
        .disabled
        .contains(&DisabledPart::Gt(bel.die, bel.col, reg))
    {
        vrf.verify_bel(bel, "GTM_DUAL", &pins, &[]);
    }
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel = vrf.find_bel_sibling(bel, "GTM_REFCLK");
    for (pin, opin) in [
        ("REFCLK2HROW", "REFCLK2HROW"),
        ("MGTREFCLK_CLEAN", "MGTREFCLK_CLEAN"),
    ] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire(opin));
    }
    if let Some(obel_n) = vrf.find_bel_delta(bel, 0, 60, bel.key) {
        vrf.verify_node(&[bel.fwire("REFCLK_DIST2PLL1"), obel_n.fwire("SOUTHCLKOUT")]);
    } else {
        vrf.claim_node(&[bel.fwire("NORTHCLKOUT")]);
    }
    if let Some(obel_s) = vrf.find_bel_delta(bel, 0, -60, bel.key) {
        vrf.verify_node(&[bel.fwire("REFCLK_DIST2PLL0"), obel_s.fwire("NORTHCLKOUT")]);
    } else {
        vrf.claim_node(&[bel.fwire("SOUTHCLKOUT")]);
    }
    vrf.claim_pip(
        bel.crd(),
        bel.wire("NORTHCLKOUT"),
        bel.wire("REFCLK_DIST2PLL0"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("NORTHCLKOUT"),
        obel.wire("MGTREFCLK_CLEAN"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("NORTHCLKOUT"),
        bel.wire("NORTHCLKOUT_DUMMY0"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("NORTHCLKOUT"),
        bel.wire("NORTHCLKOUT_DUMMY1"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("SOUTHCLKOUT"),
        bel.wire("REFCLK_DIST2PLL1"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("SOUTHCLKOUT"),
        obel.wire("MGTREFCLK_CLEAN"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("SOUTHCLKOUT"),
        bel.wire("SOUTHCLKOUT_DUMMY0"),
    );
    vrf.claim_pip(
        bel.crd(),
        bel.wire("SOUTHCLKOUT"),
        bel.wire("SOUTHCLKOUT_DUMMY1"),
    );
}

fn verify_gtm_refclk(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let grid = edev.grids[bel.die];
    let pins = [
        ("HROW_TEST_CK_FS", SitePinDir::In),
        ("REFCLKPDB_SA", SitePinDir::In),
        ("RXRECCLK0_INT", SitePinDir::In),
        ("RXRECCLK1_INT", SitePinDir::In),
        ("RXRECCLK2_INT", SitePinDir::In),
        ("RXRECCLK3_INT", SitePinDir::In),
        ("MGTREFCLK_CLEAN", SitePinDir::Out),
        ("REFCLK2HROW", SitePinDir::Out),
    ];
    let reg = grid.row_to_reg(bel.row);
    if !edev
        .disabled
        .contains(&DisabledPart::Gt(bel.die, bel.col, reg))
    {
        vrf.verify_bel(bel, "GTM_REFCLK", &pins, &[]);
    }
    for (pin, _) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
    }
    let obel = vrf.find_bel_sibling(bel, "GTM_DUAL");
    for (pin, opin) in [
        ("HROW_TEST_CK_FS", "HROW_TEST_CK_SA"),
        ("REFCLKPDB_SA", "REFCLKPDB_SA"),
        ("RXRECCLK0_INT", "RXRECCLK0_INT"),
        ("RXRECCLK1_INT", "RXRECCLK1_INT"),
    ] {
        vrf.claim_pip(bel.crd(), bel.wire(pin), obel.wire(opin));
    }
}

fn verify_hsadc_hsdac(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let grid = edev.grids[bel.die];
    let mut pins = vec![
        // to/from north/south
        ("SYSREF_IN_SOUTH_P", SitePinDir::In),
        ("SYSREF_IN_NORTH_P", SitePinDir::In),
        ("SYSREF_OUT_SOUTH_P", SitePinDir::Out),
        ("SYSREF_OUT_NORTH_P", SitePinDir::Out),
        // to BUFG_*
        ("PLL_DMON_OUT", SitePinDir::Out),
        ("PLL_REFCLK_OUT", SitePinDir::Out),
    ];
    // to BUFG_*
    if bel.key.ends_with("ADC") {
        pins.extend([
            ("CLK_ADC", SitePinDir::Out),
            ("CLK_ADC_SPARE", SitePinDir::Out),
        ]);
    } else {
        pins.extend([
            ("CLK_DAC", SitePinDir::Out),
            ("CLK_DAC_SPARE", SitePinDir::Out),
        ]);
    }
    // to/from north/south
    if bel.key.starts_with("RF") {
        pins.extend([
            ("CLK_DISTR_IN_NORTH", SitePinDir::In),
            ("CLK_DISTR_IN_SOUTH", SitePinDir::In),
            ("CLK_DISTR_OUT_NORTH", SitePinDir::Out),
            ("CLK_DISTR_OUT_SOUTH", SitePinDir::Out),
            ("T1_ALLOWED_NORTH", SitePinDir::In),
            ("T1_ALLOWED_SOUTH", SitePinDir::Out),
        ]);
    }
    let reg = grid.row_to_reg(bel.row);
    if !edev
        .disabled
        .contains(&DisabledPart::Gt(bel.die, bel.col, reg))
    {
        vrf.verify_bel(bel, bel.key, &pins, &[]);
    }
    for (pin, dir) in pins {
        vrf.claim_node(&[bel.fwire(pin)]);
        if dir == SitePinDir::In {
            vrf.claim_pip(bel.crd(), bel.wire(pin), bel.wire_far(pin));
        }
    }

    let okey = match bel.key {
        "HSADC" => "HSDAC",
        "HSDAC" => "HSADC",
        "RFADC" => "RFDAC",
        "RFDAC" => "RFADC",
        _ => unreachable!(),
    };

    if let Some(obel_n) = vrf
        .find_bel_delta(bel, 0, 60, bel.key)
        .or_else(|| vrf.find_bel_delta(bel, 0, 60, okey))
    {
        vrf.verify_node(&[
            bel.fwire_far("SYSREF_IN_NORTH_P"),
            obel_n.fwire("SYSREF_OUT_SOUTH_P"),
        ]);
        if bel.key.starts_with("RF") {
            if obel_n.key == bel.key {
                vrf.verify_node(&[
                    bel.fwire_far("CLK_DISTR_IN_NORTH"),
                    obel_n.fwire("CLK_DISTR_OUT_SOUTH"),
                ]);
            } else {
                vrf.claim_node(&[bel.fwire_far("CLK_DISTR_IN_NORTH")]);
            }
            vrf.verify_node(&[
                bel.fwire_far("T1_ALLOWED_NORTH"),
                obel_n.fwire("T1_ALLOWED_SOUTH"),
            ]);
        }
    } else {
        if grid.row_to_reg(bel.row).to_idx() == grid.regs - 1 {
            vrf.verify_node(&[
                bel.fwire_far("SYSREF_IN_NORTH_P"),
                bel.fwire("SYSREF_OUT_NORTH_P"),
            ]);
            if bel.key.starts_with("RF") {
                vrf.verify_node(&[
                    bel.fwire_far("CLK_DISTR_IN_NORTH"),
                    bel.fwire("CLK_DISTR_OUT_NORTH"),
                ]);
                vrf.claim_node(&[bel.fwire_far("T1_ALLOWED_NORTH")]);
            }
        } else {
            vrf.claim_node(&[bel.fwire_far("SYSREF_IN_NORTH_P")]);
        }
    }
    if let Some(obel_s) = vrf
        .find_bel_delta(bel, 0, -60, bel.key)
        .or_else(|| vrf.find_bel_delta(bel, 0, -60, okey))
    {
        vrf.verify_node(&[
            bel.fwire_far("SYSREF_IN_SOUTH_P"),
            obel_s.fwire("SYSREF_OUT_NORTH_P"),
        ]);
        if bel.key.starts_with("RF") {
            if obel_s.key == bel.key {
                vrf.verify_node(&[
                    bel.fwire_far("CLK_DISTR_IN_SOUTH"),
                    obel_s.fwire("CLK_DISTR_OUT_NORTH"),
                ]);
            } else {
                vrf.verify_node(&[
                    bel.fwire_far("CLK_DISTR_IN_SOUTH"),
                    bel.fwire("T1_ALLOWED_SOUTH"),
                ]);
            }
        }
    } else {
        vrf.verify_node(&[
            bel.fwire_far("SYSREF_IN_SOUTH_P"),
            bel.fwire("SYSREF_OUT_SOUTH_P"),
        ]);
        if bel.key.starts_with("RF") {
            vrf.verify_node(&[
                bel.fwire_far("CLK_DISTR_IN_SOUTH"),
                bel.fwire("CLK_DISTR_OUT_SOUTH"),
            ]);
        }
    }
}

fn verify_rclk_gt(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    let is_l = bel.key == "RCLK_GT_L";
    let obel_vcc = vrf.find_bel_sibling(bel, "VCC.GT");
    for i in 0..24 {
        let lr = if is_l { 'R' } else { 'L' };
        let hr = format!("HROUTE{i}_{lr}");
        let hd = format!("HDISTR{i}_{lr}");
        let obel = vrf.find_bel_sibling(bel, &format!("BUFG_GT{i}"));
        vrf.claim_pip(bel.crd(), bel.wire(&hr), obel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), bel.wire(&hr), obel.wire("CLK_OUT"));
        vrf.claim_pip(bel.crd(), bel.wire(&hd), obel_vcc.wire("VCC"));
        vrf.claim_pip(bel.crd(), bel.wire(&hd), obel.wire("CLK_OUT"));
    }
    if is_l {
        let obel_hd = find_hdistr_src(edev, vrf, bel.die, bel.col, bel.row, ColSide::Left);
        let obel_hr = find_hroute_src(edev, vrf, bel.die, bel.col, bel.row, ColSide::Left);
        for i in 0..24 {
            vrf.verify_node(&[
                bel.fwire(&format!("HDISTR{i}_R")),
                obel_hd.fwire(&format!("HDISTR{i}_L")),
            ]);
            vrf.verify_node(&[
                bel.fwire(&format!("HROUTE{i}_R")),
                obel_hr.fwire(&format!("HROUTE{i}_L")),
            ]);
        }
    } else {
        for i in 0..24 {
            vrf.claim_node(&[bel.fwire(&format!("HDISTR{i}_L"))]);
            vrf.claim_node(&[bel.fwire(&format!("HROUTE{i}_L"))]);
        }
    }
}

fn verify_bel(edev: &ExpandedDevice, vrf: &mut Verifier, bel: &BelContext<'_>) {
    match bel.key {
        "SLICE_L" | "SLICE_R" => verify_slice(edev, vrf, bel),
        "DSP0" | "DSP1" => verify_dsp(edev, vrf, bel),
        "BRAM_F" => verify_bram_f(edev, vrf, bel),
        "BRAM_H0" | "BRAM_H1" => verify_bram_h(vrf, bel),
        _ if bel.key.starts_with("HARD_SYNC") => vrf.verify_bel(bel, "HARD_SYNC", &[], &[]),
        _ if bel.key.starts_with("URAM") => verify_uram(vrf, bel),
        "LAGUNA0" | "LAGUNA1" | "LAGUNA2" | "LAGUNA3" => verify_laguna(edev, vrf, bel),
        "LAGUNA_EXTRA" => verify_laguna_extra(edev, vrf, bel),
        _ if bel.key.starts_with("VCC") => verify_vcc(vrf, bel),

        "PCIE" | "PCIE4" | "PCIE4C" => verify_pcie(vrf, bel),
        "CMAC" => verify_cmac(edev, vrf, bel),
        "ILKN" => verify_ilkn(edev, vrf, bel),
        "PMV"
        | "PMV2"
        | "PMVIOB"
        | "MTBF3"
        | "CFGIO_SITE"
        | "DFE_A"
        | "DFE_B"
        | "DFE_C"
        | "DFE_D"
        | "DFE_E"
        | "DFE_F"
        | "DFE_G"
        | "FE"
        | "BLI_HBM_APB_INTF"
        | "BLI_HBM_AXI_INTF"
        | "HDLOGIC_CSSD"
        | "HDIO_VREF"
        | "HDIO_BIAS"
        | "HPIO_ZMATCH_BLK_HCLK"
        | "HPIO_RCLK_PRBS" => vrf.verify_bel(bel, bel.key, &[], &[]),
        "CFG" => vrf.verify_bel(bel, "CONFIG_SITE", &[], &[]),
        "SYSMON" => verify_sysmon(edev, vrf, bel),
        "PS" => verify_ps(vrf, bel),
        "VCU" => verify_vcu(vrf, bel),
        _ if bel.key.starts_with("ABUS_SWITCH") => verify_abus_switch(edev, vrf, bel),
        _ if bel.key.starts_with("VBUS_SWITCH") => vrf.verify_bel(bel, "VBUS_SWITCH", &[], &[]),

        _ if bel.key.starts_with("BUFCE_LEAF_X16") => verify_bufce_leaf_x16(vrf, bel),
        _ if bel.key.starts_with("BUFCE_LEAF") => verify_bufce_leaf(vrf, bel),
        "RCLK_INT" => verify_rclk_int(edev, vrf, bel),

        "RCLK_SPLITTER" => verify_rclk_splitter(edev, vrf, bel),
        "RCLK_HROUTE_SPLITTER" => verify_rclk_hroute_splitter(edev, vrf, bel),

        _ if bel.key.starts_with("BUFCE_ROW_L") || bel.key.starts_with("BUFCE_ROW_R") => {
            verify_bufce_row(edev, vrf, bel)
        }
        _ if bel.key.starts_with("GCLK_TEST_BUF") => verify_gclk_test_buf(vrf, bel),

        _ if bel.key.starts_with("BUFG_PS") => verify_bufg_ps(vrf, bel),
        "RCLK_PS" => verify_rclk_ps(edev, vrf, bel),

        _ if bel.key.starts_with("HDIOB") => verify_hdiob(edev, vrf, bel),
        _ if bel.key.starts_with("HDIODIFFIN") => verify_hdiodiffin(edev, vrf, bel),
        _ if bel.key.starts_with("HDIOLOGIC") => verify_hdiologic(vrf, bel),
        _ if bel.key.starts_with("BUFGCE_HDIO") => verify_bufgce_hdio(vrf, bel),
        "RCLK_HDIO" => verify_rclk_hdio(edev, vrf, bel),

        _ if bel.key.starts_with("BUFCE_ROW_IO") => verify_bufce_row_io(vrf, bel),
        _ if bel.key.starts_with("BUFGCE_DIV") => verify_bufgce_div(vrf, bel),
        _ if bel.key.starts_with("BUFGCE") => verify_bufgce(edev, vrf, bel),
        _ if bel.key.starts_with("BUFGCTRL") => verify_bufgctrl(vrf, bel),
        "MMCM" => verify_mmcm(edev, vrf, bel),
        "PLL0" | "PLL1" => verify_pll(edev, vrf, bel),
        "HBM_REF_CLK0" | "HBM_REF_CLK1" => verify_hbm_ref_clk(vrf, bel),
        "CMT" => verify_cmt(edev, vrf, bel),

        _ if bel.key.starts_with("BITSLICE_RX_TX") => verify_bitslice_rx_tx(edev, vrf, bel),
        _ if bel.key.starts_with("BITSLICE_TX") => verify_bitslice_tx(edev, vrf, bel),
        _ if bel.key.starts_with("BITSLICE_CONTROL") => verify_bitslice_control(edev, vrf, bel),
        _ if bel.key.starts_with("PLL_SELECT") => verify_pll_select(edev, vrf, bel),
        _ if bel.key.starts_with("RIU_OR") => verify_riu_or(vrf, bel),
        _ if bel.key.starts_with("XIPHY_FEEDTHROUGH") => verify_xiphy_feedthrough(edev, vrf, bel),
        "XIPHY_BYTE" => verify_xiphy_byte(edev, vrf, bel),
        "RCLK_XIPHY" => verify_rclk_xiphy(edev, vrf, bel),

        _ if bel.key.starts_with("HPIOB") => verify_hpiob(edev, vrf, bel),
        _ if bel.key.starts_with("HPIODIFFIN") => verify_hpiodiffin(edev, vrf, bel),
        _ if bel.key.starts_with("HPIODIFFOUT") => verify_hpiodiffout(edev, vrf, bel),
        "HPIO_VREF" => verify_hpio_vref(vrf, bel),
        "HPIO_BIAS" => vrf.verify_bel(bel, "BIAS", &[], &[]),
        _ if bel.key.starts_with("HPIO_DCI") => verify_hpio_dci(edev, vrf, bel),

        _ if bel.key.starts_with("HRIOB") => verify_hriob(edev, vrf, bel),
        _ if bel.key.starts_with("HRIODIFFIN") => verify_hriodiffin(vrf, bel),
        _ if bel.key.starts_with("HRIODIFFOUT") => verify_hriodiffout(vrf, bel),

        _ if bel.key.starts_with("BUFG_GT_SYNC") => verify_bufg_gt_sync(edev, vrf, bel),
        _ if bel.key.starts_with("BUFG_GT") => verify_bufg_gt(edev, vrf, bel),
        _ if bel.key.starts_with("GTH_CHANNEL")
            || bel.key.starts_with("GTY_CHANNEL")
            || bel.key.starts_with("GTF_CHANNEL") =>
        {
            verify_gt_channel(edev, vrf, bel)
        }
        "GTH_COMMON" | "GTY_COMMON" | "GTF_COMMON" => verify_gt_common(edev, vrf, bel),
        "GTM_DUAL" => verify_gtm_dual(edev, vrf, bel),
        "GTM_REFCLK" => verify_gtm_refclk(edev, vrf, bel),
        "HSADC" | "HSDAC" | "RFADC" | "RFDAC" => verify_hsadc_hsdac(edev, vrf, bel),
        _ if bel.key.starts_with("RCLK_GT") => verify_rclk_gt(edev, vrf, bel),

        _ => println!("MEOW {} {:?}", bel.key, bel.name),
    }
}

fn verify_extra(edev: &ExpandedDevice, vrf: &mut Verifier) {
    // XXX
    //vrf.skip_residual_pips();
    //vrf.skip_residual_nodes();
    for w in [
        "CLK_VDISTR_FT0",
        "CLK_VROUTE_FT0",
        "CLK_VDISTR_FT0_0",
        "CLK_VROUTE_FT0_0",
        "CLK_VDISTR_FT0_1",
        "CLK_VROUTE_FT0_1",
        "CASMBIST12IN",
        "SYNC_CLK_B_TOP0",
        "SYNC_CLK_B_TOP1",
        "SYNC_CLK_B_TOP2",
        "SYNC_CLK_B_TOP3",
        "SYNC_CLK_TOP0",
        "SYNC_CLK_TOP1",
        "SYNC_CLK_TOP2",
        "SYNC_CLK_TOP3",
        "SYNC_DIN_TOP0",
        "SYNC_DIN_TOP1",
        "SYNC_DIN_TOP2",
        "SYNC_DIN_TOP3",
        "SYNC_SR_TOP0",
        "SYNC_SR_TOP1",
        "SYNC_SR_TOP2",
        "SYNC_SR_TOP3",
        "SYNC_DOUT_BOT0",
        "SYNC_DOUT_BOT1",
        "SYNC_DOUT_BOT2",
        "SYNC_DOUT_BOT3",
        "SYNC_DOUT_TERM0",
        "SYNC_DOUT_TERM1",
        "SYNC_DOUT_TERM2",
        "SYNC_DOUT_TERM3",
        "CLOCK_DR_FT0",
        "SHIFT_DR_FT0",
        "UPDATE_DR_FT0",
        "UPDATE_DR_FT1",
        "EXTEST_FT0",
        "EXTEST_FT1",
        "INTEST_FT0",
        "INTEST_FT1",
        "MISR_JTAG_LOAD_FT0",
        "AC_MODE_FT0",
        "RESET_TAP_B_FT0",
        "FST_CFG_B_FT0",
        "FST_CFG_B_FT1",
        "GPWRDWN_B_FT0",
        "GPWRDWN_B_FT1",
        "GTS_CFG_B_FT0",
        "GTS_CFG_B_FT1",
        "GTS_USR_B_FT0",
        "GTS_USR_B_FT1",
        "POR_B_FT0",
        "POR_B_FT1",
        "CLOCK_DR_IN",
        "SHIFT_DR_IN",
        "RESET_TAP_BP",
        "IO_TO_CTR_FT0_0",
        "IO_TO_CTR_FT0_1",
        "IO_TO_CTR_FT0_2",
        "IO_TO_CTR_FT0_3",
        "REMOTE_DIODE_FN_FT0_0",
        "REMOTE_DIODE_FN_FT0_1",
        "REMOTE_DIODE_FN_FT0_2",
        "REMOTE_DIODE_FP_FT0",
        "REMOTE_DIODE_SN_FT0_0",
        "REMOTE_DIODE_SN_FT0_1",
        "REMOTE_DIODE_SN_FT0_2",
        "REMOTE_DIODE_SP_FT0",
        "DIODE_N_OPT",
        "DIODE_P_OPT",
    ] {
        vrf.kill_stub_in_cond(w);
    }
    if edev.kind == GridKind::Ultrascale {
        for i in 0..104 {
            vrf.kill_stub_in_cond_tk("INT_TERM_L_IO", &format!("GND_WIRE{i}"));
            vrf.kill_stub_in_cond_tk("HPIO_TERM_L", &format!("GND_WIRE{i}"));
            vrf.kill_stub_in_cond_tk("HPIO_K3_TERM_L_FT", &format!("GND_WIRE{i}"));
            vrf.kill_stub_in_cond_tk("HPIO_VH_TERM_L_FT", &format!("GND_WIRE{i}"));
        }
        for i in 0..4 {
            vrf.kill_stub_in_cond_tk("INT_IBRK_LEFT_L_FT", &format!("GND_WIRE{i}"));
        }
        vrf.kill_stub_in_cond_tk("CFRM_L_TERM_T", "GND_WIRE0");
        vrf.kill_stub_in_cond_tk("CFRM_L_TERM_T", "GND_WIRE2");
    } else {
        vrf.kill_stub_in_cond_tk("RCLK_AMS_CFGIO", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("RCLK_AMS_CFGIO", "VCC_WIRE2");
        vrf.kill_stub_in_cond_tk("RCLK_AMS_CFGIO", "VCC_WIRE3");
        vrf.kill_stub_in_cond_tk("CFG_CONFIG", "VCC_WIRE2");
        vrf.kill_stub_in_cond_tk("CFG_CONFIG", "VCC_WIRE3");
        vrf.kill_stub_in_cond_tk("CFG_CONFIG", "VCC_WIRE4");
        vrf.kill_stub_in_cond_tk("RCLK_INT_L", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("RCLK_INT_L", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("RCLK_CLEL_L_L", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("RCLK_CLEL_L_L", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("RCLK_CLEL_R_L", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("RCLK_CLEL_R_L", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("RCLK_CLEM_L", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("RCLK_CLEM_L", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("RCLK_CLEM_CLKBUF_L", "VCC_WIRE4");
        vrf.kill_stub_in_cond_tk("RCLK_HDIO", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("RCLK_HDIO", "VCC_WIRE1");
        vrf.kill_stub_in_cond_tk("HPIO_RIGHT_TERM_T", "VCC_WIRE");
        vrf.kill_stub_in_cond_tk("HPIO_RIGHT_TERM_T", "GND_WIRE1");
        vrf.kill_stub_in_cond_tk("HPIO_RIGHT_TERM_T", "GND_WIRE3");
        vrf.kill_stub_in_cond_tk("HDIO_BOT_RIGHT", "VCC_WIRE0");
        vrf.kill_stub_in_cond_tk("HDIO_BOT_RIGHT", "VCC_WIRE2");
        vrf.kill_stub_in_cond_tk("HDIO_BOT_RIGHT", "VCC_WIRE4");
        vrf.kill_stub_in_cond_tk("HDIO_BOT_RIGHT", "VCC_WIRE6");
        vrf.kill_stub_in_cond_tk("HDIO_BOT_RIGHT", "GND_WIRE0");
        vrf.kill_stub_in_cond_tk("HDIO_BOT_RIGHT", "GND_WIRE2");
        vrf.kill_stub_in_cond_tk("HDIO_BOT_RIGHT", "GND_WIRE5");
    }
}

pub fn verify_device(edev: &ExpandedDevice, rd: &Part) {
    verify(
        rd,
        &edev.egrid,
        |_| (),
        |vrf, bel| verify_bel(edev, vrf, bel),
        |vrf| verify_extra(edev, vrf),
    );
}
