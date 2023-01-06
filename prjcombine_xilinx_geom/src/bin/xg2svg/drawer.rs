use std::{collections::HashMap, io::Write};

struct Poly {
    cls: &'static str,
    points: Vec<(f64, f64)>,
}

pub struct Drawer {
    name: String,
    width: f64,
    height: f64,
    bel_polys: Vec<Poly>,
    bel_classes: HashMap<&'static str, String>,
}

impl Drawer {
    pub fn new(name: String, width: f64, height: f64) -> Drawer {
        Drawer {
            name,
            width,
            height,
            bel_polys: vec![],
            bel_classes: HashMap::new(),
        }
    }

    pub fn bel_poly(&mut self, points: Vec<(f64, f64)>, cls: &'static str) {
        self.bel_polys.push(Poly { points, cls })
    }

    pub fn bel_rect(&mut self, xl: f64, xr: f64, yb: f64, yt: f64, cls: &'static str) {
        self.bel_poly(vec![(xl, yb), (xl, yt), (xr, yt), (xr, yb)], cls);
    }

    pub fn bel_class(&mut self, cls: &'static str, color: impl Into<String>) {
        self.bel_classes.insert(cls, color.into());
    }

    pub fn emit(self, mut f: impl Write) -> Result<(), std::io::Error> {
        writeln!(f, "<html><head><title>{n}</title><style>", n = self.name)?;
        for (k, v) in self.bel_classes {
            writeln!(f, "  polygon.{k} {{ fill: {v}; stroke: black; }}")?;
        }
        writeln!(
            f,
            "</style></head><body><svg width=\"{w}\" height=\"{h}\">",
            w = self.width,
            h = self.height
        )?;
        for poly in self.bel_polys {
            write!(f, "<polygon class=\"{c}\" points=\"", c = poly.cls)?;
            for (x, y) in poly.points {
                let y = self.height - y;
                write!(f, "{x},{y} ")?;
            }
            writeln!(f, "\"/>")?;
        }
        writeln!(f, "</svg></body></html>")?;
        Ok(())
    }
}
