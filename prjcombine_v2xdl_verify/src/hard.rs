use crate::types::{Test, SrcInst, TgtInst, TestGenCtx, BitVal};
use rand::Rng;

#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub enum EmacMode {
    Virtex4,
    Virtex5,
    Virtex6,
}

fn make_param_bool(ctx: &mut TestGenCtx, inst: &mut SrcInst, ti: &mut TgtInst, name: &str) {
    let val = ctx.rng.gen();
    inst.param_bool(name, val);
    ti.cfg_bool(name, val);
}

fn make_param_hex(ctx: &mut TestGenCtx, inst: &mut SrcInst, ti: &mut TgtInst, name: &str, num: usize) {
    let val = ctx.gen_bits(num);
    inst.param_bits(name, &val);
    ti.cfg_hex(name, &val, true);
}

fn make_ins(test: &mut Test, ctx: &mut TestGenCtx, inst: &mut SrcInst, ti: &mut TgtInst, name: &str, msb: usize, lsb: usize) {
    if msb < lsb {
        let w = test.make_ins(ctx, lsb - msb + 1);
        inst.connect_bus(name, &w);
        for (i, w) in w.iter().enumerate() {
            ti.pin_in(&format!("{name}{ii}", ii = lsb - i), w);
        }
    } else {
        let w = test.make_ins(ctx, msb - lsb + 1);
        inst.connect_bus(name, &w);
        for (i, w) in w.iter().enumerate() {
            ti.pin_in(&format!("{name}{ii}", ii = lsb + i), w);
        }
    }
}

fn make_ins_inv(test: &mut Test, ctx: &mut TestGenCtx, inst: &mut SrcInst, ti: &mut TgtInst, name: &str, msb: usize, lsb: usize) {
    let mut w = Vec::new();
    if msb < lsb {
        for i in 0..(lsb-msb+1) {
            let (w_v, w_x, w_inv) = test.make_in_inv(ctx);
            ti.pin_in_inv(&format!("{name}{ii}", ii = lsb - i), &w_x, w_inv);
            w.push(w_v);
        }
    } else {
        for i in 0..(msb-lsb+1) {
            let (w_v, w_x, w_inv) = test.make_in_inv(ctx);
            ti.pin_in_inv(&format!("{name}{ii}", ii = lsb + i), &w_x, w_inv);
            w.push(w_v);
        }
    }
    inst.connect_bus(name, &w);
}

fn make_outs(test: &mut Test, ctx: &mut TestGenCtx, inst: &mut SrcInst, ti: &mut TgtInst, name: &str, msb: usize, lsb: usize) {
    if msb < lsb {
        let w = test.make_outs(ctx, lsb - msb + 1);
        inst.connect_bus(name, &w);
        for (i, w) in w.iter().enumerate() {
            ti.pin_out(&format!("{name}{ii}", ii = lsb - i), w);
        }
    } else {
        let w = test.make_outs(ctx, msb - lsb + 1);
        inst.connect_bus(name, &w);
        for (i, w) in w.iter().enumerate() {
            ti.pin_out(&format!("{name}{ii}", ii = lsb + i), w);
        }
    }
}

fn make_in(test: &mut Test, ctx: &mut TestGenCtx, inst: &mut SrcInst, ti: &mut TgtInst, name: &str) {
    let w = test.make_in(ctx);
    inst.connect(name, &w);
    ti.pin_in(name, &w);
}

fn make_in_inv_fake(test: &mut Test, ctx: &mut TestGenCtx, inst: &mut SrcInst, ti: &mut TgtInst, name: &str) {
    let w = test.make_in(ctx);
    inst.connect(name, &w);
    ti.pin_in_inv(name, &w, false);
}

fn make_in_inv(test: &mut Test, ctx: &mut TestGenCtx, inst: &mut SrcInst, ti: &mut TgtInst, name: &str) {
    let (w_v, w_x, w_inv) = test.make_in_inv(ctx);
    inst.connect(name, &w_v);
    ti.pin_in_inv(name, &w_x, w_inv);
}

fn make_out(test: &mut Test, ctx: &mut TestGenCtx, inst: &mut SrcInst, ti: &mut TgtInst, name: &str) {
    let w = test.make_out(ctx);
    inst.connect(name, &w);
    ti.pin_out(name, &w);
}

struct EmacDcr {
    clk: String,
    enable: String,
    read: String,
    write: String,
    dbus_w: Vec<String>,
    dbus_r: Vec<String>,
    abus: Vec<String>,
    ack: String,
}

fn make_emac(test: &mut Test, ctx: &mut TestGenCtx, dcr: Option<EmacDcr>, mode: EmacMode) {
    let prim = match mode {
        EmacMode::Virtex4 => "EMAC",
        EmacMode::Virtex5 => "TEMAC",
        EmacMode::Virtex6 => "TEMAC_SINGLE",
    };
    let mut inst = SrcInst::new(ctx, prim);
    let mut ti = TgtInst::new(&[prim]);
    ti.bel(prim, &inst.name, "");

    let emacs = if mode == EmacMode::Virtex6 {&["EMAC"][..]} else {&["EMAC0", "EMAC1"][..]};

    // CLIENT
    for emac in emacs {
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTANINTERRUPT"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTRXBADFRAME"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTRXCLIENTCLKOUT"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTRXDVLD"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTRXDVLDMSW"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTRXFRAMEDROP"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTRXGOODFRAME"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTRXSTATSBYTEVLD"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTRXSTATSVLD"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTTXACK"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTTXCLIENTCLKOUT"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTTXCOLLISION"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTTXRETRANSMIT"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTTXSTATS"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTTXSTATSBYTEVLD"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTTXSTATSVLD"));
        make_outs(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTRXSTATS"), 6, 0);
        make_outs(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTRXD"), 15, 0);
        make_in(test, ctx, &mut inst, &mut ti, &format!("CLIENT{emac}DCMLOCKED"));
        make_in(test, ctx, &mut inst, &mut ti, &format!("CLIENT{emac}PAUSEREQ"));
        if mode != EmacMode::Virtex6 {
            make_in_inv(test, ctx, &mut inst, &mut ti, &format!("CLIENT{emac}RXCLIENTCLKIN"));
            make_in_inv(test, ctx, &mut inst, &mut ti, &format!("CLIENT{emac}TXCLIENTCLKIN"));
        } else {
            make_in(test, ctx, &mut inst, &mut ti, &format!("CLIENT{emac}RXCLIENTCLKIN"));
            make_in(test, ctx, &mut inst, &mut ti, &format!("CLIENT{emac}TXCLIENTCLKIN"));
        }
        make_in(test, ctx, &mut inst, &mut ti, &format!("CLIENT{emac}TXDVLD"));
        make_in(test, ctx, &mut inst, &mut ti, &format!("CLIENT{emac}TXDVLDMSW"));
        make_in(test, ctx, &mut inst, &mut ti, &format!("CLIENT{emac}TXFIRSTBYTE"));
        make_in(test, ctx, &mut inst, &mut ti, &format!("CLIENT{emac}TXUNDERRUN"));
        make_ins(test, ctx, &mut inst, &mut ti, &format!("CLIENT{emac}TXD"), 15, 0);
        make_ins(test, ctx, &mut inst, &mut ti, &format!("CLIENT{emac}PAUSEVAL"), 15, 0);
        make_ins(test, ctx, &mut inst, &mut ti, &format!("CLIENT{emac}TXIFGDELAY"), 7, 0);
        if mode == EmacMode::Virtex4 {
            make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTRXDVREG6"));
            make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}CLIENTTXGMIIMIICLKOUT"));
            make_in_inv(test, ctx, &mut inst, &mut ti, &format!("CLIENT{emac}TXGMIIMIICLKIN"));
        }
    }

    // PHY
    for emac in emacs {
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}PHYENCOMMAALIGN"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}PHYLOOPBACKMSB"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}PHYMCLKOUT"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}PHYMDOUT"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}PHYMDTRI"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}PHYMGTRXRESET"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}PHYMGTTXRESET"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}PHYPOWERDOWN"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}PHYSYNCACQSTATUS"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}PHYTXCHARDISPMODE"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}PHYTXCHARDISPVAL"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}PHYTXCHARISK"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}PHYTXCLK"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}PHYTXEN"));
        make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}PHYTXER"));
        make_outs(test, ctx, &mut inst, &mut ti, &format!("{emac}PHYTXD"), 7, 0);
        make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}COL"));
        make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}CRS"));
        if mode != EmacMode::Virtex6 {
            make_in_inv(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}GTXCLK"));
            make_in_inv(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}MCLKIN"));
            make_in_inv(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}MIITXCLK"));
            make_in_inv(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}RXCLK"));
        } else {
            make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}GTXCLK"));
            make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}MCLKIN"));
            make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}MIITXCLK"));
            make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}RXCLK"));
        }
        make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}MDIN"));
        make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}RXCHARISCOMMA"));
        make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}RXCHARISK"));
        make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}RXDISPERR"));
        make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}RXDV"));
        make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}RXER"));
        make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}RXNOTINTABLE"));
        make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}RXRUNDISP"));
        make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}SIGNALDET"));
        make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}TXBUFERR"));
        make_ins(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}RXBUFSTATUS"), 1, 0);
        make_ins(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}RXCLKCORCNT"), 2, 0);
        make_ins(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}PHYAD"), 4, 0);
        make_ins(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}RXD"), 7, 0);
        if mode != EmacMode::Virtex6 {
            make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}RXBUFERR"));
            make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}RXCHECKINGCRC"));
            make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}RXCOMMADET"));
            make_ins(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}RXLOSSOFSYNC"), 1, 0);
        }
        if mode != EmacMode::Virtex4 {
            make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}PHYTXGMIIMIICLKOUT"));
            if mode != EmacMode::Virtex6 {
                make_in_inv(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}TXGMIIMIICLKIN"));
            } else {
                make_in(test, ctx, &mut inst, &mut ti, &format!("PHY{emac}TXGMIIMIICLKIN"));
            }
        }
    }

    // HOST
    make_out(test, ctx, &mut inst, &mut ti, "DCRHOSTDONEIR");
    make_out(test, ctx, &mut inst, &mut ti, "HOSTMIIMRDY");
    make_outs(test, ctx, &mut inst, &mut ti, "HOSTRDDATA", 31, 0);
    if mode != EmacMode::Virtex6 {
        make_in_inv(test, ctx, &mut inst, &mut ti, "HOSTCLK");
    } else {
        make_in(test, ctx, &mut inst, &mut ti, "HOSTCLK");
    }
    if mode != EmacMode::Virtex6 {
        make_in(test, ctx, &mut inst, &mut ti, "HOSTEMAC1SEL");
    }
    make_in(test, ctx, &mut inst, &mut ti, "HOSTMIIMSEL");
    make_in(test, ctx, &mut inst, &mut ti, "HOSTREQ");
    make_ins(test, ctx, &mut inst, &mut ti, "HOSTOPCODE", 1, 0);
    make_ins(test, ctx, &mut inst, &mut ti, "HOSTWRDATA", 31, 0);
    make_ins(test, ctx, &mut inst, &mut ti, "HOSTADDR", 9, 0);

    // DCR
    if mode == EmacMode::Virtex4 {
        if let Some(dcr) = dcr {
            inst.connect("DCREMACCLK", &dcr.clk);
            inst.connect("DCREMACENABLE", &dcr.enable);
            inst.connect("DCREMACREAD", &dcr.read);
            inst.connect("DCREMACWRITE", &dcr.write);
            inst.connect("EMACDCRACK", &dcr.ack);
            ti.pin_in("DCREMACCLK", &dcr.clk);
            ti.pin_in("DCREMACENABLE", &dcr.enable);
            ti.pin_in("DCREMACREAD", &dcr.read);
            ti.pin_in("DCREMACWRITE", &dcr.write);
            ti.pin_out("EMACDCRACK", &dcr.ack);
            inst.connect_bus("DCREMACDBUS", &dcr.dbus_w);
            inst.connect_bus("DCREMACABUS", &dcr.abus);
            inst.connect_bus("EMACDCRDBUS", &dcr.dbus_r);
            for i in 0..32 {
                ti.pin_in(&format!("DCREMACDBUS{ii}", ii=31-i), &dcr.dbus_w[i]);
                ti.pin_out(&format!("EMACDCRDBUS{ii}", ii=31-i), &dcr.dbus_r[i]);
            }
            for i in 0..2 {
                ti.pin_in(&format!("DCREMACABUS{ii}", ii=9-i), &dcr.abus[i]);
            }
        } else {
            ti.pin_tie("DCREMACENABLE", false);
        }
    } else {
        if mode != EmacMode::Virtex6 {
            make_in_inv(test, ctx, &mut inst, &mut ti, "DCREMACCLK");
        } else {
            make_in(test, ctx, &mut inst, &mut ti, "DCREMACCLK");
        }
        make_in(test, ctx, &mut inst, &mut ti, "DCREMACENABLE");
        make_in(test, ctx, &mut inst, &mut ti, "DCREMACREAD");
        make_in(test, ctx, &mut inst, &mut ti, "DCREMACWRITE");
        make_out(test, ctx, &mut inst, &mut ti, "EMACDCRACK");
        make_ins(test, ctx, &mut inst, &mut ti, "DCREMACDBUS", 0, 31);
        make_ins(test, ctx, &mut inst, &mut ti, "DCREMACABUS", 0, 9);
        make_outs(test, ctx, &mut inst, &mut ti, "EMACDCRDBUS", 0, 31);
    }

    // misc
    make_in(test, ctx, &mut inst, &mut ti, "RESET");
    if mode == EmacMode::Virtex4 {
        make_ins(test, ctx, &mut inst, &mut ti, "TIEEMAC0UNICASTADDR", 47, 0);
        make_ins_inv(test, ctx, &mut inst, &mut ti, "TIEEMAC1UNICASTADDR", 47, 0);
        for i in 0..2 {
            make_ins(test, ctx, &mut inst, &mut ti, &format!("TIEEMAC{i}CONFIGVEC"), 79, 0);
        }
    } else {
        for emac in emacs {
            make_out(test, ctx, &mut inst, &mut ti, &format!("{emac}SPEEDIS10100"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_1000BASEX_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_ADDRFILTER_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_BYTEPHY"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_GTLOOPBACK"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_HOST_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_LTCHECK_DISABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_MDIO_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_PHYINITAUTONEG_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_PHYISOLATE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_PHYLOOPBACKMSB"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_PHYPOWERDOWN"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_PHYRESET"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_RGMII_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_RX16BITCLIENT_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_RXFLOWCTRL_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_RXHALFDUPLEX"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_RXINBANDFCS_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_RXJUMBOFRAME_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_RXRESET"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_RXVLAN_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_RX_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_SGMII_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_SPEED_LSB"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_SPEED_MSB"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_TX16BITCLIENT_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_TXFLOWCTRL_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_TXHALFDUPLEX"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_TXIFGADJUST_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_TXINBANDFCS_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_TXJUMBOFRAME_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_TXRESET"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_TXVLAN_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_TX_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_UNIDIRECTION_ENABLE"));
            make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_USECLKEN"));
            make_param_hex(ctx, &mut inst, &mut ti, &format!("{emac}_DCRBASEADDR"), 8);
            make_param_hex(ctx, &mut inst, &mut ti, &format!("{emac}_UNICASTADDR"), 48);
            make_param_hex(ctx, &mut inst, &mut ti, &format!("{emac}_PAUSEADDR"), 48);
            make_param_hex(ctx, &mut inst, &mut ti, &format!("{emac}_LINKTIMERVAL"), 9);
            if mode != EmacMode::Virtex4 {
                ti.cfg(&format!("{emac}_FUNCTION"), "0");
            }
            if mode != EmacMode::Virtex6 {
                make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_CONFIGVEC_79"));
            } else {
                make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_CTRLLENCHECK_DISABLE"));
                make_param_bool(ctx, &mut inst, &mut ti, &format!("{emac}_MDIO_IGNORE_PHYADZERO"));
                ti.cfg("EMAC_CONFIGVEC_79", "TRUE");
            }
        }
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

pub fn gen_ppc405(test: &mut Test, ctx: &mut TestGenCtx, is_adv: bool) {
    let prim = if is_adv {"PPC405_ADV"} else {"PPC405"};
    let mut inst = SrcInst::new(ctx, prim);
    let mut ti = TgtInst::new(&[prim]);
    ti.bel(prim, &inst.name, "");

    // DSOCM
    make_outs(test, ctx, &mut inst, &mut ti, "DSOCMBRAMABUS", 8, 29);
    make_outs(test, ctx, &mut inst, &mut ti, "DSOCMBRAMBYTEWRITE", 0, 3);
    make_out(test, ctx, &mut inst, &mut ti, "DSOCMBRAMEN");
    make_out(test, ctx, &mut inst, &mut ti, "DSOCMBUSY");
    make_outs(test, ctx, &mut inst, &mut ti, "DSOCMBRAMWRDBUS", 0, 31);
    make_ins(test, ctx, &mut inst, &mut ti, "BRAMDSOCMRDDBUS", 0, 31);
    make_in_inv_fake(test, ctx, &mut inst, &mut ti, "BRAMDSOCMCLK");
    if !is_adv {
        make_ins_inv(test, ctx, &mut inst, &mut ti, "TIEDSOCMDCRADDR", 0, 7);
        ti.pin_tie("BRAMDSOCMRDDACK", false);
        for i in 0..8 {
            ti.pin_tie(&format!("TSTDSOCMDBUSI{i}"), false);
        }
        for i in 0..32 {
            ti.pin_tie(&format!("TSTDSOCMWRDBUSI{i}"), false);
        }
        for i in 0..30 {
            ti.pin_tie(&format!("TSTDSOCMABUSI{i}"), false);
        }
        ti.pin_tie("TSTDSOCMABORTOPI", false);
        ti.pin_tie("TSTDSOCMABORTREQI", false);
        for i in 0..4 {
            ti.pin_tie(&format!("TSTDSOCMBYTEENI{i}"), false);
        }
        ti.pin_tie("TSTDSOCMCOMPLETEI", false);
        ti.pin_tie("TSTDSOCMDCRACKI", false);
        ti.pin_tie("TSTDSOCMHOLDI", false);
        ti.pin_tie("TSTDSOCMLOADREQI", false);
        ti.pin_tie("TSTDSOCMSTOREREQI", false);
        ti.pin_tie("TSTDSOCMWAITI", false);
        ti.pin_tie("TSTDSOCMXLATEVALIDI", false);
    } else {
        make_out(test, ctx, &mut inst, &mut ti, "DSOCMRDADDRVALID");
        make_out(test, ctx, &mut inst, &mut ti, "DSOCMWRADDRVALID");
        make_in(test, ctx, &mut inst, &mut ti, "DSOCMRWCOMPLETE");
    }

    // ISOCM
    make_outs(test, ctx, &mut inst, &mut ti, "ISOCMBRAMWRABUS", 8, 28);
    make_outs(test, ctx, &mut inst, &mut ti, "ISOCMBRAMRDABUS", 8, 28);
    make_out(test, ctx, &mut inst, &mut ti, "ISOCMBRAMEVENWRITEEN");
    make_out(test, ctx, &mut inst, &mut ti, "ISOCMBRAMODDWRITEEN");
    make_out(test, ctx, &mut inst, &mut ti, "ISOCMBRAMEN");
    make_outs(test, ctx, &mut inst, &mut ti, "ISOCMBRAMWRDBUS", 0, 31);
    make_ins(test, ctx, &mut inst, &mut ti, "BRAMISOCMRDDBUS", 0, 63);
    make_in_inv_fake(test, ctx, &mut inst, &mut ti, "BRAMISOCMCLK");
    if !is_adv {
        make_ins_inv(test, ctx, &mut inst, &mut ti, "TIEISOCMDCRADDR", 0, 7);
        ti.pin_tie("BRAMISOCMRDDACK", false);
        for i in 0..64 {
            ti.pin_tie(&format!("TSTISOCMRDATAI{i}"), false);
        }
        for i in 0..30 {
            ti.pin_tie(&format!("TSTISOCMABUSI{i}"), false);
        }
        ti.pin_tie("TSTISOCMABORTI", false);
        ti.pin_tie("TSTISOCMHOLDI", false);
        ti.pin_tie("TSTISOCMICUREADYI", false);
        ti.pin_tie("TSTISOCMRDDVALIDI0", false);
        ti.pin_tie("TSTISOCMRDDVALIDI1", false);
        ti.pin_tie("TSTISOCMREQPENDI", false);
        ti.pin_tie("TSTISOCMXLATEVALIDI", false);
        ti.pin_tie("TSTISOPFWDI", false);
    } else {
        make_out(test, ctx, &mut inst, &mut ti, "ISOCMDCRBRAMEVENEN");
        make_out(test, ctx, &mut inst, &mut ti, "ISOCMDCRBRAMODDEN");
        make_out(test, ctx, &mut inst, &mut ti, "ISOCMDCRBRAMRDSELECT");
        make_ins(test, ctx, &mut inst, &mut ti, "BRAMISOCMDCRRDDBUS", 0, 31);
    }

    // CPM
    make_out(test, ctx, &mut inst, &mut ti, "C405CPMCORESLEEPREQ");
    make_out(test, ctx, &mut inst, &mut ti, "C405CPMMSRCE");
    make_out(test, ctx, &mut inst, &mut ti, "C405CPMMSREE");
    make_out(test, ctx, &mut inst, &mut ti, "C405CPMTIMERIRQ");
    make_out(test, ctx, &mut inst, &mut ti, "C405CPMTIMERRESETREQ");
    make_in_inv(test, ctx, &mut inst, &mut ti, "CPMC405CLOCK");
    make_in(test, ctx, &mut inst, &mut ti, "CPMC405CORECLKINACTIVE");
    if !is_adv {
        make_in_inv(test, ctx, &mut inst, &mut ti, "CPMC405CPUCLKEN");
        make_in_inv(test, ctx, &mut inst, &mut ti, "CPMC405JTAGCLKEN");
        make_in_inv(test, ctx, &mut inst, &mut ti, "CPMC405TIMERCLKEN");
        make_in_inv(test, ctx, &mut inst, &mut ti, "CPMC405TIMERTICK");
    } else {
        make_in(test, ctx, &mut inst, &mut ti, "CPMC405CPUCLKEN");
        make_in(test, ctx, &mut inst, &mut ti, "CPMC405JTAGCLKEN");
        make_in(test, ctx, &mut inst, &mut ti, "CPMC405TIMERCLKEN");
        make_in(test, ctx, &mut inst, &mut ti, "CPMC405TIMERTICK");
    }
    if is_adv {
        if ctx.rng.gen() {
            inst.connect("CPMC405SYNCBYPASS", "1'b1");
            ti.cfg("PLB_SYNC_MODE", "SYNCBYPASS");
            ti.pin_tie("CPMC405SYNCBYPASS", true);
        } else {
            inst.connect("CPMC405SYNCBYPASS", "1'b0");
            ti.cfg("PLB_SYNC_MODE", "SYNCACTIVE");
            ti.pin_tie("CPMC405SYNCBYPASS", false);
        }
        make_in_inv(test, ctx, &mut inst, &mut ti, "CPMDCRCLK");
        make_in_inv(test, ctx, &mut inst, &mut ti, "CPMFCMCLK");
    }

    // DBG
    if is_adv {
        make_out(test, ctx, &mut inst, &mut ti, "C405DBGLOADDATAONAPUDBUS");
    }
    make_out(test, ctx, &mut inst, &mut ti, "C405DBGMSRWE");
    make_out(test, ctx, &mut inst, &mut ti, "C405DBGSTOPACK");
    make_out(test, ctx, &mut inst, &mut ti, "C405DBGWBCOMPLETE");
    make_out(test, ctx, &mut inst, &mut ti, "C405DBGWBFULL");
    make_outs(test, ctx, &mut inst, &mut ti, "C405DBGWBIAR", 0, 29);
    make_in(test, ctx, &mut inst, &mut ti, "DBGC405DEBUGHALT");
    make_in(test, ctx, &mut inst, &mut ti, "DBGC405EXTBUSHOLDACK");
    make_in(test, ctx, &mut inst, &mut ti, "DBGC405UNCONDDEBUGEVENT");

    // DCR
    if !is_adv {
        make_outs(test, ctx, &mut inst, &mut ti, "C405DCRABUS", 0, 9);
        make_outs(test, ctx, &mut inst, &mut ti, "C405DCRDBUSOUT", 0, 31);
        make_out(test, ctx, &mut inst, &mut ti, "C405DCRREAD");
        make_out(test, ctx, &mut inst, &mut ti, "C405DCRWRITE");
        make_in(test, ctx, &mut inst, &mut ti, "DCRC405ACK");
        make_ins(test, ctx, &mut inst, &mut ti, "DCRC405DBUSIN", 0, 31);
        for i in 0..32 {
            ti.pin_tie(&format!("TSTC405DCRDBUSOUTI{i}"), false);
        }
        for i in 0..32 {
            ti.pin_tie(&format!("TSTDCRBUSI{i}"), false);
        }
        for i in 0..10 {
            ti.pin_tie(&format!("TSTC405DCRABUSI{i}"), false);
        }
        ti.pin_tie("TSTC405DCRREADI", false);
        ti.pin_tie("TSTC405DCRWRITEI", false);
        ti.pin_tie("TSTDCRACKI", false);
    } else {
        make_outs(test, ctx, &mut inst, &mut ti, "EXTDCRABUS", 0, 9);
        make_outs(test, ctx, &mut inst, &mut ti, "EXTDCRDBUSOUT", 0, 31);
        make_out(test, ctx, &mut inst, &mut ti, "EXTDCRREAD");
        make_out(test, ctx, &mut inst, &mut ti, "EXTDCRWRITE");
        make_in(test, ctx, &mut inst, &mut ti, "EXTDCRACK");
        make_ins(test, ctx, &mut inst, &mut ti, "EXTDCRDBUSIN", 0, 31);
        make_ins_inv(test, ctx, &mut inst, &mut ti, "TIEDCRADDR", 0, 5);

        if ctx.rng.gen() {
            let dcr = EmacDcr {
                clk: test.make_wire(ctx),
                enable: test.make_wire(ctx),
                read: test.make_wire(ctx),
                write: test.make_wire(ctx),
                abus: test.make_bus(ctx, 2),
                dbus_w: test.make_bus(ctx, 32),
                dbus_r: test.make_bus(ctx, 32),
                ack: test.make_wire(ctx),
            };
            inst.connect("DCREMACCLK", &dcr.clk);
            inst.connect("DCREMACENABLER", &dcr.enable);
            inst.connect("DCREMACREAD", &dcr.read);
            inst.connect("DCREMACWRITE", &dcr.write);
            inst.connect("EMACDCRACK", &dcr.ack);
            ti.pin_out("DCREMACCLK", &dcr.clk);
            ti.pin_out("DCREMACENABLER", &dcr.enable);
            ti.pin_out("DCREMACREAD", &dcr.read);
            ti.pin_out("DCREMACWRITE", &dcr.write);
            ti.pin_in("EMACDCRACK", &dcr.ack);
            inst.connect_bus("DCREMACDBUS", &dcr.dbus_w);
            inst.connect_bus("DCREMACABUS", &dcr.abus);
            inst.connect_bus("EMACDCRDBUS", &dcr.dbus_r);
            for i in 0..32 {
                ti.pin_out(&format!("DCREMACDBUS{ii}", ii=31-i), &dcr.dbus_w[i]);
                ti.pin_in(&format!("EMACDCRDBUS{ii}", ii=31-i), &dcr.dbus_r[i]);
            }
            for i in 0..2 {
                ti.pin_out(&format!("DCREMACABUS{ii}", ii=9-i), &dcr.abus[i]);
            }
            make_emac(test, ctx, Some(dcr), EmacMode::Virtex4);
        }
    }

    // JTG
    make_out(test, ctx, &mut inst, &mut ti, "C405JTGCAPTUREDR");
    make_out(test, ctx, &mut inst, &mut ti, "C405JTGEXTEST");
    make_out(test, ctx, &mut inst, &mut ti, "C405JTGPGMOUT");
    make_out(test, ctx, &mut inst, &mut ti, "C405JTGSHIFTDR");
    make_out(test, ctx, &mut inst, &mut ti, "C405JTGTDO");
    make_out(test, ctx, &mut inst, &mut ti, "C405JTGTDOEN");
    make_out(test, ctx, &mut inst, &mut ti, "C405JTGUPDATEDR");
    make_in(test, ctx, &mut inst, &mut ti, "JTGC405BNDSCANTDO");
    make_in_inv(test, ctx, &mut inst, &mut ti, "JTGC405TCK");
    make_in(test, ctx, &mut inst, &mut ti, "JTGC405TDI");
    make_in(test, ctx, &mut inst, &mut ti, "JTGC405TMS");
    make_in(test, ctx, &mut inst, &mut ti, "JTGC405TRSTNEG");
    if !is_adv {
        ti.pin_tie("TSTTRSTNEGI", true);
    }

    // PLB DCU
    make_out(test, ctx, &mut inst, &mut ti, "C405PLBDCUABORT");
    make_outs(test, ctx, &mut inst, &mut ti, "C405PLBDCUABUS", 0, 31);
    make_outs(test, ctx, &mut inst, &mut ti, "C405PLBDCUBE", 0, 7);
    make_out(test, ctx, &mut inst, &mut ti, "C405PLBDCUCACHEABLE");
    make_out(test, ctx, &mut inst, &mut ti, "C405PLBDCUGUARDED");
    make_outs(test, ctx, &mut inst, &mut ti, "C405PLBDCUPRIORITY", 0, 1);
    make_out(test, ctx, &mut inst, &mut ti, "C405PLBDCUREQUEST");
    make_out(test, ctx, &mut inst, &mut ti, "C405PLBDCURNW");
    make_out(test, ctx, &mut inst, &mut ti, "C405PLBDCUSIZE2");
    make_out(test, ctx, &mut inst, &mut ti, "C405PLBDCUU0ATTR");
    make_outs(test, ctx, &mut inst, &mut ti, "C405PLBDCUWRDBUS", 0, 63);
    make_in(test, ctx, &mut inst, &mut ti, "PLBC405DCUADDRACK");
    make_in(test, ctx, &mut inst, &mut ti, "PLBC405DCUBUSY");
    make_in(test, ctx, &mut inst, &mut ti, "PLBC405DCUERR");
    make_in(test, ctx, &mut inst, &mut ti, "PLBC405DCURDDACK");
    make_ins(test, ctx, &mut inst, &mut ti, "PLBC405DCURDDBUS", 0, 63);
    make_ins(test, ctx, &mut inst, &mut ti, "PLBC405DCURDWDADDR", 1, 3);
    make_in(test, ctx, &mut inst, &mut ti, "PLBC405DCUSSIZE1");
    make_in(test, ctx, &mut inst, &mut ti, "PLBC405DCUWRDACK");

    // PLB ICU
    make_out(test, ctx, &mut inst, &mut ti, "C405PLBDCUWRITETHRU");
    make_out(test, ctx, &mut inst, &mut ti, "C405PLBICUABORT");
    make_outs(test, ctx, &mut inst, &mut ti, "C405PLBICUABUS", 0, 29);
    make_out(test, ctx, &mut inst, &mut ti, "C405PLBICUCACHEABLE");
    make_outs(test, ctx, &mut inst, &mut ti, "C405PLBICUPRIORITY", 0, 1);
    make_out(test, ctx, &mut inst, &mut ti, "C405PLBICUREQUEST");
    make_outs(test, ctx, &mut inst, &mut ti, "C405PLBICUSIZE", 2, 3);
    make_out(test, ctx, &mut inst, &mut ti, "C405PLBICUU0ATTR");
    make_in(test, ctx, &mut inst, &mut ti, "PLBC405ICUADDRACK");
    make_in(test, ctx, &mut inst, &mut ti, "PLBC405ICUBUSY");
    make_in(test, ctx, &mut inst, &mut ti, "PLBC405ICUERR");
    make_in(test, ctx, &mut inst, &mut ti, "PLBC405ICURDDACK");
    make_ins(test, ctx, &mut inst, &mut ti, "PLBC405ICURDDBUS", 0, 63);
    make_ins(test, ctx, &mut inst, &mut ti, "PLBC405ICURDWDADDR", 1, 3);
    make_in(test, ctx, &mut inst, &mut ti, "PLBC405ICUSSIZE1");

    make_in_inv(test, ctx, &mut inst, &mut ti, "PLBCLK");

    // RST
    make_out(test, ctx, &mut inst, &mut ti, "C405RSTCHIPRESETREQ");
    make_out(test, ctx, &mut inst, &mut ti, "C405RSTCORERESETREQ");
    make_out(test, ctx, &mut inst, &mut ti, "C405RSTSYSRESETREQ");
    if !is_adv {
        make_in_inv(test, ctx, &mut inst, &mut ti, "RSTC405RESETCHIP");
        make_in_inv(test, ctx, &mut inst, &mut ti, "RSTC405RESETCORE");
        make_in_inv(test, ctx, &mut inst, &mut ti, "RSTC405RESETSYS");
    } else {
        make_in(test, ctx, &mut inst, &mut ti, "RSTC405RESETCHIP");
        make_in(test, ctx, &mut inst, &mut ti, "RSTC405RESETCORE");
        make_in(test, ctx, &mut inst, &mut ti, "RSTC405RESETSYS");
    }
    if !is_adv {
        ti.pin_tie("TSTRESETCHIPI", false);
        ti.pin_tie("TSTRESETCOREI", false);
        ti.pin_tie("TSTRESETSYSI", false);
    }

    // TRC
    make_out(test, ctx, &mut inst, &mut ti, "C405TRCCYCLE");
    make_outs(test, ctx, &mut inst, &mut ti, "C405TRCEVENEXECUTIONSTATUS", 0, 1);
    make_outs(test, ctx, &mut inst, &mut ti, "C405TRCODDEXECUTIONSTATUS", 0, 1);
    make_outs(test, ctx, &mut inst, &mut ti, "C405TRCTRACESTATUS", 0, 3);
    make_out(test, ctx, &mut inst, &mut ti, "C405TRCTRIGGEREVENTOUT");
    make_outs(test, ctx, &mut inst, &mut ti, "C405TRCTRIGGEREVENTTYPE", 0, 10);
    make_in(test, ctx, &mut inst, &mut ti, "TRCC405TRACEDISABLE");
    make_in(test, ctx, &mut inst, &mut ti, "TRCC405TRIGGEREVENTIN");

    // EIC
    make_in(test, ctx, &mut inst, &mut ti, "EICC405CRITINPUTIRQ");
    make_in(test, ctx, &mut inst, &mut ti, "EICC405EXTINPUTIRQ");

    // APU
    if !is_adv {
        ti.pin_tie("APUC405DCDAPUOP", false);
        ti.pin_tie("APUC405DCDCREN", false);
        ti.pin_tie("APUC405DCDFORCEALGN", false);
        ti.pin_tie("APUC405DCDFORCEBESTEERING", false);
        ti.pin_tie("APUC405DCDFPUOP", false);
        ti.pin_tie("APUC405DCDGPRWRITE", false);
        ti.pin_tie("APUC405DCDLDSTBYTE", false);
        ti.pin_tie("APUC405DCDLDSTDW", false);
        ti.pin_tie("APUC405DCDLDSTHW", false);
        ti.pin_tie("APUC405DCDLDSTQW", false);
        ti.pin_tie("APUC405DCDLDSTWD", false);
        ti.pin_tie("APUC405DCDLOAD", false);
        ti.pin_tie("APUC405DCDPRIVOP", false);
        ti.pin_tie("APUC405DCDRAEN", false);
        ti.pin_tie("APUC405DCDRBEN", false);
        ti.pin_tie("APUC405DCDSTORE", false);
        ti.pin_tie("APUC405DCDTRAPBE", false);
        ti.pin_tie("APUC405DCDTRAPLE", false);
        ti.pin_tie("APUC405DCDUPDATE", false);
        ti.pin_tie("APUC405DCDVALIDOP", false);
        ti.pin_tie("APUC405DCDXERCAEN", false);
        ti.pin_tie("APUC405DCDXEROVEN", false);
        ti.pin_tie("APUC405EXCEPTION", false);
        ti.pin_tie("APUC405EXEBLOCKINGMCO", false);
        ti.pin_tie("APUC405EXEBUSY", false);
        for i in 0..4 {
            ti.pin_tie(&format!("APUC405EXECR{i}"), false);
        }
        for i in 0..3 {
            ti.pin_tie(&format!("APUC405EXECRFIELD{i}"), false);
        }
        ti.pin_tie("APUC405EXELDDEPEND", false);
        ti.pin_tie("APUC405EXENONBLOCKINGMCO", false);
        for i in 0..32 {
            ti.pin_tie(&format!("APUC405EXERESULT{i}"), false);
        }
        ti.pin_tie("APUC405EXEXERCA", false);
        ti.pin_tie("APUC405EXEXEROV", false);
        ti.pin_tie("APUC405FPUEXCEPTION", false);
        ti.pin_tie("APUC405LWBLDDEPEND", false);
        ti.pin_tie("APUC405WBLDDEPEND", false);
        ti.pin_tie("APUC405SLEEPREQ", true);
        ti.pin_tie_inv("TIEC405APUDIVEN", true, true);
        ti.pin_tie_inv("TIEC405APUPRESENT", true, true);
    } else {
        make_out(test, ctx, &mut inst, &mut ti, "APUFCMDECODED");
        make_out(test, ctx, &mut inst, &mut ti, "APUFCMDECUDIVALID");
        make_out(test, ctx, &mut inst, &mut ti, "APUFCMENDIAN");
        make_out(test, ctx, &mut inst, &mut ti, "APUFCMFLUSH");
        make_out(test, ctx, &mut inst, &mut ti, "APUFCMINSTRVALID");
        make_out(test, ctx, &mut inst, &mut ti, "APUFCMLOADDVALID");
        make_out(test, ctx, &mut inst, &mut ti, "APUFCMOPERANDVALID");
        make_out(test, ctx, &mut inst, &mut ti, "APUFCMWRITEBACKOK");
        make_out(test, ctx, &mut inst, &mut ti, "APUFCMXERCA");
        make_outs(test, ctx, &mut inst, &mut ti, "APUFCMDECUDI", 0, 2);
        make_outs(test, ctx, &mut inst, &mut ti, "APUFCMINSTRUCTION", 0, 31);
        make_outs(test, ctx, &mut inst, &mut ti, "APUFCMLOADDATA", 0, 31);
        make_outs(test, ctx, &mut inst, &mut ti, "APUFCMRADATA", 0, 31);
        make_outs(test, ctx, &mut inst, &mut ti, "APUFCMRBDATA", 0, 31);
        make_outs(test, ctx, &mut inst, &mut ti, "APUFCMLOADBYTEEN", 0, 3);
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDCREN");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDFORCEALIGN");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDFORCEBESTEERING");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDFPUOP");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDGPRWRITE");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDLDSTBYTE");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDLDSTDW");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDLDSTHW");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDLDSTQW");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDLDSTWD");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDLOAD");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDPRIVOP");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDRAEN");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDRBEN");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDSTORE");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDTRAPBE");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDTRAPLE");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDUPDATE");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDXERCAEN");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDCDXEROVEN");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDECODEBUSY");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUDONE");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUEXCEPTION");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUEXEBLOCKINGMCO");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUEXENONBLOCKINGMCO");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUINSTRACK");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPULOADWAIT");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPURESULTVALID");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUSLEEPNOTREADY");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUXERCA");
        make_in(test, ctx, &mut inst, &mut ti, "FCMAPUXEROV");
        make_ins(test, ctx, &mut inst, &mut ti, "FCMAPUEXECRFIELD", 0, 2);
        make_ins(test, ctx, &mut inst, &mut ti, "FCMAPURESULT", 0, 31);
        make_ins(test, ctx, &mut inst, &mut ti, "FCMAPUCR", 0, 3);
        make_ins_inv(test, ctx, &mut inst, &mut ti, "TIEAPUCONTROL", 0, 15);
        make_ins_inv(test, ctx, &mut inst, &mut ti, "TIEAPUUDI1", 0, 23);
        make_ins_inv(test, ctx, &mut inst, &mut ti, "TIEAPUUDI2", 0, 23);
        make_ins_inv(test, ctx, &mut inst, &mut ti, "TIEAPUUDI3", 0, 23);
        make_ins_inv(test, ctx, &mut inst, &mut ti, "TIEAPUUDI4", 0, 23);
        make_ins_inv(test, ctx, &mut inst, &mut ti, "TIEAPUUDI5", 0, 23);
        make_ins_inv(test, ctx, &mut inst, &mut ti, "TIEAPUUDI6", 0, 23);
        make_ins_inv(test, ctx, &mut inst, &mut ti, "TIEAPUUDI7", 0, 23);
        make_ins_inv(test, ctx, &mut inst, &mut ti, "TIEAPUUDI8", 0, 23);
    }

    // LSSD
    if !is_adv {
        ti.pin_tie("LSSDC405ACLK", false);
        ti.pin_tie("LSSDC405ARRAYCCLKNEG", false);
        ti.pin_tie("LSSDC405CNTLPOINT", false);
        ti.pin_tie("LSSDC405SCANGATE", false);
        for i in 0..10 {
            ti.pin_tie(&format!("LSSDC405SCANIN{i}"), false);
        }
        ti.pin_tie("LSSDC405TESTEVS", false);
        ti.pin_tie("LSSDC405TESTM1", false);
        ti.pin_tie("LSSDC405TESTM3", false);
        ti.pin_tie("LSSDC405BCLK", true);
        ti.pin_tie("LSSDC405BISTCCLK", true);
    }

    // misc
    make_out(test, ctx, &mut inst, &mut ti, "C405XXXMACHINECHECK");

    make_ins_inv(test, ctx, &mut inst, &mut ti, "DSARCVALUE", 0, 7);
    make_ins_inv(test, ctx, &mut inst, &mut ti, "ISARCVALUE", 0, 7);
    if !is_adv {
        {
            let mut w = Vec::new();
            for i in 0..4 {
                let w_v = test.make_in(ctx);
                ti.pin_in_inv(&format!("DSCNTLVALUE{ii}", ii = 7 - i), &w_v, false);
                w.push(w_v);
            }
            for i in 4..8 {
                let (w_v, w_x, w_inv) = test.make_in_inv(ctx);
                ti.pin_in_inv(&format!("DSCNTLVALUE{ii}", ii = 7 - i), &w_x, w_inv);
                w.push(w_v);
            }
            inst.connect_bus("DSCNTLVALUE", &w);
        }
        {
            let mut w = Vec::new();
            for i in 0..4 {
                let w_v = test.make_in(ctx);
                ti.pin_in(&format!("ISCNTLVALUE{ii}", ii = 7 - i), &w_v);
                w.push(w_v);
            }
            for i in 4..8 {
                let (w_v, w_x, w_inv) = test.make_in_inv(ctx);
                ti.pin_in_inv(&format!("ISCNTLVALUE{ii}", ii = 7 - i), &w_x, w_inv);
                w.push(w_v);
            }
            inst.connect_bus("ISCNTLVALUE", &w);
        }
    } else {
        make_ins_inv(test, ctx, &mut inst, &mut ti, "DSCNTLVALUE", 0, 7);
        make_ins_inv(test, ctx, &mut inst, &mut ti, "ISCNTLVALUE", 0, 7);
    }

    make_in_inv_fake(test, ctx, &mut inst, &mut ti, "MCBCPUCLKEN");
    make_in_inv_fake(test, ctx, &mut inst, &mut ti, "MCBJTAGEN");
    make_in_inv_fake(test, ctx, &mut inst, &mut ti, "MCBTIMEREN");
    make_in_inv_fake(test, ctx, &mut inst, &mut ti, "MCPPCRST");
    if !is_adv {
        ti.pin_tie("TSTCPUCLKI", false);
        ti.pin_tie("TSTCPUCLKENI", false);
        ti.pin_tie("TSTJTAGENI", false);
        ti.pin_tie("TSTTIMERENI", false);
    }

    make_in_inv(test, ctx, &mut inst, &mut ti, "TIEC405DETERMINISTICMULT");
    make_in_inv(test, ctx, &mut inst, &mut ti, "TIEC405DISOPERANDFWD");
    make_in_inv(test, ctx, &mut inst, &mut ti, "TIEC405MMUEN");

    if !is_adv {
        ti.pin_tie_inv("TESTSELI", true, false);
        for i in 0..32 {
            ti.pin_tie_inv(&format!("TIEC405PVR{i}"), true, !matches!(i, 2 | 15 | 20 | 26));
        }
        for i in 0..32 {
            ti.pin_tie(&format!("TSTRDDBUSI{i}"), false);
        }

        ti.pin_tie_inv("TIERAMTAP1", true, true);
        ti.pin_tie_inv("TIERAMTAP2", true, false);
        ti.pin_tie_inv("TIETAGTAP1", true, true);
        ti.pin_tie_inv("TIETAGTAP2", true, false);
        ti.pin_tie_inv("TIEUTLBTAP1", true, true);
        ti.pin_tie_inv("TIEUTLBTAP2", true, false);

        ti.pin_tie("TSTCLKINACTI", false);
        ti.pin_tie("TSTPLBSAMPLECYCLEI", false);
    } else {
        make_in_inv(test, ctx, &mut inst, &mut ti, "TIEPVRBIT8");
        make_in_inv(test, ctx, &mut inst, &mut ti, "TIEPVRBIT9");
        make_in_inv(test, ctx, &mut inst, &mut ti, "TIEPVRBIT10");
        make_in_inv(test, ctx, &mut inst, &mut ti, "TIEPVRBIT11");
        make_in_inv(test, ctx, &mut inst, &mut ti, "TIEPVRBIT28");
        make_in_inv(test, ctx, &mut inst, &mut ti, "TIEPVRBIT29");
        make_in_inv(test, ctx, &mut inst, &mut ti, "TIEPVRBIT30");
        make_in_inv(test, ctx, &mut inst, &mut ti, "TIEPVRBIT31");
    }

    test.src_insts.push(inst);
    test.tgt_insts.push(ti);
}

pub fn gen_emac(test: &mut Test, ctx: &mut TestGenCtx, mode: EmacMode) {
    make_emac(test, ctx, None, mode);
}
