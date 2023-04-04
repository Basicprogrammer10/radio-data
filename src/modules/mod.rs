use std::{sync::Arc, rc::Rc};

use cpal::SupportedStreamConfig;

mod range_test;

pub fn modules() -> [Box<Arc<dyn Module + Send + Sync + 'static>>; 1] {
    [Box::new(range_test::RangeTest::new())]
}

pub trait Module {
    fn name(&self) -> &'static str;
    fn input(&self, _input: &[f32], _channels: Arc<SupportedStreamConfig>) {}
    fn output(&self, _output: &mut [f32], _channels: Arc<SupportedStreamConfig>) {}
}
