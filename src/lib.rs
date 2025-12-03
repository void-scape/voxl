use crate::camera::Camera;
use crate::chunk::Chunks;
use crate::voxel::VoxelRenderer;
use glam::Vec3;
use glazer::glow::{self, HasContext};
use glazer::winit::event::{KeyEvent, WindowEvent};
use glazer::winit::keyboard::{KeyCode, PhysicalKey};

mod camera;
mod chunk;
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
    wireframes: bool,
    fog: bool,
    view_distance: usize,
    camera: Camera,
    chunks: Chunks,
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

    if let glazer::Input::Window(event) = &input {
        if world.gui.handle_input(window, event) {
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
                KeyCode::KeyF if state.is_pressed() => {
                    if world.fog {
                        world.voxel_renderer.bind_view_distance(
                            gl,
                            world.view_distance,
                            chunk::CHUNK_SIZE,
                        );
                    } else {
                        world.voxel_renderer.bind_view_distance(
                            gl,
                            world.view_distance * 10,
                            chunk::CHUNK_SIZE,
                        );
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

    camera::handle_input(&input, &mut world.camera);
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
    let world = memory.world.get_or_insert_with(|| World {
        gui: gui::Egui::new(event_loop, window, gl),
        voxel_renderer: VoxelRenderer::new(gl, width, height, view_distance, "assets/terrain.png"),
        wireframes: false,
        fog: false,
        view_distance,
        camera: Camera::new(100.0, Vec3::ZERO, 0.0, 0.0),
        chunks: Chunks::from_noise(vec![(1.0, 80.0), (2.0, 40.0), (6.0, 30.0)]),
    });

    camera::update(&mut world.camera, delta);
    chunk::update(&mut world.chunks, world.view_distance, &world.camera);

    unsafe {
        gl.clear_color(0.0, 0.0, 0.0, 1.0);
        gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

        if world.wireframes {
            gl.polygon_mode(glow::FRONT_AND_BACK, glow::LINE);
        } else {
            gl.polygon_mode(glow::FRONT_AND_BACK, glow::FILL);
        }

        chunk::render(world, gl);
    }

    unsafe {
        gl.polygon_mode(glow::FRONT_AND_BACK, glow::FILL);
    }
    world.gui.show(|ui| {
        egui::Window::new("Voxl").show(ui, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.add(egui::Slider::new(&mut world.view_distance, 1..=32).text("View Distance"));
                chunk::ui(ui, &mut world.chunks, world.view_distance, &world.camera);
            })
        });
    });
    world.gui.paint();
}
