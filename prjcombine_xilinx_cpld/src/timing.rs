use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Timing {
    pub del_ibuf_imux: Option<i64>,
    pub del_ibuf_fclk: Option<i64>,
    pub del_ibuf_foe: Option<i64>,
    pub del_ibuf_fsr: Option<i64>,
    pub del_ibuf_d: Option<i64>,
    pub del_uim_imux: Option<i64>,
    pub del_fbk_imux: Option<i64>,
    pub del_mc_foe: Option<i64>,

    pub del_imux_d_hp: Option<i64>,
    pub del_imux_d_lp: Option<i64>,
    pub del_exp_exp: Option<i64>,
    pub del_exp_d: Option<i64>,
    pub del_imux_pt_clk: Option<i64>,
    pub del_imux_pt_oe: Option<i64>,
    pub del_imux_pt_sr: Option<i64>,
    pub del_imux_pt_ce: Option<i64>,

    pub del_imux_pt: Option<i64>,
    pub del_imux_or: Option<i64>,
    pub del_imux_fbn: Option<i64>,
    pub del_imux_ct: Option<i64>,
    pub del_pt_ut: Option<i64>,

    pub del_d_q_comb: Option<i64>,
    pub del_d_q_latch: Option<i64>,
    pub setup_d_clk: Option<i64>,
    pub hold_d_clk: Option<i64>,
    pub setup_d_clk_pt_pt: Option<i64>,
    pub setup_d_clk_pt_fclk: Option<i64>,
    pub setup_d_clk_ibuf_pt: Option<i64>,
    pub setup_d_clk_ibuf_fclk: Option<i64>,
    pub hold_d_clk_pt_pt: Option<i64>,
    pub hold_d_clk_pt_fclk: Option<i64>,
    pub hold_d_clk_ibuf_pt: Option<i64>,
    pub hold_d_clk_ibuf_fclk: Option<i64>,
    pub del_clk_q: Option<i64>,
    pub width_clk: Option<i64>,
    pub width_clk_pt: Option<i64>,
    pub del_sr_q: Option<i64>,
    pub width_sr: Option<i64>,
    pub recovery_sr_clk: Option<i64>,
    pub setup_ce_clk: Option<i64>,
    pub hold_ce_clk: Option<i64>,

    pub del_ibuf_dge: Option<i64>,
    pub setup_ibuf_dge: Option<i64>,
    pub hold_ibuf_dge: Option<i64>,
    pub width_ibuf_dge: Option<i64>,

    pub del_obuf_fast: Option<i64>,
    pub del_obuf_slow: Option<i64>,
    pub del_obuf_oe: Option<i64>,

    pub setup_cd_rst: Option<i64>,
    pub hold_cd_rst: Option<i64>,

    pub iostd: HashMap<String, IostdTiming>,
}

#[derive(Debug, Default)]
pub struct IostdTiming {
    pub del_ibuf_plain: Option<i64>,
    pub del_ibuf_schmitt: Option<i64>,
    pub del_obuf_fast: Option<i64>,
    pub del_obuf_slow: Option<i64>,
}
