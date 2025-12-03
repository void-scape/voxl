use crate::{World, camera::Camera};
use glam::{FloatExt, Vec2, Vec3};
use glazer::glow;
use std::collections::HashMap;

pub const CHUNK_SIZE: usize = 16;

#[derive(Default)]
pub struct Chunks {
    loaded_chunks: HashMap<(i64, i64), Chunk>,
    unloaded_chunks: Vec<Chunk>,
    pub noise_layers: Vec<(f32, f32)>,
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
    translations: Vec<Vec3>,
    uvs: Vec<[Vec2; 6]>,
}

impl Chunk {
    fn with_capacity(capacity: usize) -> Self {
        Self {
            translations: Vec::with_capacity(capacity),
            uvs: Vec::with_capacity(capacity),
        }
    }
}

pub fn update(chunks: &mut Chunks, view_distance: usize, camera: &Camera) {
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
                load_chunk(chunks, x, z);
            }
        }
    }
}

pub fn ui(ui: &mut egui::Ui, chunks: &mut Chunks, view_distance: usize, camera: &Camera) {
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
        update(chunks, view_distance, camera);
    }
}

pub fn render(world: &mut World, gl: &glow::Context) {
    let light_source = Vec3::new(20.0, 100.0, 20.0);
    world.voxel_renderer.bind_light_source(gl, light_source);

    let view = world.camera.view_matrix();
    world
        .voxel_renderer
        .bind_view(gl, world.camera.translation, view);

    // TODO: Arena allocation for voxels?
    for chunk in world.chunks.loaded_chunks.values() {
        world
            .voxel_renderer
            .render_batch(gl, &chunk.translations, &chunk.uvs);
    }
}

fn load_chunk(chunks: &mut Chunks, x: i64, z: i64) {
    let perlin_scale = 200;

    let mut chunk = chunks
        .unloaded_chunks
        .pop()
        .unwrap_or_else(|| Chunk::with_capacity(CHUNK_SIZE * CHUNK_SIZE));
    chunk.translations.clear();
    chunk.uvs.clear();

    let zoffset = z as f32 * CHUNK_SIZE as f32;
    let xoffset = x as f32 * CHUNK_SIZE as f32;
    for z in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let x = x as f32 + xoffset;
            let z = z as f32 + zoffset;
            let uv = Vec2::new(x / perlin_scale as f32, z / perlin_scale as f32);

            let mut surface = 0.0;
            for (uv_scale, weight) in chunks.noise_layers.iter() {
                surface += (perlin(uv * *uv_scale) * 0.5 + 0.5) * weight;
            }

            chunk
                .translations
                .push(Vec3::new(x, surface.round() - 80.0, z));
            chunk.uvs.push([Vec2::new(0.0, 1.0); 6]);
            // for y in 0..surface.round() as usize {
            //     voxels.push(Vec3::new(x as f32, y as f32 - 80.0, z as f32));
            // }
        }
    }
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
    let f = st.fract();
    let u = f * f * (3.0 - 2.0 * f);

    let left = random2(i)
        .dot(f)
        .lerp(random2(i + Vec2::X).dot(f - Vec2::X), u.x);
    let right = random2(i + Vec2::Y)
        .dot(f - Vec2::Y)
        .lerp(random2(i + Vec2::ONE).dot(f - Vec2::ONE), u.x);
    left.lerp(right, u.y)
}
