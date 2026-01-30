use prjcombine_entity::EntityId;
use prjcombine_interconnect::dir::DirH;
use prjcombine_interconnect::grid::BelCoord;
use prjcombine_re_xilinx_naming::db::RawTileId;
use prjcombine_re_xilinx_naming_virtex2::ExpandedNamedDevice;
use prjcombine_re_xilinx_rdverify::{SitePinDir, Verifier};
use prjcombine_virtex2::chip::{ChipKind, IoDiffKind};
use prjcombine_virtex2::defs::bslots;
use prjcombine_virtex2::iob::IobKind;

use crate::get_bel_iob;

pub fn verify_ioi(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let idx = bslots::IOI.index_of(bcrd.slot).unwrap();
    let bel = &vrf.get_legacy_bel(bcrd);
    let io = endev.chip.get_io_crd(bel.bel);
    let io_info = endev.chip.get_io_info(io);
    if io_info.pad_kind == Some(IobKind::Clk) {
        vrf.verify_legacy_bel(
            bel,
            ["CLK_P", "CLK_N"][io.iob().to_idx() % 2],
            &[("I", SitePinDir::Out)],
            &[],
        );
        vrf.claim_net(&[bel.wire("I")]);
        vrf.claim_net(&[bel.wire_far("I")]);
        vrf.claim_pip(bel.wire_far("I"), bel.wire("I"));
    } else {
        let tn = &bel.ntile.names[RawTileId::from_idx(0)];
        let is_ipad = tn.contains("IBUFS") || (tn.contains("IOIB") && idx == 2);
        let kind = if matches!(
            endev.chip.kind,
            ChipKind::Spartan3A | ChipKind::Spartan3ADsp
        ) {
            let is_tb = matches!(io_info.bank, 0 | 2);
            match (io_info.diff, is_ipad) {
                (IoDiffKind::P(_), false) => {
                    if is_tb {
                        "DIFFMTB"
                    } else {
                        "DIFFMLR"
                    }
                }
                (IoDiffKind::P(_), true) => "DIFFMI_NDT",
                (IoDiffKind::N(_), false) => {
                    if is_tb {
                        "DIFFSTB"
                    } else {
                        "DIFFSLR"
                    }
                }
                (IoDiffKind::N(_), true) => "DIFFSI_NDT",
                (IoDiffKind::None, false) => "IOB",
                (IoDiffKind::None, true) => "IBUF",
            }
        } else {
            match (io_info.diff, is_ipad) {
                (IoDiffKind::P(_), false) => "DIFFM",
                (IoDiffKind::P(_), true) => "DIFFMI",
                (IoDiffKind::N(_), false) => "DIFFS",
                (IoDiffKind::N(_), true) => "DIFFSI",
                (IoDiffKind::None, false) => "IOB",
                (IoDiffKind::None, true) => "IBUF",
            }
        };
        let mut pins = vec![
            ("PADOUT", SitePinDir::Out),
            ("DIFFI_IN", SitePinDir::In),
            ("DIFFO_OUT", SitePinDir::Out),
            ("DIFFO_IN", SitePinDir::In),
        ];
        if matches!(
            endev.chip.kind,
            ChipKind::Spartan3E | ChipKind::Spartan3A | ChipKind::Spartan3ADsp
        ) {
            pins.extend([
                ("PCI_RDY", SitePinDir::Out),
                ("PCI_CE", SitePinDir::In),
                ("ODDROUT1", SitePinDir::Out),
                ("ODDROUT2", SitePinDir::Out),
                ("ODDRIN1", SitePinDir::In),
                ("ODDRIN2", SitePinDir::In),
                ("IDDRIN1", SitePinDir::In),
                ("IDDRIN2", SitePinDir::In),
            ]);
        }
        if endev.chip.kind == ChipKind::Spartan3ADsp {
            pins.extend([("OAUX", SitePinDir::In), ("TAUX", SitePinDir::In)]);
        }
        vrf.verify_legacy_bel(bel, kind, &pins, &["CLKPAD"]);
        // diff pairing
        if !endev.chip.kind.is_virtex2() || io_info.diff != IoDiffKind::None {
            for pin in ["PADOUT", "DIFFI_IN", "DIFFO_IN", "DIFFO_OUT"] {
                vrf.claim_net(&[bel.wire(pin)]);
            }
            match io_info.diff {
                IoDiffKind::P(oiob) => {
                    let obel = get_bel_iob(endev, vrf, io.with_iob(oiob));
                    vrf.claim_pip(bel.wire("DIFFI_IN"), obel.wire("PADOUT"));
                }
                IoDiffKind::N(oiob) => {
                    let obel = get_bel_iob(endev, vrf, io.with_iob(oiob));
                    vrf.claim_pip(bel.wire("DIFFI_IN"), obel.wire("PADOUT"));
                    vrf.claim_pip(bel.wire("DIFFO_IN"), obel.wire("DIFFO_OUT"));
                }
                IoDiffKind::None => (),
            }
        }
        if matches!(
            endev.chip.kind,
            ChipKind::Spartan3E | ChipKind::Spartan3A | ChipKind::Spartan3ADsp
        ) {
            for pin in [
                "ODDRIN1", "ODDRIN2", "ODDROUT1", "ODDROUT2", "IDDRIN1", "IDDRIN2", "PCI_CE",
                "PCI_RDY",
            ] {
                vrf.claim_net(&[bel.wire(pin)]);
            }
            // ODDR, IDDR
            if let IoDiffKind::P(oiob) | IoDiffKind::N(oiob) = io_info.diff {
                let obel = get_bel_iob(endev, vrf, io.with_iob(oiob));
                vrf.claim_pip(bel.wire("ODDRIN1"), obel.wire("ODDROUT2"));
                vrf.claim_pip(bel.wire("ODDRIN2"), obel.wire("ODDROUT1"));
                vrf.claim_pip(bel.wire("IDDRIN1"), obel.wire("IQ1"));
                vrf.claim_pip(bel.wire("IDDRIN2"), obel.wire("IQ2"));
            }
            vrf.claim_pip(bel.wire("PCI_CE"), bel.wire_far("PCI_CE"));
            let scol = if bel.col < endev.chip.col_clk {
                endev.chip.col_w()
            } else {
                endev.chip.col_e()
            };
            let obel = bel
                .cell
                .with_cr(scol, endev.chip.row_mid())
                .bel(bslots::PCILOGICSE);
            vrf.verify_net(&[vrf.bel_pip_owire(obel, "PCI_CE", 0), bel.wire_far("PCI_CE")]);
        }
        if endev.chip.kind == ChipKind::Spartan3ADsp {
            for pin in ["OAUX", "TAUX"] {
                vrf.claim_net(&[bel.wire(pin)]);
            }
        }
        let is_clock = if endev.chip.kind.is_virtex2() {
            bcrd.col == endev.chip.col_clk - 1 || bcrd.col == endev.chip.col_clk
        } else if !endev.chip.kind.is_spartan3ea() {
            (bcrd.col == endev.chip.col_clk - 1 || bcrd.col == endev.chip.col_clk) && idx < 2
        } else {
            (0..8).any(|idx| endev.chip.get_clk_io(io.edge(), idx) == Some(io))
        };
        if is_clock {
            vrf.claim_pip(bel.wire("CLKPAD"), bel.wire("I"));
        }
    }
}

pub fn verify_pcilogicse(endev: &ExpandedNamedDevice, vrf: &mut Verifier, bcrd: BelCoord) {
    let mut bel = vrf
        .verify_bel(bcrd)
        .extra_in("IRDY")
        .extra_in("TRDY")
        .extra_out("PCI_CE");
    let edge = if bcrd.col == endev.chip.col_w() {
        DirH::W
    } else if bcrd.col == endev.chip.col_e() {
        DirH::E
    } else {
        unreachable!()
    };
    let pci_rdy = endev.chip.get_pci_io(edge);
    for (pin, crd) in ["IRDY", "TRDY"].into_iter().zip(pci_rdy) {
        bel.claim_net(&[bel.wire(pin)]);
        bel.claim_pip(bel.wire(pin), bel.wire_far(pin));
        let obel = endev.chip.get_io_loc(crd);
        bel.claim_net(&[bel.wire_far(pin), bel.bel_wire(obel, "PCI_RDY_IN")]);
        bel.claim_pip(
            bel.bel_wire(obel, "PCI_RDY_IN"),
            bel.bel_wire(obel, "PCI_RDY"),
        );
    }
    let (wt, wf) = bel.pip("PCI_CE", 0);
    bel.claim_net(&[bel.wire("PCI_CE"), wf]);
    bel.claim_pip(wt, wf);
    bel.claim_net(&[wt]);
    bel.commit();
}
