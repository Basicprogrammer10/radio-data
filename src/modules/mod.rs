mod range_test;

fn modules() -> [Box<dyn Module>; 1] {
    [Box::new(range_test::RangeTest::default())]
}

pub trait Module {
    fn name(&self) -> &'static str;
    fn input(&self, _input: &[f32]) {}
    fn output(&self, _output: &mut [f32]) {}
}
