use prjcombine_re_hammer::Session;

use crate::{
    backend::IseBackend, diff::CollectorCtx, fgen::TileBits, fuzz::FuzzCtx, fuzz_enum,
    fuzz_multi_attr_bin, fuzz_multi_attr_dec, fuzz_multi_attr_hex, fuzz_one,
};

pub fn add_fuzzers<'a>(session: &mut Session<IseBackend<'a>>, backend: &IseBackend<'a>) {
    let Some(ctx) = FuzzCtx::try_new(session, backend, "PCIE", "PCIE", TileBits::MainAuto) else {
        return;
    };
    fuzz_one!(ctx, "PRESENT", "1", [], [(mode "PCIE_A1")]);

    for attr in [
        "DEV_CAP_EXT_TAG_SUPPORTED",
        "DEV_CAP_ROLE_BASED_ERROR",
        "DISABLE_BAR_FILTERING",
        "DISABLE_ID_CHECK",
        "DISABLE_SCRAMBLING",
        "ENABLE_RX_TD_ECRC_TRIM",
        "FAST_TRAIN",
        "LINK_STATUS_SLOT_CLOCK_CONFIG",
        "LL_ACK_TIMEOUT_EN",
        "LL_REPLAY_TIMEOUT_EN",
        "PCIE_CAP_SLOT_IMPLEMENTED",
        "PLM_AUTO_CONFIG",
        "PM_CAP_DSI",
        "PM_CAP_D1SUPPORT",
        "PM_CAP_D2SUPPORT",
        "PM_CAP_PME_CLOCK",
        "SLOT_CAP_ATT_BUTTON_PRESENT",
        "SLOT_CAP_ATT_INDICATOR_PRESENT",
        "SLOT_CAP_POWER_INDICATOR_PRESENT",
        "TL_TFC_DISABLE",
        "TL_TX_CHECKS_DISABLE",
        "USR_CFG",
        "USR_EXT_CFG",
        "VC0_CPL_INFINITE",
    ] {
        fuzz_enum!(ctx, attr, ["FALSE", "TRUE"], [(mode "PCIE_A1")]);
    }

    for (attr, width) in [
        ("BAR0", 32),
        ("BAR1", 32),
        ("BAR2", 32),
        ("BAR3", 32),
        ("BAR4", 32),
        ("BAR5", 32),
        ("CARDBUS_CIS_POINTER", 32),
        ("CLASS_CODE", 24),
        ("EXPANSION_ROM", 22),
        ("LL_ACK_TIMEOUT", 15),
        ("LL_REPLAY_TIMEOUT", 15),
        ("PCIE_CAP_CAPABILITY_VERSION", 4),
        ("PCIE_CAP_DEVICE_PORT_TYPE", 4),
        ("PCIE_GENERIC", 12),
        ("PM_DATA0", 8),
        ("PM_DATA1", 8),
        ("PM_DATA2", 8),
        ("PM_DATA3", 8),
        ("PM_DATA4", 8),
        ("PM_DATA5", 8),
        ("PM_DATA6", 8),
        ("PM_DATA7", 8),
        ("VC0_RX_RAM_LIMIT", 12),
    ] {
        fuzz_multi_attr_hex!(ctx, attr, width, [(mode "PCIE_A1")]);
    }
    for (attr, width) in [
        ("DEV_CAP_ENDPOINT_L0S_LATENCY", 3),
        ("DEV_CAP_ENDPOINT_L1_LATENCY", 3),
        ("DEV_CAP_MAX_PAYLOAD_SUPPORTED", 3),
        ("DEV_CAP_PHANTOM_FUNCTIONS_SUPPORT", 2),
        ("GTP_SEL", 1),
        ("LINK_CAP_ASPM_SUPPORT", 2),
        ("LINK_CAP_L0S_EXIT_LATENCY", 3),
        ("LINK_CAP_L1_EXIT_LATENCY", 3),
        ("MSI_CAP_MULTIMSG_EXTENSION", 1),
        ("MSI_CAP_MULTIMSGCAP", 3),
        ("PM_CAP_AUXCURRENT", 3),
        ("PM_CAP_VERSION", 3),
        ("TL_RX_RAM_RADDR_LATENCY", 1),
        ("TL_RX_RAM_RDATA_LATENCY", 2),
        ("TL_RX_RAM_WRITE_LATENCY", 1),
        ("TL_TX_RAM_RADDR_LATENCY", 1),
        ("TL_TX_RAM_RDATA_LATENCY", 2),
        ("VC0_TOTAL_CREDITS_CD", 11),
        ("VC0_TOTAL_CREDITS_CH", 7),
        ("VC0_TOTAL_CREDITS_NPH", 7),
        ("VC0_TOTAL_CREDITS_PD", 11),
        ("VC0_TOTAL_CREDITS_PH", 7),
        ("VC0_TX_LASTPACKET", 5),
    ] {
        fuzz_multi_attr_dec!(ctx, attr, width, [(mode "PCIE_A1")]);
    }
    for (attr, width) in [
        ("PCIE_CAP_INT_MSG_NUM", 5),
        ("PM_CAP_PMESUPPORT", 5),
        ("PM_DATA_SCALE0", 2),
        ("PM_DATA_SCALE1", 2),
        ("PM_DATA_SCALE2", 2),
        ("PM_DATA_SCALE3", 2),
        ("PM_DATA_SCALE4", 2),
        ("PM_DATA_SCALE5", 2),
        ("PM_DATA_SCALE6", 2),
        ("PM_DATA_SCALE7", 2),
    ] {
        fuzz_multi_attr_bin!(ctx, attr, width, [(mode "PCIE_A1")]);
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let egrid = ctx.edev.egrid();
    let node = egrid.db.get_node("PCIE");
    if egrid.node_index[node].is_empty() {
        return;
    }
    let tile = "PCIE";
    let bel = "PCIE";
    ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
    for attr in [
        "DEV_CAP_EXT_TAG_SUPPORTED",
        "DEV_CAP_ROLE_BASED_ERROR",
        "DISABLE_BAR_FILTERING",
        "DISABLE_ID_CHECK",
        "DISABLE_SCRAMBLING",
        "ENABLE_RX_TD_ECRC_TRIM",
        "FAST_TRAIN",
        "LINK_STATUS_SLOT_CLOCK_CONFIG",
        "LL_ACK_TIMEOUT_EN",
        "LL_REPLAY_TIMEOUT_EN",
        "PCIE_CAP_SLOT_IMPLEMENTED",
        "PLM_AUTO_CONFIG",
        "PM_CAP_DSI",
        "PM_CAP_D1SUPPORT",
        "PM_CAP_D2SUPPORT",
        "PM_CAP_PME_CLOCK",
        "SLOT_CAP_ATT_BUTTON_PRESENT",
        "SLOT_CAP_ATT_INDICATOR_PRESENT",
        "SLOT_CAP_POWER_INDICATOR_PRESENT",
        "TL_TFC_DISABLE",
        "TL_TX_CHECKS_DISABLE",
        "USR_CFG",
        "USR_EXT_CFG",
        "VC0_CPL_INFINITE",
    ] {
        ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
    }
    for attr in [
        "BAR0",
        "BAR1",
        "BAR2",
        "BAR3",
        "BAR4",
        "BAR5",
        "CARDBUS_CIS_POINTER",
        "CLASS_CODE",
        "EXPANSION_ROM",
        "LL_ACK_TIMEOUT",
        "LL_REPLAY_TIMEOUT",
        "PCIE_CAP_CAPABILITY_VERSION",
        "PCIE_CAP_DEVICE_PORT_TYPE",
        "PCIE_GENERIC",
        "PM_DATA0",
        "PM_DATA1",
        "PM_DATA2",
        "PM_DATA3",
        "PM_DATA4",
        "PM_DATA5",
        "PM_DATA6",
        "PM_DATA7",
        "VC0_RX_RAM_LIMIT",
        "DEV_CAP_ENDPOINT_L0S_LATENCY",
        "DEV_CAP_ENDPOINT_L1_LATENCY",
        "DEV_CAP_MAX_PAYLOAD_SUPPORTED",
        "DEV_CAP_PHANTOM_FUNCTIONS_SUPPORT",
        "GTP_SEL",
        "LINK_CAP_ASPM_SUPPORT",
        "LINK_CAP_L0S_EXIT_LATENCY",
        "LINK_CAP_L1_EXIT_LATENCY",
        "MSI_CAP_MULTIMSG_EXTENSION",
        "MSI_CAP_MULTIMSGCAP",
        "PM_CAP_AUXCURRENT",
        "PM_CAP_VERSION",
        "TL_RX_RAM_RADDR_LATENCY",
        "TL_RX_RAM_RDATA_LATENCY",
        "TL_RX_RAM_WRITE_LATENCY",
        "TL_TX_RAM_RADDR_LATENCY",
        "TL_TX_RAM_RDATA_LATENCY",
        "VC0_TOTAL_CREDITS_CD",
        "VC0_TOTAL_CREDITS_CH",
        "VC0_TOTAL_CREDITS_NPH",
        "VC0_TOTAL_CREDITS_PD",
        "VC0_TOTAL_CREDITS_PH",
        "VC0_TX_LASTPACKET",
        "PCIE_CAP_INT_MSG_NUM",
        "PM_CAP_PMESUPPORT",
        "PM_DATA_SCALE0",
        "PM_DATA_SCALE1",
        "PM_DATA_SCALE2",
        "PM_DATA_SCALE3",
        "PM_DATA_SCALE4",
        "PM_DATA_SCALE5",
        "PM_DATA_SCALE6",
        "PM_DATA_SCALE7",
    ] {
        ctx.collect_bitvec(tile, bel, attr, "");
    }
}
