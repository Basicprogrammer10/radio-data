//! A Ring Buffer implementation.
//! A ring buffer is a type of buffer with a set length that will
//! overwrite the oldest values it holds when new ones are added.

use num_traits::Float;

/// Ring buffer that can hold any type.
/// The size of the buffer is defined as SIZE at compile time so it can be stored on the stack.
pub struct RingBuffer<T, const SIZE: usize> {
    pub data: [T; SIZE],
    pub index: usize,
    pub filled: bool,
}

impl<T: Default + Copy, const SIZE: usize> RingBuffer<T, SIZE> {
    /// Create a new RingBuffer using T::default().
    pub fn new() -> Self {
        Self {
            data: [T::default(); SIZE],
            index: 0,
            filled: false,
        }
    }
}

impl<T, const SIZE: usize> RingBuffer<T, SIZE> {
    /// Adds a new value to the buffer
    pub fn push(&mut self, val: T) {
        self.data[self.index] = val;
        let idx = self.index + 1;
        self.index = idx % SIZE;

        if !self.filled && idx == SIZE {
            self.filled = true;
        }
    }

    /// Gets the values that have actually been set.
    /// If self.filled is true, this will be the whole buffer,
    /// if not it will just be the values added by the user.
    fn real(&self) -> &[T] {
        if self.filled {
            return &self.data;
        }

        &self.data[..self.index]
    }
}

impl<T: Float, const SIZE: usize> RingBuffer<T, SIZE> {
    /// Get the min value from the buffer.
    /// Inf is retune if there are no values.
    pub fn min(&self) -> T {
        self.real().iter().fold(T::infinity(), |a, &b| a.min(b))
    }

    /// Get the max value from the buffer.
    /// -Inf is retune if there are no values.
    pub fn max(&self) -> T {
        self.real().iter().fold(T::neg_infinity(), |a, &b| a.max(b))
    }

    /// Get the average of the values from the buffer.
    pub fn avg(&self) -> T {
        let real = self.real();
        let sum = real.iter().fold(T::zero(), |a, &b| a + b);
        sum / T::from(real.len()).unwrap()
    }
}

#[cfg(test)]
mod test {
    use super::RingBuffer;

    #[test]
    fn test_ring_buffer_real() {
        let mut ring = RingBuffer::<f32, 10>::new();
        ring.push(2.0);
        ring.push(4.0);

        assert_eq!(ring.real(), &[2.0, 4.0]);
        assert_eq!(ring.min(), 2.0);
        assert_eq!(ring.max(), 4.0);
        assert_eq!(ring.avg(), 3.0);
    }

    #[test]
    fn test_ring_buffer_real_full() {
        let mut ring = RingBuffer::<f32, 10>::new();
        for i in 0..10 {
            ring.push(i as f32);
        }

        assert_eq!(
            ring.real(),
            &[
                0.0 as f32, 1.0 as f32, 2.0 as f32, 3.0 as f32, 4.0 as f32, 5.0 as f32, 6.0 as f32,
                7.0 as f32, 8.0 as f32, 9.0 as f32,
            ]
        );
    }
}
