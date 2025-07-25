use prjcombine_interconnect::{bels, db::BelSlotId};

use crate::tslots;

bels![
    INT: tslots::INT,
    RCLK_INT: tslots::RCLK_INT,
    INTF_DELAY: tslots::INTF,
    INTF_TESTMUX: tslots::INTF,
    SLICE: tslots::BEL,
    LAGUNA0: tslots::BEL,
    LAGUNA1: tslots::BEL,
    LAGUNA2: tslots::BEL,
    LAGUNA3: tslots::BEL,
    LAGUNA_EXTRA: tslots::BEL,
    VCC_LAGUNA: tslots::BEL,
    BRAM_F: tslots::BEL,
    BRAM_H0: tslots::BEL,
    BRAM_H1: tslots::BEL,
    HARD_SYNC0: tslots::RCLK_BEL,
    HARD_SYNC1: tslots::RCLK_BEL,
    HARD_SYNC2: tslots::RCLK_BEL,
    HARD_SYNC3: tslots::RCLK_BEL,
    DSP0: tslots::BEL,
    DSP1: tslots::BEL,
    URAM0: tslots::BEL,
    URAM1: tslots::BEL,
    URAM2: tslots::BEL,
    URAM3: tslots::BEL,
    HPIOB0: tslots::IOB,
    HPIOB1: tslots::IOB,
    HPIOB2: tslots::IOB,
    HPIOB3: tslots::IOB,
    HPIOB4: tslots::IOB,
    HPIOB5: tslots::IOB,
    HPIOB6: tslots::IOB,
    HPIOB7: tslots::IOB,
    HPIOB8: tslots::IOB,
    HPIOB9: tslots::IOB,
    HPIOB10: tslots::IOB,
    HPIOB11: tslots::IOB,
    HPIOB12: tslots::IOB,
    HPIOB13: tslots::IOB,
    HPIOB14: tslots::IOB,
    HPIOB15: tslots::IOB,
    HPIOB16: tslots::IOB,
    HPIOB17: tslots::IOB,
    HPIOB18: tslots::IOB,
    HPIOB19: tslots::IOB,
    HPIOB20: tslots::IOB,
    HPIOB21: tslots::IOB,
    HPIOB22: tslots::IOB,
    HPIOB23: tslots::IOB,
    HPIOB24: tslots::IOB,
    HPIOB25: tslots::IOB,
    HPIOB_DIFF_IN0: tslots::IOB,
    HPIOB_DIFF_IN1: tslots::IOB,
    HPIOB_DIFF_IN2: tslots::IOB,
    HPIOB_DIFF_IN3: tslots::IOB,
    HPIOB_DIFF_IN4: tslots::IOB,
    HPIOB_DIFF_IN5: tslots::IOB,
    HPIOB_DIFF_IN6: tslots::IOB,
    HPIOB_DIFF_IN7: tslots::IOB,
    HPIOB_DIFF_IN8: tslots::IOB,
    HPIOB_DIFF_IN9: tslots::IOB,
    HPIOB_DIFF_IN10: tslots::IOB,
    HPIOB_DIFF_IN11: tslots::IOB,
    HPIOB_DIFF_OUT0: tslots::IOB,
    HPIOB_DIFF_OUT1: tslots::IOB,
    HPIOB_DIFF_OUT2: tslots::IOB,
    HPIOB_DIFF_OUT3: tslots::IOB,
    HPIOB_DIFF_OUT4: tslots::IOB,
    HPIOB_DIFF_OUT5: tslots::IOB,
    HPIOB_DIFF_OUT6: tslots::IOB,
    HPIOB_DIFF_OUT7: tslots::IOB,
    HPIOB_DIFF_OUT8: tslots::IOB,
    HPIOB_DIFF_OUT9: tslots::IOB,
    HPIOB_DIFF_OUT10: tslots::IOB,
    HPIOB_DIFF_OUT11: tslots::IOB,
    HPIOB_DCI0: tslots::IOB,
    HPIOB_DCI1: tslots::IOB,
    HPIO_VREF: tslots::IOB,
    HPIO_BIAS: tslots::IOB,
    ABUS_SWITCH_HPIO0: tslots::RCLK_IOB,
    ABUS_SWITCH_HPIO1: tslots::RCLK_IOB,
    ABUS_SWITCH_HPIO2: tslots::RCLK_IOB,
    ABUS_SWITCH_HPIO3: tslots::RCLK_IOB,
    ABUS_SWITCH_HPIO4: tslots::RCLK_IOB,
    ABUS_SWITCH_HPIO5: tslots::RCLK_IOB,
    ABUS_SWITCH_HPIO6: tslots::RCLK_IOB,
    HPIO_ZMATCH: tslots::RCLK_IOB,
    HPIO_PRBS: tslots::RCLK_IOB,
    HRIOB0: tslots::IOB,
    HRIOB1: tslots::IOB,
    HRIOB2: tslots::IOB,
    HRIOB3: tslots::IOB,
    HRIOB4: tslots::IOB,
    HRIOB5: tslots::IOB,
    HRIOB6: tslots::IOB,
    HRIOB7: tslots::IOB,
    HRIOB8: tslots::IOB,
    HRIOB9: tslots::IOB,
    HRIOB10: tslots::IOB,
    HRIOB11: tslots::IOB,
    HRIOB12: tslots::IOB,
    HRIOB13: tslots::IOB,
    HRIOB14: tslots::IOB,
    HRIOB15: tslots::IOB,
    HRIOB16: tslots::IOB,
    HRIOB17: tslots::IOB,
    HRIOB18: tslots::IOB,
    HRIOB19: tslots::IOB,
    HRIOB20: tslots::IOB,
    HRIOB21: tslots::IOB,
    HRIOB22: tslots::IOB,
    HRIOB23: tslots::IOB,
    HRIOB24: tslots::IOB,
    HRIOB25: tslots::IOB,
    HRIOB_DIFF_IN0: tslots::IOB,
    HRIOB_DIFF_IN1: tslots::IOB,
    HRIOB_DIFF_IN2: tslots::IOB,
    HRIOB_DIFF_IN3: tslots::IOB,
    HRIOB_DIFF_IN4: tslots::IOB,
    HRIOB_DIFF_IN5: tslots::IOB,
    HRIOB_DIFF_IN6: tslots::IOB,
    HRIOB_DIFF_IN7: tslots::IOB,
    HRIOB_DIFF_IN8: tslots::IOB,
    HRIOB_DIFF_IN9: tslots::IOB,
    HRIOB_DIFF_IN10: tslots::IOB,
    HRIOB_DIFF_IN11: tslots::IOB,
    HRIOB_DIFF_OUT0: tslots::IOB,
    HRIOB_DIFF_OUT1: tslots::IOB,
    HRIOB_DIFF_OUT2: tslots::IOB,
    HRIOB_DIFF_OUT3: tslots::IOB,
    HRIOB_DIFF_OUT4: tslots::IOB,
    HRIOB_DIFF_OUT5: tslots::IOB,
    HRIOB_DIFF_OUT6: tslots::IOB,
    HRIOB_DIFF_OUT7: tslots::IOB,
    HRIOB_DIFF_OUT8: tslots::IOB,
    HRIOB_DIFF_OUT9: tslots::IOB,
    HRIOB_DIFF_OUT10: tslots::IOB,
    HRIOB_DIFF_OUT11: tslots::IOB,
    ABUS_SWITCH_HRIO0: tslots::RCLK_IOB,
    ABUS_SWITCH_HRIO1: tslots::RCLK_IOB,
    ABUS_SWITCH_HRIO2: tslots::RCLK_IOB,
    ABUS_SWITCH_HRIO3: tslots::RCLK_IOB,
    ABUS_SWITCH_HRIO4: tslots::RCLK_IOB,
    ABUS_SWITCH_HRIO5: tslots::RCLK_IOB,
    ABUS_SWITCH_HRIO6: tslots::RCLK_IOB,
    ABUS_SWITCH_HRIO7: tslots::RCLK_IOB,
    HDIOB0: tslots::BEL,
    HDIOB1: tslots::BEL,
    HDIOB2: tslots::BEL,
    HDIOB3: tslots::BEL,
    HDIOB4: tslots::BEL,
    HDIOB5: tslots::BEL,
    HDIOB6: tslots::BEL,
    HDIOB7: tslots::BEL,
    HDIOB8: tslots::BEL,
    HDIOB9: tslots::BEL,
    HDIOB10: tslots::BEL,
    HDIOB11: tslots::BEL,
    HDIOB12: tslots::BEL,
    HDIOB13: tslots::BEL,
    HDIOB14: tslots::BEL,
    HDIOB15: tslots::BEL,
    HDIOB16: tslots::BEL,
    HDIOB17: tslots::BEL,
    HDIOB18: tslots::BEL,
    HDIOB19: tslots::BEL,
    HDIOB20: tslots::BEL,
    HDIOB21: tslots::BEL,
    HDIOB22: tslots::BEL,
    HDIOB23: tslots::BEL,
    HDIOB24: tslots::BEL,
    HDIOB25: tslots::BEL,
    HDIOB26: tslots::BEL,
    HDIOB27: tslots::BEL,
    HDIOB28: tslots::BEL,
    HDIOB29: tslots::BEL,
    HDIOB30: tslots::BEL,
    HDIOB31: tslots::BEL,
    HDIOB32: tslots::BEL,
    HDIOB33: tslots::BEL,
    HDIOB34: tslots::BEL,
    HDIOB35: tslots::BEL,
    HDIOB36: tslots::BEL,
    HDIOB37: tslots::BEL,
    HDIOB38: tslots::BEL,
    HDIOB39: tslots::BEL,
    HDIOB40: tslots::BEL,
    HDIOB41: tslots::BEL,
    HDIOB_DIFF_IN0: tslots::BEL,
    HDIOB_DIFF_IN1: tslots::BEL,
    HDIOB_DIFF_IN2: tslots::BEL,
    HDIOB_DIFF_IN3: tslots::BEL,
    HDIOB_DIFF_IN4: tslots::BEL,
    HDIOB_DIFF_IN5: tslots::BEL,
    HDIOB_DIFF_IN6: tslots::BEL,
    HDIOB_DIFF_IN7: tslots::BEL,
    HDIOB_DIFF_IN8: tslots::BEL,
    HDIOB_DIFF_IN9: tslots::BEL,
    HDIOB_DIFF_IN10: tslots::BEL,
    HDIOB_DIFF_IN11: tslots::BEL,
    HDIOB_DIFF_IN12: tslots::BEL,
    HDIOB_DIFF_IN13: tslots::BEL,
    HDIOB_DIFF_IN14: tslots::BEL,
    HDIOB_DIFF_IN15: tslots::BEL,
    HDIOB_DIFF_IN16: tslots::BEL,
    HDIOB_DIFF_IN17: tslots::BEL,
    HDIOB_DIFF_IN18: tslots::BEL,
    HDIOB_DIFF_IN19: tslots::BEL,
    HDIOB_DIFF_IN20: tslots::BEL,
    HDIOLOGIC0: tslots::BEL,
    HDIOLOGIC1: tslots::BEL,
    HDIOLOGIC2: tslots::BEL,
    HDIOLOGIC3: tslots::BEL,
    HDIOLOGIC4: tslots::BEL,
    HDIOLOGIC5: tslots::BEL,
    HDIOLOGIC6: tslots::BEL,
    HDIOLOGIC7: tslots::BEL,
    HDIOLOGIC8: tslots::BEL,
    HDIOLOGIC9: tslots::BEL,
    HDIOLOGIC10: tslots::BEL,
    HDIOLOGIC11: tslots::BEL,
    HDIOLOGIC12: tslots::BEL,
    HDIOLOGIC13: tslots::BEL,
    HDIOLOGIC14: tslots::BEL,
    HDIOLOGIC15: tslots::BEL,
    HDIOLOGIC16: tslots::BEL,
    HDIOLOGIC17: tslots::BEL,
    HDIOLOGIC18: tslots::BEL,
    HDIOLOGIC19: tslots::BEL,
    HDIOLOGIC20: tslots::BEL,
    HDIOLOGIC21: tslots::BEL,
    HDIOLOGIC22: tslots::BEL,
    HDIOLOGIC23: tslots::BEL,
    HDIOLOGIC24: tslots::BEL,
    HDIOLOGIC25: tslots::BEL,
    HDIOLOGIC26: tslots::BEL,
    HDIOLOGIC27: tslots::BEL,
    HDIOLOGIC28: tslots::BEL,
    HDIOLOGIC29: tslots::BEL,
    HDIOLOGIC30: tslots::BEL,
    HDIOLOGIC31: tslots::BEL,
    HDIOLOGIC32: tslots::BEL,
    HDIOLOGIC33: tslots::BEL,
    HDIOLOGIC34: tslots::BEL,
    HDIOLOGIC35: tslots::BEL,
    HDIOLOGIC36: tslots::BEL,
    HDIOLOGIC37: tslots::BEL,
    HDIOLOGIC38: tslots::BEL,
    HDIOLOGIC39: tslots::BEL,
    HDIOLOGIC40: tslots::BEL,
    HDIOLOGIC41: tslots::BEL,
    HDLOGIC_CSSD0: tslots::BEL,
    HDLOGIC_CSSD1: tslots::BEL,
    HDLOGIC_CSSD2: tslots::BEL,
    HDLOGIC_CSSD3: tslots::BEL,
    HDIO_VREF0: tslots::BEL,
    HDIO_VREF1: tslots::BEL,
    HDIO_VREF2: tslots::BEL,
    HDIO_BIAS: tslots::BEL,
    BUFGCE_HDIO0: tslots::RCLK_BEL,
    BUFGCE_HDIO1: tslots::RCLK_BEL,
    BUFGCE_HDIO2: tslots::RCLK_BEL,
    BUFGCE_HDIO3: tslots::RCLK_BEL,
    ABUS_SWITCH_HDIO0: tslots::RCLK_BEL,
    ABUS_SWITCH_HDIO1: tslots::RCLK_BEL,
    ABUS_SWITCH_HDIO2: tslots::RCLK_BEL,
    ABUS_SWITCH_HDIO3: tslots::RCLK_BEL,
    ABUS_SWITCH_HDIO4: tslots::RCLK_BEL,
    ABUS_SWITCH_HDIO5: tslots::RCLK_BEL,
    ABUS_SWITCH_HDIO6: tslots::RCLK_BEL,
    ABUS_SWITCH_HDIO7: tslots::RCLK_BEL,
    ABUS_SWITCH_HDIO8: tslots::RCLK_BEL,
    ABUS_SWITCH_HDIO9: tslots::RCLK_BEL,
    ABUS_SWITCH_HDIO10: tslots::RCLK_BEL,
    ABUS_SWITCH_HDIO11: tslots::RCLK_BEL,
    LPDDRMC: tslots::BEL,
    XP5IOB0: tslots::BEL,
    XP5IOB1: tslots::BEL,
    XP5IOB2: tslots::BEL,
    XP5IOB3: tslots::BEL,
    XP5IOB4: tslots::BEL,
    XP5IOB5: tslots::BEL,
    XP5IOB6: tslots::BEL,
    XP5IOB7: tslots::BEL,
    XP5IOB8: tslots::BEL,
    XP5IOB9: tslots::BEL,
    XP5IOB10: tslots::BEL,
    XP5IOB11: tslots::BEL,
    XP5IOB12: tslots::BEL,
    XP5IOB13: tslots::BEL,
    XP5IOB14: tslots::BEL,
    XP5IOB15: tslots::BEL,
    XP5IOB16: tslots::BEL,
    XP5IOB17: tslots::BEL,
    XP5IOB18: tslots::BEL,
    XP5IOB19: tslots::BEL,
    XP5IOB20: tslots::BEL,
    XP5IOB21: tslots::BEL,
    XP5IOB22: tslots::BEL,
    XP5IOB23: tslots::BEL,
    XP5IOB24: tslots::BEL,
    XP5IOB25: tslots::BEL,
    XP5IOB26: tslots::BEL,
    XP5IOB27: tslots::BEL,
    XP5IOB28: tslots::BEL,
    XP5IOB29: tslots::BEL,
    XP5IOB30: tslots::BEL,
    XP5IOB31: tslots::BEL,
    XP5IOB32: tslots::BEL,
    XP5IO_VREF0: tslots::BEL,
    XP5IO_VREF1: tslots::BEL,
    XP5IO_VREF2: tslots::BEL,
    XP5IO_VREF3: tslots::BEL,
    XP5IO_VREF4: tslots::BEL,
    XP5IO_VREF5: tslots::BEL,
    XP5IO_VREF6: tslots::BEL,
    XP5IO_VREF7: tslots::BEL,
    XP5IO_VREF8: tslots::BEL,
    XP5IO_VREF9: tslots::BEL,
    XP5IO_VREF10: tslots::BEL,
    X5PHY_LS0: tslots::BEL,
    X5PHY_LS1: tslots::BEL,
    X5PHY_LS2: tslots::BEL,
    X5PHY_LS3: tslots::BEL,
    X5PHY_LS4: tslots::BEL,
    X5PHY_LS5: tslots::BEL,
    X5PHY_LS6: tslots::BEL,
    X5PHY_LS7: tslots::BEL,
    X5PHY_LS8: tslots::BEL,
    X5PHY_LS9: tslots::BEL,
    X5PHY_LS10: tslots::BEL,
    X5PHY_HS0: tslots::BEL,
    X5PHY_HS1: tslots::BEL,
    X5PHY_HS2: tslots::BEL,
    X5PHY_HS3: tslots::BEL,
    X5PHY_HS4: tslots::BEL,
    X5PHY_HS5: tslots::BEL,
    X5PHY_HS6: tslots::BEL,
    X5PHY_HS7: tslots::BEL,
    X5PHY_HS8: tslots::BEL,
    X5PHY_HS9: tslots::BEL,
    X5PHY_HS10: tslots::BEL,
    X5PHY_PLL_SELECT0: tslots::BEL,
    X5PHY_PLL_SELECT1: tslots::BEL,
    X5PHY_PLL_SELECT2: tslots::BEL,
    X5PHY_PLL_SELECT3: tslots::BEL,
    X5PHY_PLL_SELECT4: tslots::BEL,
    X5PHY_PLL_SELECT5: tslots::BEL,
    X5PHY_PLL_SELECT6: tslots::BEL,
    X5PHY_PLL_SELECT7: tslots::BEL,
    X5PHY_PLL_SELECT8: tslots::BEL,
    X5PHY_PLL_SELECT9: tslots::BEL,
    X5PHY_PLL_SELECT10: tslots::BEL,
    XP5PIO_CMU_ANA: tslots::BEL,
    XP5PIO_CMU_DIG_TOP: tslots::BEL,
    ABUS_SWITCH_XP5IO0: tslots::BEL,
    ABUS_SWITCH_XP5IO1: tslots::BEL,
    VCC_XP5IO: tslots::BEL,
    CFG: tslots::BEL,
    ABUS_SWITCH_CFG: tslots::BEL,
    PMV: tslots::BEL,
    PMV2: tslots::BEL,
    PMVIOB: tslots::BEL,
    MTBF3: tslots::BEL,
    CFGIO: tslots::BEL,
    SYSMON: tslots::BEL,
    PCIE3: tslots::BEL,
    PCIE4: tslots::BEL,
    PCIE4C: tslots::BEL,
    PCIE4CE: tslots::BEL,
    CMAC: tslots::BEL,
    ILKN: tslots::BEL,
    FE: tslots::BEL,
    DFE_A: tslots::BEL,
    DFE_B: tslots::BEL,
    DFE_C: tslots::BEL,
    DFE_D: tslots::BEL,
    DFE_E: tslots::BEL,
    DFE_F: tslots::BEL,
    DFE_G: tslots::BEL,
    BUFG_GT0: tslots::BEL,
    BUFG_GT1: tslots::BEL,
    BUFG_GT2: tslots::BEL,
    BUFG_GT3: tslots::BEL,
    BUFG_GT4: tslots::BEL,
    BUFG_GT5: tslots::BEL,
    BUFG_GT6: tslots::BEL,
    BUFG_GT7: tslots::BEL,
    BUFG_GT8: tslots::BEL,
    BUFG_GT9: tslots::BEL,
    BUFG_GT10: tslots::BEL,
    BUFG_GT11: tslots::BEL,
    BUFG_GT12: tslots::BEL,
    BUFG_GT13: tslots::BEL,
    BUFG_GT14: tslots::BEL,
    BUFG_GT15: tslots::BEL,
    BUFG_GT16: tslots::BEL,
    BUFG_GT17: tslots::BEL,
    BUFG_GT18: tslots::BEL,
    BUFG_GT19: tslots::BEL,
    BUFG_GT20: tslots::BEL,
    BUFG_GT21: tslots::BEL,
    BUFG_GT22: tslots::BEL,
    BUFG_GT23: tslots::BEL,
    BUFG_GT_SYNC0: tslots::BEL,
    BUFG_GT_SYNC1: tslots::BEL,
    BUFG_GT_SYNC2: tslots::BEL,
    BUFG_GT_SYNC3: tslots::BEL,
    BUFG_GT_SYNC4: tslots::BEL,
    BUFG_GT_SYNC5: tslots::BEL,
    BUFG_GT_SYNC6: tslots::BEL,
    BUFG_GT_SYNC7: tslots::BEL,
    BUFG_GT_SYNC8: tslots::BEL,
    BUFG_GT_SYNC9: tslots::BEL,
    BUFG_GT_SYNC10: tslots::BEL,
    BUFG_GT_SYNC11: tslots::BEL,
    BUFG_GT_SYNC12: tslots::BEL,
    BUFG_GT_SYNC13: tslots::BEL,
    BUFG_GT_SYNC14: tslots::BEL,
    ABUS_SWITCH_GT0: tslots::BEL,
    ABUS_SWITCH_GT1: tslots::BEL,
    ABUS_SWITCH_GT2: tslots::BEL,
    ABUS_SWITCH_GT3: tslots::BEL,
    ABUS_SWITCH_GT4: tslots::BEL,
    GTH_COMMON: tslots::BEL,
    GTH_CHANNEL0: tslots::BEL,
    GTH_CHANNEL1: tslots::BEL,
    GTH_CHANNEL2: tslots::BEL,
    GTH_CHANNEL3: tslots::BEL,
    GTY_COMMON: tslots::BEL,
    GTY_CHANNEL0: tslots::BEL,
    GTY_CHANNEL1: tslots::BEL,
    GTY_CHANNEL2: tslots::BEL,
    GTY_CHANNEL3: tslots::BEL,
    GTF_COMMON: tslots::BEL,
    GTF_CHANNEL0: tslots::BEL,
    GTF_CHANNEL1: tslots::BEL,
    GTF_CHANNEL2: tslots::BEL,
    GTF_CHANNEL3: tslots::BEL,
    GTM_REFCLK: tslots::BEL,
    GTM_DUAL: tslots::BEL,
    HSDAC: tslots::BEL,
    HSADC: tslots::BEL,
    RFDAC: tslots::BEL,
    RFADC: tslots::BEL,
    PS: tslots::BEL,
    VCU: tslots::BEL,
    BLI_HBM_APB_INTF: tslots::BEL,
    BLI_HBM_AXI_INTF: tslots::BEL,
    ABUS_SWITCH_HBM0: tslots::CMT,
    ABUS_SWITCH_HBM1: tslots::CMT,
    ABUS_SWITCH_HBM2: tslots::CMT,
    ABUS_SWITCH_HBM3: tslots::CMT,
    ABUS_SWITCH_HBM4: tslots::CMT,
    ABUS_SWITCH_HBM5: tslots::CMT,
    ABUS_SWITCH_HBM6: tslots::CMT,
    ABUS_SWITCH_HBM7: tslots::CMT,
    BUFG_PS0: tslots::RCLK_BEL,
    BUFG_PS1: tslots::RCLK_BEL,
    BUFG_PS2: tslots::RCLK_BEL,
    BUFG_PS3: tslots::RCLK_BEL,
    BUFG_PS4: tslots::RCLK_BEL,
    BUFG_PS5: tslots::RCLK_BEL,
    BUFG_PS6: tslots::RCLK_BEL,
    BUFG_PS7: tslots::RCLK_BEL,
    BUFG_PS8: tslots::RCLK_BEL,
    BUFG_PS9: tslots::RCLK_BEL,
    BUFG_PS10: tslots::RCLK_BEL,
    BUFG_PS11: tslots::RCLK_BEL,
    BUFG_PS12: tslots::RCLK_BEL,
    BUFG_PS13: tslots::RCLK_BEL,
    BUFG_PS14: tslots::RCLK_BEL,
    BUFG_PS15: tslots::RCLK_BEL,
    BUFG_PS16: tslots::RCLK_BEL,
    BUFG_PS17: tslots::RCLK_BEL,
    BUFG_PS18: tslots::RCLK_BEL,
    BUFG_PS19: tslots::RCLK_BEL,
    BUFG_PS20: tslots::RCLK_BEL,
    BUFG_PS21: tslots::RCLK_BEL,
    BUFG_PS22: tslots::RCLK_BEL,
    BUFG_PS23: tslots::RCLK_BEL,
    BUFCE_LEAF_X16_S: tslots::RCLK_INT,
    BUFCE_LEAF_X16_N: tslots::RCLK_INT,
    BUFCE_LEAF_S0: tslots::RCLK_INT,
    BUFCE_LEAF_S1: tslots::RCLK_INT,
    BUFCE_LEAF_S2: tslots::RCLK_INT,
    BUFCE_LEAF_S3: tslots::RCLK_INT,
    BUFCE_LEAF_S4: tslots::RCLK_INT,
    BUFCE_LEAF_S5: tslots::RCLK_INT,
    BUFCE_LEAF_S6: tslots::RCLK_INT,
    BUFCE_LEAF_S7: tslots::RCLK_INT,
    BUFCE_LEAF_S8: tslots::RCLK_INT,
    BUFCE_LEAF_S9: tslots::RCLK_INT,
    BUFCE_LEAF_S10: tslots::RCLK_INT,
    BUFCE_LEAF_S11: tslots::RCLK_INT,
    BUFCE_LEAF_S12: tslots::RCLK_INT,
    BUFCE_LEAF_S13: tslots::RCLK_INT,
    BUFCE_LEAF_S14: tslots::RCLK_INT,
    BUFCE_LEAF_S15: tslots::RCLK_INT,
    BUFCE_LEAF_N0: tslots::RCLK_INT,
    BUFCE_LEAF_N1: tslots::RCLK_INT,
    BUFCE_LEAF_N2: tslots::RCLK_INT,
    BUFCE_LEAF_N3: tslots::RCLK_INT,
    BUFCE_LEAF_N4: tslots::RCLK_INT,
    BUFCE_LEAF_N5: tslots::RCLK_INT,
    BUFCE_LEAF_N6: tslots::RCLK_INT,
    BUFCE_LEAF_N7: tslots::RCLK_INT,
    BUFCE_LEAF_N8: tslots::RCLK_INT,
    BUFCE_LEAF_N9: tslots::RCLK_INT,
    BUFCE_LEAF_N10: tslots::RCLK_INT,
    BUFCE_LEAF_N11: tslots::RCLK_INT,
    BUFCE_LEAF_N12: tslots::RCLK_INT,
    BUFCE_LEAF_N13: tslots::RCLK_INT,
    BUFCE_LEAF_N14: tslots::RCLK_INT,
    BUFCE_LEAF_N15: tslots::RCLK_INT,
    RCLK_INT_CLK: tslots::RCLK_INT,
    BUFCE_ROW_RCLK0: tslots::RCLK_V,
    BUFCE_ROW_RCLK1: tslots::RCLK_V,
    BUFCE_ROW_RCLK2: tslots::RCLK_V,
    BUFCE_ROW_RCLK3: tslots::RCLK_V,
    BUFCE_ROW_CMT0: tslots::CMT,
    BUFCE_ROW_CMT1: tslots::CMT,
    BUFCE_ROW_CMT2: tslots::CMT,
    BUFCE_ROW_CMT3: tslots::CMT,
    BUFCE_ROW_CMT4: tslots::CMT,
    BUFCE_ROW_CMT5: tslots::CMT,
    BUFCE_ROW_CMT6: tslots::CMT,
    BUFCE_ROW_CMT7: tslots::CMT,
    BUFCE_ROW_CMT8: tslots::CMT,
    BUFCE_ROW_CMT9: tslots::CMT,
    BUFCE_ROW_CMT10: tslots::CMT,
    BUFCE_ROW_CMT11: tslots::CMT,
    BUFCE_ROW_CMT12: tslots::CMT,
    BUFCE_ROW_CMT13: tslots::CMT,
    BUFCE_ROW_CMT14: tslots::CMT,
    BUFCE_ROW_CMT15: tslots::CMT,
    BUFCE_ROW_CMT16: tslots::CMT,
    BUFCE_ROW_CMT17: tslots::CMT,
    BUFCE_ROW_CMT18: tslots::CMT,
    BUFCE_ROW_CMT19: tslots::CMT,
    BUFCE_ROW_CMT20: tslots::CMT,
    BUFCE_ROW_CMT21: tslots::CMT,
    BUFCE_ROW_CMT22: tslots::CMT,
    BUFCE_ROW_CMT23: tslots::CMT,
    GCLK_TEST_BUF_RCLK0: tslots::RCLK_V,
    GCLK_TEST_BUF_RCLK1: tslots::RCLK_V,
    GCLK_TEST_BUF_RCLK2: tslots::RCLK_V,
    GCLK_TEST_BUF_RCLK3: tslots::RCLK_V,
    GCLK_TEST_BUF_CMT0: tslots::CMT,
    GCLK_TEST_BUF_CMT1: tslots::CMT,
    GCLK_TEST_BUF_CMT2: tslots::CMT,
    GCLK_TEST_BUF_CMT3: tslots::CMT,
    GCLK_TEST_BUF_CMT4: tslots::CMT,
    GCLK_TEST_BUF_CMT5: tslots::CMT,
    GCLK_TEST_BUF_CMT6: tslots::CMT,
    GCLK_TEST_BUF_CMT7: tslots::CMT,
    GCLK_TEST_BUF_CMT8: tslots::CMT,
    GCLK_TEST_BUF_CMT9: tslots::CMT,
    GCLK_TEST_BUF_CMT10: tslots::CMT,
    GCLK_TEST_BUF_CMT11: tslots::CMT,
    GCLK_TEST_BUF_CMT12: tslots::CMT,
    GCLK_TEST_BUF_CMT13: tslots::CMT,
    GCLK_TEST_BUF_CMT14: tslots::CMT,
    GCLK_TEST_BUF_CMT15: tslots::CMT,
    GCLK_TEST_BUF_CMT16: tslots::CMT,
    GCLK_TEST_BUF_CMT17: tslots::CMT,
    GCLK_TEST_BUF_CMT18: tslots::CMT,
    GCLK_TEST_BUF_CMT19: tslots::CMT,
    GCLK_TEST_BUF_CMT20: tslots::CMT,
    GCLK_TEST_BUF_CMT21: tslots::CMT,
    GCLK_TEST_BUF_CMT22: tslots::CMT,
    GCLK_TEST_BUF_CMT23: tslots::CMT,
    BUFGCE0: tslots::CMT,
    BUFGCE1: tslots::CMT,
    BUFGCE2: tslots::CMT,
    BUFGCE3: tslots::CMT,
    BUFGCE4: tslots::CMT,
    BUFGCE5: tslots::CMT,
    BUFGCE6: tslots::CMT,
    BUFGCE7: tslots::CMT,
    BUFGCE8: tslots::CMT,
    BUFGCE9: tslots::CMT,
    BUFGCE10: tslots::CMT,
    BUFGCE11: tslots::CMT,
    BUFGCE12: tslots::CMT,
    BUFGCE13: tslots::CMT,
    BUFGCE14: tslots::CMT,
    BUFGCE15: tslots::CMT,
    BUFGCE16: tslots::CMT,
    BUFGCE17: tslots::CMT,
    BUFGCE18: tslots::CMT,
    BUFGCE19: tslots::CMT,
    BUFGCE20: tslots::CMT,
    BUFGCE21: tslots::CMT,
    BUFGCE22: tslots::CMT,
    BUFGCE23: tslots::CMT,
    BUFGCTRL0: tslots::CMT,
    BUFGCTRL1: tslots::CMT,
    BUFGCTRL2: tslots::CMT,
    BUFGCTRL3: tslots::CMT,
    BUFGCTRL4: tslots::CMT,
    BUFGCTRL5: tslots::CMT,
    BUFGCTRL6: tslots::CMT,
    BUFGCTRL7: tslots::CMT,
    BUFGCE_DIV0: tslots::CMT,
    BUFGCE_DIV1: tslots::CMT,
    BUFGCE_DIV2: tslots::CMT,
    BUFGCE_DIV3: tslots::CMT,
    PLL0: tslots::CMT,
    PLL1: tslots::CMT,
    PLLXP0: tslots::CMT,
    PLLXP1: tslots::CMT,
    MMCM: tslots::CMT,
    CMT: tslots::CMT,
    CMTXP: tslots::CMT,
    VCC_CMT: tslots::CMT,
    BITSLICE0: tslots::BEL,
    BITSLICE1: tslots::BEL,
    BITSLICE2: tslots::BEL,
    BITSLICE3: tslots::BEL,
    BITSLICE4: tslots::BEL,
    BITSLICE5: tslots::BEL,
    BITSLICE6: tslots::BEL,
    BITSLICE7: tslots::BEL,
    BITSLICE8: tslots::BEL,
    BITSLICE9: tslots::BEL,
    BITSLICE10: tslots::BEL,
    BITSLICE11: tslots::BEL,
    BITSLICE12: tslots::BEL,
    BITSLICE13: tslots::BEL,
    BITSLICE14: tslots::BEL,
    BITSLICE15: tslots::BEL,
    BITSLICE16: tslots::BEL,
    BITSLICE17: tslots::BEL,
    BITSLICE18: tslots::BEL,
    BITSLICE19: tslots::BEL,
    BITSLICE20: tslots::BEL,
    BITSLICE21: tslots::BEL,
    BITSLICE22: tslots::BEL,
    BITSLICE23: tslots::BEL,
    BITSLICE24: tslots::BEL,
    BITSLICE25: tslots::BEL,
    BITSLICE26: tslots::BEL,
    BITSLICE27: tslots::BEL,
    BITSLICE28: tslots::BEL,
    BITSLICE29: tslots::BEL,
    BITSLICE30: tslots::BEL,
    BITSLICE31: tslots::BEL,
    BITSLICE32: tslots::BEL,
    BITSLICE33: tslots::BEL,
    BITSLICE34: tslots::BEL,
    BITSLICE35: tslots::BEL,
    BITSLICE36: tslots::BEL,
    BITSLICE37: tslots::BEL,
    BITSLICE38: tslots::BEL,
    BITSLICE39: tslots::BEL,
    BITSLICE40: tslots::BEL,
    BITSLICE41: tslots::BEL,
    BITSLICE42: tslots::BEL,
    BITSLICE43: tslots::BEL,
    BITSLICE44: tslots::BEL,
    BITSLICE45: tslots::BEL,
    BITSLICE46: tslots::BEL,
    BITSLICE47: tslots::BEL,
    BITSLICE48: tslots::BEL,
    BITSLICE49: tslots::BEL,
    BITSLICE50: tslots::BEL,
    BITSLICE51: tslots::BEL,
    BITSLICE_T0: tslots::BEL,
    BITSLICE_T1: tslots::BEL,
    BITSLICE_T2: tslots::BEL,
    BITSLICE_T3: tslots::BEL,
    BITSLICE_T4: tslots::BEL,
    BITSLICE_T5: tslots::BEL,
    BITSLICE_T6: tslots::BEL,
    BITSLICE_T7: tslots::BEL,
    BITSLICE_CONTROL0: tslots::BEL,
    BITSLICE_CONTROL1: tslots::BEL,
    BITSLICE_CONTROL2: tslots::BEL,
    BITSLICE_CONTROL3: tslots::BEL,
    BITSLICE_CONTROL4: tslots::BEL,
    BITSLICE_CONTROL5: tslots::BEL,
    BITSLICE_CONTROL6: tslots::BEL,
    BITSLICE_CONTROL7: tslots::BEL,
    PLL_SELECT0: tslots::BEL,
    PLL_SELECT1: tslots::BEL,
    PLL_SELECT2: tslots::BEL,
    PLL_SELECT3: tslots::BEL,
    PLL_SELECT4: tslots::BEL,
    PLL_SELECT5: tslots::BEL,
    PLL_SELECT6: tslots::BEL,
    PLL_SELECT7: tslots::BEL,
    RIU_OR0: tslots::BEL,
    RIU_OR1: tslots::BEL,
    RIU_OR2: tslots::BEL,
    RIU_OR3: tslots::BEL,
    XIPHY_FEEDTHROUGH0: tslots::BEL,
    XIPHY_FEEDTHROUGH1: tslots::BEL,
    XIPHY_FEEDTHROUGH2: tslots::BEL,
    XIPHY_FEEDTHROUGH3: tslots::BEL,
    ABUS_SWITCH_CMT: tslots::CMT,
    HBM_REF_CLK0: tslots::CMT,
    HBM_REF_CLK1: tslots::CMT,
    VBUS_SWITCH0: tslots::RCLK_V,
    VBUS_SWITCH1: tslots::RCLK_V,
    VBUS_SWITCH2: tslots::RCLK_V,
    VCC_RCLK_V: tslots::RCLK_V,
    RCLK_SPLITTER: tslots::RCLK_SPLITTER,
    VCC_RCLK_SPLITTER: tslots::RCLK_SPLITTER,
    RCLK_HROUTE_SPLITTER: tslots::RCLK_SPLITTER,
    VCC_RCLK_HROUTE_SPLITTER: tslots::RCLK_SPLITTER,
    RCLK_GT: tslots::BEL,
    VCC_GT: tslots::BEL,
    RCLK_PS: tslots::RCLK_BEL,
    VCC_RCLK_PS: tslots::RCLK_BEL,
    XIPHY_BYTE: tslots::BEL,
    RCLK_XIPHY: tslots::RCLK_BEL,
    VCC_RCLK_XIPHY: tslots::RCLK_BEL,
    RCLK_HDIO: tslots::RCLK_BEL,
    RCLK_HDIOS: tslots::RCLK_BEL,
    RCLK_HDIOL: tslots::RCLK_BEL,
    VCC_RCLK_HDIO: tslots::RCLK_BEL,
];

pub const LAGUNA: [BelSlotId; 4] = [LAGUNA0, LAGUNA1, LAGUNA2, LAGUNA3];

pub const DSP: [BelSlotId; 2] = [DSP0, DSP1];

pub const HARD_SYNC: [BelSlotId; 4] = [HARD_SYNC0, HARD_SYNC1, HARD_SYNC2, HARD_SYNC3];

pub const URAM: [BelSlotId; 4] = [URAM0, URAM1, URAM2, URAM3];

pub const HPIOB: [BelSlotId; 26] = [
    HPIOB0, HPIOB1, HPIOB2, HPIOB3, HPIOB4, HPIOB5, HPIOB6, HPIOB7, HPIOB8, HPIOB9, HPIOB10,
    HPIOB11, HPIOB12, HPIOB13, HPIOB14, HPIOB15, HPIOB16, HPIOB17, HPIOB18, HPIOB19, HPIOB20,
    HPIOB21, HPIOB22, HPIOB23, HPIOB24, HPIOB25,
];

pub const HRIOB: [BelSlotId; 26] = [
    HRIOB0, HRIOB1, HRIOB2, HRIOB3, HRIOB4, HRIOB5, HRIOB6, HRIOB7, HRIOB8, HRIOB9, HRIOB10,
    HRIOB11, HRIOB12, HRIOB13, HRIOB14, HRIOB15, HRIOB16, HRIOB17, HRIOB18, HRIOB19, HRIOB20,
    HRIOB21, HRIOB22, HRIOB23, HRIOB24, HRIOB25,
];

pub const HPIOB_DIFF_IN: [BelSlotId; 12] = [
    HPIOB_DIFF_IN0,
    HPIOB_DIFF_IN1,
    HPIOB_DIFF_IN2,
    HPIOB_DIFF_IN3,
    HPIOB_DIFF_IN4,
    HPIOB_DIFF_IN5,
    HPIOB_DIFF_IN6,
    HPIOB_DIFF_IN7,
    HPIOB_DIFF_IN8,
    HPIOB_DIFF_IN9,
    HPIOB_DIFF_IN10,
    HPIOB_DIFF_IN11,
];

pub const HPIOB_DIFF_OUT: [BelSlotId; 12] = [
    HPIOB_DIFF_OUT0,
    HPIOB_DIFF_OUT1,
    HPIOB_DIFF_OUT2,
    HPIOB_DIFF_OUT3,
    HPIOB_DIFF_OUT4,
    HPIOB_DIFF_OUT5,
    HPIOB_DIFF_OUT6,
    HPIOB_DIFF_OUT7,
    HPIOB_DIFF_OUT8,
    HPIOB_DIFF_OUT9,
    HPIOB_DIFF_OUT10,
    HPIOB_DIFF_OUT11,
];

pub const HPIOB_DCI: [BelSlotId; 2] = [HPIOB_DCI0, HPIOB_DCI1];

pub const HRIOB_DIFF_IN: [BelSlotId; 12] = [
    HRIOB_DIFF_IN0,
    HRIOB_DIFF_IN1,
    HRIOB_DIFF_IN2,
    HRIOB_DIFF_IN3,
    HRIOB_DIFF_IN4,
    HRIOB_DIFF_IN5,
    HRIOB_DIFF_IN6,
    HRIOB_DIFF_IN7,
    HRIOB_DIFF_IN8,
    HRIOB_DIFF_IN9,
    HRIOB_DIFF_IN10,
    HRIOB_DIFF_IN11,
];

pub const HRIOB_DIFF_OUT: [BelSlotId; 12] = [
    HRIOB_DIFF_OUT0,
    HRIOB_DIFF_OUT1,
    HRIOB_DIFF_OUT2,
    HRIOB_DIFF_OUT3,
    HRIOB_DIFF_OUT4,
    HRIOB_DIFF_OUT5,
    HRIOB_DIFF_OUT6,
    HRIOB_DIFF_OUT7,
    HRIOB_DIFF_OUT8,
    HRIOB_DIFF_OUT9,
    HRIOB_DIFF_OUT10,
    HRIOB_DIFF_OUT11,
];

pub const ABUS_SWITCH_HPIO: [BelSlotId; 7] = [
    ABUS_SWITCH_HPIO0,
    ABUS_SWITCH_HPIO1,
    ABUS_SWITCH_HPIO2,
    ABUS_SWITCH_HPIO3,
    ABUS_SWITCH_HPIO4,
    ABUS_SWITCH_HPIO5,
    ABUS_SWITCH_HPIO6,
];

pub const ABUS_SWITCH_HRIO: [BelSlotId; 8] = [
    ABUS_SWITCH_HRIO0,
    ABUS_SWITCH_HRIO1,
    ABUS_SWITCH_HRIO2,
    ABUS_SWITCH_HRIO3,
    ABUS_SWITCH_HRIO4,
    ABUS_SWITCH_HRIO5,
    ABUS_SWITCH_HRIO6,
    ABUS_SWITCH_HRIO7,
];

pub const HDIOB: [BelSlotId; 42] = [
    HDIOB0, HDIOB1, HDIOB2, HDIOB3, HDIOB4, HDIOB5, HDIOB6, HDIOB7, HDIOB8, HDIOB9, HDIOB10,
    HDIOB11, HDIOB12, HDIOB13, HDIOB14, HDIOB15, HDIOB16, HDIOB17, HDIOB18, HDIOB19, HDIOB20,
    HDIOB21, HDIOB22, HDIOB23, HDIOB24, HDIOB25, HDIOB26, HDIOB27, HDIOB28, HDIOB29, HDIOB30,
    HDIOB31, HDIOB32, HDIOB33, HDIOB34, HDIOB35, HDIOB36, HDIOB37, HDIOB38, HDIOB39, HDIOB40,
    HDIOB41,
];

pub const HDIOB_DIFF_IN: [BelSlotId; 21] = [
    HDIOB_DIFF_IN0,
    HDIOB_DIFF_IN1,
    HDIOB_DIFF_IN2,
    HDIOB_DIFF_IN3,
    HDIOB_DIFF_IN4,
    HDIOB_DIFF_IN5,
    HDIOB_DIFF_IN6,
    HDIOB_DIFF_IN7,
    HDIOB_DIFF_IN8,
    HDIOB_DIFF_IN9,
    HDIOB_DIFF_IN10,
    HDIOB_DIFF_IN11,
    HDIOB_DIFF_IN12,
    HDIOB_DIFF_IN13,
    HDIOB_DIFF_IN14,
    HDIOB_DIFF_IN15,
    HDIOB_DIFF_IN16,
    HDIOB_DIFF_IN17,
    HDIOB_DIFF_IN18,
    HDIOB_DIFF_IN19,
    HDIOB_DIFF_IN20,
];

pub const HDIOLOGIC: [BelSlotId; 42] = [
    HDIOLOGIC0,
    HDIOLOGIC1,
    HDIOLOGIC2,
    HDIOLOGIC3,
    HDIOLOGIC4,
    HDIOLOGIC5,
    HDIOLOGIC6,
    HDIOLOGIC7,
    HDIOLOGIC8,
    HDIOLOGIC9,
    HDIOLOGIC10,
    HDIOLOGIC11,
    HDIOLOGIC12,
    HDIOLOGIC13,
    HDIOLOGIC14,
    HDIOLOGIC15,
    HDIOLOGIC16,
    HDIOLOGIC17,
    HDIOLOGIC18,
    HDIOLOGIC19,
    HDIOLOGIC20,
    HDIOLOGIC21,
    HDIOLOGIC22,
    HDIOLOGIC23,
    HDIOLOGIC24,
    HDIOLOGIC25,
    HDIOLOGIC26,
    HDIOLOGIC27,
    HDIOLOGIC28,
    HDIOLOGIC29,
    HDIOLOGIC30,
    HDIOLOGIC31,
    HDIOLOGIC32,
    HDIOLOGIC33,
    HDIOLOGIC34,
    HDIOLOGIC35,
    HDIOLOGIC36,
    HDIOLOGIC37,
    HDIOLOGIC38,
    HDIOLOGIC39,
    HDIOLOGIC40,
    HDIOLOGIC41,
];

pub const HDLOGIC_CSSD: [BelSlotId; 4] =
    [HDLOGIC_CSSD0, HDLOGIC_CSSD1, HDLOGIC_CSSD2, HDLOGIC_CSSD3];

pub const HDIO_VREF: [BelSlotId; 3] = [HDIO_VREF0, HDIO_VREF1, HDIO_VREF2];

pub const BUFGCE_HDIO: [BelSlotId; 4] = [BUFGCE_HDIO0, BUFGCE_HDIO1, BUFGCE_HDIO2, BUFGCE_HDIO3];

pub const ABUS_SWITCH_HDIO: [BelSlotId; 12] = [
    ABUS_SWITCH_HDIO0,
    ABUS_SWITCH_HDIO1,
    ABUS_SWITCH_HDIO2,
    ABUS_SWITCH_HDIO3,
    ABUS_SWITCH_HDIO4,
    ABUS_SWITCH_HDIO5,
    ABUS_SWITCH_HDIO6,
    ABUS_SWITCH_HDIO7,
    ABUS_SWITCH_HDIO8,
    ABUS_SWITCH_HDIO9,
    ABUS_SWITCH_HDIO10,
    ABUS_SWITCH_HDIO11,
];

pub const XP5IOB: [BelSlotId; 33] = [
    XP5IOB0, XP5IOB1, XP5IOB2, XP5IOB3, XP5IOB4, XP5IOB5, XP5IOB6, XP5IOB7, XP5IOB8, XP5IOB9,
    XP5IOB10, XP5IOB11, XP5IOB12, XP5IOB13, XP5IOB14, XP5IOB15, XP5IOB16, XP5IOB17, XP5IOB18,
    XP5IOB19, XP5IOB20, XP5IOB21, XP5IOB22, XP5IOB23, XP5IOB24, XP5IOB25, XP5IOB26, XP5IOB27,
    XP5IOB28, XP5IOB29, XP5IOB30, XP5IOB31, XP5IOB32,
];

pub const XP5IO_VREF: [BelSlotId; 11] = [
    XP5IO_VREF0,
    XP5IO_VREF1,
    XP5IO_VREF2,
    XP5IO_VREF3,
    XP5IO_VREF4,
    XP5IO_VREF5,
    XP5IO_VREF6,
    XP5IO_VREF7,
    XP5IO_VREF8,
    XP5IO_VREF9,
    XP5IO_VREF10,
];

pub const X5PHY_LS: [BelSlotId; 11] = [
    X5PHY_LS0, X5PHY_LS1, X5PHY_LS2, X5PHY_LS3, X5PHY_LS4, X5PHY_LS5, X5PHY_LS6, X5PHY_LS7,
    X5PHY_LS8, X5PHY_LS9, X5PHY_LS10,
];

pub const X5PHY_HS: [BelSlotId; 11] = [
    X5PHY_HS0, X5PHY_HS1, X5PHY_HS2, X5PHY_HS3, X5PHY_HS4, X5PHY_HS5, X5PHY_HS6, X5PHY_HS7,
    X5PHY_HS8, X5PHY_HS9, X5PHY_HS10,
];

pub const X5PHY_PLL_SELECT: [BelSlotId; 11] = [
    X5PHY_PLL_SELECT0,
    X5PHY_PLL_SELECT1,
    X5PHY_PLL_SELECT2,
    X5PHY_PLL_SELECT3,
    X5PHY_PLL_SELECT4,
    X5PHY_PLL_SELECT5,
    X5PHY_PLL_SELECT6,
    X5PHY_PLL_SELECT7,
    X5PHY_PLL_SELECT8,
    X5PHY_PLL_SELECT9,
    X5PHY_PLL_SELECT10,
];

pub const ABUS_SWITCH_XP5IO: [BelSlotId; 2] = [ABUS_SWITCH_XP5IO0, ABUS_SWITCH_XP5IO1];

pub const BUFG_GT: [BelSlotId; 24] = [
    BUFG_GT0, BUFG_GT1, BUFG_GT2, BUFG_GT3, BUFG_GT4, BUFG_GT5, BUFG_GT6, BUFG_GT7, BUFG_GT8,
    BUFG_GT9, BUFG_GT10, BUFG_GT11, BUFG_GT12, BUFG_GT13, BUFG_GT14, BUFG_GT15, BUFG_GT16,
    BUFG_GT17, BUFG_GT18, BUFG_GT19, BUFG_GT20, BUFG_GT21, BUFG_GT22, BUFG_GT23,
];

pub const BUFG_GT_SYNC: [BelSlotId; 15] = [
    BUFG_GT_SYNC0,
    BUFG_GT_SYNC1,
    BUFG_GT_SYNC2,
    BUFG_GT_SYNC3,
    BUFG_GT_SYNC4,
    BUFG_GT_SYNC5,
    BUFG_GT_SYNC6,
    BUFG_GT_SYNC7,
    BUFG_GT_SYNC8,
    BUFG_GT_SYNC9,
    BUFG_GT_SYNC10,
    BUFG_GT_SYNC11,
    BUFG_GT_SYNC12,
    BUFG_GT_SYNC13,
    BUFG_GT_SYNC14,
];

pub const ABUS_SWITCH_GT: [BelSlotId; 5] = [
    ABUS_SWITCH_GT0,
    ABUS_SWITCH_GT1,
    ABUS_SWITCH_GT2,
    ABUS_SWITCH_GT3,
    ABUS_SWITCH_GT4,
];

pub const GTH_CHANNEL: [BelSlotId; 4] = [GTH_CHANNEL0, GTH_CHANNEL1, GTH_CHANNEL2, GTH_CHANNEL3];

pub const GTY_CHANNEL: [BelSlotId; 4] = [GTY_CHANNEL0, GTY_CHANNEL1, GTY_CHANNEL2, GTY_CHANNEL3];

pub const GTF_CHANNEL: [BelSlotId; 4] = [GTF_CHANNEL0, GTF_CHANNEL1, GTF_CHANNEL2, GTF_CHANNEL3];

pub const BUFG_PS: [BelSlotId; 24] = [
    BUFG_PS0, BUFG_PS1, BUFG_PS2, BUFG_PS3, BUFG_PS4, BUFG_PS5, BUFG_PS6, BUFG_PS7, BUFG_PS8,
    BUFG_PS9, BUFG_PS10, BUFG_PS11, BUFG_PS12, BUFG_PS13, BUFG_PS14, BUFG_PS15, BUFG_PS16,
    BUFG_PS17, BUFG_PS18, BUFG_PS19, BUFG_PS20, BUFG_PS21, BUFG_PS22, BUFG_PS23,
];

pub const ABUS_SWITCH_HBM: [BelSlotId; 8] = [
    ABUS_SWITCH_HBM0,
    ABUS_SWITCH_HBM1,
    ABUS_SWITCH_HBM2,
    ABUS_SWITCH_HBM3,
    ABUS_SWITCH_HBM4,
    ABUS_SWITCH_HBM5,
    ABUS_SWITCH_HBM6,
    ABUS_SWITCH_HBM7,
];

pub const BUFCE_LEAF_S: [BelSlotId; 16] = [
    BUFCE_LEAF_S0,
    BUFCE_LEAF_S1,
    BUFCE_LEAF_S2,
    BUFCE_LEAF_S3,
    BUFCE_LEAF_S4,
    BUFCE_LEAF_S5,
    BUFCE_LEAF_S6,
    BUFCE_LEAF_S7,
    BUFCE_LEAF_S8,
    BUFCE_LEAF_S9,
    BUFCE_LEAF_S10,
    BUFCE_LEAF_S11,
    BUFCE_LEAF_S12,
    BUFCE_LEAF_S13,
    BUFCE_LEAF_S14,
    BUFCE_LEAF_S15,
];

pub const BUFCE_LEAF_N: [BelSlotId; 16] = [
    BUFCE_LEAF_N0,
    BUFCE_LEAF_N1,
    BUFCE_LEAF_N2,
    BUFCE_LEAF_N3,
    BUFCE_LEAF_N4,
    BUFCE_LEAF_N5,
    BUFCE_LEAF_N6,
    BUFCE_LEAF_N7,
    BUFCE_LEAF_N8,
    BUFCE_LEAF_N9,
    BUFCE_LEAF_N10,
    BUFCE_LEAF_N11,
    BUFCE_LEAF_N12,
    BUFCE_LEAF_N13,
    BUFCE_LEAF_N14,
    BUFCE_LEAF_N15,
];

pub const BUFCE_LEAF: [BelSlotId; 32] = [
    BUFCE_LEAF_S0,
    BUFCE_LEAF_S1,
    BUFCE_LEAF_S2,
    BUFCE_LEAF_S3,
    BUFCE_LEAF_S4,
    BUFCE_LEAF_S5,
    BUFCE_LEAF_S6,
    BUFCE_LEAF_S7,
    BUFCE_LEAF_S8,
    BUFCE_LEAF_S9,
    BUFCE_LEAF_S10,
    BUFCE_LEAF_S11,
    BUFCE_LEAF_S12,
    BUFCE_LEAF_S13,
    BUFCE_LEAF_S14,
    BUFCE_LEAF_S15,
    BUFCE_LEAF_N0,
    BUFCE_LEAF_N1,
    BUFCE_LEAF_N2,
    BUFCE_LEAF_N3,
    BUFCE_LEAF_N4,
    BUFCE_LEAF_N5,
    BUFCE_LEAF_N6,
    BUFCE_LEAF_N7,
    BUFCE_LEAF_N8,
    BUFCE_LEAF_N9,
    BUFCE_LEAF_N10,
    BUFCE_LEAF_N11,
    BUFCE_LEAF_N12,
    BUFCE_LEAF_N13,
    BUFCE_LEAF_N14,
    BUFCE_LEAF_N15,
];

pub const BUFCE_ROW_CMT: [BelSlotId; 24] = [
    BUFCE_ROW_CMT0,
    BUFCE_ROW_CMT1,
    BUFCE_ROW_CMT2,
    BUFCE_ROW_CMT3,
    BUFCE_ROW_CMT4,
    BUFCE_ROW_CMT5,
    BUFCE_ROW_CMT6,
    BUFCE_ROW_CMT7,
    BUFCE_ROW_CMT8,
    BUFCE_ROW_CMT9,
    BUFCE_ROW_CMT10,
    BUFCE_ROW_CMT11,
    BUFCE_ROW_CMT12,
    BUFCE_ROW_CMT13,
    BUFCE_ROW_CMT14,
    BUFCE_ROW_CMT15,
    BUFCE_ROW_CMT16,
    BUFCE_ROW_CMT17,
    BUFCE_ROW_CMT18,
    BUFCE_ROW_CMT19,
    BUFCE_ROW_CMT20,
    BUFCE_ROW_CMT21,
    BUFCE_ROW_CMT22,
    BUFCE_ROW_CMT23,
];

pub const GCLK_TEST_BUF_CMT: [BelSlotId; 24] = [
    GCLK_TEST_BUF_CMT0,
    GCLK_TEST_BUF_CMT1,
    GCLK_TEST_BUF_CMT2,
    GCLK_TEST_BUF_CMT3,
    GCLK_TEST_BUF_CMT4,
    GCLK_TEST_BUF_CMT5,
    GCLK_TEST_BUF_CMT6,
    GCLK_TEST_BUF_CMT7,
    GCLK_TEST_BUF_CMT8,
    GCLK_TEST_BUF_CMT9,
    GCLK_TEST_BUF_CMT10,
    GCLK_TEST_BUF_CMT11,
    GCLK_TEST_BUF_CMT12,
    GCLK_TEST_BUF_CMT13,
    GCLK_TEST_BUF_CMT14,
    GCLK_TEST_BUF_CMT15,
    GCLK_TEST_BUF_CMT16,
    GCLK_TEST_BUF_CMT17,
    GCLK_TEST_BUF_CMT18,
    GCLK_TEST_BUF_CMT19,
    GCLK_TEST_BUF_CMT20,
    GCLK_TEST_BUF_CMT21,
    GCLK_TEST_BUF_CMT22,
    GCLK_TEST_BUF_CMT23,
];

pub const BUFCE_ROW_RCLK: [BelSlotId; 4] = [
    BUFCE_ROW_RCLK0,
    BUFCE_ROW_RCLK1,
    BUFCE_ROW_RCLK2,
    BUFCE_ROW_RCLK3,
];

pub const GCLK_TEST_BUF_RCLK: [BelSlotId; 4] = [
    GCLK_TEST_BUF_RCLK0,
    GCLK_TEST_BUF_RCLK1,
    GCLK_TEST_BUF_RCLK2,
    GCLK_TEST_BUF_RCLK3,
];

pub const BUFGCE: [BelSlotId; 24] = [
    BUFGCE0, BUFGCE1, BUFGCE2, BUFGCE3, BUFGCE4, BUFGCE5, BUFGCE6, BUFGCE7, BUFGCE8, BUFGCE9,
    BUFGCE10, BUFGCE11, BUFGCE12, BUFGCE13, BUFGCE14, BUFGCE15, BUFGCE16, BUFGCE17, BUFGCE18,
    BUFGCE19, BUFGCE20, BUFGCE21, BUFGCE22, BUFGCE23,
];

pub const BUFGCTRL: [BelSlotId; 8] = [
    BUFGCTRL0, BUFGCTRL1, BUFGCTRL2, BUFGCTRL3, BUFGCTRL4, BUFGCTRL5, BUFGCTRL6, BUFGCTRL7,
];

pub const BUFGCE_DIV: [BelSlotId; 4] = [BUFGCE_DIV0, BUFGCE_DIV1, BUFGCE_DIV2, BUFGCE_DIV3];

pub const PLL: [BelSlotId; 2] = [PLL0, PLL1];
pub const PLLXP: [BelSlotId; 2] = [PLLXP0, PLLXP1];

pub const VBUS_SWITCH: [BelSlotId; 3] = [VBUS_SWITCH0, VBUS_SWITCH1, VBUS_SWITCH2];

pub const HBM_REF_CLK: [BelSlotId; 2] = [HBM_REF_CLK0, HBM_REF_CLK1];

pub const BITSLICE: [BelSlotId; 52] = [
    BITSLICE0, BITSLICE1, BITSLICE2, BITSLICE3, BITSLICE4, BITSLICE5, BITSLICE6, BITSLICE7,
    BITSLICE8, BITSLICE9, BITSLICE10, BITSLICE11, BITSLICE12, BITSLICE13, BITSLICE14, BITSLICE15,
    BITSLICE16, BITSLICE17, BITSLICE18, BITSLICE19, BITSLICE20, BITSLICE21, BITSLICE22, BITSLICE23,
    BITSLICE24, BITSLICE25, BITSLICE26, BITSLICE27, BITSLICE28, BITSLICE29, BITSLICE30, BITSLICE31,
    BITSLICE32, BITSLICE33, BITSLICE34, BITSLICE35, BITSLICE36, BITSLICE37, BITSLICE38, BITSLICE39,
    BITSLICE40, BITSLICE41, BITSLICE42, BITSLICE43, BITSLICE44, BITSLICE45, BITSLICE46, BITSLICE47,
    BITSLICE48, BITSLICE49, BITSLICE50, BITSLICE51,
];

pub const BITSLICE_T: [BelSlotId; 8] = [
    BITSLICE_T0,
    BITSLICE_T1,
    BITSLICE_T2,
    BITSLICE_T3,
    BITSLICE_T4,
    BITSLICE_T5,
    BITSLICE_T6,
    BITSLICE_T7,
];

pub const BITSLICE_CONTROL: [BelSlotId; 8] = [
    BITSLICE_CONTROL0,
    BITSLICE_CONTROL1,
    BITSLICE_CONTROL2,
    BITSLICE_CONTROL3,
    BITSLICE_CONTROL4,
    BITSLICE_CONTROL5,
    BITSLICE_CONTROL6,
    BITSLICE_CONTROL7,
];

pub const PLL_SELECT: [BelSlotId; 8] = [
    PLL_SELECT0,
    PLL_SELECT1,
    PLL_SELECT2,
    PLL_SELECT3,
    PLL_SELECT4,
    PLL_SELECT5,
    PLL_SELECT6,
    PLL_SELECT7,
];

pub const RIU_OR: [BelSlotId; 4] = [RIU_OR0, RIU_OR1, RIU_OR2, RIU_OR3];

pub const XIPHY_FEEDTHROUGH: [BelSlotId; 4] = [
    XIPHY_FEEDTHROUGH0,
    XIPHY_FEEDTHROUGH1,
    XIPHY_FEEDTHROUGH2,
    XIPHY_FEEDTHROUGH3,
];
