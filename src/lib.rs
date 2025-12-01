use crate::voxel::VoxelRenderer;
use glam::{FloatExt, Mat4, Quat, Vec2, Vec3};
use glazer::glow::{self, HasContext};
use glazer::winit::event::{DeviceEvent, KeyEvent, WindowEvent};
use glazer::winit::keyboard::{KeyCode, PhysicalKey};

mod gui;
mod shader;
mod voxel;

#[derive(Default)]
pub struct Memory {
    world: Option<World>,
}

struct World {
    gui: gui::Egui,
    voxel_renderer: VoxelRenderer,
    camera_controller: CameraController,
    voxels: Vec<Vec3>,
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
                world.camera_controller.pitch = world.camera_controller.pitch.clamp(-89.0, 89.0);
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

    let uv_size = 200;
    let voxel_count = 200;
    let speed = 100.0;

    let pitch = -0.599;
    let yaw = 1.569;
    let world = memory.world.get_or_insert_with(|| World {
        gui: gui::Egui::new(event_loop, window, gl),
        voxel_renderer: VoxelRenderer::new(gl, width, height),
        camera_controller: CameraController {
            look_at: look_at(pitch, yaw),
            translation: Vec3::new(
                -(voxel_count as f32 / 2.0),
                -(voxel_count as f32 / 3.0),
                10.0,
            ),
            pitch,
            yaw,
            ..Default::default()
        },
        voxels: {
            let mut voxels = Vec::with_capacity(voxel_count * voxel_count);
            for z in 0..voxel_count {
                for x in 0..voxel_count {
                    let uv = Vec2::new(x as f32 / uv_size as f32, z as f32 / uv_size as f32);
                    let low = perlin(uv) * 0.5 + 0.5;
                    let medium = perlin(uv * 2.0) * 0.5 + 0.5;
                    let high = perlin(uv * 6.0) * 0.5 + 0.5;
                    let surface = low * 80.0 + medium * 40.0 + high * 30.0;
                    for y in 0..surface.round() as usize {
                        voxels.push(Vec3::new(x as f32, y as f32 - 80.0, z as f32));
                    }
                }
            }
            voxels
        },
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
        world.voxel_renderer.render_batch(gl, &world.voxels);
    }

    world.gui.show(|ui| {
        egui::Window::new("Voxl").show(ui, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.heading("Hello World!");
                if ui.button("Quit").clicked() {
                    std::process::exit(0);
                }
            })
        });
    });
    world.gui.paint();
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
