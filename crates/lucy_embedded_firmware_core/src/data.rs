use core::sync::atomic::{AtomicU64};

const MAX_ACTUATORS: usize = 32;

#[repr(C, align(64))]
pub struct CommandBlock {
    pub command_seq: AtomicU64,
    pub hw_commands: [f64; MAX_ACTUATORS],
    pub hw_velocities: [f64; MAX_ACTUATORS],
    pub hw_accelerations: [f64; MAX_ACTUATORS],
    pub hw_torque_enabled: [f64; MAX_ACTUATORS],
}

#[repr(C, align(64))]
pub struct StateBlock {
    pub state_seq: AtomicU64,
    pub hw_positions: [f64; MAX_ACTUATORS],
}

#[repr(C, align(64))]
pub struct HeartbeatBlock {
    pub heartbeat: AtomicU64,
}

#[repr(C, align(64))]
pub struct ActuatorSharedState {
    pub commands: CommandBlock,
    pub states: StateBlock,
    pub heartbeat: HeartbeatBlock,
}
