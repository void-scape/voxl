use crate::{
    World,
    camera::Camera,
    voxel::{Lighting, VoxelInstanceBuffer, VoxelRenderer},
};
use glam::{FloatExt, Vec2, Vec3};
use glazer::glow;
use std::collections::{HashMap, HashSet};

pub const CHUNK_SIZE: usize = 16;

#[derive(Default)]
pub struct Chunks {
    loaded_chunks: HashMap<(i64, i64), Chunk>,
    unloaded_chunks: Vec<Chunk>,
    noise_layers: Vec<(f32, f32)>,
}

impl Chunks {
    pub fn from_noise(noise: Vec<(f32, f32)>) -> Self {
        Self {
            noise_layers: noise,
            ..Default::default()
        }
    }

    pub fn clear(&mut self) {
        self.unloaded_chunks
            .extend(self.loaded_chunks.drain().map(|(_, v)| v));
    }
}

#[derive(Default)]
struct Chunk {
    buffers: Option<(VoxelInstanceBuffer, Vec<[Vec2; 6]>)>,
}

impl Chunk {
    fn with_capacity(capacity: usize) -> Self {
        Self { buffers: None }
    }
}

pub fn update(
    gl: &glow::Context,
    voxel_renderer: &VoxelRenderer,
    chunks: &mut Chunks,
    view_distance: usize,
    camera: &Camera,
) {
    let view_distance = view_distance as i64;
    let current_chunk = (-camera.translation / CHUNK_SIZE as f32).as_i64vec3();
    let zrange = current_chunk.z - view_distance..=current_chunk.z + view_distance;
    let xrange = current_chunk.x - view_distance..=current_chunk.x + view_distance;

    chunks.loaded_chunks.retain(|(x, z), buffer| {
        if !xrange.contains(x) || !zrange.contains(z) {
            chunks.unloaded_chunks.push(core::mem::take(buffer));
            false
        } else {
            true
        }
    });

    for z in zrange {
        for x in xrange.clone() {
            if !chunks.loaded_chunks.contains_key(&(x, z)) {
                load_chunk(gl, voxel_renderer, chunks, x, z);
            }
        }
    }
}

pub fn ui(
    ui: &mut egui::Ui,
    gl: &glow::Context,
    voxel_renderer: &VoxelRenderer,
    chunks: &mut Chunks,
    view_distance: usize,
    camera: &Camera,
) {
    let mut changed_chunk_generation = false;

    ui.label("Noise Layers");
    ui.horizontal(|ui| {
        if ui.button("-").clicked() {
            chunks.noise_layers.pop();
        }
        if ui.button("+").clicked() {
            chunks.noise_layers.push((1.0, 1.0));
        }
    });
    for (i, (scale, weight)) in chunks.noise_layers.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.label(format!("Layer {i}"));
            changed_chunk_generation |= ui
                .add(egui::Slider::new(scale, -100.0..=100.0).text("UV Scale"))
                .changed();
            changed_chunk_generation |= ui
                .add(egui::Slider::new(weight, -100.0..=100.0).text("Weight"))
                .changed();
        });
    }

    if changed_chunk_generation {
        chunks.clear();
        update(gl, voxel_renderer, chunks, view_distance, camera);
    }
}

pub fn render(world: &mut World, gl: &glow::Context, width: usize, height: usize) {
    let lighting = Lighting {
        light_source: Vec3::new(0.0, 50.0, -120.0),
        light_color: Vec3::ONE,
        ambient_brightness: 0.4,
    };
    let view = world.camera.view_matrix();

    let (fog_near, fog_far) = if world.fog {
        let near = ((world.view_distance - 2) * CHUNK_SIZE) as f32;
        let far = (world.view_distance * CHUNK_SIZE) as f32;
        (near, far)
    } else {
        (
            (world.view_distance * CHUNK_SIZE * 10) as f32,
            (world.view_distance * CHUNK_SIZE * 10) as f32,
        )
    };

    let translations_for_shadow_pass = world
        .chunks
        .loaded_chunks
        .values()
        .flat_map(|chunk| chunk.buffers.as_ref().map(|(instances, _)| *instances));
    let translations_and_atlas_uvs = world.chunks.loaded_chunks.values().flat_map(|chunk| {
        chunk
            .buffers
            .as_ref()
            .map(|(instances, uvs)| (*instances, uvs.as_slice()))
    });

    world.voxel_renderer.render_pass(
        gl,
        width,
        height,
        lighting,
        view,
        fog_near,
        fog_far,
        translations_for_shadow_pass,
        translations_and_atlas_uvs,
    );

    world.sprite_renderer.render(
        gl,
        Vec3::new(950.0, 400.0, 0.0),
        Vec2::ONE * 0.5,
        glam::Quat::default(),
        world.voxel_renderer.shadow_map,
        1024,
        1024,
    );
}

// TODO: this shit is trash
fn load_chunk(
    gl: &glow::Context,
    voxel_renderer: &VoxelRenderer,
    chunks: &mut Chunks,
    x: i64,
    z: i64,
) {
    let perlin_scale = 200;

    let mut chunk = chunks
        .unloaded_chunks
        .pop()
        .unwrap_or_else(|| Chunk::with_capacity(CHUNK_SIZE * CHUNK_SIZE));

    let mut translations = Vec::new();
    let mut uvs = Vec::new();

    let zoffset = z as f32 * CHUNK_SIZE as f32;
    let xoffset = x as f32 * CHUNK_SIZE as f32;
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let z = z as f32 + zoffset;
            let x = x as f32 + xoffset;
            let uv = Vec2::new(x / perlin_scale as f32, z / perlin_scale as f32);

            let mut surface = 0.0;
            for (uv_scale, weight) in chunks.noise_layers.iter() {
                surface += (perlin(uv * *uv_scale) * 0.5 + 0.5) * weight;
            }

            translations.push(Vec3::new(x, surface.round() - 80.0, z));
            uvs.push([Vec2::new(0.0, 1.0); 6]);
            // for y in 0..surface.round() as usize {
            //     translations.push(Vec3::new(x, y as f32 - 80.0, z));
            //     uvs.push([Vec2::new(0.0, 1.0); 6]);
            // }
        }
    }

    let mut hash = HashSet::<(i64, i64, i64)>::from_iter(
        translations
            .iter()
            .map(|t| (t.x as i64, t.y as i64, t.z as i64)),
    );
    let mut remove = Vec::new();
    'outer: for (i, translation) in translations.iter().enumerate() {
        for z in -1..=1 {
            for y in -1..=1 {
                for x in -1..=1 {
                    if hash.insert((
                        translation.x as i64 + x,
                        translation.y as i64 + y,
                        translation.z as i64 + z,
                    )) {
                        continue 'outer;
                    }
                }
            }
        }
        remove.push(i);
    }

    for index in remove.into_iter().rev() {
        translations.swap_remove(index);
        uvs.swap_remove(index);
    }

    chunk.buffers = Some((
        voxel_renderer.generate_translation_buffer(gl, &translations),
        uvs,
    ));

    assert!(chunks.loaded_chunks.insert((x, z), chunk).is_none());
}

// https://thebookofshaders.com/edit.php#11/2d-gnoise.frag
fn perlin(st: Vec2) -> f32 {
    fn random2(st: Vec2) -> Vec2 {
        fn random(v: f32) -> f32 {
            -1.0 + 2.0 * (v.sin() * 43758.547).fract()
        }

        let st = Vec2::new(
            st.dot(Vec2::new(127.1, 311.7)),
            st.dot(Vec2::new(269.5, 183.3)),
        );
        Vec2::new(random(st.x), random(st.y))
    }

    let i = st.floor();
    let f = st - i;
    let u = f * f * (3.0 - 2.0 * f);

    let left = random2(i)
        .dot(f)
        .lerp(random2(i + Vec2::X).dot(f - Vec2::X), u.x);
    let right = random2(i + Vec2::Y)
        .dot(f - Vec2::Y)
        .lerp(random2(i + Vec2::ONE).dot(f - Vec2::ONE), u.x);
    left.lerp(right, u.y)
}
