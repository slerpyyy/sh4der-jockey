use std::ops::{Add, Div};

/// A statically sized ring buffer for computing a running average.
#[derive(Clone, Copy)]
pub struct RunningAverage<T, const SIZE: usize> {
    pub buffer: [T; SIZE],
    pub index: usize,
}

impl<T, const SIZE: usize> std::fmt::Debug for RunningAverage<T, SIZE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(RunningAverage))
            .field("buffer", &"[..]")
            .field("index", &self.index)
            .finish()
    }
}

impl<T, const SIZE: usize> RunningAverage<T, SIZE>
where
    T: Add<Output = T> + Div<Output = T> + From<u8> + Copy,
{
    /// Creates a new ring buffer which is filled with zeros.
    pub fn new() -> Self {
        Self {
            buffer: [T::from(0); SIZE],
            index: 0,
        }
    }

    /// Appends a new value to the ring buffer.
    ///
    /// If there are more than `SIZE` elements in the ring buffer already,
    /// the oldest element will be overwritten.
    pub fn push(&mut self, value: T) {
        self.buffer[self.index] = value;
        self.index = (self.index + 1) % SIZE;
    }

    /// Computes the average of all elements in the ring buffer.
    ///
    /// This is done recursively to ensure high precision even for floating
    /// point values. The result may be slightly weighted towards the beginning
    /// of the buffer if `SIZE` is not a power of two.
    pub fn get(&self) -> T {
        fn recurse<T>(slice: &[T]) -> T
        where
            T: Add<Output = T> + Div<Output = T> + From<u8> + Copy,
        {
            if let &[x] = slice {
                return x;
            }

            let mid = slice.len() / 2;
            let (a, b) = slice.split_at(mid);
            (recurse(a) + recurse(b)) / T::from(2)
        }

        recurse(&self.buffer)
    }
}

#[cfg(test)]
mod test {
    use super::RunningAverage;

    #[test]
    fn running_average_init() {
        let ra = RunningAverage::<i32, 4>::new();
        assert_eq!(ra.get(), 0);
    }

    #[test]
    fn running_average_simple() {
        let mut ra = RunningAverage::<f64, 16>::new();

        ra.push(16.0);
        assert_eq!(ra.get(), 1.0);

        ra.push(16.0);
        assert_eq!(ra.get(), 2.0);
    }

    #[test]
    fn running_average_cycle() {
        let mut ra = RunningAverage::<f32, 8>::new();

        for _ in 0..25 {
            ra.push(2.0);
            ra.push(4.0);
        }
        assert_eq!(ra.get(), 3.0);
    }
}
