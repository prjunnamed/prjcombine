use prjcombine_interconnect::{db::RegionSlotId, regions};

regions![
    PCLK0, PCLK1, PCLK2, PCLK3, SCLK0, SCLK1, SCLK2, SCLK3, VSDCLK,
];

pub const PCLK: [RegionSlotId; 4] = [PCLK0, PCLK1, PCLK2, PCLK3];
pub const SCLK: [RegionSlotId; 4] = [SCLK0, SCLK1, SCLK2, SCLK3];
