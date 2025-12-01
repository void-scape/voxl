use crate::voxel::VoxelRenderer;
use glam::{FloatExt, Mat4, Quat, Vec2, Vec3};
use glazer::glow::{self, HasContext};
use glazer::winit::event::{DeviceEvent, KeyEvent, WindowEvent};
use glazer::winit::keyboard::{KeyCode, PhysicalKey};
use std::collections::HashMap;

mod gui;
mod shader;
mod voxel;

const CHUNK_WIDTH: usize = 16;

#[derive(Default)]
pub struct Memory {
    world: Option<World>,
}

struct World {
    gui: gui::Egui,
    voxel_renderer: VoxelRenderer,
    wireframes: bool,
    fog: bool,
    view_distance: usize,
    camera_controller: CameraController,
    loaded_chunks: HashMap<(i64, i64), Chunk>,
    unloaded_chunks: Vec<Chunk>,
    noise_layers: Vec<(f32, f32)>,
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

#[derive(Default)]
struct CameraController {
    enabled: bool,
    left: bool,
    right: bool,
    forward: bool,
    back: bool,
    up: bool,
    down: bool,
    translation: Vec3,
    yaw: f32,
    pitch: f32,
    look_at: Vec3,
}

#[unsafe(no_mangle)]
pub fn handle_input(
    glazer::PlatformInput {
        memory,
        window,
        input,
        gl,
        ..
    }: glazer::PlatformInput<Memory>,
) {
    let Some(world) = &mut memory.world else {
        return;
    };

    match input {
        glazer::Input::Window(event) => {
            if world.gui.handle_input(window, &event) {
                return;
            }
            match event {
                WindowEvent::Resized(size) => {
                    let w = size.width as usize;
                    let h = size.height as usize;
                    world.voxel_renderer.resize(gl, w, h);
                }
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: PhysicalKey::Code(code),
                            state,
                            repeat: false,
                            ..
                        },
                    ..
                } => match code {
                    KeyCode::Escape => {
                        std::process::exit(0);
                    }
                    KeyCode::KeyI => {
                        if state.is_pressed() {
                            world.camera_controller.enabled = !world.camera_controller.enabled;
                        }
                    }
                    KeyCode::KeyA => {
                        world.camera_controller.left = state.is_pressed();
                    }
                    KeyCode::KeyD => {
                        world.camera_controller.right = state.is_pressed();
                    }
                    KeyCode::KeyW => {
                        world.camera_controller.forward = state.is_pressed();
                    }
                    KeyCode::KeyS => {
                        world.camera_controller.back = state.is_pressed();
                    }
                    KeyCode::Space | KeyCode::ControlLeft => {
                        world.camera_controller.up = state.is_pressed();
                    }
                    KeyCode::ShiftLeft => {
                        world.camera_controller.down = state.is_pressed();
                    }
                    KeyCode::KeyF if state.is_pressed() => {
                        if world.fog {
                            world
                                .voxel_renderer
                                .bind_view_distance(gl, world.view_distance);
                        } else {
                            world
                                .voxel_renderer
                                .bind_view_distance(gl, world.view_distance * 10);
                        }
                        world.fog = !world.fog;
                    }
                    KeyCode::KeyV if state.is_pressed() => {
                        world.wireframes = !world.wireframes;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        glazer::Input::Device(event) => {
            // https://learnopengl.com/code_viewer_gh.php?code=src/1.getting_started/7.3.camera_mouse_zoom/camera_mouse_zoom.cpp
            if let DeviceEvent::MouseMotion { delta } = event
                && world.camera_controller.enabled
            {
                let sensitivity = 0.005;
                world.camera_controller.yaw += delta.0 as f32 * sensitivity;
                world.camera_controller.pitch -= delta.1 as f32 * sensitivity;
                world.camera_controller.pitch = world.camera_controller.pitch.clamp(
                    -std::f32::consts::FRAC_PI_2 + 0.0001,
                    std::f32::consts::FRAC_PI_2 - 0.0001,
                );
                let yaw = world.camera_controller.yaw;
                let pitch = world.camera_controller.pitch;
                world.camera_controller.look_at = look_at(pitch, yaw);
            }
        }
    }
}

fn look_at(pitch: f32, yaw: f32) -> Vec3 {
    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize_or(Vec3::Z)
}

#[unsafe(no_mangle)]
pub fn update_and_render(
    glazer::PlatformUpdate {
        memory,
        delta,
        //
        window,
        event_loop,
        //
        gl,
        width,
        height,
        ..
    }: glazer::PlatformUpdate<Memory>,
) {
    window.set_title(&format!("Voxl - {:.2}", 1.0 / delta));

    let view_distance = 12;
    let speed = 100.0;
    let pitch = -0.599;
    let yaw = 1.569;
    let world = memory.world.get_or_insert_with(|| World {
        gui: gui::Egui::new(event_loop, window, gl),
        voxel_renderer: VoxelRenderer::new(gl, width, height, view_distance, "assets/terrain.png"),
        wireframes: false,
        fog: false,
        view_distance,
        camera_controller: CameraController {
            look_at: look_at(pitch, yaw),
            translation: Vec3::new(0.0, 0.0, 10.0),
            pitch,
            yaw,
            ..Default::default()
        },
        loaded_chunks: HashMap::new(),
        unloaded_chunks: Vec::new(),
        noise_layers: vec![(1.0, 80.0), (2.0, 40.0), (6.0, 30.0)],
    });

    if world.camera_controller.enabled {
        let dt = -speed * delta;
        // https://sotrh.github.io/learn-wgpu/intermediate/tutorial12-camera/#the-projection
        let (yaw_sin, yaw_cos) = world.camera_controller.yaw.sin_cos();
        let forward = Vec3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vec3::new(-yaw_sin, 0.0, yaw_cos).normalize();
        world.camera_controller.translation += forward
            * (world.camera_controller.forward as u32 as f32
                - world.camera_controller.back as u32 as f32)
            * dt;
        world.camera_controller.translation += right
            * (world.camera_controller.right as u32 as f32
                - world.camera_controller.left as u32 as f32)
            * dt;

        if world.camera_controller.down {
            world.camera_controller.translation.y -= dt;
        }
        if world.camera_controller.up {
            world.camera_controller.translation.y += dt;
        }
    }

    update_chunks(world);

    unsafe {
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

        let light_source = Vec3::new(20.0, 100.0, 20.0);
        world.voxel_renderer.bind_light_source(gl, light_source);

        let view = Mat4::from_quat(Quat::look_to_rh(world.camera_controller.look_at, Vec3::Y))
            * Mat4::from_translation(world.camera_controller.translation);
        world
            .voxel_renderer
            .bind_view(gl, world.camera_controller.translation, view);

        if world.wireframes {
            gl.polygon_mode(glow::FRONT_AND_BACK, glow::LINE);
        } else {
            gl.polygon_mode(glow::FRONT_AND_BACK, glow::FILL);
        }
        // TODO: Arena allocation for voxels?
        for chunk in world.loaded_chunks.values() {
            world
                .voxel_renderer
                .render_batch(gl, &chunk.translations, &chunk.uvs);
        }
    }

    let mut changed_chunk_generation = false;
    world.gui.show(|ui| {
        egui::Window::new("Voxl").show(ui, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.add(egui::Slider::new(&mut world.view_distance, 1..=32).text("View Distance"));

                ui.label("Noise Layers");
                ui.horizontal(|ui| {
                    if ui.button("-").clicked() {
                        world.noise_layers.pop();
                    }
                    if ui.button("+").clicked() {
                        world.noise_layers.push((1.0, 1.0));
                    }
                });
                for (i, (scale, weight)) in world.noise_layers.iter_mut().enumerate() {
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
            })
        });
    });

    if changed_chunk_generation {
        world
            .unloaded_chunks
            .extend(world.loaded_chunks.drain().map(|(_, v)| v));
        update_chunks(world);
    }

    world.gui.paint();
}

fn update_chunks(world: &mut World) {
    let view_distance = world.view_distance as i64;
    let current_chunk = (-world.camera_controller.translation / CHUNK_WIDTH as f32).as_i64vec3();
    let zrange = current_chunk.z - view_distance..=current_chunk.z + view_distance;
    let xrange = current_chunk.x - view_distance..=current_chunk.x + view_distance;

    world.loaded_chunks.retain(|(x, z), buffer| {
        if !xrange.contains(x) || !zrange.contains(z) {
            world.unloaded_chunks.push(core::mem::take(buffer));
            false
        } else {
            true
        }
    });

    for z in zrange {
        for x in xrange.clone() {
            if !world.loaded_chunks.contains_key(&(x, z)) {
                load_chunk(world, x, z);
            }
        }
    }
}

fn load_chunk(world: &mut World, x: i64, z: i64) {
    let perlin_scale = 200;

    let mut chunk = world
        .unloaded_chunks
        .pop()
        .unwrap_or_else(|| Chunk::with_capacity(CHUNK_WIDTH * CHUNK_WIDTH));
    chunk.translations.clear();
    chunk.uvs.clear();

    let zoffset = z as f32 * CHUNK_WIDTH as f32;
    let xoffset = x as f32 * CHUNK_WIDTH as f32;
    for z in 0..CHUNK_WIDTH {
        for x in 0..CHUNK_WIDTH {
            let x = x as f32 + xoffset;
            let z = z as f32 + zoffset;
            let uv = Vec2::new(x / perlin_scale as f32, z / perlin_scale as f32);

            let mut surface = 0.0;
            for (uv_scale, weight) in world.noise_layers.iter() {
                surface += (perlin(uv * uv_scale) * 0.5 + 0.5) * weight;
            }

            chunk
                .translations
                .push(Vec3::new(x, surface.round() - 80.0, z));
            chunk.uvs.push([Vec2::new(8.0, 6.0); 6]);
            // for y in 0..surface.round() as usize {
            //     voxels.push(Vec3::new(x as f32, y as f32 - 80.0, z as f32));
            // }
        }
    }
    assert!(world.loaded_chunks.insert((x, z), chunk).is_none());
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
