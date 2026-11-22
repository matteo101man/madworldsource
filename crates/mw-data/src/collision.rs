//! Original static Havok geometry. Loader categories are from DOL 0x802140e4.
//! These data do not by themselves reproduce Havok contact or filter behavior.
use serde::Deserialize;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Debug, Deserialize)]
pub struct CollisionBody {
    pub id: String,
    pub source: String,
    pub name: String,
    #[serde(rename = "filterInfo")]
    pub filter_info: u32,
    pub friction: f64,
    pub restitution: f64,
    pub shapes: Vec<String>,
}

impl CollisionBody {
    /// Original loader argument; its filter semantics are still unresolved.
    pub fn loader_category(&self) -> Option<u32> {
        match self.source.as_str() {
            "col.hkx" => Some(0x16), "col_cm.hkx" => Some(0x11),
            "col_ef.hkx" => Some(0x15), "col_obj.hkx" => Some(0x14),
            "col_pl.hkx" => Some(0x13), _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct CollisionShape {
    pub id: String,
    pub kind: String,
    pub radius: f64,
    pub vertices: Vec<[f64; 3]>,
    pub triangles: Vec<[usize; 3]>,
    pub planes: Vec<[f64; 4]>,
    #[serde(rename = "originalTriangles")]
    pub original_triangles: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CollisionWorld {
    pub bodies: Vec<CollisionBody>,
    pub shapes: Vec<CollisionShape>,
    pub failures: Vec<serde_json::Value>,
}

impl CollisionWorld {
    pub fn parse(text: &str) -> Result<Self, String> {
        let world: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if !world.failures.is_empty() { return Err("incomplete native collision staging".into()); }
        let mut shapes = HashMap::new();
        for s in &world.shapes {
            if shapes.insert(&s.id, s).is_some() { return Err("duplicate collision shape".into()); }
            if !s.radius.is_finite() || s.radius < 0. || s.vertices.is_empty()
                || !s.vertices.iter().flatten().chain(s.planes.iter().flatten()).all(|v| v.is_finite())
                || s.triangles.iter().flatten().any(|&i| i >= s.vertices.len()) {
                return Err(format!("{}: invalid collision geometry", s.id));
            }
            match s.kind.as_str() {
                "hkpConvexVerticesShape" if !s.original_triangles && !s.planes.is_empty() => {
                    if s.planes.iter().any(|p| ((p[0]*p[0]+p[1]*p[1]+p[2]*p[2]).sqrt()-1.).abs()>1e-5) {
                        return Err(format!("{}: non-unit plane", s.id));
                    }
                }
                "hkpSimpleMeshShape" if s.original_triangles && s.planes.is_empty() => {}
                _ => return Err(format!("{}: unsupported collision shape layout", s.id)),
            }
        }
        let mut bodies = HashSet::new();
        let mut owned = HashSet::new();
        for b in &world.bodies {
            if !bodies.insert(&b.id) || b.loader_category().is_none() || b.shapes.is_empty()
                || !b.friction.is_finite() || !b.restitution.is_finite() {
                return Err(format!("{}: invalid native collision body", b.id));
            }
            for id in &b.shapes {
                if !shapes.contains_key(id) || !owned.insert(id) { return Err("missing/shared collision leaf".into()); }
            }
        }
        if owned.len() != shapes.len() { return Err("unowned collision leaf".into()); }
        Ok(world)
    }
}
