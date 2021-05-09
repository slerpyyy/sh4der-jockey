/// A statically sized ring buffer for computing a running average.
#[derive(Clone)]
pub struct RingBuffer<T> {
    pub buffer: Vec<T>,
    pub index: usize,
    pub size: usize,
}

impl<T> std::fmt::Debug for RingBuffer<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(RingBuffer))
            .field("buffer", &"[..]")
            .field("index", &self.index)
            .field("size", &self.size)
            .finish()
    }
}

impl<T> RingBuffer<T>
where
    T: Copy + Default,
{
    pub fn new(size: usize) -> Self {
        Self {
            buffer: vec![T::default(); size],
            index: 0,
            size,
        }
    }

    pub fn push(&mut self, val: &T) {
        self.buffer[self.index] = *val;
        self.index = (self.index + 1) % self.size;
    }

    #[allow(dead_code)]
    pub fn push_slice(&mut self, val: &[T]) {
        for x in val.iter() {
            self.push(x);
        }
    }

    // start at oldest index
    // end at freshest index
    pub fn get(&self, i: usize) -> T {
        let idx = (self.index + i) % self.size;
        self.buffer[idx]
    }

    pub fn copy_to_slice(&self, vec: &mut [T]) {
        for (k, slot) in vec.iter_mut().enumerate() {
            *slot = self.get(k);
        }
    }
}

#[cfg(test)]
mod test {
    use super::RingBuffer;

    #[test]
    fn create() {
        let rb = RingBuffer::<f32>::new(8192);
        assert_eq!(rb.size, rb.buffer.len());
    }

    #[test]
    fn push() {
        let mut rb = RingBuffer::<f32>::new(8192);
        rb.push(&12_f32);
        assert_eq!(rb.get(rb.size - 1), 12_f32);
    }

    #[test]
    fn push_slice() {
        let mut rb = RingBuffer::<f32>::new(8192);
        let slice = [13_f32; 8192];
        rb.push_slice(&slice);
        for i in 0..8192 {
            assert_eq!(rb.get(i), 13_f32);
        }
    }

    #[test]
    fn copy_to_slice() {
        let mut rb = RingBuffer::<f32>::new(8192);
        let pre_slice = [13_f32; 8192];
        rb.push_slice(&pre_slice);

        let mut ret_slice = [0_f32; 8192];
        rb.copy_to_slice(&mut ret_slice);

        for i in 0..8192 {
            assert_eq!(pre_slice[i], ret_slice[i]);
        }
    }
}
