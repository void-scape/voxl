fn main() {
    glazer::run(
        voxl::Memory::default(),
        2560,
        1440,
        voxl::handle_input,
        voxl::update_and_render,
        glazer::debug_target(),
    );
}
