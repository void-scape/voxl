use glazer::glow::{self, HasContext};

pub fn uniform<F: FnOnce(Option<&glow::UniformLocation>)>(
    gl: &glow::Context,
    program: glow::Program,
    uniform_ident: &str,
    f: F,
) {
    unsafe {
        match gl.get_uniform_location(program, uniform_ident) {
            Some(location) => f(Some(&location)),
            None => glazer::log!("[ERROR] failed to find location of uniform {uniform_ident}"),
        }
    }
}

#[macro_export]
macro_rules! compile_shader {
    ($gl:ident, $vert:literal, $frag:literal) => {
        $crate::shader::compile_shader(
            $gl,
            #[cfg(not(target_arch = "wasm32"))]
            concat!("#version 330 core\n", include_str!($vert),),
            #[cfg(target_arch = "wasm32")]
            concat!("#version 300 es\n", include_str!($vert),),
            #[cfg(not(target_arch = "wasm32"))]
            concat!("#version 330 core\n", include_str!($frag),),
            #[cfg(target_arch = "wasm32")]
            concat!(
                "#version 300 es\nprecision mediump float;\n",
                include_str!($frag),
            ),
        )
    };
}

// https://learnopengl.com/Getting-started/Shaders
pub fn compile_shader(gl: &glow::Context, vertex: &str, fragment: &str) -> glow::Program {
    unsafe {
        let vert = gl.create_shader(glow::VERTEX_SHADER).unwrap();
        let frag = gl.create_shader(glow::FRAGMENT_SHADER).unwrap();
        gl.shader_source(vert, vertex);
        gl.shader_source(frag, fragment);

        gl.compile_shader(vert);
        if !gl.get_shader_compile_status(vert) {
            let err = gl.get_shader_info_log(vert);
            glazer::log!("[ERROR] failed to compile vertex shader: {err}");
            std::process::exit(1);
        }

        gl.compile_shader(frag);
        if !gl.get_shader_compile_status(frag) {
            let err = gl.get_shader_info_log(frag);
            glazer::log!("[ERROR] failed to compile fragment shader: {err}");
            std::process::exit(1);
        }

        let shader = gl.create_program().unwrap();
        gl.attach_shader(shader, vert);
        gl.attach_shader(shader, frag);
        gl.link_program(shader);
        if !gl.get_program_link_status(shader) {
            let err = gl.get_program_info_log(shader);
            glazer::log!("[ERROR] failed to compile fragment shader: {err}");
            std::process::exit(1);
        }

        gl.delete_shader(vert);
        gl.delete_shader(frag);

        shader
    }
}
