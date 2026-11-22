//! Diagnostic raw-surface ray tests; deliberately exclude native margins,
//! welding, filtering and motion/contact resolution until those are verified.
use mw_data::collision::{CollisionBody, CollisionShape, CollisionWorld};

#[derive(Debug)]
pub struct SurfaceHit<'a> {
    pub body: &'a CollisionBody,
    pub shape: &'a CollisionShape,
    pub distance: f64,
    pub normal: [f64; 3],
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 { a.iter().zip(b).map(|(x,y)| x*y).sum() }
fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] { [a[0]-b[0], a[1]-b[1], a[2]-b[2]] }
fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]
}

/// Unit direction required, so the parameter is a distance. Inside a convex
/// body returns its exit surface. Meshes are double-sided for this diagnostic.
/// `source` selects one original resource, not an inferred collision filter.
pub fn raw_surface_ray<'a>(world: &'a CollisionWorld, origin: [f64; 3], direction: [f64; 3], max_distance: f64, source: Option<&str>) -> Result<Vec<SurfaceHit<'a>>, &'static str> {
    if !origin.iter().chain(&direction).all(|v| v.is_finite()) || !max_distance.is_finite()
        || max_distance < 0. || (dot(direction,direction)-1.).abs()>1e-8 { return Err("invalid diagnostic ray"); }
    let shapes: std::collections::HashMap<_,_> = world.shapes.iter().map(|s| (s.id.as_str(),s)).collect();
    let mut hits = Vec::new();
    for body in &world.bodies {
        if source.is_some_and(|s| body.source != s) { continue; }
        for id in &body.shapes {
            let Some(&shape) = shapes.get(id.as_str()) else { return Err("unvalidated collision world"); };
            if shape.original_triangles {
                for tri in &shape.triangles {
                    let [a,b,c] = tri.map(|i| shape.vertices[i]);
                    let e1=sub(b,a); let e2=sub(c,a); let p=cross(direction,e2); let det=dot(e1,p);
                    if det.abs()<1e-12 { continue; }
                    let t=sub(origin,a); let u=dot(t,p)/det;
                    if !(0. ..=1.).contains(&u) { continue; }
                    let q=cross(t,e1); let v=dot(direction,q)/det;
                    if v<0. || u+v>1. { continue; }
                    let distance=dot(e2,q)/det;
                    if distance<0. || distance>max_distance { continue; }
                    let n=cross(e1,e2); let length=dot(n,n).sqrt();
                    hits.push(SurfaceHit {body,shape,distance,normal:n.map(|x| x/length)});
                }
            } else {
                let mut enter=f64::NEG_INFINITY; let mut leave=f64::INFINITY;
                let mut enter_n=[0.;3]; let mut leave_n=[0.;3];
                for p in &shape.planes {
                    let n=[p[0],p[1],p[2]]; let denom=dot(n,direction); let offset=dot(n,origin)+p[3];
                    if denom.abs()<1e-12 { if offset>0. { enter=f64::INFINITY; break; } continue; }
                    let t=-offset/denom;
                    if denom<0. && t>enter { enter=t; enter_n=n; }
                    if denom>0. && t<leave { leave=t; leave_n=n; }
                }
                let (distance,normal)=if enter>=0. {(enter,enter_n)} else {(leave,leave_n)};
                if enter<=leave && distance>=0. && distance<=max_distance {
                    hits.push(SurfaceHit {body,shape,distance,normal});
                }
            }
        }
    }
    hits.sort_by(|a,b| a.distance.total_cmp(&b.distance));
    Ok(hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn staged_roof_matches_independent_havok_plane_probe() {
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native");
        let world=CollisionWorld::parse(&std::fs::read_to_string(root.join("collision/world.json")).unwrap()).unwrap();
        assert_eq!((world.bodies.len(),world.shapes.len()),(353,362));
        let arena:mw_data::Arena=serde_json::from_str(&std::fs::read_to_string(root.join("arena-meta.json")).unwrap()).unwrap();
        let position=arena.spawns.iter().find(|s|s.id==0x20440).unwrap().position.map(f64::from);
        let hits=raw_surface_ray(&world,position,[0.,-1.,0.],100.,Some("col.hkx")).unwrap();
        assert_eq!(hits[0].body.name,"cube3219_ST0c");
        assert!((position[1]-hits[0].distance-17.54882725887).abs()<1e-8);
        assert_eq!(hits[0].body.loader_category(),Some(0x16));
        assert!(raw_surface_ray(&world,position,[0.,-2.,0.],100.,None).is_err());
        assert!(raw_surface_ray(&world,position,[0.,-1.,0.],0.1,Some("col.hkx")).unwrap().is_empty());
        assert!(CollisionWorld::parse(&serde_json::json!({"bodies":[],"shapes":[],"failures":["truncated"]}).to_string()).is_err());
        let mut broken:serde_json::Value=serde_json::from_str(&std::fs::read_to_string(root.join("collision/world.json")).unwrap()).unwrap();
        broken["shapes"][0]["triangles"][0][0]=serde_json::json!(usize::MAX);
        assert!(CollisionWorld::parse(&broken.to_string()).is_err());
    }
    #[test]
    fn recorded_gate_side_contact_recovers_along_its_original_plane() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/native");
        let world = CollisionWorld::parse(&std::fs::read_to_string(root.join("collision/world.json")).unwrap()).unwrap();
        // Original live.jsonl frames 751 -> 752. cube3292's side plane is x=0.0623703.
        let pos = [0.656382442, 17.5988274, -58.9051361];
        let (step, contacts) = convex_contacts(&world, pos, [-0.0178, -0.0039558], 1. / 30.);
        assert!(contacts > 0);
        assert!((pos[0] + step[0] - 0.656582057).abs() < 0.00001);
        assert!((step[1] + 0.0039558).abs() < 0.00001);
    }
    #[test]
    fn gate_retains_a_contact_across_a_small_positive_gap() {
        let mut retained = ProxyContacts::default();
        let limit = GATE_FACE_Z - GATE_KEEP;
        let pos = [1.737,17.5988274,-59.2];
        // A new cast can enter its margin; it creates the retained plane.
        let (step, hits) = barrier(pos, [0.,0.6], 1./30., &mut retained);
        assert_eq!(hits, 1);
        assert!((pos[2] + step[1] - (GATE_FACE_Z - 0.45)).abs() < 1e-5);
        // Independent recording 640 -> 641: a roughly 0.001 positive gap must
        // not turn an existing gate plane into a new cast permitting 0.1 travel.
        let pos = [pos[0],pos[1],limit - 0.0011];
        let (step, _) = barrier(pos, [-0.008,0.1], 1./30., &mut retained);
        assert!((pos[2] + step[1] - limit).abs() < 1e-6);
        assert_eq!(step[0], -0.008);
        // Moving away is unrestricted and eventually removes the contact.
        let (step, _) = barrier(pos, [0.,-0.2], 1./30., &mut retained);
        assert_eq!(step, [0.,-0.2]);
        let far = [pos[0],pos[1],pos[2] + step[1]];
        barrier(far, [0.;2], 1./30., &mut retained);
        assert!(!retained.gate);
    }

    #[test]
    fn recorded_death_hit_rate_holds_then_recovers_exponentially() {
        let mut clock = ProxyClock::default();
        clock.death_hit();
        // Independent contact capture fight-contacts.jsonl, lethal frame 1418.
        let expected = [(1, 1.), (2, 0.001), (10, 0.001), (11, 0.003), (12, 0.00375),
                        (31, 0.260208517), (32, 0.325260639), (35, 0.800263226), (36, 1.)];
        for frame in 1..=36 {
            let actual = clock.animation_rate(false);
            if let Some((_, rate)) = expected.iter().find(|(f, _)| *f == frame) {
                assert!((actual - rate).abs() < 0.000001, "frame {frame}: {actual} != {rate}");
            }
            clock.advance(false);
        }
    }

    #[test]
    fn convex_parallel_inside_and_mesh_boundary_probes() {
        let mut world=CollisionWorld::parse(r#"{"bodies":[{"id":"b","name":"box","source":"col.hkx","filterInfo":0,"friction":0.5,"restitution":0,"shapes":["s"]}],"shapes":[{"id":"s","kind":"hkpConvexVerticesShape","radius":0.05,"vertices":[[-1,-1,-1],[1,1,1]],"triangles":[],"planes":[[1,0,0,-1],[-1,0,0,-1],[0,1,0,-1],[0,-1,0,-1],[0,0,1,-1],[0,0,-1,-1]],"originalTriangles":false}],"failures":[]}"#).unwrap();
        let h=raw_surface_ray(&world,[0.,3.,0.],[0.,-1.,0.],10.,None).unwrap();
        assert_eq!((h[0].distance,h[0].normal),(2.,[0.,1.,0.]));
        let h=raw_surface_ray(&world,[0.,0.,0.],[0.,-1.,0.],10.,None).unwrap();
        assert_eq!((h[0].distance,h[0].normal),(1.,[0.,-1.,0.]));
        assert!(raw_surface_ray(&world,[2.,3.,0.],[0.,-1.,0.],10.,None).unwrap().is_empty());
        let s=&mut world.shapes[0]; s.kind="hkpSimpleMeshShape".into(); s.original_triangles=true;
        s.planes.clear(); s.vertices=vec![[0.,0.,0.],[1.,0.,0.],[0.,0.,1.]]; s.triangles=vec![[0,1,2]];
        assert_eq!(raw_surface_ray(&world,[0.,1.,0.],[0.,-1.,0.],1.,None).unwrap()[0].distance,1.);
        assert_eq!(raw_surface_ray(&world,[0.,-1.,0.],[0.,1.,0.],1.,None).unwrap()[0].distance,1.);
        assert!(raw_surface_ray(&world,[1.,1.,1.],[0.,-1.,0.],10.,None).unwrap().is_empty());
    }
}


/// Wall sources the movement slides against (mw-play's presenter and the frame replay share this).
pub const WALL_SOURCES: [&str; 2] = ["col.hkx", "col_pl.hkx"];
/// Body radius for wall slides [inferred].
pub const BODY_RADIUS: f32 = 0.35;

/// cScene smooths the physics rate by halves/doubles (802150F0); animations may
/// still advance at one frame. The proxy simplex solver does not advance time
/// at or below 8058B0F0 = 0.0001 seconds (80466E20).
#[derive(Clone, Debug)]
pub struct ProxyClock {
    rate: f32,
    target: f32,
    hit_frames: u8,
    motion_rate: f32,
    was_demo: bool,
    death_hold: u8,
    death_ramp: bool,
}
impl Default for ProxyClock {
    fn default() -> Self { Self { rate: 1., target: 1., hit_frames: 0, motion_rate: 1., was_demo: false, death_hold: 0, death_ramp: false } }
}
impl ProxyClock {
    pub fn seconds(&self) -> f32 { self.rate * (1. / 30.) }
    pub fn hit(&mut self, frames: u8) { self.hit_frames = frames; self.target = 0.0003; self.death_ramp = false; }
    /// 802727B0 lethal hit: mode 4, 70 frames, 10-frame hold at 0.001.
    /// The hit frame itself has already consumed the first timer tick.
    pub fn death_hit(&mut self) { self.hit_frames = 69; self.death_hold = 9; self.target = 0.001; self.death_ramp = true; }
    pub fn animation_rate(&self, demo_active: bool) -> f32 { if demo_active { 1. } else { self.motion_rate } }
    pub fn advance(&mut self, demo_active: bool) {
        self.rate = if self.rate > self.target { (self.rate * 0.5).max(self.target) }
                    else { (self.rate * 2.).min(self.target) };
        self.motion_rate = self.target;
        // 80279D10 consumes the timer before requesting the ordinary rate on the
        // following update. During the intro getSpeedRate's override already
        // returns 1, so its "if != 1" reset leaves the actual request latched.
        if self.death_ramp {
            if self.hit_frames > 0 {
                self.hit_frames -= 1;
                if self.death_hold > 0 {
                    self.death_hold -= 1;
                    if self.death_hold == 0 { self.target = 0.003; }
                } else { self.target = (self.target * if self.target >= 0.3 { 1.35 } else { 1.25 }).min(1.); }
            } else { self.target = 1.; self.death_ramp = false; }
        } else if self.hit_frames > 0 { self.hit_frames -= 1; }
        else if !demo_active { self.target = 1.; }
        if self.was_demo && !demo_active { self.motion_rate = self.target; }
        self.was_demo = demo_active;
    }
}

pub fn proxy_advances(seconds: f32) -> bool { seconds > 0.0001 }

/// Floor height under `pos` (nearest col.hkx surface below a point 1.5 above it).
pub fn floor(world: &CollisionWorld, pos: [f32; 3]) -> Option<f32> {
    let o = [pos[0] as f64, pos[1] as f64 + 1.5, pos[2] as f64];
    raw_surface_ray(world, o, [0., -1., 0.], 6., Some("col.hkx")).ok()
        .and_then(|hits| hits.iter().map(|h| h.distance).min_by(f64::total_cmp).map(|d| (o[1] - d) as f32))
}

/// Slide a horizontal step along collision walls (rays at knee and chest height); returns the step and the number of
/// wall contacts. Not the native character controller (its margins and resolution are not translated) [inferred].
pub fn slide(world: &CollisionWorld, pos: [f32; 3], step: [f32; 2]) -> ([f32; 2], u32) {
    let (mut sx, mut sz) = (step[0], step[1]);
    let mut contacts = 0;
    if (sx * sx + sz * sz).sqrt() < 1e-6 { return (step, 0); }
    for h in [0.5f32, 1.2] {
        let len = (sx * sx + sz * sz).sqrt();
        if len < 1e-9 { break; }
        let (dx, dz) = (sx as f64, sz as f64); let l = (dx * dx + dz * dz).sqrt();   // unit length in f64
        let o = [pos[0] as f64, (pos[1] + h) as f64, pos[2] as f64];
        let mut hits = Vec::new();
        for src in WALL_SOURCES {
            if let Ok(hs) = raw_surface_ray(world, o, [dx / l, 0., dz / l], (len + BODY_RADIUS) as f64, Some(src)) { hits.extend(hs); }
        }
        hits.sort_by(|a, b| a.distance.total_cmp(&b.distance));
        if let Some(hit) = hits.first() {
            let (mut nx, mut nz) = (hit.normal[0] as f32, hit.normal[2] as f32);
            let nl = (nx * nx + nz * nz).sqrt(); if nl < 1e-6 { continue; }
            nx /= nl; nz /= nl;
            if nx * sx + nz * sz > 0. { nx = -nx; nz = -nz; }               // mesh winding is not consistent
            contacts += 1;
            let into = sx * nx + sz * nz; sx -= nx * into; sz -= nz * into;   // remove the into-wall component
            let gap = hit.distance as f32 - BODY_RADIUS; if gap < 0. { sx -= nx * gap; sz -= nz * gap; }
        }
    }
    ([sx, sz], contacts)
}

/// The gate behind the player spawn: ObjTbl.obt places bl0510_N001_g01 at (2.5, 17.55, -58.24), yaw 0; bl0510's
/// col.hkx is one keyframed rigid body with an hkpBoxShape (half extents 2.4, 3.5, 0.05, convex radius 0.05) under an
/// hkpConvexTranslateShape (0, 3.5, 0) [native data]. So a wall over x 0.1..4.9, y 17.55..24.55 whose face toward
/// the arena is at z = -58.24 - 0.05 - 0.05. The player's character proxy is a capsule of radius 0.4, height 1.6
/// (802705DC -> cHavokCharacterCollision::create 800B2500 with 80569F04 / 80569F68) [native]; in the frame
/// recordings bodies stop 0.55 in front of the face (0.4 + 0.15: the proxy's keep distance [inferred, fitted]).
/// This object's collision is not in the staged static world (its scene data does not parse with the Havok adapter;
/// the box was read from the packfile directly).
pub const GATE_X: [f32; 2] = [2.5 - 2.4, 2.5 + 2.4];
pub const GATE_FACE_Z: f32 = -58.24 - 0.1;
pub const GATE_Y: [f32; 2] = [17.55, 17.55 + 7.0];
pub const GATE_KEEP: f32 = 0.55;
// cinfo +0x28 (800B28F0, constant 80535E60), copied to proxy +0x5C.
pub const CONTACT_TOLERANCE: f32 = 0.1;

/// Bounded retained-contact state for the separately staged gate box. This is
/// not the full Havok manifold (normal matching, other bodies and simplex remain open).
#[derive(Clone, Default, Debug)]
pub struct ProxyContacts { gate: bool }

fn barrier(pos: [f32; 3], mut s: [f32; 2], seconds: f32, contacts: &mut ProxyContacts) -> ([f32; 2], u32) {
    // only bodies on the arena side, level with the gate and within its width (plus the body radius)
    if pos[1] < GATE_Y[0] - 1. || pos[1] > GATE_Y[1] || pos[2] > GATE_FACE_Z
        || pos[0] < GATE_X[0] - GATE_KEEP || pos[0] > GATE_X[1] + GATE_KEEP {
        contacts.gate = false; return (s, 0);
    }
    let limit = GATE_FACE_Z - GATE_KEEP;
    if pos[2] < limit - CONTACT_TOLERANCE { contacts.gate = false; }
    if contacts.gate || pos[2] > limit {
        contacts.gate = true;
        // 80446914 queries contacts out to keepDistance + keepContactTolerance.
        // A retained positive-gap plane permits just the gap (80447FA0).
        // Penetrating planes recover by depth * timestep.
        let gap = limit - pos[2];
        // The native proxy recovers penetration at 1 unit/second times the
        // penetration depth (80447FA0), rather than teleporting to keepDistance.
        s[1] = s[1].min(if gap < 0. { gap * seconds } else { gap });
        return (s, 1);
    }
    // An unmatched new cast still has its separate margin. 804489B4 matches
    // old planes before adding only the closest new query contacts/cast hit.
    let cast_limit = GATE_FACE_Z - 0.4 - 0.05;
    if pos[2] + s[1] > cast_limit && s[1] > 0. {
        contacts.gate = true;
        s[1] = (cast_limit - pos[2]).max(0.); return (s, 1);
    }
    (s, 0)
}

/// Convex capsule contacts use the original planes and shape radius. The old
/// ray-only slide missed side faces when movement was almost parallel to them.
/// col_obj participation is supported by the recorded cube3292 side contact;
/// full native broadphase/filter parity remains to be translated.
fn convex_contacts(world: &CollisionWorld, pos: [f32; 3], mut step: [f32; 2], seconds: f32) -> ([f32; 2], u32) {
    let mut contacts = 0;
    for body in &world.bodies {
        if !matches!(body.source.as_str(), "col.hkx" | "col_pl.hkx" | "col_obj.hkx") { continue; }
        for id in &body.shapes {
            let Some(shape) = world.shapes.iter().find(|s| &s.id == id) else { continue; };
            if shape.original_triangles || shape.planes.is_empty() { continue; }
            let mut nearest = (f32::NEG_INFINITY, [0.; 3]);
            let mut inside = true;
            for plane in &shape.planes {
                let n = [plane[0] as f32, plane[1] as f32, plane[2] as f32];
                let distance = n[0] * pos[0] + n[1] * (pos[1] + 0.8) + n[2] * pos[2] + plane[3] as f32
                    - (0.4 + 0.4 * n[1].abs() + 0.15 + shape.radius as f32);
                if distance > 0. { inside = false; break; }
                if distance > nearest.0 { nearest = (distance, n); }
            }
            if !inside || nearest.1[1].abs() > 0.7 { continue; }
            let (depth, n) = nearest;
            let into = step[0] * n[0] + step[1] * n[2];
            let recovery = -depth * seconds;
            if into < recovery {
                step[0] += n[0] * (recovery - into);
                step[1] += n[2] * (recovery - into);
                contacts += 1;
            }
        }
    }
    (step, contacts)
}

/// mw-play's movement for one actor: slide, refuse steps off the floor, snap to the floor unless airborne.
pub fn move_on_floor(world: &CollisionWorld, pos: [f32; 3], step: [f32; 2], airborne: bool) -> ([f32; 3], u32) {
    move_on_floor_timed(world, pos, step, airborne, 1. / 30.)
}

pub fn move_on_floor_timed(world: &CollisionWorld, pos: [f32; 3], step: [f32; 2], airborne: bool, seconds: f32) -> ([f32; 3], u32) {
    move_on_floor_contacts(world, pos, step, airborne, seconds, &mut ProxyContacts::default())
}

pub fn move_on_floor_contacts(world: &CollisionWorld, pos: [f32; 3], step: [f32; 2], airborne: bool, seconds: f32, retained: &mut ProxyContacts) -> ([f32; 3], u32) {
    let (s, mut contacts) = slide(world, pos, step);
    let (s, c) = convex_contacts(world, pos, s, if proxy_advances(seconds) { seconds } else { 0. }); contacts += c;
    let (mut s, c) = barrier(pos, s, if proxy_advances(seconds) { seconds } else { 0. }, retained); contacts += c;
    if s != [0.; 2] && floor(world, [pos[0] + s[0], pos[1], pos[2] + s[1]]).is_none() { s = [0.; 2]; contacts += 1; }
    let mut next = [pos[0] + s[0], pos[1], pos[2] + s[1]];
    // keep the actor's current height above the col.hkx floor (the original stands the player 0.05 and RinRin 0.1
    // above it in the frame recording [observed])
    if !airborne { if let (Some(y), Some(y0)) = (floor(world, next), floor(world, pos)) { next[1] = y + (pos[1] - y0); } }
    (next, contacts)
}
