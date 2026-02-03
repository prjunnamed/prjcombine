use prjcombine_re_hammer::Session;
use prjcombine_virtex4::defs;

use crate::{backend::IseBackend, collector::CollectorCtx, generic::fbuild::FuzzCtx};

const EMAC_BOOL_ATTRS: &[&str] = &[
    "EMAC_1000BASEX_ENABLE",
    "EMAC_USECLKEN",
    "EMAC_UNIDIRECTION_ENABLE",
    "EMAC_TX16BITCLIENT_ENABLE",
    "EMAC_TXVLAN_ENABLE",
    "EMAC_TXRESET",
    "EMAC_TXJUMBOFRAME_ENABLE",
    "EMAC_TXINBANDFCS_ENABLE",
    "EMAC_TXIFGADJUST_ENABLE",
    "EMAC_TXHALFDUPLEX",
    "EMAC_TXFLOWCTRL_ENABLE",
    "EMAC_TX_ENABLE",
    "EMAC_SPEED_MSB",
    "EMAC_SPEED_LSB",
    "EMAC_SGMII_ENABLE",
    "EMAC_RX16BITCLIENT_ENABLE",
    "EMAC_RXVLAN_ENABLE",
    "EMAC_RXRESET",
    "EMAC_RXJUMBOFRAME_ENABLE",
    "EMAC_RXINBANDFCS_ENABLE",
    "EMAC_RXHALFDUPLEX",
    "EMAC_RXFLOWCTRL_ENABLE",
    "EMAC_RX_ENABLE",
    "EMAC_RGMII_ENABLE",
    "EMAC_PHYRESET",
    "EMAC_PHYPOWERDOWN",
    "EMAC_PHYLOOPBACKMSB",
    "EMAC_PHYISOLATE",
    "EMAC_PHYINITAUTONEG_ENABLE",
    "EMAC_MDIO_IGNORE_PHYADZERO",
    "EMAC_MDIO_ENABLE",
    "EMAC_LTCHECK_DISABLE",
    "EMAC_HOST_ENABLE",
    "EMAC_GTLOOPBACK",
    "EMAC_CTRLLENCHECK_DISABLE",
    "EMAC_CONFIGVEC_79",
    "EMAC_BYTEPHY",
    "EMAC_ADDRFILTER_ENABLE",
];

const EMAC_HEX_ATTRS: &[(&str, usize)] = &[
    ("EMAC_DCRBASEADDR", 8),
    ("EMAC_FUNCTION", 3),
    ("EMAC_LINKTIMERVAL", 9),
    ("EMAC_PAUSEADDR", 48),
    ("EMAC_UNICASTADDR", 48),
];

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let Some(mut ctx) = FuzzCtx::try_new_legacy(session, backend, "EMAC") else {
        return;
    };
    let mut bctx = ctx.bel(defs::bslots::EMAC);
    let mode = "TEMAC_SINGLE";

    bctx.test_manual_legacy("PRESENT", "1").mode(mode).commit();

    for &attr in EMAC_BOOL_ATTRS {
        bctx.mode(mode).test_enum_legacy(attr, &["FALSE", "TRUE"]);
    }
    for &(attr, width) in EMAC_HEX_ATTRS {
        bctx.mode(mode).test_multi_attr_hex_legacy(attr, width);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    if !ctx.has_tile_legacy("EMAC") {
        return;
    }
    let tile = "EMAC";
    let bel = "EMAC";
    ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
        .assert_empty();
    for &attr in EMAC_BOOL_ATTRS {
        if attr == "EMAC_MDIO_IGNORE_PHYADZERO" {
            ctx.get_diff_legacy(tile, bel, attr, "FALSE").assert_empty();
            ctx.get_diff_legacy(tile, bel, attr, "TRUE").assert_empty();
        } else {
            ctx.collect_bit_bi_legacy(tile, bel, attr, "FALSE", "TRUE");
        }
    }
    for &(attr, _) in EMAC_HEX_ATTRS {
        ctx.collect_bitvec_legacy(tile, bel, attr, "");
    }
}
