use glam::{Mat4, Quat, Vec3};
use glazer::winit::{
    event::{DeviceEvent, KeyEvent, WindowEvent},
    keyboard::{KeyCode, PhysicalKey},
};

#[derive(Default)]
pub struct Camera {
    left: bool,
    right: bool,
    forward: bool,
    back: bool,
    up: bool,
    down: bool,
    enabled: bool,
    pub translation: Vec3,
    yaw: f32,
    pitch: f32,
    look_at: Vec3,
    speed: f32,
}

impl Camera {
    pub fn new(speed: f32, translation: Vec3, pitch: f32, yaw: f32) -> Self {
        Self {
            enabled: true,
            look_at: look_at(pitch, yaw),
            translation,
            pitch,
            yaw,
            speed,
            ..Default::default()
        }
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::from_quat(Quat::look_to_rh(self.look_at, Vec3::Y))
            * Mat4::from_translation(self.translation)
    }
}

pub fn handle_input(input: &glazer::Input, camera: &mut Camera) {
    match input {
        glazer::Input::Window(event) => {
            if let WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state,
                        repeat: false,
                        ..
                    },
                ..
            } = event
            {
                match code {
                    KeyCode::KeyI => {
                        if state.is_pressed() {
                            camera.enabled = !camera.enabled;
                        }
                    }
                    KeyCode::KeyA => {
                        camera.left = state.is_pressed();
                    }
                    KeyCode::KeyD => {
                        camera.right = state.is_pressed();
                    }
                    KeyCode::KeyW => {
                        camera.forward = state.is_pressed();
                    }
                    KeyCode::KeyS => {
                        camera.back = state.is_pressed();
                    }
                    KeyCode::Space | KeyCode::ControlLeft => {
                        camera.up = state.is_pressed();
                    }
                    KeyCode::ShiftLeft => {
                        camera.down = state.is_pressed();
                    }
                    _ => {}
                }
            }
        }
        glazer::Input::Device(event) => {
            // https://learnopengl.com/code_viewer_gh.php?code=src/1.getting_started/7.3.camera_mouse_zoom/camera_mouse_zoom.cpp
            if let DeviceEvent::MouseMotion { delta } = event
                && camera.enabled
            {
                let sensitivity = 0.005;
                camera.yaw += delta.0 as f32 * sensitivity;
                camera.pitch -= delta.1 as f32 * sensitivity;
                camera.pitch = camera.pitch.clamp(
                    -std::f32::consts::FRAC_PI_2 + 0.0001,
                    std::f32::consts::FRAC_PI_2 - 0.0001,
                );
                let yaw = camera.yaw;
                let pitch = camera.pitch;
                camera.look_at = look_at(pitch, yaw);
            }
        }
    }
}

fn look_at(pitch: f32, yaw: f32) -> Vec3 {
    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize_or(Vec3::Z)
}

pub fn update(camera: &mut Camera, delta: f32) {
    if camera.enabled {
        // https://sotrh.github.io/learn-wgpu/intermediate/tutorial12-camera/#the-projection
        let (yaw_sin, yaw_cos) = camera.yaw.sin_cos();
        let forward = Vec3::new(yaw_cos, 0.0, yaw_sin).normalize();
        let right = Vec3::new(-yaw_sin, 0.0, yaw_cos).normalize();
        let mut dxz = Vec3::ZERO;
        dxz += forward * (camera.forward as u32 as f32 - camera.back as u32 as f32);
        dxz += right * (camera.right as u32 as f32 - camera.left as u32 as f32);
        camera.translation += dxz.normalize_or_zero() * -camera.speed * delta;

        if camera.down {
            camera.translation.y -= -camera.speed * delta;
        }
        if camera.up {
            camera.translation.y += -camera.speed * delta;
        }
    }
}
