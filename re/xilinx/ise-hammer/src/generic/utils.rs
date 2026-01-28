use prjcombine_entity::EntityBundleItemIndex;
use prjcombine_interconnect::db::{BelClassId, BelInputId};
use prjcombine_re_xilinx_geom::ExpandedDevice;

pub fn get_input_name(edev: &ExpandedDevice, bcid: BelClassId, pid: BelInputId) -> String {
    let bcls = &edev.db[bcid];
    let (name, idx) = bcls.inputs.key(pid);
    match idx {
        EntityBundleItemIndex::Single => name.to_string(),
        EntityBundleItemIndex::Array { index, .. } => {
            let idx = bcls.inputs[pid].indexing.phys_to_virt(index);
            format!("{name}{idx}")
        }
    }
}
