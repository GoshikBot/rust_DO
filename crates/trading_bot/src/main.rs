fn main() {
    log4rs::init_file("step_log4rs.yml", Default::default()).unwrap();

    log::info!(target: "step", "New message");
}
