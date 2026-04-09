pub struct TuningScale {
    value: usize,
    max: usize,
}

impl TuningScale {
    pub fn new(max: usize) -> TuningScale {
        TuningScale {
            value: 0,
            max: max - 1,
        }
    }

    pub fn set(&mut self, value: usize) -> usize {
        if value <= self.max {
            self.value = value;
        } else {
            self.value = self.max;
        }
        self.value
    }

    pub fn get(&self) -> usize {
        self.value
    }

    pub fn increment(&mut self) -> usize {
        let mut value = self.get();
        value += 1;
        self.set(value)
    }

    pub fn decrement(&mut self) -> usize {
        let mut value = self.get();
        value -= 1;
        self.set(value)
    }
}
