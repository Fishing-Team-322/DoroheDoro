use crate::pipeline::{DetectionContext, DetectionMode, DetectionOutcome, SignalEnvelope};

pub trait Detector: Send + Sync {
    fn name(&self) -> &'static str;
    fn min_mode(&self) -> DetectionMode {
        DetectionMode::Light
    }
    fn cost_units(&self) -> u32 {
        1
    }
    fn detect(&self, envelope: &SignalEnvelope, ctx: &DetectionContext) -> DetectionOutcome;
}

pub struct DetectorRegistry {
    detectors: Vec<Box<dyn Detector>>,
}

impl DetectorRegistry {
    pub fn new() -> Self {
        Self {
            detectors: Vec::new(),
        }
    }

    pub fn register<D>(&mut self, detector: D)
    where
        D: Detector + 'static,
    {
        self.detectors.push(Box::new(detector));
    }

    pub fn iter(&self) -> impl Iterator<Item = &Box<dyn Detector>> {
        self.detectors.iter()
    }
}
