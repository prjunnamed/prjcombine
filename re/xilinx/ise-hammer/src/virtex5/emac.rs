use prjcombine_re_hammer::Session;
use prjcombine_virtex4::defs::{self, virtex5::tcls};

use crate::{backend::IseBackend, collector::CollectorCtx, generic::fbuild::FuzzCtx};

const EMAC_INVPINS: &[&str] = &[
    "CLIENTEMAC0RXCLIENTCLKIN",
    "CLIENTEMAC0TXCLIENTCLKIN",
    "CLIENTEMAC1RXCLIENTCLKIN",
    "CLIENTEMAC1TXCLIENTCLKIN",
    "DCREMACCLK",
    "HOSTCLK",
    "PHYEMAC0GTXCLK",
    "PHYEMAC0MCLKIN",
    "PHYEMAC0MIITXCLK",
    "PHYEMAC0RXCLK",
    "PHYEMAC0TXGMIIMIICLKIN",
    "PHYEMAC1GTXCLK",
    "PHYEMAC1MCLKIN",
    "PHYEMAC1MIITXCLK",
    "PHYEMAC1RXCLK",
    "PHYEMAC1TXGMIIMIICLKIN",
];

const EMAC_BOOL_ATTRS: &[&str] = &[
    "EMAC0_RXHALFDUPLEX",
    "EMAC0_RXVLAN_ENABLE",
    "EMAC0_RX_ENABLE",
    "EMAC0_RXINBANDFCS_ENABLE",
    "EMAC0_RXJUMBOFRAME_ENABLE",
    "EMAC0_RXRESET",
    "EMAC0_TXIFGADJUST_ENABLE",
    "EMAC0_TXHALFDUPLEX",
    "EMAC0_TXVLAN_ENABLE",
    "EMAC0_TX_ENABLE",
    "EMAC0_TXINBANDFCS_ENABLE",
    "EMAC0_TXJUMBOFRAME_ENABLE",
    "EMAC0_TXRESET",
    "EMAC0_TXFLOWCTRL_ENABLE",
    "EMAC0_RXFLOWCTRL_ENABLE",
    "EMAC0_LTCHECK_DISABLE",
    "EMAC0_ADDRFILTER_ENABLE",
    "EMAC0_RX16BITCLIENT_ENABLE",
    "EMAC0_TX16BITCLIENT_ENABLE",
    "EMAC0_HOST_ENABLE",
    "EMAC0_1000BASEX_ENABLE",
    "EMAC0_SGMII_ENABLE",
    "EMAC0_RGMII_ENABLE",
    "EMAC0_SPEED_LSB",
    "EMAC0_SPEED_MSB",
    "EMAC0_MDIO_ENABLE",
    "EMAC0_PHYLOOPBACKMSB",
    "EMAC0_PHYPOWERDOWN",
    "EMAC0_PHYISOLATE",
    "EMAC0_PHYINITAUTONEG_ENABLE",
    "EMAC0_PHYRESET",
    "EMAC0_CONFIGVEC_79",
    "EMAC0_UNIDIRECTION_ENABLE",
    "EMAC0_GTLOOPBACK",
    "EMAC0_BYTEPHY",
    "EMAC0_USECLKEN",
    "EMAC1_RXHALFDUPLEX",
    "EMAC1_RXVLAN_ENABLE",
    "EMAC1_RX_ENABLE",
    "EMAC1_RXINBANDFCS_ENABLE",
    "EMAC1_RXJUMBOFRAME_ENABLE",
    "EMAC1_RXRESET",
    "EMAC1_TXIFGADJUST_ENABLE",
    "EMAC1_TXHALFDUPLEX",
    "EMAC1_TXVLAN_ENABLE",
    "EMAC1_TX_ENABLE",
    "EMAC1_TXINBANDFCS_ENABLE",
    "EMAC1_TXJUMBOFRAME_ENABLE",
    "EMAC1_TXRESET",
    "EMAC1_TXFLOWCTRL_ENABLE",
    "EMAC1_RXFLOWCTRL_ENABLE",
    "EMAC1_LTCHECK_DISABLE",
    "EMAC1_ADDRFILTER_ENABLE",
    "EMAC1_RX16BITCLIENT_ENABLE",
    "EMAC1_TX16BITCLIENT_ENABLE",
    "EMAC1_HOST_ENABLE",
    "EMAC1_1000BASEX_ENABLE",
    "EMAC1_SGMII_ENABLE",
    "EMAC1_RGMII_ENABLE",
    "EMAC1_SPEED_LSB",
    "EMAC1_SPEED_MSB",
    "EMAC1_MDIO_ENABLE",
    "EMAC1_PHYLOOPBACKMSB",
    "EMAC1_PHYPOWERDOWN",
    "EMAC1_PHYISOLATE",
    "EMAC1_PHYINITAUTONEG_ENABLE",
    "EMAC1_PHYRESET",
    "EMAC1_CONFIGVEC_79",
    "EMAC1_UNIDIRECTION_ENABLE",
    "EMAC1_GTLOOPBACK",
    "EMAC1_BYTEPHY",
    "EMAC1_USECLKEN",
];

const EMAC_HEX_ATTRS: &[(&str, usize)] = &[
    ("EMAC0_DCRBASEADDR", 8),
    ("EMAC0_FUNCTION", 3),
    ("EMAC0_LINKTIMERVAL", 9),
    ("EMAC0_PAUSEADDR", 48),
    ("EMAC0_UNICASTADDR", 48),
    ("EMAC1_DCRBASEADDR", 8),
    ("EMAC1_FUNCTION", 3),
    ("EMAC1_LINKTIMERVAL", 9),
    ("EMAC1_PAUSEADDR", 48),
    ("EMAC1_UNICASTADDR", 48),
];

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::EMAC) else {
        return;
    };
    let mut bctx = ctx.bel(defs::bslots::EMAC);
    let mode = "TEMAC";
    bctx.test_manual_legacy("PRESENT", "1").mode(mode).commit();

    for &pin in EMAC_INVPINS {
        bctx.mode(mode).test_inv_legacy(pin);
    }
    for &attr in EMAC_BOOL_ATTRS {
        bctx.mode(mode).test_enum_legacy(attr, &["FALSE", "TRUE"]);
    }
    for &(attr, width) in EMAC_HEX_ATTRS {
        bctx.mode(mode).test_multi_attr_hex_legacy(attr, width);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    if !ctx.has_tcls(tcls::EMAC) {
        return;
    }
    let tile = "EMAC";
    let bel = "EMAC";
    ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
        .assert_empty();
    for &pin in EMAC_INVPINS {
        ctx.collect_inv_legacy(tile, bel, pin);
    }
    for &attr in EMAC_BOOL_ATTRS {
        ctx.collect_bit_bi_legacy(tile, bel, attr, "FALSE", "TRUE");
    }
    for &(attr, _) in EMAC_HEX_ATTRS {
        ctx.collect_bitvec_legacy(tile, bel, attr, "");
    }
}
