use glam::{DVec3, Mat4, Vec2, Vec3, Vec4};
use omsi_geometry::MeshData;
use omsi_render::{AlphaMode, Renderer, Scene};
use std::path::Path;

struct Skin {
    names: Vec<String>,
    inverse: Vec<Mat4>,
}
struct Part {
    mesh: MeshData,
    joints: Vec<[u16; 4]>,
    weights: Vec<[f32; 4]>,
    skin: usize,
    colour: [f32; 4],
    image: Option<usize>,
    alpha: AlphaMode,
    gpu: Option<(usize, usize, usize)>,
}
pub(super) struct Visual {
    skins: Vec<Skin>,
    parts: Vec<Part>,
    images: Vec<omsi_texture::Image>,
    textures: Vec<usize>,
}

impl Visual {
    pub fn load(path: &Path) -> Result<Self, String> {
        let glb = gltf::Gltf::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let bytes = glb
            .blob
            .as_ref()
            .ok_or("Skater model must have an embedded GLB buffer")?;
        if glb
            .buffers()
            .any(|b| !matches!(b.source(), gltf::buffer::Source::Bin))
        {
            return Err("External skater buffers are unsupported".into());
        }
        let read_buffer = |_: gltf::Buffer<'_>| Some(bytes.as_slice());
        let mut skins = Vec::new();
        for skin in glb.skins() {
            let names: Result<Vec<_>, _> = skin
                .joints()
                .map(|j| {
                    j.name()
                        .map(str::to_owned)
                        .ok_or("Skater joint has no name")
                })
                .collect();
            let names = names?;
            let inverse = skin
                .reader(read_buffer)
                .read_inverse_bind_matrices()
                .map(|m| m.map(|m| Mat4::from_cols_array_2d(&m)).collect())
                .unwrap_or_else(|| vec![Mat4::IDENTITY; names.len()]);
            if inverse.len() != names.len() {
                return Err("Skater inverse bind count differs from its joints".into());
            }
            skins.push(Skin { names, inverse });
        }
        let mut images = Vec::new();
        for image in glb.images() {
            let gltf::image::Source::View { view, .. } = image.source() else {
                return Err("External skater images are unsupported".into());
            };
            let range = view.offset()..view.offset() + view.length();
            let img =
                image::load_from_memory(bytes.get(range).ok_or("Skater image exceeds GLB buffer")?)
                    .map_err(|e| e.to_string())?
                    .into_rgba8();
            images.push(omsi_texture::Image {
                width: img.width(),
                height: img.height(),
                rgba: img.into_raw(),
                has_alpha: true,
            });
        }
        let mut parts = Vec::new();
        for node in glb.nodes() {
            let Some(mesh) = node.mesh() else {
                continue;
            };
            let skin = node
                .skin()
                .ok_or("Unskinned skater mesh is unsupported")?
                .index();
            for primitive in mesh.primitives() {
                if primitive.mode() != gltf::mesh::Mode::Triangles {
                    return Err("Skater mesh must contain triangles".into());
                }
                let reader = primitive.reader(read_buffer);
                let positions: Vec<_> = reader
                    .read_positions()
                    .ok_or("Skater positions missing")?
                    .map(Vec3::from_array)
                    .collect();
                let normals = reader
                    .read_normals()
                    .ok_or("Skater normals missing")?
                    .map(Vec3::from_array)
                    .collect::<Vec<_>>();
                let uvs = reader
                    .read_tex_coords(0)
                    .map(|v| v.into_f32().map(Vec2::from_array).collect())
                    .unwrap_or_else(|| vec![Vec2::ZERO; positions.len()]);
                let joints = reader
                    .read_joints(0)
                    .ok_or("Skater joints missing")?
                    .into_u16()
                    .collect::<Vec<_>>();
                let weights = reader
                    .read_weights(0)
                    .ok_or("Skater weights missing")?
                    .into_f32()
                    .collect::<Vec<_>>();
                let indices: Vec<u32> = reader
                    .read_indices()
                    .map(|v| v.into_u32().collect())
                    .unwrap_or_else(|| (0..positions.len() as u32).collect());
                if positions.is_empty()
                    || indices.len() % 3 != 0
                    || indices.iter().any(|&i| i as usize >= positions.len())
                    || [normals.len(), uvs.len(), joints.len(), weights.len()]
                        .iter()
                        .any(|&n| n != positions.len())
                    || joints
                        .iter()
                        .flatten()
                        .any(|&j| j as usize >= skins[skin].names.len())
                {
                    return Err("Invalid skater skin geometry".into());
                }
                let material = primitive.material();
                let pbr = material.pbr_metallic_roughness();
                let colour = pbr.base_color_factor();
                let image = pbr
                    .base_color_texture()
                    .map(|t| t.texture().source().index());
                let alpha = match material.alpha_mode() {
                    gltf::material::AlphaMode::Opaque => AlphaMode::Opaque,
                    gltf::material::AlphaMode::Mask => AlphaMode::Test,
                    gltf::material::AlphaMode::Blend => AlphaMode::Blend,
                };
                let count = indices.len() as u32;
                parts.push(Part {
                    mesh: MeshData {
                        positions,
                        normals,
                        uvs,
                        indices,
                        ranges: vec![(0, count, 0)],
                        one_sided: false,
                    },
                    joints,
                    weights,
                    skin,
                    colour,
                    image,
                    alpha,
                    gpu: None,
                });
            }
        }
        if parts.is_empty() {
            return Err("Skater GLB contains no skinned meshes".into());
        }
        Ok(Self {
            skins,
            parts,
            images,
            textures: Vec::new(),
        })
    }

    pub fn draw(
        &mut self,
        r: &Renderer,
        scene: &mut Scene,
        origin: DVec3,
        pose: &omsi_skate::Pose,
    ) -> Result<(), String> {
        if self.textures.is_empty() {
            self.textures = self
                .images
                .iter()
                .map(|i| r.add_texture(scene, i, true))
                .collect();
        }
        let root = Mat4::from_cols_array(&pose.root);
        let basis = Mat4::from_cols(Vec4::X, -Vec4::Z, Vec4::Y, Vec4::W);
        let to_omsi = basis.inverse();
        let matrices: Result<Vec<Vec<Mat4>>, String> = self
            .skins
            .iter()
            .map(|skin| {
                skin.names
                    .iter()
                    .zip(&skin.inverse)
                    .map(|(name, inverse)| {
                        let i = pose
                            .names
                            .iter()
                            .position(|n| n.eq_ignore_ascii_case(name))
                            .ok_or_else(|| format!("Skater animation joint missing: {name}"))?;
                        let bone = pose.bones.get(i).ok_or("Skater pose has too few bones")?;
                        Ok(to_omsi * root * Mat4::from_cols_array(bone) * basis * *inverse)
                    })
                    .collect()
            })
            .collect();
        let matrices = matrices?;
        for part in &mut self.parts {
            let mut positions = Vec::with_capacity(part.mesh.positions.len());
            let mut normals = Vec::with_capacity(part.mesh.positions.len());
            for (i, &p) in part.mesh.positions.iter().enumerate() {
                let mut out = Vec3::ZERO;
                let mut normal = Vec3::ZERO;
                let sum: f32 = part.weights[i].iter().sum();
                if !sum.is_finite() || sum <= 0.0 {
                    return Err("Invalid skater skin weight".into());
                }
                for k in 0..4 {
                    let weight = part.weights[i][k] / sum;
                    let m = matrices[part.skin][part.joints[i][k] as usize];
                    out += m.transform_point3(p) * weight;
                    normal += m.transform_vector3(part.mesh.normals[i]) * weight;
                }
                positions.push(out);
                normals.push(normal.normalize_or_zero());
            }
            let (mesh, _, instance) = *part.gpu.get_or_insert_with(|| {
                let mut data = part.mesh.clone();
                data.positions.clone_from(&positions);
                data.normals.clone_from(&normals);
                let mesh = r.add_mesh(scene, &data);
                let texture = part.image.and_then(|i| self.textures.get(i).copied());
                let material = r.add_material(scene, texture, part.alpha, part.colour, false);
                let instance = r.add_instance(scene, mesh, origin, Mat4::IDENTITY, vec![material]);
                (mesh, material, instance)
            });
            r.update_mesh(scene, mesh, &positions, &normals, &part.mesh.uvs);
            r.set_transform(scene, instance, origin, Mat4::IDENTITY);
        }
        Ok(())
    }

    pub fn free(self, r: &Renderer, scene: &mut Scene) {
        for part in self.parts {
            if let Some((mesh, material, instance)) = part.gpu {
                r.remove_instance(scene, instance);
                r.free_mesh(scene, mesh);
                r.free_material(scene, material);
            }
        }
        for texture in self.textures {
            r.free_texture(scene, texture);
        }
    }
}
