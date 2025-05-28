use prjcombine_re_hammer::Session;
use prjcombine_types::bsdata::{TileBit, TileItem};
use prjcombine_virtex4::bels;

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::relation::Delta,
    },
};

const PCIE_BOOL_ATTRS: &[&str] = &[
    "VSEC_CAP_ON",
    "VSEC_CAP_IS_LINK_VISIBLE",
    "VC0_CPL_INFINITE",
    "VC_CAP_REJECT_SNOOP_TRANSACTIONS",
    "VC_CAP_ON",
    "UR_INV_REQ",
    "UPSTREAM_FACING",
    "UPCONFIG_CAPABLE",
    "TL_TX_CHECKS_DISABLE",
    "TL_TFC_DISABLE",
    "TL_RBYPASS",
    "TEST_MODE_PIN_CHAR",
    "SLOT_CAP_POWER_INDICATOR_PRESENT",
    "SLOT_CAP_POWER_CONTROLLER_PRESENT",
    "SLOT_CAP_NO_CMD_COMPLETED_SUPPORT",
    "SLOT_CAP_MRL_SENSOR_PRESENT",
    "SLOT_CAP_HOTPLUG_SURPRISE",
    "SLOT_CAP_HOTPLUG_CAPABLE",
    "SLOT_CAP_ELEC_INTERLOCK_PRESENT",
    "SLOT_CAP_ATT_INDICATOR_PRESENT",
    "SLOT_CAP_ATT_BUTTON_PRESENT",
    "SELECT_DLL_IF",
    "ROOT_CAP_CRS_SW_VISIBILITY",
    "RECRC_CHK_TRIM",
    "PM_CSR_NOSOFTRST",
    "PM_CSR_B2B3",
    "PM_CSR_BPCCEN",
    "PM_CAP_PME_CLOCK",
    "PM_CAP_ON",
    "PM_CAP_D2SUPPORT",
    "PM_CAP_D1SUPPORT",
    "PM_CAP_DSI",
    "PL_FAST_TRAIN",
    "PCIE_CAP_SLOT_IMPLEMENTED",
    "PCIE_CAP_ON",
    "MSIX_CAP_ON",
    "MSI_CAP_64_BIT_ADDR_CAPABLE",
    "MSI_CAP_PER_VECTOR_MASKING_CAPABLE",
    "MSI_CAP_ON",
    "LL_REPLAY_TIMEOUT_EN",
    "LL_ACK_TIMEOUT_EN",
    "LINK_STATUS_SLOT_CLOCK_CONFIG",
    "LINK_CTRL2_HW_AUTONOMOUS_SPEED_DISABLE",
    "LINK_CTRL2_DEEMPHASIS",
    "LINK_CAP_SURPRISE_DOWN_ERROR_CAPABLE",
    "LINK_CAP_LINK_BANDWIDTH_NOTIFICATION_CAP",
    "LINK_CAP_DLL_LINK_ACTIVE_REPORTING_CAP",
    "LINK_CAP_CLOCK_POWER_MANAGEMENT",
    "IS_SWITCH",
    "EXIT_LOOPBACK_ON_EI",
    "ENTER_RVRY_EI_L0",
    "ENABLE_RX_TD_ECRC_TRIM",
    "DSN_CAP_ON",
    "DISABLE_SCRAMBLING",
    "DISABLE_RX_TC_FILTER",
    "DISABLE_LANE_REVERSAL",
    "DISABLE_ID_CHECK",
    "DISABLE_BAR_FILTERING",
    "DISABLE_ASPM_L1_TIMER",
    "DEV_CONTROL_AUX_POWER_SUPPORTED",
    "DEV_CAP_ROLE_BASED_ERROR",
    "DEV_CAP_FUNCTION_LEVEL_RESET_CAPABLE",
    "DEV_CAP_EXT_TAG_SUPPORTED",
    "DEV_CAP_ENABLE_SLOT_PWR_LIMIT_VALUE",
    "DEV_CAP_ENABLE_SLOT_PWR_LIMIT_SCALE",
    "CPL_TIMEOUT_DISABLE_SUPPORTED",
    "CMD_INTX_IMPLEMENTED",
    "ALLOW_X8_GEN2",
    "AER_CAP_PERMIT_ROOTERR_UPDATE",
    "AER_CAP_ON",
    "AER_CAP_ECRC_GEN_CAPABLE",
    "AER_CAP_ECRC_CHECK_CAPABLE",
];

const PCIE_HEX_ATTRS: &[(&str, usize)] = &[
    ("AER_BASE_PTR", 12),
    ("AER_CAP_ID", 16),
    ("AER_CAP_INT_MSG_NUM_MSI", 5),
    ("AER_CAP_INT_MSG_NUM_MSIX", 5),
    ("AER_CAP_NEXTPTR", 12),
    ("AER_CAP_VERSION", 4),
    ("BAR0", 32),
    ("BAR1", 32),
    ("BAR2", 32),
    ("BAR3", 32),
    ("BAR4", 32),
    ("BAR5", 32),
    ("CAPABILITIES_PTR", 8),
    ("CARDBUS_CIS_POINTER", 32),
    ("CLASS_CODE", 24),
    ("CPL_TIMEOUT_RANGES_SUPPORTED", 4),
    ("CRM_MODULE_RSTS", 7),
    ("DEVICE_ID", 16),
    ("DEV_CAP_ENDPOINT_L0S_LATENCY", 3),
    ("DEV_CAP_ENDPOINT_L1_LATENCY", 3),
    ("DEV_CAP_MAX_PAYLOAD_SUPPORTED", 3),
    ("DEV_CAP_PHANTOM_FUNCTIONS_SUPPORT", 2),
    ("DEV_CAP_RSVD_14_12", 3),
    ("DEV_CAP_RSVD_17_16", 2),
    ("DEV_CAP_RSVD_31_29", 3),
    ("DNSTREAM_LINK_NUM", 8),
    ("DSN_BASE_PTR", 12),
    ("DSN_CAP_ID", 16),
    ("DSN_CAP_NEXTPTR", 12),
    ("DSN_CAP_VERSION", 4),
    ("ENABLE_MSG_ROUTE", 11),
    ("EXPANSION_ROM", 32),
    ("EXT_CFG_CAP_PTR", 6),
    ("EXT_CFG_XP_CAP_PTR", 10),
    ("HEADER_TYPE", 8),
    ("INFER_EI", 5),
    ("INTERRUPT_PIN", 8),
    ("LAST_CONFIG_DWORD", 10),
    ("LINK_CAP_ASPM_SUPPORT", 2),
    ("LINK_CAP_L0S_EXIT_LATENCY_COMCLK_GEN1", 3),
    ("LINK_CAP_L0S_EXIT_LATENCY_COMCLK_GEN2", 3),
    ("LINK_CAP_L0S_EXIT_LATENCY_GEN1", 3),
    ("LINK_CAP_L0S_EXIT_LATENCY_GEN2", 3),
    ("LINK_CAP_L1_EXIT_LATENCY_COMCLK_GEN1", 3),
    ("LINK_CAP_L1_EXIT_LATENCY_COMCLK_GEN2", 3),
    ("LINK_CAP_L1_EXIT_LATENCY_GEN1", 3),
    ("LINK_CAP_L1_EXIT_LATENCY_GEN2", 3),
    ("LINK_CAP_MAX_LINK_SPEED", 4),
    ("LINK_CAP_MAX_LINK_WIDTH", 6),
    ("LINK_CAP_RSVD_23_22", 2),
    ("LINK_CONTROL_RCB", 1),
    ("LINK_CTRL2_TARGET_LINK_SPEED", 4),
    ("LL_ACK_TIMEOUT", 15),
    ("LL_ACK_TIMEOUT_FUNC", 2),
    ("LL_REPLAY_TIMEOUT", 15),
    ("LL_REPLAY_TIMEOUT_FUNC", 2),
    ("LTSSM_MAX_LINK_WIDTH", 6),
    ("MSIX_BASE_PTR", 8),
    ("MSIX_CAP_ID", 8),
    ("MSIX_CAP_NEXTPTR", 8),
    ("MSIX_CAP_PBA_BIR", 3),
    ("MSIX_CAP_PBA_OFFSET", 29),
    ("MSIX_CAP_TABLE_BIR", 3),
    ("MSIX_CAP_TABLE_OFFSET", 29),
    ("MSIX_CAP_TABLE_SIZE", 11),
    ("MSI_BASE_PTR", 8),
    ("MSI_CAP_ID", 8),
    ("MSI_CAP_MULTIMSGCAP", 3),
    ("MSI_CAP_MULTIMSG_EXTENSION", 1),
    ("MSI_CAP_NEXTPTR", 8),
    ("PCIE_BASE_PTR", 8),
    ("PCIE_CAP_CAPABILITY_ID", 8),
    ("PCIE_CAP_CAPABILITY_VERSION", 4),
    ("PCIE_CAP_DEVICE_PORT_TYPE", 4),
    ("PCIE_CAP_INT_MSG_NUM", 5),
    ("PCIE_CAP_NEXTPTR", 8),
    ("PCIE_CAP_RSVD_15_14", 2),
    ("PGL0_LANE", 3),
    ("PGL1_LANE", 3),
    ("PGL2_LANE", 3),
    ("PGL3_LANE", 3),
    ("PGL4_LANE", 3),
    ("PGL5_LANE", 3),
    ("PGL6_LANE", 3),
    ("PGL7_LANE", 3),
    ("PL_AUTO_CONFIG", 3),
    ("PM_BASE_PTR", 8),
    ("PM_CAP_AUXCURRENT", 3),
    ("PM_CAP_ID", 8),
    ("PM_CAP_NEXTPTR", 8),
    ("PM_CAP_PMESUPPORT", 5),
    ("PM_CAP_RSVD_04", 1),
    ("PM_CAP_VERSION", 3),
    ("PM_DATA0", 8),
    ("PM_DATA1", 8),
    ("PM_DATA2", 8),
    ("PM_DATA3", 8),
    ("PM_DATA4", 8),
    ("PM_DATA5", 8),
    ("PM_DATA6", 8),
    ("PM_DATA7", 8),
    ("PM_DATA_SCALE0", 2),
    ("PM_DATA_SCALE1", 2),
    ("PM_DATA_SCALE2", 2),
    ("PM_DATA_SCALE3", 2),
    ("PM_DATA_SCALE4", 2),
    ("PM_DATA_SCALE5", 2),
    ("PM_DATA_SCALE6", 2),
    ("PM_DATA_SCALE7", 2),
    ("RECRC_CHK", 2),
    ("REVISION_ID", 8),
    ("SLOT_CAP_PHYSICAL_SLOT_NUM", 13),
    ("SLOT_CAP_SLOT_POWER_LIMIT_SCALE", 2),
    ("SLOT_CAP_SLOT_POWER_LIMIT_VALUE", 8),
    ("SPARE_BIT0", 1),
    ("SPARE_BIT1", 1),
    ("SPARE_BIT2", 1),
    ("SPARE_BIT3", 1),
    ("SPARE_BIT4", 1),
    ("SPARE_BIT5", 1),
    ("SPARE_BIT6", 1),
    ("SPARE_BIT7", 1),
    ("SPARE_BIT8", 1),
    ("SPARE_BYTE0", 8),
    ("SPARE_BYTE1", 8),
    ("SPARE_BYTE2", 8),
    ("SPARE_BYTE3", 8),
    ("SPARE_WORD0", 32),
    ("SPARE_WORD1", 32),
    ("SPARE_WORD2", 32),
    ("SPARE_WORD3", 32),
    ("SUBSYSTEM_ID", 16),
    ("SUBSYSTEM_VENDOR_ID", 16),
    ("TL_RX_RAM_RADDR_LATENCY", 1),
    ("TL_RX_RAM_RDATA_LATENCY", 2),
    ("TL_RX_RAM_WRITE_LATENCY", 1),
    ("TL_TX_RAM_RADDR_LATENCY", 1),
    ("TL_TX_RAM_RDATA_LATENCY", 2),
    ("TL_TX_RAM_WRITE_LATENCY", 1),
    ("USER_CLK_FREQ", 3),
    ("VC0_RX_RAM_LIMIT", 13),
    ("VC_BASE_PTR", 12),
    ("VC_CAP_ID", 16),
    ("VC_CAP_NEXTPTR", 12),
    ("VC_CAP_VERSION", 4),
    ("VENDOR_ID", 16),
    ("VSEC_BASE_PTR", 12),
    ("VSEC_CAP_HDR_ID", 16),
    ("VSEC_CAP_HDR_LENGTH", 12),
    ("VSEC_CAP_HDR_REVISION", 4),
    ("VSEC_CAP_ID", 16),
    ("VSEC_CAP_NEXTPTR", 12),
    ("VSEC_CAP_VERSION", 4),
];

const PCIE_DEC_ATTRS: &[(&str, usize)] = &[
    ("N_FTS_COMCLK_GEN1", 8),
    ("N_FTS_COMCLK_GEN2", 8),
    ("N_FTS_GEN1", 8),
    ("N_FTS_GEN2", 8),
    ("PCIE_REVISION", 4),
    ("VC0_TOTAL_CREDITS_CD", 11),
    ("VC0_TOTAL_CREDITS_CH", 7),
    ("VC0_TOTAL_CREDITS_NPH", 7),
    ("VC0_TOTAL_CREDITS_PD", 11),
    ("VC0_TOTAL_CREDITS_PH", 7),
    ("VC0_TX_LASTPACKET", 5),
];

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let Some(mut ctx) = FuzzCtx::try_new(session, backend, "PCIE") else {
        return;
    };
    let mut bctx = ctx.bel(bels::PCIE);
    let mode = "PCIE_2_0";

    bctx.build()
        .extra_tile_attr(Delta::new(3, 20, "HCLK"), "HCLK", "DRP_MASK_PCIE", "1")
        .test_manual("PRESENT", "1")
        .mode(mode)
        .commit();
    for &attr in PCIE_BOOL_ATTRS {
        bctx.mode(mode).test_enum(attr, &["FALSE", "TRUE"]);
    }
    for &(attr, width) in PCIE_HEX_ATTRS {
        bctx.mode(mode).test_multi_attr_hex(attr, width);
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

    fn pcie_drp_bit(reg: usize, bit: usize) -> TileBit {
        let tile = reg / 6;
        let frame = 26 + (bit & 1);
        let bit = (bit >> 1) | (reg % 6) << 3;
        TileBit::new(tile, frame, bit)
    }
    for reg in 0..0x78 {
        ctx.tiledb.insert(
            tile,
            bel,
            format!("DRP{reg:02X}"),
            TileItem::from_bitvec((0..16).map(|bit| pcie_drp_bit(reg, bit)).collect(), false),
        );
    }

    ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
    for &attr in PCIE_BOOL_ATTRS {
        ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
    }
    for &(attr, _) in PCIE_HEX_ATTRS {
        ctx.collect_bitvec(tile, bel, attr, "");
    }
    for &(attr, _) in PCIE_DEC_ATTRS {
        ctx.collect_bitvec(tile, bel, attr, "");
    }
    let tile = "HCLK";
    let bel = "HCLK";
    let item = ctx.extract_bit(tile, bel, "DRP_MASK_PCIE", "1");
    ctx.tiledb.insert(tile, bel, "DRP_MASK_BELOW", item);
}
