use prjcombine_re_fpga_hammer::{xlat_bit, xlat_bitvec, xlat_enum, xlat_enum_int};
use prjcombine_re_hammer::Session;
use prjcombine_re_xilinx_geom::ExpandedDevice;
use prjcombine_types::bits;
use prjcombine_virtex4::{bels, chip::ChipKind};

use crate::{
    backend::{IseBackend, MultiValue},
    collector::CollectorCtx,
    generic::fbuild::{FuzzBuilderBase, FuzzCtx},
};

pub fn add_fuzzers<'a>(session: &mut Session<'a, IseBackend<'a>>, backend: &'a IseBackend<'a>) {
    let ExpandedDevice::Virtex4(edev) = backend.edev else {
        unreachable!()
    };
    // TODO: globals: RSR[BT] RSR[BT]P EN_TSTEFUSEDLYCTRL
    let mut ctx = FuzzCtx::new(session, backend, "BRAM");
    {
        let mut bctx = ctx.bel(bels::BRAM_F);
        let mode = "RAMB36E1";
        bctx.build()
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .test_manual("PRESENT", "1")
            .mode(mode)
            .commit();

        for pin in [
            "CLKARDCLKL",
            "CLKARDCLKU",
            "CLKBWRCLKL",
            "CLKBWRCLKU",
            "REGCLKARDRCLKL",
            "REGCLKARDRCLKU",
            "REGCLKBL",
            "REGCLKBU",
            "ENARDENL",
            "ENARDENU",
            "ENBWRENL",
            "ENBWRENU",
            "RSTREGARSTREGL",
            "RSTREGARSTREGU",
            "RSTREGBL",
            "RSTREGBU",
            "RSTRAMARSTRAML",
            "RSTRAMARSTRAMU",
            "RSTRAMBL",
            "RSTRAMBU",
        ] {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "FULL")
                .test_inv(pin);
        }

        for attr in ["INIT_A", "INIT_B", "SRVAL_A", "SRVAL_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "FULL")
                .attr("READ_WIDTH_A", "36")
                .attr("READ_WIDTH_B", "36")
                .attr("RAM_MODE", "TDP")
                .test_multi_attr_hex(attr, 36);
        }
        for i in 0..0x80 {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "FULL")
                .attr("READ_WIDTH_A", "36")
                .attr("READ_WIDTH_B", "36")
                .attr("RAM_MODE", "TDP")
                .test_multi_attr_hex(format!("INIT_{i:02X}"), 256);
        }
        for i in 0..0x10 {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "FULL")
                .attr("READ_WIDTH_A", "36")
                .attr("READ_WIDTH_B", "36")
                .attr("RAM_MODE", "TDP")
                .test_multi_attr_hex(format!("INITP_{i:02X}"), 256);
        }
        bctx.mode(mode)
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .test_enum("SAVEDATA", &["FALSE", "TRUE"]);

        bctx.mode(mode)
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .test_enum("RAM_MODE", &["TDP", "SDP"]);

        for attr in ["WRITE_MODE_A", "WRITE_MODE_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "FULL")
                .attr("RAM_MODE", "TDP")
                .test_enum(attr, &["READ_FIRST", "WRITE_FIRST", "NO_CHANGE"]);
        }
        for attr in ["DOA_REG", "DOB_REG"] {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "FULL")
                .test_enum(attr, &["0", "1"]);
        }
        for attr in ["RAM_EXTENSION_A", "RAM_EXTENSION_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "FULL")
                .test_enum(attr, &["NONE", "LOWER", "UPPER"]);
        }
        for attr in ["READ_WIDTH_A", "WRITE_WIDTH_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "FULL")
                .attr("DOA_REG", "0")
                .attr("DOB_REG", "0")
                .attr("RAM_MODE", "SDP")
                .test_enum_suffix(attr, "SDP", &["0", "1", "2", "4", "9", "18", "36", "72"]);
        }
        for attr in [
            "READ_WIDTH_A",
            "READ_WIDTH_B",
            "WRITE_WIDTH_A",
            "WRITE_WIDTH_B",
        ] {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "FULL")
                .attr("DOA_REG", "0")
                .attr("DOB_REG", "0")
                .attr("RAM_MODE", "TDP")
                .test_enum(attr, &["0", "1", "2", "4", "9", "18", "36"]);
        }
        for attr in ["RSTREG_PRIORITY_A", "RSTREG_PRIORITY_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "FULL")
                .test_enum(attr, &["REGCE", "RSTREG"]);
        }

        bctx.mode(mode)
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .test_enum(
                "RDADDR_COLLISION_HWCONFIG",
                &["DELAYED_WRITE", "PERFORMANCE"],
            );
        if edev.kind == ChipKind::Virtex7 {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "FULL")
                .test_enum("EN_PWRGATE", &["NONE", "LEFT", "RIGHT", "BOTH"]);
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "FULL")
                .test_enum("EN_SDBITERR_INIT_V6", &["FALSE", "TRUE"]);
        }

        bctx.mode(mode)
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .attr("EN_ECC_WRITE", "FALSE")
            .test_enum("EN_ECC_READ", &["FALSE", "TRUE"]);
        bctx.mode(mode)
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .attr("EN_ECC_READ", "FALSE")
            .test_enum("EN_ECC_WRITE", &["FALSE", "TRUE"]);
        bctx.mode(mode)
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .attr("EN_ECC_READ", "TRUE")
            .test_enum_suffix("EN_ECC_WRITE", "READ", &["FALSE", "TRUE"]);

        for opt in ["BYPASS_RSR", "SWAP_CFGPORT"] {
            bctx.mode(mode)
                .global_mutex_here("BRAM_OPT")
                .test_manual(opt, "ENABLED")
                .global(opt, "ENABLED")
                .commit();
        }
        for opt in ["EN_TSTBRAMRST", "DIS_TSTFIFORST"] {
            bctx.mode(mode)
                .global_mutex_here("BRAM_OPT")
                .test_manual(opt, "1")
                .global(opt, "1")
                .commit();
        }
        for val in ["NO_WW", "WW0", "WW1"] {
            bctx.mode(mode)
                .global_mutex_here("BRAM_OPT")
                .test_manual("WEAK_WRITE", val)
                .global("WEAK_WRITE", val)
                .commit();
        }
        for val in ["0", "1", "10", "11", "100", "101", "110", "111"] {
            for opt in ["TRD_DLY_L", "TRD_DLY_U"] {
                bctx.mode(mode)
                    .global_mutex_here("BRAM_OPT")
                    .test_manual(opt, val)
                    .global(opt, val)
                    .commit();
            }
        }

        if edev.kind == ChipKind::Virtex6 {
            for val in ["0", "1", "10", "11", "100", "101", "110", "111"] {
                for opt in ["TWR_DLY_L", "TWR_DLY_U"] {
                    bctx.mode(mode)
                        .global_mutex_here("BRAM_OPT")
                        .test_manual(opt, val)
                        .global(opt, val)
                        .commit();
                }
            }
            for val in ["0", "1", "10", "11"] {
                for opt in ["EN_TSTREFBL", "EN_TSTRSRW"] {
                    bctx.mode(mode)
                        .global_mutex_here("BRAM_OPT")
                        .test_manual(opt, val)
                        .global(opt, val)
                        .commit();
                }
            }
            for val in ["0", "1"] {
                bctx.mode(mode)
                    .global_mutex_here("BRAM_OPT")
                    .test_manual("EN_TSTBLCLAMP", val)
                    .global("EN_TSTBLCLAMP", val)
                    .commit();
            }
        } else {
            for val in [
                "0", "1", "10", "11", "100", "101", "110", "111", "1000", "1001", "1010", "1011",
                "1100", "1101", "1110", "1111",
            ] {
                for opt in ["TWR_DLY_A_L", "TWR_DLY_A_U", "TWR_DLY_B_L", "TWR_DLY_B_U"] {
                    bctx.mode(mode)
                        .global_mutex_here("BRAM_OPT")
                        .test_manual(opt, val)
                        .global(opt, val)
                        .commit();
                }
            }

            for val in ["0", "1", "10", "11"] {
                for opt in [
                    "TSTREFBL_CTRL",
                    "TSTRSR_RWCTRL",
                    "EN_TSTRFMODE_DLY",
                    "EN_TSTPULSEPU_DLY",
                    "EN_TSTEXTCLK",
                    "EN_TSTRSTC_PW",
                    "EN_TSTBLPC_DLY",
                    "EN_TST_REGOUT_DLY_SEL",
                    "TST_RNG_OSC",
                ] {
                    bctx.mode(mode)
                        .global_mutex_here("BRAM_OPT")
                        .test_manual(opt, val)
                        .global(opt, val)
                        .commit();
                }
            }
            for val in ["0", "1"] {
                for opt in ["DIS_TSTBLCLAMP", "TST_SSRLAT_WF"] {
                    bctx.mode(mode)
                        .global_mutex_here("BRAM_OPT")
                        .test_manual(opt, val)
                        .global(opt, val)
                        .commit();
                }
            }

            for val in ["NO", "YES"] {
                bctx.mode(mode)
                    .global_mutex_here("BRAM_OPT")
                    .test_manual("EN_TSTBLCLAMP_RD", val)
                    .global("EN_TSTBLCLAMP_RD", val)
                    .commit();
            }

            for val in ["DISABLE", "ENABLE"] {
                for opt in [
                    "EN_TSTBLCLMP_WW",
                    "EN_TSTSNM",
                    "LAT_RST_DLYEN",
                    "STUCK_DET_EN",
                    "TST_PULSEPU_SFT",
                    "TST_BIST_CTL",
                ] {
                    bctx.mode(mode)
                        .global_mutex_here("BRAM_OPT")
                        .test_manual(opt, val)
                        .global(opt, val)
                        .commit();
                }
            }
        }

        let mode = "FIFO36E1";

        bctx.build()
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .force_bel_name("FIFO_F")
            .test_manual("PRESENT", "1")
            .mode(mode)
            .commit();

        for pin in ["RDCLK", "WRCLK", "RDRCLK", "RDEN", "WREN", "RSTREG"] {
            for ul in ['U', 'L'] {
                bctx.mode(mode)
                    .force_bel_name("FIFO_F")
                    .global_mutex("BRAM_OPT", "NONE")
                    .tile_mutex("MODE", "FULL")
                    .test_inv(format!("{pin}{ul}"));
            }
        }
        bctx.mode(mode)
            .force_bel_name("FIFO_F")
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .test_inv("RST");

        for attr in ["INIT", "SRVAL"] {
            bctx.mode(mode)
                .force_bel_name("FIFO_F")
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "FULL")
                .attr("DATA_WIDTH", "72")
                .attr("FIFO_MODE", "FIFO36_72")
                .test_multi_attr_hex(attr, 72);
        }

        bctx.mode(mode)
            .force_bel_name("FIFO_F")
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .test_enum("FIFO_MODE", &["FIFO36", "FIFO36_72"]);
        bctx.mode(mode)
            .force_bel_name("FIFO_F")
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .test_enum("EN_SYN", &["FALSE", "TRUE"]);
        bctx.mode(mode)
            .force_bel_name("FIFO_F")
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .test_enum("FIRST_WORD_FALL_THROUGH", &["FALSE", "TRUE"]);
        bctx.mode(mode)
            .force_bel_name("FIFO_F")
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .attr("EN_SYN", "TRUE")
            .test_multi_attr_hex("ALMOST_FULL_OFFSET", 13);
        bctx.mode(mode)
            .force_bel_name("FIFO_F")
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .attr("EN_SYN", "TRUE")
            .test_multi_attr_hex("ALMOST_EMPTY_OFFSET", 13);

        bctx.mode(mode)
            .force_bel_name("FIFO_F")
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .test_enum("DO_REG", &["0", "1"]);

        bctx.mode(mode)
            .force_bel_name("FIFO_F")
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .attr("DO_REG", "0")
            .attr("FIFO_MODE", "FIFO36")
            .test_enum("DATA_WIDTH", &["4", "9", "18", "36", "72"]);
        bctx.mode(mode)
            .force_bel_name("FIFO_F")
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .attr("DO_REG", "0")
            .attr("FIFO_MODE", "FIFO36_72")
            .test_enum_suffix("DATA_WIDTH", "SDP", &["4", "9", "18", "36", "72"]);

        if edev.kind == ChipKind::Virtex6 {
            bctx.mode(mode)
                .force_bel_name("FIFO_F")
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "FULL")
                .test_enum("RSTREG_PRIORITY", &["REGCE", "RSTREG"]);
        } else {
            bctx.mode(mode)
                .force_bel_name("FIFO_F")
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "FULL")
                .test_enum("EN_PWRGATE", &["NONE", "LEFT", "RIGHT", "BOTH"]);
            bctx.mode(mode)
                .force_bel_name("FIFO_F")
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "FULL")
                .test_enum("EN_SDBITERR_INIT_V6", &["FALSE", "TRUE"]);
            bctx.mode(mode)
                .force_bel_name("FIFO_F")
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "FULL")
                .test_enum(
                    "RDADDR_COLLISION_HWCONFIG",
                    &["DELAYED_WRITE", "PERFORMANCE"],
                );
        }

        bctx.mode(mode)
            .force_bel_name("FIFO_F")
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .attr("EN_ECC_WRITE", "FALSE")
            .test_enum("EN_ECC_READ", &["FALSE", "TRUE"]);
        bctx.mode(mode)
            .force_bel_name("FIFO_F")
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .attr("EN_ECC_READ", "FALSE")
            .test_enum("EN_ECC_WRITE", &["FALSE", "TRUE"]);
        bctx.mode(mode)
            .force_bel_name("FIFO_F")
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "FULL")
            .attr("EN_ECC_READ", "TRUE")
            .test_enum_suffix("EN_ECC_WRITE", "READ", &["FALSE", "TRUE"]);

        for opt in ["TEST_FIFO_FLAG", "TEST_FIFO_OFFSET", "TEST_FIFO_CNT"] {
            bctx.mode(mode)
                .force_bel_name("FIFO_F")
                .global_mutex_here("BRAM_OPT")
                .test_manual(opt, "ENABLED")
                .global(opt, "ENABLED")
                .commit();
        }
    }
    for i in 0..2 {
        let mut bctx = ctx.bel(bels::BRAM_H[i]);
        let mode = "RAMB18E1";
        bctx.build()
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "HALF")
            .test_manual("PRESENT", "1")
            .mode(mode)
            .commit();

        for pin in [
            "CLKARDCLK",
            "CLKBWRCLK",
            "REGCLKARDRCLK",
            "REGCLKB",
            "ENARDEN",
            "ENBWREN",
            "RSTREGARSTREG",
            "RSTREGB",
            "RSTRAMARSTRAM",
            "RSTRAMB",
        ] {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .test_inv(pin);
        }

        for attr in ["INIT_A", "INIT_B", "SRVAL_A", "SRVAL_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .attr("READ_WIDTH_A", "18")
                .attr("READ_WIDTH_B", "18")
                .attr("RAM_MODE", "TDP")
                .test_multi_attr_hex(attr, 18);
        }
        for i in 0..0x40 {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .attr("READ_WIDTH_A", "18")
                .attr("READ_WIDTH_B", "18")
                .attr("RAM_MODE", "TDP")
                .test_multi_attr_hex(format!("INIT_{i:02X}"), 256);
        }
        for i in 0..8 {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .attr("READ_WIDTH_A", "18")
                .attr("READ_WIDTH_B", "18")
                .attr("RAM_MODE", "TDP")
                .test_multi_attr_hex(format!("INITP_{i:02X}"), 256);
        }

        bctx.mode(mode)
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "HALF")
            .test_enum("RAM_MODE", &["TDP", "SDP"]);

        for attr in ["WRITE_MODE_A", "WRITE_MODE_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .attr("RAM_MODE", "TDP")
                .test_enum(attr, &["READ_FIRST", "WRITE_FIRST", "NO_CHANGE"]);
        }
        for attr in ["DOA_REG", "DOB_REG"] {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .test_enum(attr, &["0", "1"]);
        }
        for attr in ["READ_WIDTH_A", "WRITE_WIDTH_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .attr("DOA_REG", "0")
                .attr("DOB_REG", "0")
                .pin("WEBWE0")
                .pin("WEBWE1")
                .pin("WEBWE2")
                .pin("WEBWE3")
                .pin("WEBWE4")
                .pin("WEBWE5")
                .pin("WEBWE6")
                .pin("WEBWE7")
                .attr("RAM_MODE", "SDP")
                .test_enum_suffix(attr, "SDP", &["0", "1", "2", "4", "9", "18", "36"]);
        }
        for attr in [
            "READ_WIDTH_A",
            "READ_WIDTH_B",
            "WRITE_WIDTH_A",
            "WRITE_WIDTH_B",
        ] {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .attr("DOA_REG", "0")
                .attr("DOB_REG", "0")
                .pin("WEBWE0")
                .pin("WEBWE1")
                .pin("WEBWE2")
                .pin("WEBWE3")
                .pin("WEBWE4")
                .pin("WEBWE5")
                .pin("WEBWE6")
                .pin("WEBWE7")
                .pin("WEA0")
                .pin("WEA1")
                .pin("WEA2")
                .pin("WEA3")
                .attr("RAM_MODE", "TDP")
                .test_enum(attr, &["0", "1", "2", "4", "9", "18"]);
        }

        for attr in ["RSTREG_PRIORITY_A", "RSTREG_PRIORITY_B"] {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .test_enum(attr, &["REGCE", "RSTREG"]);
        }

        bctx.mode(mode)
            .global_mutex("BRAM_OPT", "NONE")
            .tile_mutex("MODE", "HALF")
            .test_enum(
                "RDADDR_COLLISION_HWCONFIG",
                &["DELAYED_WRITE", "PERFORMANCE"],
            );

        if edev.kind == ChipKind::Virtex7 {
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .test_enum("EN_PWRGATE", &["NONE", "LEFT", "RIGHT", "BOTH"]);
            bctx.mode(mode)
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .tile_mutex("SDBITERR", format!("HALF_{i}"))
                .test_enum("EN_SDBITERR_INIT_V6", &["FALSE", "TRUE"]);
        }

        if i == 0 {
            let mode = "FIFO18E1";

            bctx.build()
                .force_bel_name("FIFO_H0")
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .test_manual("PRESENT", "1")
                .mode(mode)
                .commit();

            for pin in ["RDCLK", "WRCLK", "RDRCLK", "RDEN", "WREN", "RST", "RSTREG"] {
                bctx.mode(mode)
                    .force_bel_name("FIFO_H0")
                    .global_mutex("BRAM_OPT", "NONE")
                    .tile_mutex("MODE", "HALF")
                    .test_inv(pin);
            }

            for attr in ["INIT", "SRVAL"] {
                bctx.mode(mode)
                    .force_bel_name("FIFO_H0")
                    .global_mutex("BRAM_OPT", "NONE")
                    .tile_mutex("MODE", "HALF")
                    .attr("DATA_WIDTH", "36")
                    .attr("FIFO_MODE", "FIFO18_36")
                    .test_multi_attr_hex(attr, 36);
            }

            bctx.mode(mode)
                .force_bel_name("FIFO_H0")
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .test_enum("FIFO_MODE", &["FIFO18", "FIFO18_36"]);
            bctx.mode(mode)
                .force_bel_name("FIFO_H0")
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .test_enum("EN_SYN", &["FALSE", "TRUE"]);
            bctx.mode(mode)
                .force_bel_name("FIFO_H0")
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .test_enum("FIRST_WORD_FALL_THROUGH", &["FALSE", "TRUE"]);

            bctx.mode(mode)
                .force_bel_name("FIFO_H0")
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .attr("EN_SYN", "TRUE")
                .test_multi_attr_hex("ALMOST_FULL_OFFSET", 13);
            bctx.mode(mode)
                .force_bel_name("FIFO_H0")
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .attr("EN_SYN", "TRUE")
                .test_multi_attr_hex("ALMOST_EMPTY_OFFSET", 13);

            bctx.mode(mode)
                .force_bel_name("FIFO_H0")
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .test_enum("DO_REG", &["0", "1"]);

            bctx.mode(mode)
                .force_bel_name("FIFO_H0")
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .attr("DO_REG", "0")
                .attr("FIFO_MODE", "FIFO18")
                .test_enum("DATA_WIDTH", &["4", "9", "18", "36"]);
            bctx.mode(mode)
                .force_bel_name("FIFO_H0")
                .global_mutex("BRAM_OPT", "NONE")
                .tile_mutex("MODE", "HALF")
                .attr("DO_REG", "0")
                .attr("FIFO_MODE", "FIFO18_36")
                .test_enum_suffix("DATA_WIDTH", "SDP", &["4", "9", "18", "36"]);

            if edev.kind == ChipKind::Virtex6 {
                bctx.mode(mode)
                    .force_bel_name("FIFO_H0")
                    .global_mutex("BRAM_OPT", "NONE")
                    .tile_mutex("MODE", "HALF")
                    .test_enum("RSTREG_PRIORITY", &["REGCE", "RSTREG"]);
            } else {
                bctx.mode(mode)
                    .force_bel_name("FIFO_H0")
                    .global_mutex("BRAM_OPT", "NONE")
                    .tile_mutex("MODE", "HALF")
                    .test_enum("EN_PWRGATE", &["NONE", "LEFT", "RIGHT", "BOTH"]);
                bctx.mode(mode)
                    .force_bel_name("FIFO_H0")
                    .global_mutex("BRAM_OPT", "NONE")
                    .tile_mutex("MODE", "HALF")
                    .tile_mutex("SDBITERR", format!("HALF_{i}"))
                    .test_enum("EN_SDBITERR_INIT_V6", &["FALSE", "TRUE"]);
                bctx.mode(mode)
                    .force_bel_name("FIFO_H0")
                    .global_mutex("BRAM_OPT", "NONE")
                    .tile_mutex("MODE", "HALF")
                    .test_enum(
                        "RDADDR_COLLISION_HWCONFIG",
                        &["DELAYED_WRITE", "PERFORMANCE"],
                    );
            }
        }
    }
    if edev.kind == ChipKind::Virtex7 {
        let mut bctx = ctx.bel(bels::BRAM_ADDR);
        for (ab, abrw) in [('A', "ARD"), ('B', "BWR")] {
            for i in 0..15 {
                for (ul, lu) in [('U', 'L'), ('L', 'U')] {
                    for (val, spin) in [
                        ("CASCINBOT", format!("CASCINBOT_ADDR{abrw}ADDRU{i}")),
                        ("CASCINTOP", format!("CASCINTOP_ADDR{abrw}ADDRU{i}")),
                    ] {
                        bctx.build()
                            .global_mutex("BRAM_ADDR_CASCADE", "USE")
                            .mutex(format!("MUX.ADDR{ab}{ul}{i}"), val)
                            .mutex(format!("MUX.ADDR{ab}{lu}{i}"), val)
                            .pip(format!("ADDR{abrw}ADDR{lu}{i}"), &spin)
                            .test_manual(format!("MUX.ADDR{ab}{ul}{i}"), val)
                            .pip(format!("ADDR{abrw}ADDR{ul}{i}"), spin)
                            .commit();
                    }
                    bctx.build()
                        .mutex(format!("MUX.ADDR{ab}{ul}{i}"), "INT")
                        .test_manual(format!("MUX.ADDR{ab}{ul}{i}"), "INT")
                        .pip(
                            format!("ADDR{abrw}ADDR{ul}{i}"),
                            format!("IMUX_ADDR{abrw}ADDR{ul}{i}"),
                        )
                        .commit();
                }
                bctx.build()
                    .global_mutex("BRAM_ADDR_CASCADE", "TEST")
                    .tile_mutex(format!("CASCADE_OUT{ab}"), format!("{i}"))
                    .test_manual(format!("CASCADE_OUT.ADDR{ab}{i}"), "1")
                    .pip(
                        format!("CASCOUT_ADDR{abrw}ADDRU{i}"),
                        format!("ADDR{abrw}ADDRU{i}"),
                    )
                    .commit();
            }
        }
    }
    let mut ctx = FuzzCtx::new_null(session, backend);
    ctx.build()
        .extra_tiles_by_bel(bels::BRAM_F, "BRAM")
        .test_manual("BRAM", "TEST_ATTRIBUTES", "")
        .multi_global(
            "TEST_ATTRIBUTES",
            MultiValue::Hex(0),
            if edev.kind == ChipKind::Virtex6 {
                20
            } else {
                19
            },
        );
}

pub fn collect_fuzzers(ctx: &mut CollectorCtx) {
    let ExpandedDevice::Virtex4(edev) = ctx.edev else {
        unreachable!()
    };
    let tile = "BRAM";
    let mut present_ramb36 = ctx.state.get_diff(tile, "BRAM_F", "PRESENT", "1");
    let mut present_fifo36 = ctx.state.get_diff(tile, "FIFO_F", "PRESENT", "1");
    let mut present_ramb18_l = ctx.state.get_diff(tile, "BRAM_H0", "PRESENT", "1");
    let mut present_ramb18_u = ctx.state.get_diff(tile, "BRAM_H1", "PRESENT", "1");
    let mut present_fifo18 = ctx.state.get_diff(tile, "FIFO_H0", "PRESENT", "1");

    for pin in [
        "CLKARDCLK",
        "CLKBWRCLK",
        "REGCLKARDRCLK",
        "REGCLKB",
        "ENARDEN",
        "ENBWREN",
        "RSTREGARSTREG",
        "RSTREGB",
        "RSTRAMARSTRAM",
        "RSTRAMB",
    ] {
        for (bel, ul) in [("BRAM_H0", 'L'), ("BRAM_H1", 'U')] {
            let item = ctx.extract_inv(tile, "BRAM_F", &format!("{pin}{ul}"));
            ctx.tiledb
                .insert(tile, "BRAM", format!("INV.{pin}{ul}"), item);
            let item = ctx.extract_inv(tile, bel, pin);
            ctx.tiledb
                .insert(tile, "BRAM", format!("INV.{pin}{ul}"), item);
        }
    }

    for (hwpin, pin) in [
        ("CLKARDCLK", "RDCLK"),
        ("CLKBWRCLK", "WRCLK"),
        ("REGCLKARDRCLK", "RDRCLK"),
        ("ENARDEN", "RDEN"),
        ("ENBWREN", "WREN"),
        ("RSTREGARSTREG", "RSTREG"),
    ] {
        let item = ctx.extract_inv(tile, "FIFO_H0", pin);
        ctx.tiledb
            .insert(tile, "BRAM", format!("INV.{hwpin}L"), item);
        for ul in ['U', 'L'] {
            let item = ctx.extract_inv(tile, "FIFO_F", &format!("{pin}{ul}"));
            ctx.tiledb
                .insert(tile, "BRAM", format!("INV.{hwpin}{ul}"), item);
        }
    }
    for bel in ["FIFO_H0", "FIFO_F"] {
        let item = ctx.extract_inv(tile, bel, "RST");
        ctx.tiledb.insert(tile, "BRAM", "INV.RSTRAMARSTRAML", item);
    }

    for (attr, attr_a, attr_b) in [
        ("INIT", "INIT_A", "INIT_B"),
        ("SRVAL", "SRVAL_A", "SRVAL_B"),
    ] {
        for (bel_bram, bel_fifo) in [("BRAM_F", "FIFO_F"), ("BRAM_H0", "FIFO_H0")] {
            let diffs = ctx.state.get_diffs(tile, bel_fifo, attr, "");
            let diffs_a = ctx.state.peek_diffs(tile, bel_bram, attr_a, "");
            let diffs_b = ctx.state.peek_diffs(tile, bel_bram, attr_b, "");
            let mid = diffs_a.len();
            assert_eq!(&diffs[..mid], diffs_a);
            assert_eq!(&diffs[mid..], diffs_b);
        }
    }
    for attr in ["INIT_A", "INIT_B", "SRVAL_A", "SRVAL_B"] {
        let diffs = ctx.state.get_diffs(tile, "BRAM_F", attr, "");
        let diffs_l = ctx.state.get_diffs(tile, "BRAM_H0", attr, "");
        let diffs_u = ctx.state.get_diffs(tile, "BRAM_H1", attr, "");
        for i in 0..18 {
            assert_eq!(diffs_l[i], diffs[2 * i]);
            assert_eq!(diffs_u[i], diffs[2 * i + 1]);
        }
        let item_l = xlat_bitvec(diffs_l);
        let item_u = xlat_bitvec(diffs_u);
        present_ramb36.apply_bitvec_diff(&item_l, &bits![0; 18], &bits![1; 18]);
        present_ramb36.apply_bitvec_diff(&item_u, &bits![0; 18], &bits![1; 18]);
        present_fifo36.apply_bitvec_diff(&item_l, &bits![0; 18], &bits![1; 18]);
        present_fifo36.apply_bitvec_diff(&item_u, &bits![0; 18], &bits![1; 18]);
        present_ramb18_l.apply_bitvec_diff(&item_l, &bits![0; 18], &bits![1; 18]);
        present_ramb18_u.apply_bitvec_diff(&item_u, &bits![0; 18], &bits![1; 18]);
        present_fifo18.apply_bitvec_diff(&item_l, &bits![0; 18], &bits![1; 18]);
        ctx.tiledb.insert(tile, "BRAM", format!("{attr}_L"), item_l);
        ctx.tiledb.insert(tile, "BRAM", format!("{attr}_U"), item_u);
    }

    for (bel, ul) in [("BRAM_H0", 'L'), ("BRAM_H1", 'U')] {
        let mut data = vec![];
        let mut datap = vec![];
        for i in 0..0x40 {
            data.extend(ctx.state.get_diffs(tile, bel, format!("INIT_{i:02X}"), ""));
        }
        for i in 0..8 {
            datap.extend(ctx.state.get_diffs(tile, bel, format!("INITP_{i:02X}"), ""));
        }
        ctx.tiledb
            .insert(tile, "BRAM", format!("DATA_{ul}"), xlat_bitvec(data));
        ctx.tiledb
            .insert(tile, "BRAM", format!("DATAP_{ul}"), xlat_bitvec(datap));
    }

    let mut data = vec![];
    let mut datap = vec![];
    for i in 0..0x80 {
        data.extend(
            ctx.state
                .get_diffs(tile, "BRAM_F", format!("INIT_{i:02X}"), ""),
        );
    }
    for i in 0..0x10 {
        datap.extend(
            ctx.state
                .get_diffs(tile, "BRAM_F", format!("INITP_{i:02X}"), ""),
        );
    }
    let mut data_l = vec![];
    let mut data_u = vec![];
    for (i, diff) in data.into_iter().enumerate() {
        if i % 2 == 0 {
            data_l.push(diff);
        } else {
            data_u.push(diff);
        }
    }
    let mut datap_l = vec![];
    let mut datap_u = vec![];
    for (i, diff) in datap.into_iter().enumerate() {
        if i % 2 == 0 {
            datap_l.push(diff);
        } else {
            datap_u.push(diff);
        }
    }
    ctx.tiledb
        .insert(tile, "BRAM", "DATA_L", xlat_bitvec(data_l));
    ctx.tiledb
        .insert(tile, "BRAM", "DATA_U", xlat_bitvec(data_u));
    ctx.tiledb
        .insert(tile, "BRAM", "DATAP_L", xlat_bitvec(datap_l));
    ctx.tiledb
        .insert(tile, "BRAM", "DATAP_U", xlat_bitvec(datap_u));

    let item = ctx.extract_enum_bool_wide(tile, "BRAM_F", "SAVEDATA", "FALSE", "TRUE");
    ctx.tiledb.insert(tile, "BRAM", "SAVEDATA", item);

    for bel in ["BRAM_F", "BRAM_H0", "BRAM_H1"] {
        ctx.state
            .get_diff(tile, bel, "RAM_MODE", "TDP")
            .assert_empty();
        ctx.state
            .get_diff(tile, bel, "RAM_MODE", "SDP")
            .assert_empty();
    }
    ctx.state
        .get_diff(tile, "FIFO_F", "FIFO_MODE", "FIFO36")
        .assert_empty();
    ctx.state
        .get_diff(tile, "FIFO_F", "FIFO_MODE", "FIFO36_72")
        .assert_empty();
    ctx.state
        .get_diff(tile, "FIFO_H0", "FIFO_MODE", "FIFO18")
        .assert_empty();
    ctx.state
        .get_diff(tile, "FIFO_H0", "FIFO_MODE", "FIFO18_36")
        .assert_empty();

    for bel in ["FIFO_F", "FIFO_H0"] {
        let item = ctx.extract_enum_bool(tile, bel, "FIRST_WORD_FALL_THROUGH", "FALSE", "TRUE");
        ctx.tiledb
            .insert(tile, "BRAM", "FIRST_WORD_FALL_THROUGH", item);
        let item = ctx.extract_enum_bool(tile, bel, "EN_SYN", "FALSE", "TRUE");
        ctx.tiledb.insert(tile, "BRAM", "EN_SYN", item);
        let item = ctx.extract_bitvec(tile, bel, "ALMOST_FULL_OFFSET", "");
        ctx.tiledb.insert(tile, "BRAM", "ALMOST_FULL_OFFSET", item);
        let item = ctx.extract_bitvec(tile, bel, "ALMOST_EMPTY_OFFSET", "");
        ctx.tiledb.insert(tile, "BRAM", "ALMOST_EMPTY_OFFSET", item);
    }
    for attr in ["ALMOST_FULL_OFFSET", "ALMOST_EMPTY_OFFSET"] {
        let item = ctx.tiledb.item(tile, "BRAM", attr);
        present_ramb36.apply_bitvec_diff(item, &bits![0; 13], &bits![1; 13]);
        present_fifo36.apply_bitvec_diff(item, &bits![0; 13], &bits![1; 13]);
        present_ramb18_l.apply_bitvec_diff(item, &bits![0; 13], &bits![1; 13]);
        present_fifo18.apply_bitvec_diff(item, &bits![0; 13], &bits![1; 13]);
    }

    for attr in ["WRITE_MODE_A", "WRITE_MODE_B"] {
        for val in ["READ_FIRST", "WRITE_FIRST", "NO_CHANGE"] {
            let diff_f = ctx.state.get_diff(tile, "BRAM_F", attr, val);
            let diff_h0 = ctx.state.peek_diff(tile, "BRAM_H0", attr, val);
            let diff_h1 = ctx.state.peek_diff(tile, "BRAM_H1", attr, val);
            assert_eq!(diff_f, diff_h0.combine(diff_h1));
        }
        let item = ctx.extract_enum(
            tile,
            "BRAM_H0",
            attr,
            &["READ_FIRST", "WRITE_FIRST", "NO_CHANGE"],
        );
        present_fifo36.apply_enum_diff(&item, "NO_CHANGE", "WRITE_FIRST");
        present_fifo18.apply_enum_diff(&item, "NO_CHANGE", "WRITE_FIRST");
        ctx.tiledb.insert(tile, "BRAM", format!("{attr}_L"), item);
        let item = ctx.extract_enum(
            tile,
            "BRAM_H1",
            attr,
            &["READ_FIRST", "WRITE_FIRST", "NO_CHANGE"],
        );
        present_fifo36.apply_enum_diff(&item, "NO_CHANGE", "WRITE_FIRST");
        ctx.tiledb.insert(tile, "BRAM", format!("{attr}_U"), item);
    }
    for (bel_bram, bel_fifo) in [("BRAM_F", "FIFO_F"), ("BRAM_H0", "FIFO_H0")] {
        for val in ["0", "1"] {
            let diff_fifo = ctx.state.get_diff(tile, bel_fifo, "DO_REG", val);
            let diff_a = ctx.state.peek_diff(tile, bel_bram, "DOA_REG", val);
            let diff_b = ctx.state.peek_diff(tile, bel_bram, "DOB_REG", val);
            assert_eq!(diff_fifo, diff_a.combine(diff_b));
        }
    }
    for attr in ["DOA_REG", "DOB_REG"] {
        for val in ["0", "1"] {
            let diff_f = ctx.state.get_diff(tile, "BRAM_F", attr, val);
            let diff_h0 = ctx.state.peek_diff(tile, "BRAM_H0", attr, val);
            let diff_h1 = ctx.state.peek_diff(tile, "BRAM_H1", attr, val);
            assert_eq!(diff_f, diff_h0.combine(diff_h1));
        }
        let item = ctx.extract_enum(tile, "BRAM_H0", attr, &["0", "1"]);
        ctx.tiledb.insert(tile, "BRAM", format!("{attr}_L"), item);
        let item = ctx.extract_enum(tile, "BRAM_H1", attr, &["0", "1"]);
        ctx.tiledb.insert(tile, "BRAM", format!("{attr}_U"), item);
    }
    for attr in ["RAM_EXTENSION_A", "RAM_EXTENSION_B"] {
        let item = xlat_enum(vec![
            (
                "NONE_UPPER",
                ctx.state.get_diff(tile, "BRAM_F", attr, "NONE"),
            ),
            (
                "NONE_UPPER",
                ctx.state.get_diff(tile, "BRAM_F", attr, "UPPER"),
            ),
            ("LOWER", ctx.state.get_diff(tile, "BRAM_F", attr, "LOWER")),
        ]);
        ctx.tiledb.insert(tile, "BRAM", attr, item)
    }

    for (rw, ab, ba) in [("READ", 'A', 'B'), ("WRITE", 'B', 'A')] {
        for (ul, bel) in [('L', "BRAM_H0"), ('U', "BRAM_H1")] {
            for val in ["0", "1", "2", "4", "9", "18"] {
                let diff = ctx
                    .state
                    .get_diff(tile, bel, format!("{rw}_WIDTH_{ab}.SDP"), val);
                assert_eq!(
                    &diff,
                    ctx.state
                        .peek_diff(tile, bel, format!("{rw}_WIDTH_{ab}"), val)
                );
            }
            let mut diff = ctx
                .state
                .get_diff(tile, bel, format!("{rw}_WIDTH_{ab}.SDP"), "36");
            if ul == 'U' || rw == "WRITE" {
                diff = diff.combine(&!ctx.state.peek_diff(
                    tile,
                    bel,
                    format!("{rw}_WIDTH_{ab}"),
                    "18",
                ));
            }
            diff = diff.combine(
                &!ctx
                    .state
                    .peek_diff(tile, bel, format!("{rw}_WIDTH_{ba}"), "18"),
            );
            ctx.tiledb
                .insert(tile, "BRAM", format!("{rw}_SDP_{ul}"), xlat_bit(diff));
        }
        for val in ["0", "1", "2", "4", "9", "18", "36"] {
            let diff = ctx
                .state
                .get_diff(tile, "BRAM_F", format!("{rw}_WIDTH_{ab}.SDP"), val);
            assert_eq!(
                &diff,
                ctx.state
                    .peek_diff(tile, "BRAM_F", format!("{rw}_WIDTH_{ab}"), val)
            );
        }
        let mut diff = ctx
            .state
            .get_diff(tile, "BRAM_F", format!("{rw}_WIDTH_{ab}.SDP"), "72");
        diff = diff.combine(&!ctx.state.peek_diff(
            tile,
            "BRAM_F",
            format!("{rw}_WIDTH_{ab}"),
            "36",
        ));
        diff = diff.combine(&!ctx.state.peek_diff(
            tile,
            "BRAM_F",
            format!("{rw}_WIDTH_{ba}"),
            "36",
        ));
        diff.apply_bit_diff(
            ctx.tiledb.item(tile, "BRAM", &format!("{rw}_SDP_L")),
            true,
            false,
        );
        diff.apply_bit_diff(
            ctx.tiledb.item(tile, "BRAM", &format!("{rw}_SDP_U")),
            true,
            false,
        );
        diff.assert_empty();
    }
    for rw in ["READ", "WRITE"] {
        for ab in ['A', 'B'] {
            let diff_mux = ctx
                .state
                .get_diff(tile, "BRAM_F", format!("{rw}_WIDTH_{ab}"), "1");
            for (val_h, val_f) in [
                ("0", "0"),
                ("1", "2"),
                ("2", "4"),
                ("4", "9"),
                ("9", "18"),
                ("18", "36"),
            ] {
                let mut diff =
                    ctx.state
                        .get_diff(tile, "BRAM_F", format!("{rw}_WIDTH_{ab}"), val_f);
                diff = diff.combine(&!ctx.state.peek_diff(
                    tile,
                    "BRAM_H0",
                    format!("{rw}_WIDTH_{ab}"),
                    val_h,
                ));
                diff = diff.combine(&!ctx.state.peek_diff(
                    tile,
                    "BRAM_H1",
                    format!("{rw}_WIDTH_{ab}"),
                    val_h,
                ));
                if val_f == "9" {
                    diff = diff.combine(&!&diff_mux);
                }
                diff.assert_empty();
            }
            ctx.tiledb.insert(
                tile,
                "BRAM",
                format!("{rw}_MUX_UL_{ab}"),
                xlat_bit(diff_mux),
            );
        }
    }
    for rw in ["READ", "WRITE"] {
        for ab in ['A', 'B'] {
            for (ul, bel) in [('L', "BRAM_H0"), ('U', "BRAM_H1")] {
                let attr = format!("{rw}_WIDTH_{ab}");
                let diff = ctx.state.get_diff(tile, bel, &attr, "0");
                assert_eq!(&diff, ctx.state.peek_diff(tile, bel, &attr, "1"));
                let item = ctx.extract_enum(tile, bel, &attr, &["1", "2", "4", "9", "18"]);
                ctx.tiledb
                    .insert(tile, "BRAM", format!("{rw}_WIDTH_{ab}_{ul}"), item);
            }
        }
    }

    let mut diffs = vec![];
    for val in ["4", "9", "18", "36"] {
        let mut diff = ctx.state.get_diff(tile, "FIFO_H0", "DATA_WIDTH", val);
        let mut diff_sdp = ctx.state.get_diff(tile, "FIFO_H0", "DATA_WIDTH.SDP", val);
        let xval = if val == "36" { "18" } else { val };
        if xval == "18" {
            diff_sdp.apply_enum_diff(ctx.tiledb.item(tile, "BRAM", "READ_WIDTH_B_L"), xval, "1");
            diff_sdp.apply_enum_diff(ctx.tiledb.item(tile, "BRAM", "WRITE_WIDTH_A_L"), xval, "1");
        }
        if val == "36" {
            diff_sdp.apply_bit_diff(ctx.tiledb.item(tile, "BRAM", "READ_SDP_L"), true, false);
            diff_sdp.apply_bit_diff(ctx.tiledb.item(tile, "BRAM", "WRITE_SDP_L"), true, false);
        }
        assert_eq!(diff, diff_sdp);
        diff.apply_enum_diff(ctx.tiledb.item(tile, "BRAM", "READ_WIDTH_A_L"), xval, "1");
        diff.apply_enum_diff(ctx.tiledb.item(tile, "BRAM", "WRITE_WIDTH_B_L"), xval, "1");
        diffs.push((val, diff));
    }
    for (val, val2) in [
        ("2", "4"),
        ("4", "9"),
        ("9", "18"),
        ("18", "36"),
        ("36", "72"),
    ] {
        let mut diff = ctx.state.get_diff(tile, "FIFO_F", "DATA_WIDTH", val2);
        let mut diff_sdp = ctx.state.get_diff(tile, "FIFO_F", "DATA_WIDTH.SDP", val2);
        let xval = if val == "36" { "18" } else { val };
        if val == "36" {
            diff_sdp.apply_enum_diff(ctx.tiledb.item(tile, "BRAM", "READ_WIDTH_B_L"), xval, "1");
            diff_sdp.apply_enum_diff(ctx.tiledb.item(tile, "BRAM", "READ_WIDTH_B_U"), xval, "1");
            diff_sdp.apply_enum_diff(ctx.tiledb.item(tile, "BRAM", "WRITE_WIDTH_A_L"), xval, "1");
            diff_sdp.apply_enum_diff(ctx.tiledb.item(tile, "BRAM", "WRITE_WIDTH_A_U"), xval, "1");
            diff.apply_bit_diff(ctx.tiledb.item(tile, "BRAM", "READ_SDP_L"), true, false);
            diff.apply_bit_diff(ctx.tiledb.item(tile, "BRAM", "READ_SDP_U"), true, false);
            diff.apply_bit_diff(ctx.tiledb.item(tile, "BRAM", "WRITE_SDP_L"), true, false);
            diff.apply_bit_diff(ctx.tiledb.item(tile, "BRAM", "WRITE_SDP_U"), true, false);
            diff_sdp.apply_bit_diff(ctx.tiledb.item(tile, "BRAM", "READ_SDP_L"), true, false);
            diff_sdp.apply_bit_diff(ctx.tiledb.item(tile, "BRAM", "READ_SDP_U"), true, false);
            diff_sdp.apply_bit_diff(ctx.tiledb.item(tile, "BRAM", "WRITE_SDP_L"), true, false);
            diff_sdp.apply_bit_diff(ctx.tiledb.item(tile, "BRAM", "WRITE_SDP_U"), true, false);
        }
        if val2 == "9" {
            diff.apply_bit_diff(ctx.tiledb.item(tile, "BRAM", "READ_MUX_UL_A"), true, false);
            diff.apply_bit_diff(ctx.tiledb.item(tile, "BRAM", "READ_MUX_UL_B"), true, false);
            diff.apply_bit_diff(ctx.tiledb.item(tile, "BRAM", "WRITE_MUX_UL_A"), true, false);
            diff.apply_bit_diff(ctx.tiledb.item(tile, "BRAM", "WRITE_MUX_UL_B"), true, false);
            diff_sdp.apply_bit_diff(ctx.tiledb.item(tile, "BRAM", "READ_MUX_UL_A"), true, false);
            diff_sdp.apply_bit_diff(ctx.tiledb.item(tile, "BRAM", "READ_MUX_UL_B"), true, false);
            diff_sdp.apply_bit_diff(ctx.tiledb.item(tile, "BRAM", "WRITE_MUX_UL_A"), true, false);
            diff_sdp.apply_bit_diff(ctx.tiledb.item(tile, "BRAM", "WRITE_MUX_UL_B"), true, false);
        }

        diff.apply_enum_diff(ctx.tiledb.item(tile, "BRAM", "READ_WIDTH_A_L"), xval, "1");
        if val != "36" {
            diff.apply_enum_diff(ctx.tiledb.item(tile, "BRAM", "WRITE_WIDTH_B_L"), xval, "1");
            diff.apply_enum_diff(ctx.tiledb.item(tile, "BRAM", "READ_WIDTH_A_U"), xval, "1");
            diff.apply_enum_diff(ctx.tiledb.item(tile, "BRAM", "WRITE_WIDTH_B_U"), xval, "1");
        }
        diff_sdp.apply_enum_diff(ctx.tiledb.item(tile, "BRAM", "READ_WIDTH_A_L"), xval, "1");
        diff_sdp.apply_enum_diff(ctx.tiledb.item(tile, "BRAM", "WRITE_WIDTH_B_L"), xval, "1");
        diff_sdp.apply_enum_diff(ctx.tiledb.item(tile, "BRAM", "READ_WIDTH_A_U"), xval, "1");
        diff_sdp.apply_enum_diff(ctx.tiledb.item(tile, "BRAM", "WRITE_WIDTH_B_U"), xval, "1");

        assert_eq!(diff, diff_sdp);
        diffs.push((val, diff));
    }
    ctx.tiledb
        .insert(tile, "BRAM", "FIFO_WIDTH", xlat_enum(diffs));

    if edev.kind == ChipKind::Virtex6 {
        for (bel_bram, bel_fifo) in [("BRAM_F", "FIFO_F"), ("BRAM_H0", "FIFO_H0")] {
            for val in ["REGCE", "RSTREG"] {
                let diff = ctx.state.get_diff(tile, bel_fifo, "RSTREG_PRIORITY", val);
                let diff_a = ctx
                    .state
                    .peek_diff(tile, bel_bram, "RSTREG_PRIORITY_A", val);
                let diff_b = ctx
                    .state
                    .peek_diff(tile, bel_bram, "RSTREG_PRIORITY_B", val);
                assert_eq!(diff, diff_a.combine(diff_b));
            }
        }
    }
    for attr in ["RSTREG_PRIORITY_A", "RSTREG_PRIORITY_B"] {
        for val in ["REGCE", "RSTREG"] {
            let diff_f = ctx.state.get_diff(tile, "BRAM_F", attr, val);
            let diff_h0 = ctx.state.peek_diff(tile, "BRAM_H0", attr, val);
            let diff_h1 = ctx.state.peek_diff(tile, "BRAM_H1", attr, val);
            assert_eq!(diff_f, diff_h0.combine(diff_h1));
        }
        let item = ctx.extract_enum(tile, "BRAM_H0", attr, &["REGCE", "RSTREG"]);
        ctx.tiledb.insert(tile, "BRAM", format!("{attr}_L"), item);
        let item = ctx.extract_enum(tile, "BRAM_H1", attr, &["REGCE", "RSTREG"]);
        ctx.tiledb.insert(tile, "BRAM", format!("{attr}_U"), item);
    }

    for bel in ["BRAM_F", "FIFO_F"] {
        let item = ctx.extract_enum_bool(tile, bel, "EN_ECC_READ", "FALSE", "TRUE");
        ctx.tiledb.insert(tile, "BRAM", "EN_ECC_READ", item);
        let item = ctx.extract_enum_bool(tile, bel, "EN_ECC_WRITE.READ", "FALSE", "TRUE");
        if edev.kind == ChipKind::Virtex7 {
            let item = ctx.extract_enum_bool(tile, bel, "EN_ECC_WRITE", "FALSE", "TRUE");
            ctx.tiledb.insert(tile, "BRAM", "EN_ECC_WRITE", item);
        } else {
            ctx.state
                .get_diff(tile, bel, "EN_ECC_WRITE", "FALSE")
                .assert_empty();
            let mut diff = ctx.state.get_diff(tile, bel, "EN_ECC_WRITE", "TRUE");
            diff.apply_bit_diff(&item, true, false);
            ctx.tiledb
                .insert(tile, "BRAM", "EN_ECC_WRITE_NO_READ", xlat_bit(diff));
        }
        ctx.tiledb.insert(tile, "BRAM", "EN_ECC_WRITE", item);
    }

    for val in ["PERFORMANCE", "DELAYED_WRITE"] {
        let diff_f = ctx
            .state
            .get_diff(tile, "BRAM_F", "RDADDR_COLLISION_HWCONFIG", val);
        if edev.kind == ChipKind::Virtex7 {
            assert_eq!(
                diff_f,
                ctx.state
                    .get_diff(tile, "FIFO_F", "RDADDR_COLLISION_HWCONFIG", val)
            );
            let diff_h0f = ctx
                .state
                .get_diff(tile, "FIFO_H0", "RDADDR_COLLISION_HWCONFIG", val);
            assert_eq!(
                &diff_h0f,
                ctx.state
                    .peek_diff(tile, "BRAM_H0", "RDADDR_COLLISION_HWCONFIG", val)
            );
        }
        let diff_h0 = ctx
            .state
            .peek_diff(tile, "BRAM_H0", "RDADDR_COLLISION_HWCONFIG", val);
        let diff_h1 = ctx
            .state
            .peek_diff(tile, "BRAM_H1", "RDADDR_COLLISION_HWCONFIG", val);
        assert_eq!(diff_f, diff_h0.combine(diff_h1));
    }
    let item = ctx.extract_enum(
        tile,
        "BRAM_H0",
        "RDADDR_COLLISION_HWCONFIG",
        &["PERFORMANCE", "DELAYED_WRITE"],
    );
    ctx.tiledb
        .insert(tile, "BRAM", "RDADDR_COLLISION_HWCONFIG_L", item);
    let item = ctx.extract_enum(
        tile,
        "BRAM_H1",
        "RDADDR_COLLISION_HWCONFIG",
        &["PERFORMANCE", "DELAYED_WRITE"],
    );
    ctx.tiledb
        .insert(tile, "BRAM", "RDADDR_COLLISION_HWCONFIG_U", item);

    if edev.kind == ChipKind::Virtex7 {
        for val in ["NONE", "LEFT", "RIGHT", "BOTH"] {
            let diff_f = ctx.state.get_diff(tile, "BRAM_F", "EN_PWRGATE", val);
            assert_eq!(
                diff_f,
                ctx.state.get_diff(tile, "FIFO_F", "EN_PWRGATE", val)
            );
            let diff_h0f = ctx.state.get_diff(tile, "FIFO_H0", "EN_PWRGATE", val);
            let diff_h0 = ctx.state.peek_diff(tile, "BRAM_H0", "EN_PWRGATE", val);
            assert_eq!(*diff_h0, diff_h0f);
            let diff_h1 = ctx.state.peek_diff(tile, "BRAM_H1", "EN_PWRGATE", val);
            assert_eq!(diff_f, diff_h0.combine(diff_h1));
        }
        let item = ctx.extract_enum(
            tile,
            "BRAM_H0",
            "EN_PWRGATE",
            &["NONE", "LEFT", "RIGHT", "BOTH"],
        );
        ctx.tiledb.insert(tile, "BRAM", "EN_PWRGATE_L", item);
        let item = ctx.extract_enum(
            tile,
            "BRAM_H1",
            "EN_PWRGATE",
            &["NONE", "LEFT", "RIGHT", "BOTH"],
        );
        ctx.tiledb.insert(tile, "BRAM", "EN_PWRGATE_U", item);
        for bel in ["BRAM_F", "FIFO_F", "BRAM_H0", "BRAM_H1", "FIFO_H0"] {
            let item = ctx.extract_enum_bool(tile, bel, "EN_SDBITERR_INIT_V6", "FALSE", "TRUE");
            ctx.tiledb.insert(tile, "BRAM", "EN_SDBITERR_INIT_V6", item);
        }
    }

    present_ramb36.assert_empty();
    present_ramb18_l.assert_empty();
    present_ramb18_u.assert_empty();
    let is_fifo_u = present_fifo36.combine(&!&present_fifo18);
    ctx.tiledb
        .insert(tile, "BRAM", "IS_FIFO", xlat_bit(present_fifo18));
    ctx.tiledb
        .insert(tile, "BRAM", "IS_FIFO_U", xlat_bit(is_fifo_u));

    for (bel, attr) in [
        ("BRAM_F", "BYPASS_RSR"),
        ("BRAM_F", "SWAP_CFGPORT"),
        ("FIFO_F", "TEST_FIFO_FLAG"),
        ("FIFO_F", "TEST_FIFO_OFFSET"),
        ("FIFO_F", "TEST_FIFO_CNT"),
    ] {
        let item = ctx.extract_bit(tile, bel, attr, "ENABLED");
        ctx.tiledb.insert(tile, "BRAM", attr, item);
    }
    let item = ctx.extract_enum(tile, "BRAM_F", "WEAK_WRITE", &["NO_WW", "WW0", "WW1"]);
    ctx.tiledb.insert(tile, "BRAM", "WEAK_WRITE", item);
    for attr in ["EN_TSTBRAMRST", "DIS_TSTFIFORST"] {
        let item = ctx.extract_bit(tile, "BRAM_F", attr, "1");
        ctx.tiledb.insert(tile, "BRAM", attr, item);
    }
    ctx.collect_bitvec(tile, "BRAM", "TEST_ATTRIBUTES", "");
    for attr in ["TRD_DLY_L", "TRD_DLY_U"] {
        let mut diffs = vec![];
        for (ival, val) in ["0", "1", "10", "11", "100", "101", "110", "111"]
            .into_iter()
            .enumerate()
        {
            diffs.push((
                ival.try_into().unwrap(),
                ctx.state.get_diff(tile, "BRAM_F", attr, val),
            ));
        }
        ctx.tiledb.insert(tile, "BRAM", attr, xlat_enum_int(diffs));
    }
    if edev.kind == ChipKind::Virtex6 {
        for attr in ["TWR_DLY_L", "TWR_DLY_U"] {
            let mut diffs = vec![];
            for (ival, val) in ["0", "1", "10", "11", "100", "101", "110", "111"]
                .into_iter()
                .enumerate()
            {
                diffs.push((
                    ival.try_into().unwrap(),
                    ctx.state.get_diff(tile, "BRAM_F", attr, val),
                ));
            }
            ctx.tiledb.insert(tile, "BRAM", attr, xlat_enum_int(diffs));
        }
        for attr in ["EN_TSTREFBL", "EN_TSTRSRW"] {
            let mut diffs = vec![];
            for (ival, val) in ["0", "1", "10", "11"].into_iter().enumerate() {
                diffs.push((
                    ival.try_into().unwrap(),
                    ctx.state.get_diff(tile, "BRAM_F", attr, val),
                ));
            }
            ctx.tiledb.insert(tile, "BRAM", attr, xlat_enum_int(diffs));
        }
        let item = ctx.extract_enum_bool(tile, "BRAM_F", "EN_TSTBLCLAMP", "0", "1");
        ctx.tiledb.insert(tile, "BRAM", "EN_TSTBLCLAMP", item);
    } else {
        for attr in ["TWR_DLY_A_L", "TWR_DLY_A_U", "TWR_DLY_B_L", "TWR_DLY_B_U"] {
            let mut diffs = vec![];
            for (ival, val) in [
                "0", "1", "10", "11", "100", "101", "110", "111", "1000", "1001", "1010", "1011",
                "1100", "1101", "1110", "1111",
            ]
            .into_iter()
            .enumerate()
            {
                diffs.push((
                    ival.try_into().unwrap(),
                    ctx.state.get_diff(tile, "BRAM_F", attr, val),
                ));
            }
            ctx.tiledb.insert(tile, "BRAM", attr, xlat_enum_int(diffs));
        }

        for attr in [
            "TSTREFBL_CTRL",
            "TSTRSR_RWCTRL",
            "EN_TSTRFMODE_DLY",
            "EN_TSTPULSEPU_DLY",
            "EN_TSTEXTCLK",
            "EN_TSTRSTC_PW",
            "EN_TSTBLPC_DLY",
            "EN_TST_REGOUT_DLY_SEL",
            "TST_RNG_OSC",
        ] {
            let mut diffs = vec![];
            for (ival, val) in ["0", "1", "10", "11"].into_iter().enumerate() {
                diffs.push((
                    ival.try_into().unwrap(),
                    ctx.state.get_diff(tile, "BRAM_F", attr, val),
                ));
            }
            ctx.tiledb.insert(tile, "BRAM", attr, xlat_enum_int(diffs));
        }
        for attr in ["DIS_TSTBLCLAMP", "TST_SSRLAT_WF"] {
            let item = ctx.extract_enum_bool(tile, "BRAM_F", attr, "0", "1");
            ctx.tiledb.insert(tile, "BRAM", attr, item);
        }
        for attr in [
            "EN_TSTBLCLMP_WW",
            "EN_TSTSNM",
            "LAT_RST_DLYEN",
            "STUCK_DET_EN",
            "TST_PULSEPU_SFT",
        ] {
            let item = ctx.extract_enum_bool(tile, "BRAM_F", attr, "DISABLE", "ENABLE");
            ctx.tiledb.insert(tile, "BRAM", attr, item);
        }
        let item = ctx.extract_enum_bool(tile, "BRAM_F", "EN_TSTBLCLAMP_RD", "NO", "YES");
        ctx.tiledb.insert(tile, "BRAM", "EN_TSTBLCLAMP_RD", item);

        // hm. bug?
        ctx.state
            .get_diff(tile, "BRAM_F", "TST_BIST_CTL", "DISABLE")
            .assert_empty();
        ctx.state
            .get_diff(tile, "BRAM_F", "TST_BIST_CTL", "ENABLE")
            .assert_empty();
    }

    if edev.kind == ChipKind::Virtex7 {
        let bel = "BRAM_ADDR";
        for ab in ['A', 'B'] {
            for i in 0..15 {
                for ul in ['U', 'L'] {
                    ctx.collect_enum(
                        tile,
                        bel,
                        &format!("MUX.ADDR{ab}{ul}{i}"),
                        &["INT", "CASCINBOT", "CASCINTOP"],
                    );
                }
                let item = ctx.extract_bit(tile, bel, &format!("CASCADE_OUT.ADDR{ab}{i}"), "1");
                ctx.tiledb
                    .insert(tile, bel, format!("ADDR_CASCADE_OUT_{ab}"), item);
            }
        }
    }
}
