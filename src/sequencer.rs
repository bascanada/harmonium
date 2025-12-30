pub struct Sequencer {
    pub steps: usize,
    pub pulses: usize,
    pub pattern: Vec<bool>,
    pub current_step: usize,
    pub bpm: f32,
    pub last_tick_time: f64,
}

impl Sequencer {
    pub fn new(steps: usize, pulses: usize, bpm: f32) -> Self {
        Sequencer {
            steps,
            pulses,
            pattern: generate_euclidean_pattern(steps, pulses),
            current_step: 0,
            bpm,
            last_tick_time: 0.0,
        }
    }

    pub fn tick(&mut self) -> bool {
        let trigger = self.pattern[self.current_step];
        self.current_step = (self.current_step + 1) % self.steps;
        trigger
    }
}

pub fn generate_euclidean_pattern(steps: usize, pulses: usize) -> Vec<bool> {
    if pulses == 0 {
        return vec![false; steps];
    }
    if pulses >= steps {
        return vec![true; steps];
    }

    // Basic implementation of Bjorklund's algorithm logic
    // Start with 'pulses' groups of [1] and 'steps-pulses' groups of [0]
    let mut pattern: Vec<Vec<u8>> = Vec::new();
    for _ in 0..pulses {
        pattern.push(vec![1]);
    }
    for _ in 0..(steps - pulses) {
        pattern.push(vec![0]);
    }

    let mut count = std::cmp::min(pulses, steps - pulses);
    let mut remainder = pattern.len() - count;

    while remainder > 1 && count > 0 {
        for i in 0..count {
            let last = pattern.pop().unwrap();
            pattern[i].extend(last);
        }
        remainder = pattern.len() - count;
        count = std::cmp::min(count, remainder);
    }

    // Flatten the pattern
    let mut result = Vec::new();
    for group in pattern {
        for val in group {
            result.push(val == 1);
        }
    }
    
    // The standard Bjorklund might need rotation to match musical expectations (like starting on a beat),
    // but this mathematically correct distribution is a good start.
    result
}
