use core::result::Result;
use core::error::Error;

// Command

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointTrajectoryPoint {
    pub position: f64,
    pub velocity: f64,
    pub acceleration: f64,
}

pub trait JointTrajectoryInterface {
    type Error;
    fn set_joint_trajectory(&mut self, joint_trajectory_point: JointTrajectoryPoint) -> Result<(), Self::Error>;
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TorqueStatus {
    Enabled,
    Disabled,
}

impl From<f64> for TorqueStatus {
    #[inline]
    fn from(value: f64) -> Self {
        if value >= 0.5 && !value.is_nan() {
            TorqueStatus::Enabled
        } else {
            TorqueStatus::Disabled
        }
    }
}

pub trait TorqueEnableInterface {
    type Error;
    fn set_torque_enable(&mut self, state: TorqueStatus) -> Result<(), Self::Error>;
}

// State

pub trait TemperatureInterface {
    fn get_temperature(&mut self) -> f64;
}

pub trait JointStateInterface {
    fn get_position(&self) -> f64;
}
