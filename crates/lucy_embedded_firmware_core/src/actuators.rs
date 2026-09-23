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

pub trait JointStateInterface {
    type Error;
    fn get_position(&mut self) -> Result<f64, Self::Error>;
}

pub trait TemperatureInterface {
    type Error;
    fn get_temperature(&mut self) -> Result<f64, Self::Error>;
}

pub trait TorqueInterface {
    type Error;
    fn get_torque(&mut self) -> Result<f64, Self::Error>;
}
