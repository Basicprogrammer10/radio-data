use crate::coding::BinEncoder;

pub struct Context {
    pub encode: BinEncoder,
    pub last_val: Option<f32>,
    pub hold_time: u8,
}

impl Context {
    const HOLD_TIME: u8 = 2;

    pub fn new(encode: BinEncoder) -> Self {
        Self {
            encode,
            last_val: None,
            hold_time: 0,
        }
    }

    pub fn next(&mut self) -> Option<f32> {
        self.encode.next()

        // if let Some(i) = self.last_val {
        //     if self.hold_time > 0 {
        //         self.hold_time -= 1;
        //         return Some(i);
        //     }

        //     self.last_val.take();
        // }

        // let out = match self.encode.next() {
        //     Some(i) => i,
        //     _ => return None,
        // };
        // self.last_val = Some(out);
        // self.hold_time = Self::HOLD_TIME;
        // Some(out)
    }
}
