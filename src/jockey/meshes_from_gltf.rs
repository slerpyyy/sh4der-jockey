use std::collections::HashMap;
use std::rc::Rc;

use gl::types::*;

use crate::util::*;

use super::{Geometry, GeometryAttribute, Uniform};
use super::{POSITION_NAME, NORMAL_NAME, TEXCOORD0_NAME, MATERIAL_ALPHA_CUTOFF, MATERIAL_BASE_COLOR, MODEL_MATRIX, MATERIAL_BASE_TEXTURE};

/// Ref: https://github.com/bwasty/gltf-viewer/blob/master/src/importdata.rs#L4-L9
struct GltfImportData {
    pub doc: gltf::Document,
    pub buffers: Vec<gltf::buffer::Data>,
    pub images: Vec<gltf::image::Data>,
}

fn traverse_node<F, R>(node: &gltf::Node, world_matrix: &Matrix4, f: &F) -> Vec<R>
where
    F: Fn(&gltf::Node, &Matrix4) -> R,
{
    let mut vec: Vec<R> = Vec::new();

    let this_matrix = world_matrix.multiply(Matrix4::new(node.transform().matrix()));

    vec.push(f(&node, &this_matrix));

    let result: &mut Vec<_> = &mut node
        .children()
        .map(|child| traverse_node(&child, &this_matrix, f))
        .flatten()
        .collect();

    vec.append(result);

    vec
}

fn traverse_primitives<F, R>(root_node: &gltf::Node, world_matrix: &Matrix4, f: &F) -> Vec<R>
where
    F: Fn(&gltf::Primitive, &Matrix4) -> R,
{
    let result: Vec<_> = traverse_node(&root_node, &world_matrix, &|node, world_matrix| {
        let mut vec: Vec<R> = Vec::new();

        let mesh = match node.mesh() {
            Some(s) => s,
            None => return vec,
        };

        for primitive in mesh.primitives() {
            let result = f(&primitive, &world_matrix);
            vec.push(result);
        }

        vec
    })
    .into_iter()
    .flatten()
    .collect();

    result
}

fn geometry_from_primitive(
    primitive: &gltf::Primitive,
    imp: &GltfImportData,
) -> Result<Geometry, String> {
    let buffers = &imp.buffers;

    // See: https://github.com/bwasty/gltf-viewer/blob/1cb99cb51c04ddf7af3f2b4488757f6f4f498787/src/render/primitive.rs#L104
    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

    // position
    let (positions, count) = {
        let iter = match reader.read_positions() {
            Some(s) => s,
            None => {
                return Err(
                    "Primitive must have POSITION attribute. Ignoring the primitive".to_string(),
                )
            }
        };

        let count = iter.len();
        let vec: Vec<GLfloat> = iter.flatten().collect();

        let mut positions = GeometryAttribute::init(vec, 3, gl::FLOAT);
        positions.normalized = match primitive.get(&gltf::Semantic::Positions) {
            Some(s) => {
                if s.normalized() {
                    1
                } else {
                    0
                }
            }
            None => 0,
        };

        (positions, count)
    };

    let mut geometry = Geometry::init(count as _);
    geometry
        .attributes
        .insert(POSITION_NAME.as_ptr(), positions);

    // indices
    {
        if let Some(s) = reader.read_indices() {
            let iter = s.into_u32();
            let count = iter.len();
            let vec: Vec<GLuint> = iter.collect();

            let mut indices = GeometryAttribute::init(vec, 1, gl::UNSIGNED_INT);
            indices.target = gl::ELEMENT_ARRAY_BUFFER;

            geometry.indices = Some(indices);
            geometry.count = count as _;
        }
    };

    // normals
    {
        if let Some(iter) = reader.read_normals() {
            let vec: Vec<GLfloat> = iter.flatten().collect();

            let mut normals = GeometryAttribute::init(vec, 3, gl::FLOAT);
            normals.normalized = match primitive.get(&gltf::Semantic::Normals) {
                Some(s) => {
                    if s.normalized() {
                        1
                    } else {
                        0
                    }
                }
                None => 0,
            };

            geometry
                .attributes
                .insert(NORMAL_NAME.as_ptr(), normals);
        }
    };

    // texcoord0
    {
        if let Some(s) = reader.read_tex_coords(0) {
            let iter = s.into_f32();
            let vec: Vec<GLfloat> = iter.flatten().collect();

            let mut texcoords0 = GeometryAttribute::init(vec, 2, gl::FLOAT);
            texcoords0.normalized = match primitive.get(&gltf::Semantic::TexCoords(0)) {
                Some(s) => {
                    if s.normalized() {
                        1
                    } else {
                        0
                    }
                }
                None => 0,
            };

            geometry
                .attributes
                .insert(TEXCOORD0_NAME.as_ptr(), texcoords0);
        }
    };

    Ok(geometry)
}

fn texture_from_gltf_texture(
    texture: &gltf::texture::Texture,
    imp: &GltfImportData,
) -> Rc<dyn Texture> {
    let images = &imp.images;

    let sampler = texture.sampler();
    let image = texture.source();
    let data = &images[image.index()];

    let mut builder = TextureBuilder::new();

    builder.set_resolution(vec![data.width, data.height]);
    builder.channels = match data.format {
        gltf::image::Format::R8 => 1,
        gltf::image::Format::R8G8 => 2,
        gltf::image::Format::R8G8B8 => 3,
        gltf::image::Format::R8G8B8A8 => 4,
        _ => unreachable!(),
    };
    builder.min_filter = match sampler.min_filter() {
        Some(gltf::texture::MinFilter::Nearest) => gl::NEAREST,
        Some(gltf::texture::MinFilter::NearestMipmapNearest) => gl::NEAREST_MIPMAP_NEAREST,
        Some(gltf::texture::MinFilter::NearestMipmapLinear) => gl::NEAREST_MIPMAP_LINEAR,
        Some(gltf::texture::MinFilter::Linear) => gl::LINEAR,
        Some(gltf::texture::MinFilter::LinearMipmapNearest) => gl::LINEAR_MIPMAP_NEAREST,
        Some(gltf::texture::MinFilter::LinearMipmapLinear) => gl::LINEAR_MIPMAP_LINEAR,
        _ => builder.min_filter, // use default value
    };
    builder.mag_filter = match sampler.mag_filter() {
        Some(gltf::texture::MagFilter::Nearest) => gl::NEAREST,
        Some(gltf::texture::MagFilter::Linear) => gl::LINEAR,
        _ => builder.mag_filter, // use default value
    };
    builder.wrap_mode = match sampler.wrap_s() {
        gltf::texture::WrappingMode::ClampToEdge => gl::CLAMP_TO_EDGE,
        gltf::texture::WrappingMode::MirroredRepeat => gl::MIRRORED_REPEAT,
        gltf::texture::WrappingMode::Repeat => gl::REPEAT,
    };
    builder.mipmap = match sampler.min_filter() {
        Some(gltf::texture::MinFilter::Nearest) => false,
        Some(gltf::texture::MinFilter::NearestMipmapNearest) => true,
        Some(gltf::texture::MinFilter::NearestMipmapLinear) => true,
        Some(gltf::texture::MinFilter::Linear) => false,
        Some(gltf::texture::MinFilter::LinearMipmapNearest) => true,
        Some(gltf::texture::MinFilter::LinearMipmapLinear) => true,
        _ => builder.mipmap, // use default value
    };

    builder.build_texture_with_data(data.pixels.as_ptr() as _)
}

fn set_material_props_to_uniforms(
    uniforms: &mut HashMap<*const GLchar, Uniform>,
    textures: &mut HashMap<*const GLchar, Rc<dyn Texture>>,
    material: &gltf::Material,
    imp: &GltfImportData,
) {
    let pbr = material.pbr_metallic_roughness();

    uniforms.insert(
        MATERIAL_ALPHA_CUTOFF.as_ptr(),
        Uniform::Float(material.alpha_cutoff().unwrap_or(0.5)),
    );

    match pbr.base_color_texture() {
        Some(s) => {
            let texture = s.texture();
            let rc_texture = texture_from_gltf_texture(&texture, imp);

            textures.insert(
                MATERIAL_BASE_TEXTURE.as_ptr(),
                rc_texture,
            );
        },
        None => (),
    }

    {
        let a = pbr.base_color_factor();
        uniforms.insert(
            MATERIAL_BASE_COLOR.as_ptr(),
            Uniform::Vec4(a[0], a[1], a[2], a[3]),
        );
    };
}

pub fn meshes_from_gltf(path: String) -> Result<Vec<Mesh>, String> {
    let (doc, buffers, images) = match gltf::import(path) {
        Ok(s) => s,
        Err(error) => return Err(error.to_string()),
    };
    let imp = GltfImportData { doc, buffers, images };

    let scene = match imp.doc.default_scene() {
        Some(s) => s,
        None => return Err("No default scene provided.".to_string()),
    };

    let vec: Vec<_> = scene
        .nodes()
        .map(|scene_root_node| {
            let fu: Vec<_> = traverse_primitives(
                &scene_root_node,
                &Matrix4::identity(),
                &|primitive, world_matrix| {
                    match geometry_from_primitive(&primitive, &imp) {
                        Ok(geometry) => {
                            let material = primitive.material();

                            let mut uniforms: HashMap<*const GLchar, Uniform> =
                                HashMap::new();
                            let mut textures: HashMap<*const GLchar, Rc<dyn Texture>> = HashMap::new();

                            // matrix
                            uniforms.insert(
                                MODEL_MATRIX.as_ptr(),
                                Uniform::Mat4(world_matrix.elements_flattened()),
                            );

                            // materials
                            set_material_props_to_uniforms(&mut uniforms, &mut textures, &material, &imp);

                            // mesh
                            let mesh = Mesh { geometry, uniforms, textures };

                            Some(mesh)
                        }
                        Err(e) => {
                            log::warn!("{}", e);
                            None
                        }
                    }
                },
            )
            .into_iter()
            .filter_map(|e| e)
            .collect();

            fu
        })
        .flatten()
        .collect();

    Ok(vec)
}
