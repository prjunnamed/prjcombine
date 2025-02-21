use prjcombine_re_hammer::Session;

use crate::{
    backend::IseBackend, diff::CollectorCtx, fgen::TileBits, fuzz::FuzzCtx, fuzz_enum,
    fuzz_multi_attr_hex, fuzz_one,
};

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

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let Some(ctx) = FuzzCtx::try_new(session, backend, "EMAC", "EMAC", TileBits::MainAuto) else {
        return;
    };

    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "TEMAC_SINGLE")]);

    for &attr in EMAC_BOOL_ATTRS {
        fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [(mode "TEMAC_SINGLE")]);
    }
    for &(attr, width) in EMAC_HEX_ATTRS {
        fuzz_multi_attr_hex!(ctx, attr, width, [(mode "TEMAC_SINGLE")]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    if !ctx.has_tile("EMAC") {
        return;
    }
    let tile = "EMAC";
    let bel = "EMAC";
    ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
    for &attr in EMAC_BOOL_ATTRS {
        if attr == "EMAC_MDIO_IGNORE_PHYADZERO" {
            ctx.state.get_diff(tile, bel, attr, "FALSE").assert_empty();
            ctx.state.get_diff(tile, bel, attr, "TRUE").assert_empty();
        } else {
            ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
        }
    }
    for &(attr, _) in EMAC_HEX_ATTRS {
        ctx.collect_bitvec(tile, bel, attr, "");
    }
}
