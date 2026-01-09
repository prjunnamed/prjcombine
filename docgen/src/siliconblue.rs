use prjcombine_entity::EntityPartVec;

use crate::{
    DocgenContext,
    interconnect::gen_intdb,
    speed::{SpeedData, gen_speed},
};

pub fn gen_siliconblue(ctx: &mut DocgenContext) {
    let db = prjcombine_siliconblue::db::Database::from_file(
        ctx.ctx.root.join("../databases/siliconblue.zstd"),
    )
    .unwrap();
    gen_intdb(ctx, "siliconblue", &db.int);
    let mut speeds = EntityPartVec::new();
    for part in &db.devices {
        for (sname, &speedid) in &part.speeds {
            let speed = &db.speeds[speedid];
            if !speeds.contains_id(speedid) {
                speeds.insert(
                    speedid,
                    SpeedData {
                        names: vec![],
                        speed,
                    },
                );
            }
            speeds[speedid]
                .names
                .push(format!("{pname}{sname}", pname = part.name));
        }
    }
    gen_speed(ctx, "siliconblue", &Vec::from_iter(speeds.into_values()));
}
