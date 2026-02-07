use prjcombine_interconnect::db::BelAttributeType;
use prjcombine_re_hammer::Session;
use prjcombine_spartan6::defs::{bcls, bslots, tcls};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
    spartan6::specials,
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, tcls::PCIE) else {
        return;
    };
    let mut bctx = ctx.bel(bslots::PCIE);
    let mode = "PCIE_A1";
    bctx.build()
        .null_bits()
        .test_bel_special(specials::PRESENT)
        .mode(mode)
        .commit();

    for (aid, _, attr) in &backend.edev.db[bcls::PCIE].attributes {
        match attr.typ {
            BelAttributeType::Bool => {
                bctx.mode(mode)
                    .test_bel_attr_bool_auto(aid, "FALSE", "TRUE");
            }
            BelAttributeType::BitVec(_) => match aid {
                bcls::PCIE::PCIE_CAP_INT_MSG_NUM
                | bcls::PCIE::PM_CAP_PMESUPPORT
                | bcls::PCIE::PM_DATA_SCALE0
                | bcls::PCIE::PM_DATA_SCALE1
                | bcls::PCIE::PM_DATA_SCALE2
                | bcls::PCIE::PM_DATA_SCALE3
                | bcls::PCIE::PM_DATA_SCALE4
                | bcls::PCIE::PM_DATA_SCALE5
                | bcls::PCIE::PM_DATA_SCALE6
                | bcls::PCIE::PM_DATA_SCALE7 => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Bin);
                }
                bcls::PCIE::BAR0
                | bcls::PCIE::BAR1
                | bcls::PCIE::BAR2
                | bcls::PCIE::BAR3
                | bcls::PCIE::BAR4
                | bcls::PCIE::BAR5
                | bcls::PCIE::CARDBUS_CIS_POINTER
                | bcls::PCIE::CLASS_CODE
                | bcls::PCIE::EXPANSION_ROM
                | bcls::PCIE::LL_ACK_TIMEOUT
                | bcls::PCIE::LL_REPLAY_TIMEOUT
                | bcls::PCIE::PCIE_CAP_CAPABILITY_VERSION
                | bcls::PCIE::PCIE_CAP_DEVICE_PORT_TYPE
                | bcls::PCIE::PCIE_GENERIC
                | bcls::PCIE::PM_DATA0
                | bcls::PCIE::PM_DATA1
                | bcls::PCIE::PM_DATA2
                | bcls::PCIE::PM_DATA3
                | bcls::PCIE::PM_DATA4
                | bcls::PCIE::PM_DATA5
                | bcls::PCIE::PM_DATA6
                | bcls::PCIE::PM_DATA7
                | bcls::PCIE::VC0_RX_RAM_LIMIT => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Hex(0));
                }
                bcls::PCIE::DEV_CAP_ENDPOINT_L0S_LATENCY
                | bcls::PCIE::DEV_CAP_ENDPOINT_L1_LATENCY
                | bcls::PCIE::DEV_CAP_MAX_PAYLOAD_SUPPORTED
                | bcls::PCIE::DEV_CAP_PHANTOM_FUNCTIONS_SUPPORT
                | bcls::PCIE::GTP_SEL
                | bcls::PCIE::LINK_CAP_ASPM_SUPPORT
                | bcls::PCIE::LINK_CAP_L0S_EXIT_LATENCY
                | bcls::PCIE::LINK_CAP_L1_EXIT_LATENCY
                | bcls::PCIE::MSI_CAP_MULTIMSG_EXTENSION
                | bcls::PCIE::MSI_CAP_MULTIMSGCAP
                | bcls::PCIE::PM_CAP_AUXCURRENT
                | bcls::PCIE::PM_CAP_VERSION
                | bcls::PCIE::TL_RX_RAM_RADDR_LATENCY
                | bcls::PCIE::TL_RX_RAM_RDATA_LATENCY
                | bcls::PCIE::TL_RX_RAM_WRITE_LATENCY
                | bcls::PCIE::TL_TX_RAM_RADDR_LATENCY
                | bcls::PCIE::TL_TX_RAM_RDATA_LATENCY
                | bcls::PCIE::VC0_TOTAL_CREDITS_CD
                | bcls::PCIE::VC0_TOTAL_CREDITS_CH
                | bcls::PCIE::VC0_TOTAL_CREDITS_NPH
                | bcls::PCIE::VC0_TOTAL_CREDITS_PD
                | bcls::PCIE::VC0_TOTAL_CREDITS_PH
                | bcls::PCIE::VC0_TX_LASTPACKET => {
                    bctx.mode(mode).test_bel_attr_multi(aid, MultiValue::Dec(0));
                }
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let tcid = tcls::PCIE;
    let bslot = bslots::PCIE;
    if !ctx.has_tcls(tcid) {
        return;
    }
    for (aid, _, attr) in &ctx.edev.db[bcls::PCIE].attributes {
        match attr.typ {
            BelAttributeType::Bool => {
                ctx.collect_bel_attr_bi(tcid, bslot, aid);
            }
            BelAttributeType::BitVec(_) => {
                ctx.collect_bel_attr(tcid, bslot, aid);
            }
            _ => unreachable!(),
        }
    }
}
