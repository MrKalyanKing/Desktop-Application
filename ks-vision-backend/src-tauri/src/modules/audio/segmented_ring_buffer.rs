pub struct SegmentedRingBuffer {
    samples: Vec<f32>,
    capacity: usize,
}

impl SegmentedRingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            samples: Vec::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push_slice(&mut self, data: &[f32]) {
        self.samples.extend_from_slice(data);
        if self.samples.len() > self.capacity {
            let excess = self.samples.len() - self.capacity;
            self.samples.drain(0..excess);
        }
    }

    pub fn get_samples(&self) -> Vec<f32> {
        self.samples.clone()
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }

    pub fn rewind_samples(&mut self, count: usize) {
        if count >= self.samples.len() {
            self.samples.clear();
        } else {
            self.samples.truncate(self.samples.len() - count);
        }
    }

    pub fn discard_front_samples(&mut self, count: usize) {
        if count >= self.samples.len() {
            self.samples.clear();
        } else {
            self.samples.drain(0..count);
        }
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}
