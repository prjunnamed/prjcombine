use prjcombine_interconnect::{dir::DirH, grid::TileCoord};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bsdata::{TileBit, TileItem};
use prjcombine_virtex4::{bels, tslots};

use crate::{
    backend::IseBackend,
    collector::CollectorCtx,
    generic::{
        fbuild::{FuzzBuilderBase, FuzzCtx},
        props::relation::{Delta, TileRelation},
    },
};

const PCIE_BOOL_ATTRS: &[&str] = &[
    "AER_CAP_ECRC_CHECK_CAPABLE",
    "AER_CAP_ECRC_GEN_CAPABLE",
    "AER_CAP_MULTIHEADER",
    "AER_CAP_ON",
    "AER_CAP_PERMIT_ROOTERR_UPDATE",
    "ALLOW_X8_GEN2",
    "CMD_INTX_IMPLEMENTED",
    "CPL_TIMEOUT_DISABLE_SUPPORTED",
    "DEV_CAP_ENABLE_SLOT_PWR_LIMIT_SCALE",
    "DEV_CAP_ENABLE_SLOT_PWR_LIMIT_VALUE",
    "DEV_CAP_EXT_TAG_SUPPORTED",
    "DEV_CAP_FUNCTION_LEVEL_RESET_CAPABLE",
    "DEV_CAP_ROLE_BASED_ERROR",
    "DEV_CAP2_ARI_FORWARDING_SUPPORTED",
    "DEV_CAP2_ATOMICOP_ROUTING_SUPPORTED",
    "DEV_CAP2_ATOMICOP32_COMPLETER_SUPPORTED",
    "DEV_CAP2_ATOMICOP64_COMPLETER_SUPPORTED",
    "DEV_CAP2_CAS128_COMPLETER_SUPPORTED",
    "DEV_CAP2_ENDEND_TLP_PREFIX_SUPPORTED",
    "DEV_CAP2_EXTENDED_FMT_FIELD_SUPPORTED",
    "DEV_CAP2_LTR_MECHANISM_SUPPORTED",
    "DEV_CAP2_NO_RO_ENABLED_PRPR_PASSING",
    "DEV_CONTROL_AUX_POWER_SUPPORTED",
    "DEV_CONTROL_EXT_TAG_DEFAULT",
    "DISABLE_ASPM_L1_TIMER",
    "DISABLE_BAR_FILTERING",
    "DISABLE_ERR_MSG",
    "DISABLE_ID_CHECK",
    "DISABLE_LANE_REVERSAL",
    "DISABLE_LOCKED_FILTER",
    "DISABLE_PPM_FILTER",
    "DISABLE_RX_POISONED_RESP",
    "DISABLE_RX_TC_FILTER",
    "DISABLE_SCRAMBLING",
    "DSN_CAP_ON",
    "ENABLE_RX_TD_ECRC_TRIM",
    "ENDEND_TLP_PREFIX_FORWARDING_SUPPORTED",
    "ENTER_RVRY_EI_L0",
    "EXIT_LOOPBACK_ON_EI",
    "INTERRUPT_STAT_AUTO",
    "IS_SWITCH",
    "LINK_CAP_ASPM_OPTIONALITY",
    "LINK_CAP_CLOCK_POWER_MANAGEMENT",
    "LINK_CAP_DLL_LINK_ACTIVE_REPORTING_CAP",
    "LINK_CAP_LINK_BANDWIDTH_NOTIFICATION_CAP",
    "LINK_CAP_SURPRISE_DOWN_ERROR_CAPABLE",
    "LINK_CTRL2_DEEMPHASIS",
    "LINK_CTRL2_HW_AUTONOMOUS_SPEED_DISABLE",
    "LINK_STATUS_SLOT_CLOCK_CONFIG",
    "LL_ACK_TIMEOUT_EN",
    "LL_REPLAY_TIMEOUT_EN",
    "MPS_FORCE",
    "MSI_CAP_ON",
    "MSI_CAP_PER_VECTOR_MASKING_CAPABLE",
    "MSI_CAP_64_BIT_ADDR_CAPABLE",
    "MSIX_CAP_ON",
    "PCIE_CAP_ON",
    "PCIE_CAP_SLOT_IMPLEMENTED",
    "PL_FAST_TRAIN",
    "PM_ASPM_FASTEXIT",
    "PM_ASPML0S_TIMEOUT_EN",
    "PM_CAP_DSI",
    "PM_CAP_D1SUPPORT",
    "PM_CAP_D2SUPPORT",
    "PM_CAP_ON",
    "PM_CAP_PME_CLOCK",
    "PM_CSR_BPCCEN",
    "PM_CSR_B2B3",
    "PM_CSR_NOSOFTRST",
    "PM_MF",
    "RBAR_CAP_ON",
    "RECRC_CHK_TRIM",
    "ROOT_CAP_CRS_SW_VISIBILITY",
    "SELECT_DLL_IF",
    "SLOT_CAP_ATT_BUTTON_PRESENT",
    "SLOT_CAP_ATT_INDICATOR_PRESENT",
    "SLOT_CAP_ELEC_INTERLOCK_PRESENT",
    "SLOT_CAP_HOTPLUG_CAPABLE",
    "SLOT_CAP_HOTPLUG_SURPRISE",
    "SLOT_CAP_MRL_SENSOR_PRESENT",
    "SLOT_CAP_NO_CMD_COMPLETED_SUPPORT",
    "SLOT_CAP_POWER_CONTROLLER_PRESENT",
    "SLOT_CAP_POWER_INDICATOR_PRESENT",
    "SSL_MESSAGE_AUTO",
    "TECRC_EP_INV",
    "TEST_MODE_PIN_CHAR",
    "TL_RBYPASS",
    "TL_TFC_DISABLE",
    "TL_TX_CHECKS_DISABLE",
    "TRN_DW",
    "TRN_NP_FC",
    "UPCONFIG_CAPABLE",
    "UPSTREAM_FACING",
    "UR_ATOMIC",
    "UR_CFG1",
    "UR_INV_REQ",
    "UR_PRS_RESPONSE",
    "USE_RID_PINS",
    "USER_CLK2_DIV2",
    "VC_CAP_ON",
    "VC_CAP_REJECT_SNOOP_TRANSACTIONS",
    "VC0_CPL_INFINITE",
    "VSEC_CAP_IS_LINK_VISIBLE",
    "VSEC_CAP_ON",
];

const PCIE_HEX_ATTRS: &[(&str, usize)] = &[
    ("AER_BASE_PTR", 12),
    ("AER_CAP_ID", 16),
    ("AER_CAP_NEXTPTR", 12),
    ("AER_CAP_OPTIONAL_ERR_SUPPORT", 24),
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
    ("DEV_CAP2_MAX_ENDEND_TLP_PREFIXES", 2),
    ("DEV_CAP2_TPH_COMPLETER_SUPPORTED", 2),
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
    ("LINK_CAP_MAX_LINK_SPEED", 4),
    ("LINK_CAP_MAX_LINK_WIDTH", 6),
    ("LINK_CTRL2_TARGET_LINK_SPEED", 4),
    ("LL_ACK_TIMEOUT", 15),
    ("LL_REPLAY_TIMEOUT", 15),
    ("LTSSM_MAX_LINK_WIDTH", 6),
    ("MSIX_BASE_PTR", 8),
    ("MSIX_CAP_ID", 8),
    ("MSIX_CAP_NEXTPTR", 8),
    ("MSIX_CAP_PBA_OFFSET", 29),
    ("MSIX_CAP_TABLE_OFFSET", 29),
    ("MSIX_CAP_TABLE_SIZE", 11),
    ("MSI_BASE_PTR", 8),
    ("MSI_CAP_ID", 8),
    ("MSI_CAP_NEXTPTR", 8),
    ("PCIE_BASE_PTR", 8),
    ("PCIE_CAP_CAPABILITY_ID", 8),
    ("PCIE_CAP_CAPABILITY_VERSION", 4),
    ("PCIE_CAP_DEVICE_PORT_TYPE", 4),
    ("PCIE_CAP_NEXTPTR", 8),
    ("PM_ASPML0S_TIMEOUT", 15),
    ("PM_BASE_PTR", 8),
    ("PM_CAP_ID", 8),
    ("PM_CAP_NEXTPTR", 8),
    ("PM_CAP_PMESUPPORT", 5),
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
    ("RBAR_BASE_PTR", 12),
    ("RBAR_CAP_CONTROL_ENCODEDBAR0", 5),
    ("RBAR_CAP_CONTROL_ENCODEDBAR1", 5),
    ("RBAR_CAP_CONTROL_ENCODEDBAR2", 5),
    ("RBAR_CAP_CONTROL_ENCODEDBAR3", 5),
    ("RBAR_CAP_CONTROL_ENCODEDBAR4", 5),
    ("RBAR_CAP_CONTROL_ENCODEDBAR5", 5),
    ("RBAR_CAP_ID", 16),
    ("RBAR_CAP_INDEX0", 3),
    ("RBAR_CAP_INDEX1", 3),
    ("RBAR_CAP_INDEX2", 3),
    ("RBAR_CAP_INDEX3", 3),
    ("RBAR_CAP_INDEX4", 3),
    ("RBAR_CAP_INDEX5", 3),
    ("RBAR_CAP_NEXTPTR", 12),
    ("RBAR_CAP_SUP0", 32),
    ("RBAR_CAP_SUP1", 32),
    ("RBAR_CAP_SUP2", 32),
    ("RBAR_CAP_SUP3", 32),
    ("RBAR_CAP_SUP4", 32),
    ("RBAR_CAP_SUP5", 32),
    ("RBAR_CAP_VERSION", 4),
    ("RBAR_NUM", 3),
    ("RP_AUTO_SPD", 2),
    ("RP_AUTO_SPD_LOOPCNT", 5),
    ("SLOT_CAP_PHYSICAL_SLOT_NUM", 13),
    ("SLOT_CAP_SLOT_POWER_LIMIT_VALUE", 8),
    ("SPARE_BYTE0", 8),
    ("SPARE_BYTE1", 8),
    ("SPARE_BYTE2", 8),
    ("SPARE_BYTE3", 8),
    ("SPARE_WORD0", 32),
    ("SPARE_WORD1", 32),
    ("SPARE_WORD2", 32),
    ("SPARE_WORD3", 32),
    ("VC0_RX_RAM_LIMIT", 13),
    ("VC_BASE_PTR", 12),
    ("VC_CAP_ID", 16),
    ("VC_CAP_NEXTPTR", 12),
    ("VC_CAP_VERSION", 4),
    ("VSEC_BASE_PTR", 12),
    ("VSEC_CAP_HDR_ID", 16),
    ("VSEC_CAP_HDR_LENGTH", 12),
    ("VSEC_CAP_HDR_REVISION", 4),
    ("VSEC_CAP_ID", 16),
    ("VSEC_CAP_NEXTPTR", 12),
    ("VSEC_CAP_VERSION", 4),
];

const PCIE_DEC_ATTRS: &[(&str, usize)] = &[
    ("CFG_ECRC_ERR_CPLSTAT", 2),
    ("DEV_CAP_ENDPOINT_L0S_LATENCY", 3),
    ("DEV_CAP_ENDPOINT_L1_LATENCY", 3),
    ("DEV_CAP_MAX_PAYLOAD_SUPPORTED", 3),
    ("DEV_CAP_PHANTOM_FUNCTIONS_SUPPORT", 2),
    ("DEV_CAP_RSVD_14_12", 3),
    ("DEV_CAP_RSVD_17_16", 2),
    ("DEV_CAP_RSVD_31_29", 3),
    ("LINK_CAP_ASPM_SUPPORT", 2),
    ("LINK_CAP_L0S_EXIT_LATENCY_COMCLK_GEN1", 3),
    ("LINK_CAP_L0S_EXIT_LATENCY_COMCLK_GEN2", 3),
    ("LINK_CAP_L0S_EXIT_LATENCY_GEN1", 3),
    ("LINK_CAP_L0S_EXIT_LATENCY_GEN2", 3),
    ("LINK_CAP_L1_EXIT_LATENCY_COMCLK_GEN1", 3),
    ("LINK_CAP_L1_EXIT_LATENCY_COMCLK_GEN2", 3),
    ("LINK_CAP_L1_EXIT_LATENCY_GEN1", 3),
    ("LINK_CAP_L1_EXIT_LATENCY_GEN2", 3),
    ("LINK_CAP_RSVD_23", 1),
    ("LINK_CONTROL_RCB", 1),
    ("LL_ACK_TIMEOUT_FUNC", 2),
    ("LL_REPLAY_TIMEOUT_FUNC", 2),
    ("MSI_CAP_MULTIMSG_EXTENSION", 1),
    ("MSI_CAP_MULTIMSGCAP", 3),
    ("MSIX_CAP_PBA_BIR", 3),
    ("MSIX_CAP_TABLE_BIR", 3),
    ("PCIE_CAP_RSVD_15_14", 2),
    ("PL_AUTO_CONFIG", 3),
    ("PM_ASPML0S_TIMEOUT_FUNC", 2),
    ("PM_CAP_AUXCURRENT", 3),
    ("PM_CAP_RSVD_04", 1),
    ("PM_CAP_VERSION", 3),
    ("RECRC_CHK", 2),
    ("SLOT_CAP_SLOT_POWER_LIMIT_SCALE", 2),
    ("SPARE_BIT0", 1),
    ("SPARE_BIT1", 1),
    ("SPARE_BIT2", 1),
    ("SPARE_BIT3", 1),
    ("SPARE_BIT4", 1),
    ("SPARE_BIT5", 1),
    ("SPARE_BIT6", 1),
    ("SPARE_BIT7", 1),
    ("SPARE_BIT8", 1),
    ("TL_RX_RAM_RADDR_LATENCY", 1),
    ("TL_RX_RAM_RDATA_LATENCY", 2),
    ("TL_RX_RAM_WRITE_LATENCY", 1),
    ("TL_TX_RAM_RADDR_LATENCY", 1),
    ("TL_TX_RAM_RDATA_LATENCY", 2),
    ("TL_TX_RAM_WRITE_LATENCY", 1),
    ("USER_CLK_FREQ", 3),
    ("N_FTS_COMCLK_GEN1", 8),
    ("N_FTS_COMCLK_GEN2", 8),
    ("N_FTS_GEN1", 8),
    ("N_FTS_GEN2", 8),
    ("PCIE_REVISION", 4),
    ("VC0_TOTAL_CREDITS_CD", 11),
    ("VC0_TOTAL_CREDITS_CH", 7),
    ("VC0_TOTAL_CREDITS_NPD", 11),
    ("VC0_TOTAL_CREDITS_NPH", 7),
    ("VC0_TOTAL_CREDITS_PD", 11),
    ("VC0_TOTAL_CREDITS_PH", 7),
    ("VC0_TX_LASTPACKET", 5),
];

const PCIE3_BOOL_ATTRS: &[&str] = &[
    "ARI_CAP_ENABLE",
    "AXISTEN_IF_CC_ALIGNMENT_MODE",
    "AXISTEN_IF_CC_PARITY_CHK",
    "AXISTEN_IF_CQ_ALIGNMENT_MODE",
    "AXISTEN_IF_ENABLE_CLIENT_TAG",
    "AXISTEN_IF_ENABLE_RX_MSG_INTFC",
    "AXISTEN_IF_RC_ALIGNMENT_MODE",
    "AXISTEN_IF_RC_STRADDLE",
    "AXISTEN_IF_RQ_ALIGNMENT_MODE",
    "AXISTEN_IF_RQ_PARITY_CHK",
    "CRM_CORE_CLK_FREQ_500",
    "GEN3_PCS_RX_ELECIDLE_INTERNAL",
    "LL_ACK_TIMEOUT_EN",
    "LL_CPL_FC_UPDATE_TIMER_OVERRIDE",
    "LL_FC_UPDATE_TIMER_OVERRIDE",
    "LL_NP_FC_UPDATE_TIMER_OVERRIDE",
    "LL_P_FC_UPDATE_TIMER_OVERRIDE",
    "LL_REPLAY_TIMEOUT_EN",
    "LTR_TX_MESSAGE_ON_FUNC_POWER_STATE_CHANGE",
    "LTR_TX_MESSAGE_ON_LTR_ENABLE",
    "PF0_AER_CAP_ECRC_CHECK_CAPABLE",
    "PF0_AER_CAP_ECRC_GEN_CAPABLE",
];

const PCIE3_HEX_ATTRS: &[(&str, usize)] = &[
    ("AXISTEN_IF_ENABLE_MSG_ROUTE", 18),
    ("AXISTEN_IF_WIDTH", 2),
    ("CRM_USER_CLK_FREQ", 2),
    ("DNSTREAM_LINK_NUM", 8),
    ("GEN3_PCS_AUTO_REALIGN", 2),
    ("LL_ACK_TIMEOUT", 9),
    ("LL_CPL_FC_UPDATE_TIMER", 16),
    ("LL_FC_UPDATE_TIMER", 16),
    ("LL_NP_FC_UPDATE_TIMER", 16),
    ("LL_P_FC_UPDATE_TIMER", 16),
    ("LL_REPLAY_TIMEOUT", 9),
    ("LTR_TX_MESSAGE_MINIMUM_INTERVAL", 10),
    ("PF0_AER_CAP_NEXTPTR", 12),
    ("PF0_ARI_CAP_NEXTPTR", 12),
    ("PF0_ARI_CAP_NEXT_FUNC", 8),
    ("PF0_ARI_CAP_VER", 4),
    ("PF0_BAR0_APERTURE_SIZE", 5),
    ("PF0_BAR0_CONTROL", 3),
    ("PF0_BAR1_APERTURE_SIZE", 5),
    ("PF0_BAR1_CONTROL", 3),
    ("PF0_BAR2_APERTURE_SIZE", 5),
    ("PF0_BAR2_CONTROL", 3),
    ("PF0_BAR3_APERTURE_SIZE", 5),
    ("PF0_BAR3_CONTROL", 3),
    ("PF0_BAR4_APERTURE_SIZE", 5),
    ("PF0_BAR4_CONTROL", 3),
    ("PF0_BAR5_APERTURE_SIZE", 5),
    ("PF0_BAR5_CONTROL", 3),
    ("PF0_BIST_REGISTER", 8),
    ("PF0_CAPABILITY_POINTER", 8),
    ("PF0_CLASS_CODE", 24),
    ("PF0_DEVICE_ID", 16),
    ("PF0_DEV_CAP2_OBFF_SUPPORT", 2),
    ("PF0_DEV_CAP_MAX_PAYLOAD_SIZE", 3),
    ("PF0_DPA_CAP_NEXTPTR", 12),
    ("PF0_DPA_CAP_SUB_STATE_CONTROL", 5),
    ("PF0_DPA_CAP_SUB_STATE_POWER_ALLOCATION0", 8),
    ("PF0_DPA_CAP_SUB_STATE_POWER_ALLOCATION1", 8),
    ("PF0_DPA_CAP_SUB_STATE_POWER_ALLOCATION2", 8),
    ("PF0_DPA_CAP_SUB_STATE_POWER_ALLOCATION3", 8),
    ("PF0_DPA_CAP_SUB_STATE_POWER_ALLOCATION4", 8),
    ("PF0_DPA_CAP_SUB_STATE_POWER_ALLOCATION5", 8),
    ("PF0_DPA_CAP_SUB_STATE_POWER_ALLOCATION6", 8),
    ("PF0_DPA_CAP_SUB_STATE_POWER_ALLOCATION7", 8),
    ("PF0_DPA_CAP_VER", 4),
    ("PF0_DSN_CAP_NEXTPTR", 12),
    ("PF0_EXPANSION_ROM_APERTURE_SIZE", 5),
    ("PF0_INTERRUPT_LINE", 8),
    ("PF0_INTERRUPT_PIN", 3),
    ("PF0_LTR_CAP_MAX_NOSNOOP_LAT", 10),
    ("PF0_LTR_CAP_MAX_SNOOP_LAT", 10),
    ("PF0_LTR_CAP_NEXTPTR", 12),
    ("PF0_LTR_CAP_VER", 4),
    ("PF0_MSIX_CAP_NEXTPTR", 8),
    ("PF0_MSIX_CAP_PBA_OFFSET", 29),
    ("PF0_MSIX_CAP_TABLE_OFFSET", 29),
    ("PF0_MSIX_CAP_TABLE_SIZE", 11),
    ("PF0_MSI_CAP_NEXTPTR", 8),
    ("PF0_PB_CAP_NEXTPTR", 12),
    ("PF0_PB_CAP_VER", 4),
    ("PF0_PM_CAP_ID", 8),
    ("PF0_PM_CAP_NEXTPTR", 8),
    ("PF0_PM_CAP_VER_ID", 3),
    ("PF0_RBAR_CAP_INDEX0", 3),
    ("PF0_RBAR_CAP_INDEX1", 3),
    ("PF0_RBAR_CAP_INDEX2", 3),
    ("PF0_RBAR_CAP_NEXTPTR", 12),
    ("PF0_RBAR_CAP_SIZE0", 20),
    ("PF0_RBAR_CAP_SIZE1", 20),
    ("PF0_RBAR_CAP_SIZE2", 20),
    ("PF0_RBAR_CAP_VER", 4),
    ("PF0_RBAR_NUM", 3),
    ("PF0_REVISION_ID", 8),
    ("PF0_SRIOV_BAR0_APERTURE_SIZE", 5),
    ("PF0_SRIOV_BAR0_CONTROL", 3),
    ("PF0_SRIOV_BAR1_APERTURE_SIZE", 5),
    ("PF0_SRIOV_BAR1_CONTROL", 3),
    ("PF0_SRIOV_BAR2_APERTURE_SIZE", 5),
    ("PF0_SRIOV_BAR2_CONTROL", 3),
    ("PF0_SRIOV_BAR3_APERTURE_SIZE", 5),
    ("PF0_SRIOV_BAR3_CONTROL", 3),
    ("PF0_SRIOV_BAR4_APERTURE_SIZE", 5),
    ("PF0_SRIOV_BAR4_CONTROL", 3),
    ("PF0_SRIOV_BAR5_APERTURE_SIZE", 5),
    ("PF0_SRIOV_BAR5_CONTROL", 3),
    ("PF0_SRIOV_CAP_INITIAL_VF", 16),
    ("PF0_SRIOV_CAP_NEXTPTR", 12),
    ("PF0_SRIOV_CAP_TOTAL_VF", 16),
    ("PF0_SRIOV_CAP_VER", 4),
    ("PF0_SRIOV_FIRST_VF_OFFSET", 16),
    ("PF0_SRIOV_FUNC_DEP_LINK", 16),
    ("PF0_SRIOV_SUPPORTED_PAGE_SIZE", 32),
    ("PF0_SRIOV_VF_DEVICE_ID", 16),
    ("PF0_SUBSYSTEM_ID", 16),
    ("PF0_TPHR_CAP_NEXTPTR", 12),
    ("PF0_TPHR_CAP_ST_MODE_SEL", 3),
    ("PF0_TPHR_CAP_ST_TABLE_LOC", 2),
    ("PF0_TPHR_CAP_ST_TABLE_SIZE", 11),
    ("PF0_TPHR_CAP_VER", 4),
    ("PF0_VC_CAP_NEXTPTR", 12),
    ("PF0_VC_CAP_VER", 4),
    ("PF1_AER_CAP_NEXTPTR", 12),
    ("PF1_ARI_CAP_NEXTPTR", 12),
    ("PF1_ARI_CAP_NEXT_FUNC", 8),
    ("PF1_BAR0_APERTURE_SIZE", 5),
    ("PF1_BAR0_CONTROL", 3),
    ("PF1_BAR1_APERTURE_SIZE", 5),
    ("PF1_BAR1_CONTROL", 3),
    ("PF1_BAR2_APERTURE_SIZE", 5),
    ("PF1_BAR2_CONTROL", 3),
    ("PF1_BAR3_APERTURE_SIZE", 5),
    ("PF1_BAR3_CONTROL", 3),
    ("PF1_BAR4_APERTURE_SIZE", 5),
    ("PF1_BAR4_CONTROL", 3),
    ("PF1_BAR5_APERTURE_SIZE", 5),
    ("PF1_BAR5_CONTROL", 3),
    ("PF1_BIST_REGISTER", 8),
    ("PF1_CAPABILITY_POINTER", 8),
    ("PF1_CLASS_CODE", 24),
    ("PF1_DEVICE_ID", 16),
    ("PF1_DEV_CAP_MAX_PAYLOAD_SIZE", 3),
    ("PF1_DPA_CAP_NEXTPTR", 12),
    ("PF1_DPA_CAP_SUB_STATE_CONTROL", 5),
    ("PF1_DPA_CAP_SUB_STATE_POWER_ALLOCATION0", 8),
    ("PF1_DPA_CAP_SUB_STATE_POWER_ALLOCATION1", 8),
    ("PF1_DPA_CAP_SUB_STATE_POWER_ALLOCATION2", 8),
    ("PF1_DPA_CAP_SUB_STATE_POWER_ALLOCATION3", 8),
    ("PF1_DPA_CAP_SUB_STATE_POWER_ALLOCATION4", 8),
    ("PF1_DPA_CAP_SUB_STATE_POWER_ALLOCATION5", 8),
    ("PF1_DPA_CAP_SUB_STATE_POWER_ALLOCATION6", 8),
    ("PF1_DPA_CAP_SUB_STATE_POWER_ALLOCATION7", 8),
    ("PF1_DPA_CAP_VER", 4),
    ("PF1_DSN_CAP_NEXTPTR", 12),
    ("PF1_EXPANSION_ROM_APERTURE_SIZE", 5),
    ("PF1_INTERRUPT_LINE", 8),
    ("PF1_INTERRUPT_PIN", 3),
    ("PF1_MSIX_CAP_NEXTPTR", 8),
    ("PF1_MSIX_CAP_PBA_OFFSET", 29),
    ("PF1_MSIX_CAP_TABLE_OFFSET", 29),
    ("PF1_MSIX_CAP_TABLE_SIZE", 11),
    ("PF1_MSI_CAP_NEXTPTR", 8),
    ("PF1_PB_CAP_NEXTPTR", 12),
    ("PF1_PB_CAP_VER", 4),
    ("PF1_PM_CAP_ID", 8),
    ("PF1_PM_CAP_NEXTPTR", 8),
    ("PF1_PM_CAP_VER_ID", 3),
    ("PF1_RBAR_CAP_INDEX0", 3),
    ("PF1_RBAR_CAP_INDEX1", 3),
    ("PF1_RBAR_CAP_INDEX2", 3),
    ("PF1_RBAR_CAP_NEXTPTR", 12),
    ("PF1_RBAR_CAP_SIZE0", 20),
    ("PF1_RBAR_CAP_SIZE1", 20),
    ("PF1_RBAR_CAP_SIZE2", 20),
    ("PF1_RBAR_CAP_VER", 4),
    ("PF1_RBAR_NUM", 3),
    ("PF1_REVISION_ID", 8),
    ("PF1_SRIOV_BAR0_APERTURE_SIZE", 5),
    ("PF1_SRIOV_BAR0_CONTROL", 3),
    ("PF1_SRIOV_BAR1_APERTURE_SIZE", 5),
    ("PF1_SRIOV_BAR1_CONTROL", 3),
    ("PF1_SRIOV_BAR2_APERTURE_SIZE", 5),
    ("PF1_SRIOV_BAR2_CONTROL", 3),
    ("PF1_SRIOV_BAR3_APERTURE_SIZE", 5),
    ("PF1_SRIOV_BAR3_CONTROL", 3),
    ("PF1_SRIOV_BAR4_APERTURE_SIZE", 5),
    ("PF1_SRIOV_BAR4_CONTROL", 3),
    ("PF1_SRIOV_BAR5_APERTURE_SIZE", 5),
    ("PF1_SRIOV_BAR5_CONTROL", 3),
    ("PF1_SRIOV_CAP_INITIAL_VF", 16),
    ("PF1_SRIOV_CAP_NEXTPTR", 12),
    ("PF1_SRIOV_CAP_TOTAL_VF", 16),
    ("PF1_SRIOV_CAP_VER", 4),
    ("PF1_SRIOV_FIRST_VF_OFFSET", 16),
    ("PF1_SRIOV_FUNC_DEP_LINK", 16),
    ("PF1_SRIOV_SUPPORTED_PAGE_SIZE", 32),
    ("PF1_SRIOV_VF_DEVICE_ID", 16),
    ("PF1_SUBSYSTEM_ID", 16),
    ("PF1_TPHR_CAP_NEXTPTR", 12),
    ("PF1_TPHR_CAP_ST_MODE_SEL", 3),
    ("PF1_TPHR_CAP_ST_TABLE_LOC", 2),
    ("PF1_TPHR_CAP_ST_TABLE_SIZE", 11),
    ("PF1_TPHR_CAP_VER", 4),
    ("PL_EQ_ADAPT_ITER_COUNT", 5),
    ("PL_EQ_ADAPT_REJECT_RETRY_COUNT", 2),
    ("PL_LANE0_EQ_CONTROL", 16),
    ("PL_LANE1_EQ_CONTROL", 16),
    ("PL_LANE2_EQ_CONTROL", 16),
    ("PL_LANE3_EQ_CONTROL", 16),
    ("PL_LANE4_EQ_CONTROL", 16),
    ("PL_LANE5_EQ_CONTROL", 16),
    ("PL_LANE6_EQ_CONTROL", 16),
    ("PL_LANE7_EQ_CONTROL", 16),
    ("PL_LINK_CAP_MAX_LINK_SPEED", 3),
    ("PL_LINK_CAP_MAX_LINK_WIDTH", 4),
    ("PM_ASPML0S_TIMEOUT", 16),
    ("PM_ASPML1_ENTRY_DELAY", 20),
    ("PM_L1_REENTRY_DELAY", 32),
    ("PM_PME_SERVICE_TIMEOUT_DELAY", 20),
    ("PM_PME_TURNOFF_ACK_DELAY", 16),
    ("SPARE_BYTE0", 8),
    ("SPARE_BYTE1", 8),
    ("SPARE_BYTE2", 8),
    ("SPARE_BYTE3", 8),
    ("SPARE_WORD0", 32),
    ("SPARE_WORD1", 32),
    ("SPARE_WORD2", 32),
    ("SPARE_WORD3", 32),
    ("TL_COMPL_TIMEOUT_REG0", 24),
    ("TL_COMPL_TIMEOUT_REG1", 28),
    ("TL_CREDITS_CD", 12),
    ("TL_CREDITS_CH", 8),
    ("TL_CREDITS_NPD", 12),
    ("TL_CREDITS_NPH", 8),
    ("TL_CREDITS_PD", 12),
    ("TL_CREDITS_PH", 8),
    ("VF0_ARI_CAP_NEXTPTR", 12),
    ("VF0_CAPABILITY_POINTER", 8),
    ("VF0_MSIX_CAP_PBA_OFFSET", 29),
    ("VF0_MSIX_CAP_TABLE_OFFSET", 29),
    ("VF0_MSIX_CAP_TABLE_SIZE", 11),
    ("VF0_PM_CAP_ID", 8),
    ("VF0_PM_CAP_NEXTPTR", 8),
    ("VF0_PM_CAP_VER_ID", 3),
    ("VF0_TPHR_CAP_NEXTPTR", 12),
    ("VF0_TPHR_CAP_ST_MODE_SEL", 3),
    ("VF0_TPHR_CAP_ST_TABLE_LOC", 2),
    ("VF0_TPHR_CAP_ST_TABLE_SIZE", 11),
    ("VF0_TPHR_CAP_VER", 4),
    ("VF1_ARI_CAP_NEXTPTR", 12),
    ("VF1_MSIX_CAP_PBA_OFFSET", 29),
    ("VF1_MSIX_CAP_TABLE_OFFSET", 29),
    ("VF1_MSIX_CAP_TABLE_SIZE", 11),
    ("VF1_PM_CAP_ID", 8),
    ("VF1_PM_CAP_NEXTPTR", 8),
    ("VF1_PM_CAP_VER_ID", 3),
    ("VF1_TPHR_CAP_NEXTPTR", 12),
    ("VF1_TPHR_CAP_ST_MODE_SEL", 3),
    ("VF1_TPHR_CAP_ST_TABLE_LOC", 2),
    ("VF1_TPHR_CAP_ST_TABLE_SIZE", 11),
    ("VF1_TPHR_CAP_VER", 4),
    ("VF2_ARI_CAP_NEXTPTR", 12),
    ("VF2_MSIX_CAP_PBA_OFFSET", 29),
    ("VF2_MSIX_CAP_TABLE_OFFSET", 29),
    ("VF2_MSIX_CAP_TABLE_SIZE", 11),
    ("VF2_PM_CAP_ID", 8),
    ("VF2_PM_CAP_NEXTPTR", 8),
    ("VF2_PM_CAP_VER_ID", 3),
    ("VF2_TPHR_CAP_NEXTPTR", 12),
    ("VF2_TPHR_CAP_ST_MODE_SEL", 3),
    ("VF2_TPHR_CAP_ST_TABLE_LOC", 2),
    ("VF2_TPHR_CAP_ST_TABLE_SIZE", 11),
    ("VF2_TPHR_CAP_VER", 4),
    ("VF3_ARI_CAP_NEXTPTR", 12),
    ("VF3_MSIX_CAP_PBA_OFFSET", 29),
    ("VF3_MSIX_CAP_TABLE_OFFSET", 29),
    ("VF3_MSIX_CAP_TABLE_SIZE", 11),
    ("VF3_PM_CAP_ID", 8),
    ("VF3_PM_CAP_NEXTPTR", 8),
    ("VF3_PM_CAP_VER_ID", 3),
    ("VF3_TPHR_CAP_NEXTPTR", 12),
    ("VF3_TPHR_CAP_ST_MODE_SEL", 3),
    ("VF3_TPHR_CAP_ST_TABLE_LOC", 2),
    ("VF3_TPHR_CAP_ST_TABLE_SIZE", 11),
    ("VF3_TPHR_CAP_VER", 4),
    ("VF4_ARI_CAP_NEXTPTR", 12),
    ("VF4_MSIX_CAP_PBA_OFFSET", 29),
    ("VF4_MSIX_CAP_TABLE_OFFSET", 29),
    ("VF4_MSIX_CAP_TABLE_SIZE", 11),
    ("VF4_PM_CAP_ID", 8),
    ("VF4_PM_CAP_NEXTPTR", 8),
    ("VF4_PM_CAP_VER_ID", 3),
    ("VF4_TPHR_CAP_NEXTPTR", 12),
    ("VF4_TPHR_CAP_ST_MODE_SEL", 3),
    ("VF4_TPHR_CAP_ST_TABLE_LOC", 2),
    ("VF4_TPHR_CAP_ST_TABLE_SIZE", 11),
    ("VF4_TPHR_CAP_VER", 4),
    ("VF5_ARI_CAP_NEXTPTR", 12),
    ("VF5_MSIX_CAP_PBA_OFFSET", 29),
    ("VF5_MSIX_CAP_TABLE_OFFSET", 29),
    ("VF5_MSIX_CAP_TABLE_SIZE", 11),
    ("VF5_PM_CAP_ID", 8),
    ("VF5_PM_CAP_NEXTPTR", 8),
    ("VF5_PM_CAP_VER_ID", 3),
    ("VF5_TPHR_CAP_NEXTPTR", 12),
    ("VF5_TPHR_CAP_ST_MODE_SEL", 3),
    ("VF5_TPHR_CAP_ST_TABLE_LOC", 2),
    ("VF5_TPHR_CAP_ST_TABLE_SIZE", 11),
    ("VF5_TPHR_CAP_VER", 4),
];

const PCIE3_DEC_ATTRS: &[(&str, usize)] = &[
    ("LL_ACK_TIMEOUT_FUNC", 2),
    ("LL_REPLAY_TIMEOUT_FUNC", 2),
    ("PF0_DEV_CAP_ENDPOINT_L0S_LATENCY", 3),
    ("PF0_DEV_CAP_ENDPOINT_L1_LATENCY", 3),
    ("PL_N_FTS_COMCLK_GEN1", 8),
    ("PL_N_FTS_COMCLK_GEN2", 8),
    ("PL_N_FTS_COMCLK_GEN3", 8),
    ("PL_N_FTS_GEN1", 8),
    ("PL_N_FTS_GEN2", 8),
    ("PL_N_FTS_GEN3", 8),
];

#[derive(Copy, Clone, Debug)]
struct PcieHclkPair;

impl TileRelation for PcieHclkPair {
    fn resolve(&self, backend: &IseBackend, tcrd: TileCoord) -> Option<TileCoord> {
        let ExpandedDevice::Virtex4(edev) = backend.edev else {
            unreachable!()
        };
        let col = match edev.col_side(tcrd.col) {
            DirH::W => tcrd.col - 4,
            DirH::E => tcrd.col - 1,
        };
        let row = tcrd.row + edev.chips[tcrd.die].rows_per_reg() / 2;
        Some(tcrd.with_cr(col, row).tile(tslots::HCLK))
    }
}

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, "PCIE") {
        let mut bctx = ctx.bel(bels::PCIE);
        let mode = "PCIE_2_1";
        bctx.test_manual("PRESENT", "1").mode(mode).commit();
        // always appears in left column even when DRP is in right column — bug or intentional?
        bctx.mode(mode)
            .extra_tile_attr(PcieHclkPair, "HCLK", "DRP_MASK_PCIE", "1")
            .test_manual("DRP_MASK", "1")
            .pin("DRPWE")
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
    if let Some(mut ctx) = FuzzCtx::try_new(session, backend, "PCIE3") {
        let mut bctx = ctx.bel(bels::PCIE3);
        let mode = "PCIE_3_0";
        // always turns on the "bottom" bit even in the lower region — bug or intentional?
        bctx.test_manual("PRESENT", "1").mode(mode).commit();
        bctx.mode(mode)
            .extra_tile_attr(Delta::new(3, 0, "HCLK"), "HCLK", "DRP_MASK_PCIE", "1")
            .extra_tile_attr(Delta::new(3, 50, "HCLK"), "HCLK", "DRP_MASK_PCIE", "1")
            .test_manual("DRP_MASK", "1")
            .pin("DRPWE")
            .commit();
        for &attr in PCIE3_BOOL_ATTRS {
            bctx.mode(mode).test_enum(attr, &["FALSE", "TRUE"]);
        }
        for &(attr, width) in PCIE3_HEX_ATTRS {
            bctx.mode(mode).test_multi_attr_hex(attr, width);
        }
        for &(attr, width) in PCIE3_DEC_ATTRS {
            bctx.mode(mode).test_multi_attr_dec(attr, width);
        }
    }
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let mut got_pcie = false;
    if ctx.has_tile("PCIE") {
        let tile = "PCIE";
        let bel = "PCIE";

        fn pcie_drp_bit(reg: usize, bit: usize) -> TileBit {
            let tile = reg / 6;
            let frame = 28 + (bit & 1);
            let bit = (bit >> 1) | (reg % 6) << 3;
            TileBit::new(tile, frame, bit)
        }
        for reg in 0..0x96 {
            ctx.tiledb.insert(
                tile,
                bel,
                format!("DRP{reg:02X}"),
                TileItem::from_bitvec((0..16).map(|bit| pcie_drp_bit(reg, bit)).collect(), false),
            );
        }

        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        ctx.state
            .get_diff(tile, bel, "DRP_MASK", "1")
            .assert_empty();
        for &attr in PCIE_BOOL_ATTRS {
            ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
        }
        for &(attr, _) in PCIE_HEX_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        for &(attr, _) in PCIE_DEC_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        got_pcie = true;
    }
    if ctx.has_tile("PCIE3") {
        let tile = "PCIE3";
        let bel = "PCIE3";
        ctx.state.get_diff(tile, bel, "PRESENT", "1").assert_empty();
        ctx.state
            .get_diff(tile, bel, "DRP_MASK", "1")
            .assert_empty();
        for &attr in PCIE3_BOOL_ATTRS {
            ctx.collect_enum_bool(tile, bel, attr, "FALSE", "TRUE");
        }
        for &(attr, _) in PCIE3_HEX_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        for &(attr, _) in PCIE3_DEC_ATTRS {
            ctx.collect_bitvec(tile, bel, attr, "");
        }
        got_pcie = true;
    }
    if got_pcie {
        let tile = "HCLK";
        let bel = "HCLK";
        let item = ctx.extract_bit(tile, bel, "DRP_MASK_PCIE", "1");
        ctx.tiledb.insert(tile, bel, "DRP_MASK_BELOW_R", item);
    }
}
