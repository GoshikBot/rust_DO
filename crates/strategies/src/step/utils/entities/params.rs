use std::fmt::{Display, Formatter};

pub enum StepPointParam {
    MaxDistanceFromCorridorLeadingCandlePinsPct,
    AmountOfOrders,
    LevelExpirationDays,
    MinAmountOfCandlesInSmallCorridorBeforeActivationCrossingOfLevel,
    MinAmountOfCandlesInBigCorridorBeforeActivationCrossingOfLevel,
    MinAmountOfCandlesInCorridorDefiningEdgeBargaining,
    MaxLossPerOneChainOfOrdersPctOfBalance,
}

impl Display for StepPointParam {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            StepPointParam::MaxDistanceFromCorridorLeadingCandlePinsPct => {
                write!(f, "max_distance_from_corridor_leading_candle_pins_pct")
            }
            StepPointParam::AmountOfOrders => write!(f, "amount_of_orders"),
            StepPointParam::LevelExpirationDays => write!(f, "level_expiration_days"),
            StepPointParam::MinAmountOfCandlesInSmallCorridorBeforeActivationCrossingOfLevel => {
                write!(
                    f,
                    "min_amount_of_candles_in_small_corridor_before_activation_crossing_of_level"
                )
            }
            StepPointParam::MinAmountOfCandlesInBigCorridorBeforeActivationCrossingOfLevel => {
                write!(
                    f,
                    "min_amount_of_candles_in_big_corridor_before_activation_crossing_of_level"
                )
            }
            StepPointParam::MinAmountOfCandlesInCorridorDefiningEdgeBargaining => {
                write!(
                    f,
                    "min_amount_of_candles_in_corridor_defining_edge_bargaining"
                )
            }
            StepPointParam::MaxLossPerOneChainOfOrdersPctOfBalance => {
                write!(f, "max_loss_per_one_chain_of_orders_pct_of_balance")
            }
        }
    }
}

pub enum StepRatioParam {
    MinDistanceBetweenNewAndCurrentMaxMinAngles,
    MinDistanceBetweenCurrentMaxAndMinAnglesForNewInnerAngleToAppear,
    MinBreakDistance,
    DistanceFromLevelToFirstOrder,
    DistanceFromLevelToStopLoss,
    DistanceFromLevelForSignalingOfMovingTakeProfits,
    DistanceToMoveTakeProfits,
    DistanceFromLevelForItsDeletion,
    DistanceFromLevelToCorridorBeforeActivationCrossingOfLevel,
    DistanceDefiningNearbyLevelsOfTheSameType,
    MinDistanceOfActivationCrossingOfLevelWhenReturningToLevelForItsDeletion,
    RangeOfBigCorridorNearLevel,
}

impl Display for StepRatioParam {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match *self {
            StepRatioParam::MinDistanceBetweenNewAndCurrentMaxMinAngles => {
                write!(f, "min_distance_between_new_and_current_max_min_angles")
            }
            StepRatioParam::MinDistanceBetweenCurrentMaxAndMinAnglesForNewInnerAngleToAppear => {
                write!(f, "min_distance_between_current_max_and_min_angles_for_new_inner_angle_to_appear")
            }
            StepRatioParam::MinBreakDistance => write!(f, "min_break_distance"),
            StepRatioParam::DistanceFromLevelToFirstOrder => {
                write!(f, "distance_from_level_to_first_order")
            }
            StepRatioParam::DistanceFromLevelToStopLoss => {
                write!(f, "distance_from_level_to_stop_loss")
            }
            StepRatioParam::DistanceFromLevelForSignalingOfMovingTakeProfits => {
                write!(
                    f,
                    "distance_from_level_for_signaling_of_moving_take_profits"
                )
            }
            StepRatioParam::DistanceToMoveTakeProfits => {
                write!(f, "distance_to_move_take_profits")
            }
            StepRatioParam::DistanceFromLevelForItsDeletion => {
                write!(f, "distance_from_level_for_its_deletion")
            }
            StepRatioParam::DistanceFromLevelToCorridorBeforeActivationCrossingOfLevel => {
                write!(
                    f,
                    "distance_from_level_to_corridor_before_activation_crossing_of_level"
                )
            }
            StepRatioParam::DistanceDefiningNearbyLevelsOfTheSameType => {
                write!(f, "distance_defining_nearby_levels_of_the_same_type")
            }
            StepRatioParam::MinDistanceOfActivationCrossingOfLevelWhenReturningToLevelForItsDeletion => {
                write!(
                    f,
                    "min_distance_of_activation_crossing_of_level_when_returning_to_level_for_its_deletion"
                )
            }
            StepRatioParam::RangeOfBigCorridorNearLevel => {
                write!(f, "range_of_big_corridor_near_level")
            }
        }
    }
}
