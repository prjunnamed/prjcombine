use std::ops::{Deref, DerefMut};

use prjcombine_re_fpga_hammer::Collector;
use prjcombine_re_xilinx_xact_geom::Device;
use prjcombine_xc2000::expanded::ExpandedDevice;

pub struct CollectorCtx<'a, 'b>
where
    'b: 'a,
{
    pub collector: Collector<'b, 'a>,
    pub device: &'a Device,
    pub edev: &'a ExpandedDevice<'a>,
}

impl<'a, 'b> Deref for CollectorCtx<'a, 'b> {
    type Target = Collector<'b, 'a>;

    fn deref(&self) -> &Self::Target {
        &self.collector
    }
}

impl DerefMut for CollectorCtx<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.collector
    }
}

impl CollectorCtx<'_, '_> {
    pub fn has_tile(&self, tile: &str) -> bool {
        let tcid = self.edev.db.get_tile_class(tile);
        !self.edev.tile_index[tcid].is_empty()
    }
}
