pub trait Helpers {}

#[derive(Default)]
pub struct HelpersImpl;

impl HelpersImpl {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Helpers for HelpersImpl {}
