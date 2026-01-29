use prjcombine_re_hammer::Session;
use prjcombine_virtex4::defs;

use crate::{backend::IseBackend, collector::CollectorCtx, generic::fbuild::FuzzCtx};

const PCIE_INVPINS: &[&str] = &[
    "CRMCORECLK",
    "CRMCORECLKDLO",
    "CRMCORECLKRXO",
    "CRMCORECLKTXO",
    "CRMUSERCLK",
    "CRMUSERCLKRXO",
    "CRMUSERCLKTXO",
];

const PCIE_BOOL_ATTRS: &[&str] = &[
    "AERCAPABILITYECRCCHECKCAPABLE",
    "AERCAPABILITYECRCGENCAPABLE",
    "BAR0EXIST",
    "BAR0PREFETCHABLE",
    "BAR1EXIST",
    "BAR1PREFETCHABLE",
    "BAR2EXIST",
    "BAR2PREFETCHABLE",
    "BAR3EXIST",
    "BAR3PREFETCHABLE",
    "BAR4EXIST",
    "BAR4PREFETCHABLE",
    "BAR5EXIST",
    "BAR5PREFETCHABLE",
    "CLKDIVIDED",
    "DUALCOREENABLE",
    "DUALCORESLAVE",
    "INFINITECOMPLETIONS",
    "ISSWITCH",
    "LINKSTATUSSLOTCLOCKCONFIG",
    "LLKBYPASS",
    "PBCAPABILITYSYSTEMALLOCATED",
    "PCIECAPABILITYSLOTIMPL",
    "PMCAPABILITYD1SUPPORT",
    "PMCAPABILITYD2SUPPORT",
    "PMCAPABILITYDSI",
    "RAMSHARETXRX",
    "RESETMODE",
    "RETRYREADADDRPIPE",
    "RETRYREADDATAPIPE",
    "RETRYWRITEPIPE",
    "RXREADADDRPIPE",
    "RXREADDATAPIPE",
    "RXWRITEPIPE",
    "SELECTASMODE",
    "SELECTDLLIF",
    "SLOTCAPABILITYATTBUTTONPRESENT",
    "SLOTCAPABILITYATTINDICATORPRESENT",
    "SLOTCAPABILITYHOTPLUGCAPABLE",
    "SLOTCAPABILITYHOTPLUGSURPRISE",
    "SLOTCAPABILITYMSLSENSORPRESENT",
    "SLOTCAPABILITYPOWERCONTROLLERPRESENT",
    "SLOTCAPABILITYPOWERINDICATORPRESENT",
    "SLOTIMPLEMENTED",
    "TXREADADDRPIPE",
    "TXREADDATAPIPE",
    "TXWRITEPIPE",
    "UPSTREAMFACING",
    "XLINKSUPPORTED",
];

const PCIE_HEX_ATTRS: &[(&str, usize)] = &[
    ("BAR0ADDRWIDTH", 1),
    ("BAR0IOMEMN", 1),
    ("BAR1ADDRWIDTH", 1),
    ("BAR1IOMEMN", 1),
    ("BAR2ADDRWIDTH", 1),
    ("BAR2IOMEMN", 1),
    ("BAR3ADDRWIDTH", 1),
    ("BAR3IOMEMN", 1),
    ("BAR4ADDRWIDTH", 1),
    ("BAR4IOMEMN", 1),
    ("BAR5ADDRWIDTH", 1),
    ("BAR5IOMEMN", 1),
    ("DUALROLECFGCNTRLROOTEPN", 1),
    ("L0SEXITLATENCY", 3),
    ("L0SEXITLATENCYCOMCLK", 3),
    ("L1EXITLATENCY", 3),
    ("L1EXITLATENCYCOMCLK", 3),
    ("LOWPRIORITYVCCOUNT", 3),
    ("PCIEREVISION", 1),
    ("PMDATASCALE0", 2),
    ("PMDATASCALE1", 2),
    ("PMDATASCALE2", 2),
    ("PMDATASCALE3", 2),
    ("PMDATASCALE4", 2),
    ("PMDATASCALE5", 2),
    ("PMDATASCALE6", 2),
    ("PMDATASCALE7", 2),
    ("PMDATASCALE8", 2),
    ("RETRYRAMREADLATENCY", 3),
    ("RETRYRAMWIDTH", 1),
    ("RETRYRAMWRITELATENCY", 3),
    ("TLRAMREADLATENCY", 3),
    ("TLRAMWIDTH", 1),
    ("TLRAMWRITELATENCY", 3),
    ("XPMAXPAYLOAD", 3),
    ("XPRCBCONTROL", 1),
    ("ACTIVELANESIN", 8),
    ("AERBASEPTR", 12),
    ("AERCAPABILITYNEXTPTR", 12),
    ("DSNBASEPTR", 12),
    ("DSNCAPABILITYNEXTPTR", 12),
    ("MSIBASEPTR", 12),
    ("MSICAPABILITYNEXTPTR", 8),
    ("MSICAPABILITYMULTIMSGCAP", 3),
    ("PBBASEPTR", 12),
    ("PBCAPABILITYNEXTPTR", 12),
    ("PBCAPABILITYDW0BASEPOWER", 8),
    ("PBCAPABILITYDW0DATASCALE", 2),
    ("PBCAPABILITYDW0PMSTATE", 2),
    ("PBCAPABILITYDW0PMSUBSTATE", 3),
    ("PBCAPABILITYDW0POWERRAIL", 3),
    ("PBCAPABILITYDW0TYPE", 3),
    ("PBCAPABILITYDW1BASEPOWER", 8),
    ("PBCAPABILITYDW1DATASCALE", 2),
    ("PBCAPABILITYDW1PMSTATE", 2),
    ("PBCAPABILITYDW1PMSUBSTATE", 3),
    ("PBCAPABILITYDW1POWERRAIL", 3),
    ("PBCAPABILITYDW1TYPE", 3),
    ("PBCAPABILITYDW2BASEPOWER", 8),
    ("PBCAPABILITYDW2DATASCALE", 2),
    ("PBCAPABILITYDW2PMSTATE", 2),
    ("PBCAPABILITYDW2PMSUBSTATE", 3),
    ("PBCAPABILITYDW2POWERRAIL", 3),
    ("PBCAPABILITYDW2TYPE", 3),
    ("PBCAPABILITYDW3BASEPOWER", 8),
    ("PBCAPABILITYDW3DATASCALE", 2),
    ("PBCAPABILITYDW3PMSTATE", 2),
    ("PBCAPABILITYDW3PMSUBSTATE", 3),
    ("PBCAPABILITYDW3POWERRAIL", 3),
    ("PBCAPABILITYDW3TYPE", 3),
    ("PCIECAPABILITYNEXTPTR", 8),
    ("PCIECAPABILITYINTMSGNUM", 5),
    ("PMBASEPTR", 12),
    ("PMCAPABILITYNEXTPTR", 8),
    ("PMCAPABILITYAUXCURRENT", 3),
    ("PMCAPABILITYPMESUPPORT", 5),
    ("PMDATA0", 8),
    ("PMDATA1", 8),
    ("PMDATA2", 8),
    ("PMDATA3", 8),
    ("PMDATA4", 8),
    ("PMDATA5", 8),
    ("PMDATA6", 8),
    ("PMDATA7", 8),
    ("PMDATA8", 8),
    ("PMSTATUSCONTROLDATASCALE", 2),
    ("VCBASEPTR", 12),
    ("VCCAPABILITYNEXTPTR", 12),
    ("VC0RXFIFOBASEC", 13),
    ("VC0RXFIFOBASENP", 13),
    ("VC0RXFIFOBASEP", 13),
    ("VC0RXFIFOLIMITC", 13),
    ("VC0RXFIFOLIMITNP", 13),
    ("VC0RXFIFOLIMITP", 13),
    ("VC0TXFIFOBASEC", 13),
    ("VC0TXFIFOBASENP", 13),
    ("VC0TXFIFOBASEP", 13),
    ("VC0TXFIFOLIMITC", 13),
    ("VC0TXFIFOLIMITNP", 13),
    ("VC0TXFIFOLIMITP", 13),
    ("VC0TOTALCREDITSCD", 11),
    ("VC0TOTALCREDITSPD", 11),
    ("VC0TOTALCREDITSCH", 7),
    ("VC0TOTALCREDITSNPH", 7),
    ("VC0TOTALCREDITSPH", 7),
    ("VC1RXFIFOBASEC", 13),
    ("VC1RXFIFOBASENP", 13),
    ("VC1RXFIFOBASEP", 13),
    ("VC1RXFIFOLIMITC", 13),
    ("VC1RXFIFOLIMITNP", 13),
    ("VC1RXFIFOLIMITP", 13),
    ("VC1TXFIFOBASEC", 13),
    ("VC1TXFIFOBASENP", 13),
    ("VC1TXFIFOBASEP", 13),
    ("VC1TXFIFOLIMITC", 13),
    ("VC1TXFIFOLIMITNP", 13),
    ("VC1TXFIFOLIMITP", 13),
    ("VC1TOTALCREDITSCD", 11),
    ("VC1TOTALCREDITSPD", 11),
    ("VC1TOTALCREDITSCH", 7),
    ("VC1TOTALCREDITSNPH", 7),
    ("VC1TOTALCREDITSPH", 7),
    ("XPBASEPTR", 8),
    ("XPDEVICEPORTTYPE", 4),
    ("EXTCFGXPCAPPTR", 12),
    ("CAPABILITIESPOINTER", 8),
    ("EXTCFGCAPPTR", 8),
    ("HEADERTYPE", 8),
    ("INTERRUPTPIN", 8),
    ("BAR0MASKWIDTH", 6),
    ("BAR1MASKWIDTH", 6),
    ("BAR2MASKWIDTH", 6),
    ("BAR3MASKWIDTH", 6),
    ("BAR4MASKWIDTH", 6),
    ("BAR5MASKWIDTH", 6),
    ("CARDBUSCISPOINTER", 32),
    ("DEVICEID", 16),
    ("VENDORID", 16),
    ("SUBSYSTEMID", 16),
    ("SUBSYSTEMVENDORID", 16),
    ("REVISIONID", 8),
    ("CLASSCODE", 24),
    ("CONFIGROUTING", 3),
    ("DEVICECAPABILITYENDPOINTL0SLATENCY", 3),
    ("DEVICECAPABILITYENDPOINTL1LATENCY", 3),
    ("LINKCAPABILITYASPMSUPPORT", 2),
    ("LINKCAPABILITYMAXLINKWIDTH", 6),
    ("PORTVCCAPABILITYEXTENDEDVCCOUNT", 3),
    ("PORTVCCAPABILITYVCARBCAP", 8),
    ("PORTVCCAPABILITYVCARBTABLEOFFSET", 8),
    ("SLOTCAPABILITYPHYSICALSLOTNUM", 13),
    ("SLOTCAPABILITYSLOTPOWERLIMITSCALE", 2),
    ("SLOTCAPABILITYSLOTPOWERLIMITVALUE", 8),
    ("DEVICESERIALNUMBER", 64),
    ("RETRYRAMSIZE", 12),
];

const PCIE_DEC_ATTRS: &[(&str, usize)] = &[("TXTSNFTS", 8), ("TXTSNFTSCOMCLK", 8)];

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, "PCIE") else {
        return;
    };
    let mut bctx = ctx.bel(defs::bslots::PCIE);
    let mode = "PCIE";

    bctx.test_manual("PRESENT", "1").mode(mode).commit();

    for &pin in PCIE_INVPINS {
        bctx.mode(mode).test_inv(pin);
    }
    for &attr in PCIE_BOOL_ATTRS {
        bctx.mode(mode).test_enum(attr, &["FALSE", "TRUE"]);
    }
    for &(attr, width) in PCIE_HEX_ATTRS {
        bctx.mode(mode).test_multi_attr_hex_legacy(attr, width);
    }
    for &(attr, width) in PCIE_DEC_ATTRS {
        bctx.mode(mode).test_multi_attr_dec(attr, width);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    if !ctx.has_tile("PCIE") {
        return;
    }
    let tile = "PCIE";
    let bel = "PCIE";
    ctx.get_diff_legacy(tile, bel, "PRESENT", "1")
        .assert_empty();
    for &pin in PCIE_INVPINS {
        ctx.collect_inv(tile, bel, pin);
    }
    for &attr in PCIE_BOOL_ATTRS {
        if attr == "CLKDIVIDED" {
            ctx.get_diff_legacy(tile, bel, attr, "FALSE").assert_empty();
            ctx.get_diff_legacy(tile, bel, attr, "TRUE").assert_empty();
        } else {
            ctx.collect_bit_bi_legacy(tile, bel, attr, "FALSE", "TRUE");
        }
    }
    for &(attr, _) in PCIE_HEX_ATTRS {
        ctx.collect_bitvec_legacy(tile, bel, attr, "");
    }
    for &(attr, _) in PCIE_DEC_ATTRS {
        ctx.collect_bitvec_legacy(tile, bel, attr, "");
    }
}
