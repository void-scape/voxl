use crate::shader::uniform;
use glam::{Mat4, Vec3};
use glazer::glow::{self, HasContext};

pub struct VoxelRenderer {
    shader: glow::Program,
    texture: glow::Texture,
    instances: glow::Buffer,
    vao: glow::VertexArray,
    _vbo: glow::Buffer,
    _ebo: glow::Buffer,
}

impl VoxelRenderer {
    pub fn new(gl: &glow::Context, width: usize, height: usize) -> Self {
        #[rustfmt::skip]
        let vertices: [f32; 192] = [
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
        let indices = [
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

        unsafe {
            let vao = gl.create_vertex_array().unwrap();
            let instances = gl.create_buffer().unwrap();
            let vbo = gl.create_buffer().unwrap();
            let ebo = gl.create_buffer().unwrap();

            gl.bind_vertex_array(Some(vao));

            gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
            let data =
                core::slice::from_raw_parts(vertices.as_ptr() as *const u8, vertices.len() * 4);
            gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, data, glow::STATIC_DRAW);

            gl.bind_buffer(glow::ELEMENT_ARRAY_BUFFER, Some(ebo));
            let data =
                core::slice::from_raw_parts(indices.as_ptr() as *const u8, indices.len() * 4);
            gl.buffer_data_u8_slice(glow::ELEMENT_ARRAY_BUFFER, data, glow::STATIC_DRAW);

            let stride = 8 * 4;
            gl.vertex_attrib_pointer_f32(0, 3, glow::FLOAT, false, stride, 0);
            gl.enable_vertex_attrib_array(0);
            gl.vertex_attrib_pointer_f32(1, 3, glow::FLOAT, false, stride, 3 * 4);
            gl.enable_vertex_attrib_array(1);
            gl.vertex_attrib_pointer_f32(2, 2, glow::FLOAT, false, stride, 6 * 4);
            gl.enable_vertex_attrib_array(2);

            gl.bind_buffer(glow::ARRAY_BUFFER, Some(instances));
            gl.vertex_attrib_pointer_f32(3, 3, glow::FLOAT, false, 12, 0);
            gl.vertex_attrib_divisor(3, 1);
            gl.enable_vertex_attrib_array(3);

            gl.bind_vertex_array(None);
            gl.bind_buffer(glow::ARRAY_BUFFER, None);

            let shader = crate::compile_shader!(gl, "shaders/voxel.vert", "shaders/voxel.frag");
            gl.use_program(Some(shader));
            uniform(gl, shader, "proj", |location| {
                let proj_matrix = Mat4::perspective_rh_gl(
                    100f32.to_radians(),
                    width as f32 / height as f32,
                    1.0,
                    1_000.0,
                );
                gl.uniform_matrix_4_f32_slice(location, false, &proj_matrix.to_cols_array());
            });
            uniform(gl, shader, "ambient_brightness", |location| {
                gl.uniform_1_f32(location, 0.1);
            });
            let texture = load_texture(gl, &[255; 3], 1, 1);

            Self {
                shader,
                texture,
                instances,
                vao,
                _vbo: vbo,
                _ebo: ebo,
            }
        }
    }

    pub fn resize(&self, gl: &glow::Context, width: usize, height: usize) {
        unsafe {
            gl.use_program(Some(self.shader));
            uniform(gl, self.shader, "proj", |location| {
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

    pub fn bind_view(&self, gl: &glow::Context, _camera_translation: Vec3, view: Mat4) {
        unsafe {
            gl.use_program(Some(self.shader));
            uniform(gl, self.shader, "view", |location| {
                gl.uniform_matrix_4_f32_slice(location, false, &view.to_cols_array());
            });
        }
    }

    pub fn bind_light_source(&self, gl: &glow::Context, translation: Vec3) {
        unsafe {
            gl.use_program(Some(self.shader));
            uniform(gl, self.shader, "light_source", |location| {
                gl.uniform_3_f32(location, translation.x, translation.y, translation.z);
            });
        }
    }

    pub fn render_batch(&self, gl: &glow::Context, translations: &[Vec3]) {
        unsafe {
            gl.enable(glow::DEPTH_TEST);
            gl.depth_func(glow::LESS);
            gl.enable(glow::CULL_FACE);
            gl.cull_face(glow::BACK);
            gl.front_face(glow::CCW);
            gl.use_program(Some(self.shader));

            gl.bind_texture(glow::TEXTURE_2D, Some(self.texture));
            gl.bind_vertex_array(Some(self.vao));

            gl.bind_buffer(glow::COPY_WRITE_BUFFER, Some(self.instances));
            let data = core::slice::from_raw_parts(
                translations.as_ptr() as *const u8,
                core::mem::size_of_val(translations),
            );
            assert_eq!(
                core::mem::size_of_val(translations),
                translations.len() * 12
            );
            gl.buffer_data_u8_slice(glow::COPY_WRITE_BUFFER, data, glow::DYNAMIC_DRAW);
            gl.draw_elements_instanced(
                glow::TRIANGLES,
                36,
                glow::UNSIGNED_INT,
                0,
                translations.len() as i32,
            );

            gl.use_program(None);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.bind_vertex_array(None);
            gl.disable(glow::CULL_FACE);
        }
    }
}

fn load_texture(gl: &glow::Context, bytes: &[u8], width: u32, height: u32) -> glow::Texture {
    unsafe {
        let texture = gl.create_texture().unwrap();
        gl.bind_texture(glow::TEXTURE_2D, Some(texture));
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::REPEAT as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::REPEAT as i32);
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MIN_FILTER,
            glow::LINEAR as i32,
        );
        gl.tex_parameter_i32(
            glow::TEXTURE_2D,
            glow::TEXTURE_MAG_FILTER,
            glow::LINEAR as i32,
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
    }
}
