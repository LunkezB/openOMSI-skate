use glam::{DVec3, Vec3};
use omsi_sim::{collision::Obb, world_data::WorldData};

const RADIUS: f64 = 96.0;

#[derive(Clone, Copy)]
pub struct Frame {
    pub origin: DVec3,
}

impl Frame {
    pub fn to_skate(self, world: DVec3) -> [f32; 3] {
        let p = (world - self.origin).as_vec3();
        [p.x, p.z, -p.y]
    }
    pub fn to_world(self, skate: [f32; 3]) -> DVec3 {
        self.origin + DVec3::new(skate[0] as f64, -skate[2] as f64, skate[1] as f64)
    }
    pub fn heading(degrees: f64) -> f32 {
        (std::f64::consts::PI - degrees.to_radians()) as f32
    }
}

pub struct Collision {
    pub triangles: Vec<[[f32; 3]; 3]>,
    pub rails: Vec<Vec<[f32; 3]>>,
}

impl Collision {
    pub(crate) fn prepare_rails(mut self) -> Self {
        // The upstream detector is authored in inches and Z-up; this runs on the worker.
        let inches: Vec<_> = self
            .triangles
            .iter()
            .map(|t| t.map(|p| Vec3::new(p[0], -p[2], p[1]) / 0.0254))
            .collect();
        let (rails, _) = crate::rails::find(&inches);
        self.rails = rails
            .into_iter()
            .map(|r| {
                r.into_iter()
                    .map(|p| {
                        let p = p * 0.0254;
                        [p.x, p.z, -p.y]
                    })
                    .collect()
            })
            .collect();
        self
    }
}

pub fn collision(
    data: &WorldData,
    frame: Frame,
    centre: DVec3,
    vehicles: &[Obb],
) -> Result<Collision, String> {
    let tile = omsi_sim::world_data::tile_key(centre.x, centre.y);
    if !data.ground.terrains.contains_key(&tile) {
        return Err("Waiting for OMSI terrain around the skater".into());
    }
    let mut source = Vec::<[Vec3; 3]>::new();
    let mut push = |world: [DVec3; 3], upward: bool| {
        if world.iter().any(|p| !p.is_finite()) {
            return;
        }
        let lo = world.iter().copied().reduce(DVec3::min).unwrap();
        let hi = world.iter().copied().reduce(DVec3::max).unwrap();
        if centre.x < lo.x - RADIUS
            || centre.x > hi.x + RADIUS
            || centre.y < lo.y - RADIUS
            || centre.y > hi.y + RADIUS
        {
            return;
        }
        let mut t = world.map(|p| (p - frame.origin).as_vec3());
        let n = (t[1] - t[0]).cross(t[2] - t[0]);
        if n.length_squared() < 1e-10 {
            return;
        }
        if upward && n.z < 0.0 {
            t.swap(1, 2);
        }
        source.push(t);
    };
    let size = omsi_map::tile_size();
    for (&(tx, ty), terrain) in &data.ground.terrains {
        let offset = DVec3::new(tx as f64 * size, ty as f64 * size, 0.0);
        if (centre.x - (offset.x + size * 0.5)).abs() > RADIUS + size * 0.5
            || (centre.y - (offset.y + size * 0.5)).abs() > RADIUS + size * 0.5
        {
            continue;
        }
        let mesh = omsi_geometry::build_terrain_mesh(terrain);
        let surface = data.ground.surfaces.get(&(tx, ty));
        for tri in mesh.indices.chunks_exact(3) {
            let local = [tri[0], tri[1], tri[2]].map(|i| mesh.positions[i as usize]);
            let mid = (local[0] + local[1] + local[2]) / 3.0;
            if surface.is_some_and(|s| {
                s.cut_at(mid.x, mid.y, mid.z, omsi_sim::world_data::surface_flush())
            }) {
                continue;
            }
            push(local.map(|p| offset + p.as_dvec3()), true);
        }
    }
    for (&(tx, ty), surface) in &data.ground.surfaces {
        let offset = DVec3::new(tx as f64 * size, ty as f64 * size, 0.0);
        for tri in &surface.drive.tris {
            push(tri.map(|p| offset + p.as_dvec3()), true);
        }
    }
    for object in &data.lists.collision.meshes {
        if object.bounds.center.distance(centre.truncate()) > RADIUS + object.bounds.radius() {
            continue;
        }
        let (s, c) = object.heading.sin_cos();
        for tri in object.shape.triangles() {
            push(
                tri.map(|p| object.pos + DVec3::new(p.x * c + p.y * s, p.y * c - p.x * s, p.z)),
                false,
            );
        }
    }
    for b in data.lists.collision.boxes.iter().chain(vehicles) {
        if b.center.distance(centre.truncate()) > RADIUS + b.radius() {
            continue;
        }
        for tri in box_triangles(b) {
            push(tri, false);
        }
    }
    if source.is_empty() {
        return Err("OMSI collision around the skater is empty".into());
    }
    let convert = |p: Vec3| [p.x, p.z, -p.y];
    Ok(Collision {
        triangles: source.into_iter().map(|t| t.map(convert)).collect(),
        rails: Vec::new(),
    })
}

fn box_triangles(b: &Obb) -> Vec<[DVec3; 3]> {
    let corners = b.corners();
    let p: [DVec3; 8] =
        std::array::from_fn(|i| corners[i % 4].extend(if i < 4 { b.z0 } else { b.z1 }));
    let mut out = Vec::new();
    for face in [
        [0, 1, 2, 3],
        [4, 7, 6, 5],
        [0, 4, 5, 1],
        [1, 5, 6, 2],
        [2, 6, 7, 3],
        [3, 7, 4, 0],
    ] {
        for idx in [[0, 1, 2], [0, 2, 3]] {
            let mut t = idx.map(|i| p[face[i]]);
            let mid = (t[0] + t[1] + t[2]) / 3.0;
            let centre = b.center.extend((b.z0 + b.z1) * 0.5);
            if (t[1] - t[0]).cross(t[2] - t[0]).dot(mid - centre) < 0.0 {
                t.swap(1, 2);
            }
            out.push(t);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frame_preserves_metres_and_large_map_precision() {
        let f = Frame {
            origin: DVec3::new(4_000_000.0, 8_000_000.0, 127.0),
        };
        let p = f.origin + DVec3::new(0.125, 5.0, 1.75);
        assert_eq!(f.to_skate(p), [0.125, 1.75, -5.0]);
        assert_eq!(f.to_world(f.to_skate(p)), p);
        assert!((Frame::heading(0.0) - std::f32::consts::PI).abs() < 1e-6);
    }
    #[test]
    fn obstacle_faces_point_outward() {
        let b = Obb::point(DVec3::ZERO, 1.0);
        let tris = box_triangles(&b);
        assert_eq!(tris.len(), 12);
        for t in tris {
            assert!(
                (t[1] - t[0])
                    .cross(t[2] - t[0])
                    .dot((t[0] + t[1] + t[2]) / 3.0)
                    > 0.0
            );
        }
    }
    #[test]
    fn unloaded_tiles_are_rejected() {
        assert!(
            collision(
                &WorldData::default(),
                Frame {
                    origin: DVec3::ZERO
                },
                DVec3::ZERO,
                &[]
            )
            .is_err()
        );
    }
    #[test]
    fn streamed_road_and_terrain_keep_separate_height_layers() {
        let mut data = WorldData::default();
        let ground = std::sync::Arc::make_mut(&mut data.ground);
        ground
            .terrains
            .insert((0, 0), std::sync::Arc::new(omsi_map::Terrain::flat()));
        let mut surface = omsi_geometry::TileSurface::new(64);
        surface.drive.push([
            Vec3::new(0.0, 0.0, 3.0),
            Vec3::new(10.0, 0.0, 3.0),
            Vec3::new(0.0, 10.0, 3.0),
        ]);
        ground.surfaces.insert((0, 0), std::sync::Arc::new(surface));
        let frame = Frame {
            origin: DVec3::new(2.0, 2.0, 0.0),
        };
        let world = collision(&data, frame, frame.origin, &[]).unwrap();
        assert!(
            world
                .triangles
                .iter()
                .any(|t| t.iter().all(|p| p[1] == 0.0))
        );
        assert!(
            world
                .triangles
                .iter()
                .any(|t| t.iter().all(|p| p[1] == 3.0))
        );
        for t in &world.triangles {
            let [a, b, c] = t.map(Vec3::from_array);
            assert!((b - a).cross(c - a).y > 0.0);
        }
    }
    #[test]
    fn distant_obstacles_do_not_enter_the_local_world() {
        let mut data = WorldData::default();
        std::sync::Arc::make_mut(&mut data.ground)
            .terrains
            .insert((0, 0), std::sync::Arc::new(omsi_map::Terrain::flat()));
        let near = Obb::point(DVec3::new(5.0, 5.0, 2.0), 0.5);
        let far = Obb::point(DVec3::new(1000.0, 1000.0, 2.0), 0.5);
        let frame = Frame {
            origin: DVec3::ZERO,
        };
        let base = collision(&data, frame, DVec3::ZERO, &[]).unwrap();
        let added = collision(&data, frame, DVec3::ZERO, &[near, far]).unwrap();
        assert_eq!(added.triangles.len(), base.triangles.len() + 12);
    }
}
