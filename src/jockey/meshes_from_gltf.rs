use std::collections::HashMap;

use gl::types::*;

use super::mesh::Mesh;
use super::{Geometry, GeometryAttribute};
use super::uniformable::*;
use super::{MODEL_MATRIX, MATERIAL_ALPHA_CUTOFF, MATERIAL_BASE_COLOR};
use super::matrix4::Matrix4;

fn traverse_node<F, R>(node: &gltf::Node, world_matrix: &Matrix4, f: &F) -> Vec<R> where
    F: Fn(&gltf::Node, &Matrix4) -> R {
    let mut vec: Vec<R> = Vec::new();

    let this_matrix = world_matrix.multiply(Matrix4::new(node.transform().matrix()));

    vec.push(f(&node, &this_matrix));

    let result: &mut Vec<_> = &mut node.children().map(|child| {
        traverse_node(&child, &this_matrix, f)
    }).flatten().collect();

    vec.append(result);

    vec
}

fn traverse_primitives<F, R>(root_node: &gltf::Node, world_matrix: &Matrix4, f: &F) -> Vec<R> where
    F: Fn(&gltf::Primitive, &Matrix4) -> R {
    let result: Vec<_> = traverse_node(&root_node, &world_matrix, &|node, world_matrix| {
        let mut vec: Vec<R> = Vec::new();

        let mesh = match node.mesh() {
            Some(s) => s,
            None => return vec,
        };

        for primitive in mesh.primitives() {
            let result = f(&primitive, &world_matrix);
            vec.push(result);
        };

        vec
    }).into_iter().flatten().collect();

    result
}

fn geometry_from_primitive(
    primitive: &gltf::Primitive,
    buffers: &Vec<gltf::buffer::Data>,
) -> Result<Geometry, String> {
    // See: https://github.com/bwasty/gltf-viewer/blob/1cb99cb51c04ddf7af3f2b4488757f6f4f498787/src/render/primitive.rs#L104
    let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]) );

    // position
    let (positions, count) = {
        let iter = match reader.read_positions() {
            Some(s) => s,
            None => return Err("Primitive must have POSITION attribute. Ignoring the primitive".to_string()),
        };

        let count = iter.len();
        let mut vec: Vec<GLfloat> = Vec::with_capacity(3 * count);

        // TODO: There definitely is a better way to do this
        for v in iter { for c in v { vec.push(c); } }

        let mut positions = GeometryAttribute::init(vec, 3, gl::FLOAT);
        positions.normalized = match primitive.get(&gltf::Semantic::Positions) {
            Some(s) => if s.normalized() { 1 } else { 0 },
            None => 0,
        };

        (positions, count)
    };

    let mut geometry = Geometry::init(count as _);
    geometry.attributes.insert(Geometry::ATTRIBUTE_POSITION, positions);

    // indices
    {
        if let Some(s) = reader.read_indices() {
            let iter = s.into_u32();
            let count = iter.len();
            let mut vec: Vec<GLuint> = Vec::with_capacity(count);

            // TODO: There definitely is a better way to do this
            for i in iter { vec.push(i); }

            let mut indices = GeometryAttribute::init(vec, 1, gl::UNSIGNED_INT);
            indices.target = gl::ELEMENT_ARRAY_BUFFER;

            geometry.indices = Some(indices);
            geometry.count = count as _;
        }
    };

    // normals
    {
        if let Some(iter) = reader.read_normals() {
            let count = iter.len();
            let mut vec: Vec<GLfloat> = Vec::with_capacity(3 * count);

            // TODO: There definitely is a better way to do this
            for v in iter { for c in v { vec.push(c); } }

            let mut normals = GeometryAttribute::init(vec, 3, gl::FLOAT);
            normals.normalized = match primitive.get(&gltf::Semantic::Normals) {
                Some(s) => if s.normalized() { 1 } else { 0 },
                None => 0,
            };

            geometry.attributes.insert(Geometry::ATTRIBUTE_NORMAL, normals);
        }
    };

    // texcoord0
    {
        if let Some(s) = reader.read_tex_coords(0) {
            let iter = s.into_f32();
            let count = iter.len();
            let mut vec: Vec<GLfloat> = Vec::with_capacity(2 * count);

            // TODO: There definitely is a better way to do this
            for v in iter { for c in v { vec.push(c); } }

            let mut texcoords0 = GeometryAttribute::init(vec, 2, gl::FLOAT);
            texcoords0.normalized = match primitive.get(&gltf::Semantic::TexCoords(0)) {
                Some(s) => if s.normalized() { 1 } else { 0 },
                None => 0,
            };

            geometry.attributes.insert(Geometry::ATTRIBUTE_TEXCOORD0, texcoords0);
        }
    };

    Ok(geometry)
}

pub fn meshes_from_gltf(path: String) -> Result<Vec<Mesh>, String> {
    let (doc, buffers, _images) = match gltf::import(path) {
        Ok(s) => s,
        Err(error) => return Err(error.to_string()),
    };

    let scene = match doc.default_scene() {
        Some(s) => s,
        None => return Err("No default scene provided.".to_string()),
    };

    let vec: Vec<_> = scene.nodes().map(|scene_root_node| {
        let fu: Vec<_> = traverse_primitives(&scene_root_node, &Matrix4::identity(), &|primitive, world_matrix| {
            match geometry_from_primitive(&primitive, &buffers) {
                Ok(geometry) => {
                    let material = primitive.material();
                    let pbr = material.pbr_metallic_roughness();

                    let mut uniforms: HashMap<*const GLchar, Box<dyn Uniformable>> = HashMap::new();

                    // matrix
                    uniforms.insert(
                        MODEL_MATRIX.as_ptr(),
                        Box::new(UniformableMatrix4fv::new(world_matrix.elements.clone())),
                    );

                    // materials
                    uniforms.insert(
                        MATERIAL_ALPHA_CUTOFF.as_ptr(),
                        Box::new(Uniformable1f::new(material.alpha_cutoff().unwrap_or(0.5))),
                    );
                    uniforms.insert(
                        MATERIAL_BASE_COLOR.as_ptr(),
                        Box::new(Uniformable4f::new(pbr.base_color_factor())),
                    );

                    // mesh
                    let mesh = Mesh {
                        geometry,
                        uniforms,
                    };

                    Some(mesh)
                },
                Err(e) => {
                    log::warn!("{}", e);
                    None
                },
            }
        }).into_iter().filter_map(|e| e).collect();

        fu
    }).flatten().collect();

    Ok(vec)
}
