use rand::Rng;

/// RNG abstraction for deterministic tests.
pub trait RngLike: Send {
    fn choose_imposter(&mut self, players_len: usize) -> usize;
    fn choose_topic(&mut self, topics_len: usize) -> usize;
}

/// Production RNG using `rand`.
#[derive(Debug, Default)]
pub struct ProductionRng;

impl RngLike for ProductionRng {
    fn choose_imposter(&mut self, players_len: usize) -> usize {
        rand::rng().random_range(0..players_len)
    }

    fn choose_topic(&mut self, topics_len: usize) -> usize {
        rand::rng().random_range(0..topics_len)
    }
}

/// Deterministic RNG for tests.
#[derive(Debug)]
pub struct FixedRng {
    imposter_index: usize,
    topic_index: usize,
}

impl FixedRng {
    pub fn new(imposter_index: usize, topic_index: usize) -> Self {
        Self {
            imposter_index,
            topic_index,
        }
    }
}

impl RngLike for FixedRng {
    fn choose_imposter(&mut self, players_len: usize) -> usize {
        self.imposter_index % players_len
    }

    fn choose_topic(&mut self, topics_len: usize) -> usize {
        self.topic_index % topics_len
    }
}
