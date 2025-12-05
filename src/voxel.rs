use crate::shader::uniform;
use glam::{Mat4, Vec2, Vec3};
use glazer::glow::{self, HasContext};
use image::EncodableLayout;

pub struct Lighting {
    pub light_source: Vec3,
    pub light_color: Vec3,
    pub ambient_brightness: f32,
}

#[derive(Clone, Copy)]
pub struct VoxelInstanceBuffer {
    buffer: glow::Buffer,
    instances: usize,
}

pub struct VoxelRenderer {
    // main pipeline
    voxel_shader: glow::Program,
    texture_atlas: glow::Texture,
    voxel_vao: glow::VertexArray,
    _voxel_vbo: glow::Buffer,
    atlas_offsets: glow::Buffer,
    // shadow mapping
    shadow_framebuffer: glow::Framebuffer,
    pub shadow_map: glow::Texture,
    shadow_shader: glow::Program,
    shadow_vao: glow::VertexArray,
    _shadow_vbo: glow::Buffer,
    // shared
    _ebo: glow::Buffer,
}

impl VoxelRenderer {
    const SHADOW_SIZE: i32 = 1024;

    pub fn new(gl: &glow::Context, width: usize, height: usize, textures: &str) -> Self {
        unsafe {
            // VOXEL

            let voxel_vao = gl.create_vertex_array().unwrap();
            let voxel_vbo = gl.create_buffer().unwrap();
            let ebo = gl.create_buffer().unwrap();
            let atlas_offsets = gl.create_buffer().unwrap();

            gl.bind_vertex_array(Some(voxel_vao));

            gl.bind_buffer(glow::ARRAY_BUFFER, Some(voxel_vbo));
            let data = core::slice::from_raw_parts(
                VOXEL_VERTICES.as_ptr() as *const u8,
                VOXEL_VERTICES.len() * 4,
            );
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, data, glow::STATIC_DRAW);

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            let data =
                core::slice::from_raw_parts(INDICES.as_ptr() as *const u8, INDICES.len() * 4);
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, data, glow::STATIC_DRAW);

            let stride = 8 * 4;
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, stride, 3 * 4);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(2, 2, glow::FLOAT, false, stride, 6 * 4);
            gl.enable_vertex_attrib_array(2);

            // gl.bind_buffer(glow::ARRAY_BUFFER, Some(atlas_offsets));
            // gl.vertex_attrib_pointer_f32(3, 2, glow::FLOAT, false, 8, 0);
            // gl.enable_vertex_attrib_array(3);

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);

            let (texture_atlas, texture_atlas_size) = load_image(gl, textures);
            let voxel_shader =
                crate::compile_shader!(gl, "shaders/voxel.vert", "shaders/voxel.frag");
            gl.use_program(Some(voxel_shader));
            uniform(gl, voxel_shader, "proj", |location| {
                let proj_matrix = Mat4::perspective_rh_gl(
                    100f32.to_radians(),
                    width as f32 / height as f32,
                    1.0,
                    1_000.0,
                );
                gl.uniform_matrix_4_f32_slice(location, false, &proj_matrix.to_cols_array());
            });
            uniform(gl, voxel_shader, "atlas_size", |location| {
                gl.uniform_2_f32(location, texture_atlas_size.0, texture_atlas_size.1);
            });
            uniform(gl, voxel_shader, "texture_size", |location| {
                gl.uniform_2_f32(location, 64.0, 64.0);
            });
            uniform(gl, voxel_shader, "texture_atlas", |location| {
                gl.uniform_1_i32(location, 0);
            });
            uniform(gl, voxel_shader, "shadow_map", |location| {
                gl.uniform_1_i32(location, 1);
            });

            // SHADOW

            let shadow_vao = gl.create_vertex_array().unwrap();
            let shadow_vbo = gl.create_buffer().unwrap();

            gl.bind_vertex_array(Some(shadow_vao));

            gl.bind_buffer(glow::ARRAY_BUFFER, Some(shadow_vbo));
            let data = core::slice::from_raw_parts(
                SHADOW_VERTICES.as_ptr() as *const u8,
                SHADOW_VERTICES.len() * 4,
            );
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, data, glow::STATIC_DRAW);

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));

            let stride = 3 * 4;
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);
            gl.enable_vertex_attrib_array(0);

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);

            let shadow_shader =
                crate::compile_shader!(gl, "shaders/shadow.vert", "shaders/shadow.frag");
            let shadow_framebuffer = gl.create_framebuffer().unwrap();
            let shadow_map = Self::create_shadow_map(gl);

            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(shadow_framebuffer));
            gl.framebuffer_texture_2d(
                glow::FRAMEBUFFER,
                glow::DEPTH_ATTACHMENT,
                glow::TEXTURE_2D,
                Some(shadow_map),
                0,
            );
            // tell the framebuffer to not render color data
            gl.draw_buffer(glow::NONE);
            gl.read_buffer(glow::NONE);
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);

            Self {
                voxel_shader,
                texture_atlas,
                voxel_vao,
                _voxel_vbo: voxel_vbo,
                atlas_offsets,
                // shared
                shadow_framebuffer,
                shadow_map,
                shadow_shader,
                shadow_vao,
                _shadow_vbo: shadow_vbo,
                //
                _ebo: ebo,
            }
        }
    }

    pub fn resize(&self, gl: &glow::Context, width: usize, height: usize) {
        unsafe {
            gl.use_program(Some(self.voxel_shader));
            uniform(gl, self.voxel_shader, "proj", |location| {
                let proj_matrix = Mat4::perspective_rh_gl(
                    90f32.to_radians(),
                    width as f32 / height as f32,
                    0.1,
                    1_000.0,
                );
                gl.uniform_matrix_4_f32_slice(location, false, &proj_matrix.to_cols_array());
            });
        }
    }

    pub fn generate_translation_buffer(
        &self,
        gl: &glow::Context,
        translations: &[Vec3],
    ) -> VoxelInstanceBuffer {
        unsafe {
            let buffer = gl.create_buffer().unwrap();
            gl.bind_buffer(glow::COPY_WRITE_BUFFER, Some(buffer));
            let data = core::slice::from_raw_parts(
                translations.as_ptr() as *const u8,
                core::mem::size_of_val(translations),
            );
            gl.buffer_data_u8_slice(glow::COPY_WRITE_BUFFER, data, glow::STATIC_DRAW);
            VoxelInstanceBuffer {
                buffer,
                instances: translations.len(),
            }
        }
    }

    pub fn render_pass<'a>(
        &self,
        gl: &glow::Context,
        width: usize,
        height: usize,
        lighting: Lighting,
        view: Mat4,
        fog_near: f32,
        fog_far: f32,
        translations_for_shadow_pass: impl Iterator<Item = VoxelInstanceBuffer>,
        translations_and_atlas_uvs: impl Iterator<Item = (VoxelInstanceBuffer, &'a [[Vec2; 6]])>,
    ) {
        // write uniform data
        unsafe {
            gl.use_program(Some(self.voxel_shader));
            uniform(gl, self.voxel_shader, "view", |location| {
                gl.uniform_matrix_4_f32_slice(location, false, &view.to_cols_array());
            });

            uniform(gl, self.voxel_shader, "light_source", |location| {
                gl.uniform_3_f32(
                    location,
                    lighting.light_source.x,
                    lighting.light_source.y,
                    lighting.light_source.z,
                );
            });
            uniform(gl, self.voxel_shader, "light_color", |location| {
                gl.uniform_3_f32(
                    location,
                    lighting.light_color.x,
                    lighting.light_color.y,
                    lighting.light_color.z,
                );
            });
            uniform(gl, self.voxel_shader, "ambient_brightness", |location| {
                gl.uniform_1_f32(location, lighting.ambient_brightness);
            });

            uniform(gl, self.voxel_shader, "fog_near", |location| {
                gl.uniform_1_f32(location, fog_near);
            });
            uniform(gl, self.voxel_shader, "fog_far", |location| {
                gl.uniform_1_f32(location, fog_far);
            });

            // let camera_translation = view.w_axis.xyz();
            let size = Self::SHADOW_SIZE as f32 / 4.0;
            let proj = Mat4::orthographic_rh_gl(-size, size, -size, size, 1.0, 1_000.0);
            let view = Mat4::look_at_rh(
                // TODO: how the hell do I move this?
                // lighting.light_source + camera_translation,
                // Vec3::ZERO + camera_translation,
                lighting.light_source,
                Vec3::ZERO,
                Vec3::Y,
            );
            let light_space = proj.mul_mat4(&view);

            uniform(gl, self.voxel_shader, "light_space", |location| {
                gl.uniform_matrix_4_f32_slice(location, false, &light_space.to_cols_array());
            });

            gl.use_program(Some(self.shadow_shader));
            uniform(gl, self.shadow_shader, "light_space", |location| {
                gl.uniform_matrix_4_f32_slice(location, false, &light_space.to_cols_array());
            });
        }

        // shadow pass
        unsafe {
            // render to depth buffer with the target resolution
            gl.viewport(0, 0, Self::SHADOW_SIZE, Self::SHADOW_SIZE);
            gl.bind_framebuffer(glow::FRAMEBUFFER, Some(self.shadow_framebuffer));
            gl.clear(glow::DEPTH_BUFFER_BIT);

            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LESS);
            gl.enable(glow::CULL_FACE);
            gl.cull_face(glow::BACK);
            gl.front_face(glow::CCW);
            gl.use_program(Some(self.shadow_shader));
            gl.bind_vertex_array(Some(self.shadow_vao));

            for instances in translations_for_shadow_pass {
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(instances.buffer));
                gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, 12, 0);
                gl.vertex_attrib_divisor(1, 1);
                gl.enable_vertex_attrib_array(1);

                gl.draw_elements_instanced(
                    glow::TRIANGLES,
                    36,
                    glow::UNSIGNED_INT,
                    0,
                    instances.instances as i32,
                );
            }

            // finish render pass and return viewport to the screen resolution
            gl.bind_framebuffer(glow::FRAMEBUFFER, None);
            gl.viewport(0, 0, width as i32, height as i32);
        }

        // voxel pass
        unsafe {
            gl.clear_color(113.0 / 255.0, 197.0 / 255.0, 231.0 / 255.0, 1.0);
            gl.clear(glow::COLOR_BUFFER_BIT | glow::DEPTH_BUFFER_BIT);

            gl.use_program(Some(self.voxel_shader));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture_atlas));
            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, Some(self.shadow_map));
            gl.active_texture(glow::TEXTURE0);
            gl.bind_vertex_array(Some(self.voxel_vao));

            for (instances, atlas_uvs) in translations_and_atlas_uvs {
                gl.bind_buffer(glow::ARRAY_BUFFER, Some(instances.buffer));
                gl.vertex_attrib_pointer_f32(4, 3, glow::FLOAT, false, 12, 0);
                gl.vertex_attrib_divisor(4, 1);
                gl.enable_vertex_attrib_array(4);

                gl.bind_buffer(glow::COPY_WRITE_BUFFER, Some(self.atlas_offsets));
                let data = core::slice::from_raw_parts(
                    atlas_uvs.as_ptr() as *const u8,
                    core::mem::size_of_val(atlas_uvs),
                );
                gl.buffer_data_u8_slice(glow::COPY_WRITE_BUFFER, data, glow::DYNAMIC_DRAW);

                gl.draw_elements_instanced(
                    glow::TRIANGLES,
                    36,
                    glow::UNSIGNED_INT,
                    0,
                    instances.instances as i32,
                );
            }

            gl.use_program(None);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.bind_vertex_array(None);
            gl.disable(glow::CULL_FACE);
        }
    }

    fn create_shadow_map(gl: &glow::Context) -> glow::Texture {
        unsafe {
            let shadow_map = gl.create_texture().unwrap();
            gl.bind_texture(glow::TEXTURE_2D, Some(shadow_map));
            gl.tex_image_2d(
                glow::TEXTURE_2D,
                0,
                glow::DEPTH_COMPONENT as i32,
                Self::SHADOW_SIZE,
                Self::SHADOW_SIZE,
                0,
                glow::DEPTH_COMPONENT,
                glow::FLOAT,
                glow::PixelUnpackData::Slice(None),
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MIN_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(
                glow::TEXTURE_2D,
                glow::TEXTURE_MAG_FILTER,
                glow::NEAREST as i32,
            );
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
            gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
            shadow_map
        }
    }
}

fn load_image(gl: &glow::Context, path: &str) -> (glow::Texture, (f32, f32)) {
    let image = image::open(path).unwrap();
    let width = image.width();
    let height = image.height();
    let rgb = image.to_rgb8();
    let bytes = rgb.as_bytes();

    let texture = unsafe {
        let texture = gl.create_texture().unwrap();
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::NEAREST as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::NEAREST as i32,
        );

        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGB as i32,
            width as i32,
            height as i32,
            0,
            glow::RGB,
            glow::UNSIGNED_BYTE,
            glow::PixelUnpackData::Slice(Some(bytes)),
        );

        texture
    };

    (texture, (width as f32, height as f32))
}

#[rustfmt::skip]
const VOXEL_VERTICES: [f32; 192] = [
    // Back face
     0.5, -0.5, -0.5,  0.0, 0.0, -1.0,  1.0, 0.0,
     0.5,  0.5, -0.5,  0.0, 0.0, -1.0,  1.0, 1.0,
    -0.5, -0.5, -0.5,  0.0, 0.0, -1.0,  0.0, 0.0,
    -0.5,  0.5, -0.5,  0.0, 0.0, -1.0,  0.0, 1.0,
    // Front face
     0.5, -0.5,  0.5,  0.0, 0.0, 1.0,   1.0, 0.0,
     0.5,  0.5,  0.5,  0.0, 0.0, 1.0,   1.0, 1.0,
    -0.5,  0.5,  0.5,  0.0, 0.0, 1.0,   0.0, 1.0,
    -0.5, -0.5,  0.5,  0.0, 0.0, 1.0,   0.0, 0.0,
    // Left face
    -0.5,  0.5, -0.5, -1.0, 0.0, 0.0,   1.0, 1.0,
    -0.5, -0.5, -0.5, -1.0, 0.0, 0.0,   0.0, 1.0,
    -0.5, -0.5,  0.5, -1.0, 0.0, 0.0,   0.0, 0.0,
    -0.5,  0.5,  0.5, -1.0, 0.0, 0.0,   1.0, 0.0,
    // Right face
     0.5,  0.5, -0.5,  1.0, 0.0, 0.0,   1.0, 1.0,
     0.5, -0.5, -0.5,  1.0, 0.0, 0.0,   0.0, 1.0,
     0.5,  0.5,  0.5,  1.0, 0.0, 0.0,   1.0, 0.0,
     0.5, -0.5,  0.5,  1.0, 0.0, 0.0,   0.0, 0.0,
    // Bottom face
     0.5, -0.5, -0.5,  0.0, -1.0, 0.0,  1.0, 1.0,
     0.5, -0.5,  0.5,  0.0, -1.0, 0.0,  1.0, 0.0,
    -0.5, -0.5,  0.5,  0.0, -1.0, 0.0,  0.0, 0.0,
    -0.5, -0.5, -0.5,  0.0, -1.0, 0.0,  0.0, 1.0,
    // Top face
     0.5,  0.5, -0.5,  0.0, 1.0, 0.0,   1.0, 1.0,
     0.5,  0.5,  0.5,  0.0, 1.0, 0.0,   1.0, 0.0,
    -0.5,  0.5, -0.5,  0.0, 1.0, 0.0,   0.0, 1.0,
    -0.5,  0.5,  0.5,  0.0, 1.0, 0.0,   0.0, 0.0,
];

#[rustfmt::skip]
const SHADOW_VERTICES: [f32; 72] = [
    // Back face
     0.5, -0.5, -0.5,
     0.5,  0.5, -0.5,
    -0.5, -0.5, -0.5,
    -0.5,  0.5, -0.5,
    // Front face
     0.5, -0.5,  0.5,
     0.5,  0.5,  0.5,
    -0.5,  0.5,  0.5,
    -0.5, -0.5,  0.5,
    // Left face
    -0.5,  0.5, -0.5,
    -0.5, -0.5, -0.5,
    -0.5, -0.5,  0.5,
    -0.5,  0.5,  0.5,
    // Right face
     0.5,  0.5, -0.5,
     0.5, -0.5, -0.5,
     0.5,  0.5,  0.5,
     0.5, -0.5,  0.5,
    // Bottom face
     0.5, -0.5, -0.5,
     0.5, -0.5,  0.5,
    -0.5, -0.5,  0.5,
    -0.5, -0.5, -0.5,
    // Top face
     0.5,  0.5, -0.5,
     0.5,  0.5,  0.5,
    -0.5,  0.5, -0.5,
    -0.5,  0.5,  0.5,
];

#[rustfmt::skip]
const INDICES: [u32; 36] = [
    // Back face
    1, 0, 2,
    1, 2, 3,
    // Front face
    4, 5, 6,
    4, 6, 7,
    // Left face
    8, 9, 10,
    8, 10, 11,
    // Right face
    12, 14, 13,
    14, 15, 13,
    // Bottom face
    16, 17, 18,
    16, 18, 19,
    // Top face
    20, 22, 21,
    22, 23, 21,
];
