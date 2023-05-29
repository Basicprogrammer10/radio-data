use num_traits::Float;

pub struct RingBuffer<T, const SIZE: usize> {
    pub data: [T; SIZE],
    pub index: usize,
    pub filled: bool,
}

impl<T: Default + Copy, const SIZE: usize> RingBuffer<T, SIZE> {
    pub fn new() -> Self {
        Self {
            data: [T::default(); SIZE],
            index: 0,
            filled: false,
        }
    }

    pub fn push(&mut self, val: T) {
        self.data[self.index] = val;
        let idx = self.index + 1;
        self.index = idx % SIZE;

        if !self.filled && idx == SIZE {
            self.filled = true;
        }
    }
}

impl<T: Float, const SIZE: usize> RingBuffer<T, SIZE> {
    pub fn min(&self) -> T {
        self.data.iter().fold(T::infinity(), |a, &b| a.min(b))
    }

    pub fn max(&self) -> T {
        self.data.iter().fold(T::neg_infinity(), |a, &b| a.max(b))
    }

    pub fn avg(&self) -> T {
        let sum = self.data.iter().fold(T::zero(), |a, &b| a + b);
        sum / T::from(SIZE).unwrap()
    }
}
