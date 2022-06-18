use std::fmt::{Display, Formatter};

pub enum StepPointParam {
    MaxDistanceFromCorridorLeadingCandlePinsPct,
}

impl Display for StepPointParam {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            StepPointParam::MaxDistanceFromCorridorLeadingCandlePinsPct => {
                write!(f, "max_distance_from_corridor_leading_candle_pins_pct")
            }
        }
    }
}

pub enum StepRatioParam {
    MinDistanceBetweenMaxMinAngles,
}

impl Display for StepRatioParam {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            StepRatioParam::MinDistanceBetweenMaxMinAngles => {
                write!(f, "min_distance_between_max_min_angles")
            }
        }
    }
}
