//! A retained Skate 3 session on a worker, sharing the host's world and window.
mod geometry;
pub mod rails;

pub use geometry::{Collision, Frame, collision};
pub use skate_host::bridge::{ControllerTransport, InputFrame};
use std::{path::PathBuf, sync::mpsc};

pub struct Pose {
    pub root: [f32; 16],
    pub bones: Vec<[f32; 16]>,
    pub names: Vec<String>,
    pub camera: Option<([f32; 3], [f32; 3], f32)>,
    pub velocity: [f32; 3],
    pub state: String,
}

enum Job {
    Step(InputFrame, f32),
    World(Collision),
}

pub struct Runtime {
    jobs: mpsc::SyncSender<Job>,
    replies: mpsc::Receiver<Result<Pose, String>>,
}

impl Runtime {
    pub fn start(
        assets: PathBuf,
        world: Collision,
        spawn: [f32; 3],
        heading: f32,
    ) -> Result<Self, String> {
        if !assets.join("private/game.json").is_file() {
            return Err(format!(
                "Skate assets missing at {}. See docs/SKATE3.md",
                assets.display()
            ));
        }
        let (jobs, receive) = mpsc::sync_channel(1);
        let (publish, replies) = mpsc::channel();
        std::thread::Builder::new()
            .name("omsi-skate".into())
            .stack_size(32 * 1024 * 1024)
            .spawn(move || {
                let run = || -> Result<(), String> {
                    let world = world.prepare_rails();
                    let mut session = skate_host::bridge::Session::new(
                        &assets,
                        world.triangles,
                        world.rails,
                        spawn,
                        heading,
                    )?;
                    let initial = session.activate(spawn, heading)?;
                    if publish.send(Ok(convert(initial))).is_err() {
                        return Ok(());
                    }
                    let mut accumulated = 0.0;
                    while let Ok(job) = receive.recv() {
                        match job {
                            Job::World(world) => {
                                let world = world.prepare_rails();
                                let prepared = session
                                    .collision_builder()
                                    .build(world.triangles, world.rails)?;
                                session.install_collision(prepared)?;
                            }
                            Job::Step(input, dt) => {
                                if !dt.is_finite() || dt < 0.0 {
                                    return Err("Invalid Skate time step".into());
                                }
                                session.collect(input, dt);
                                let period = session.period();
                                if !period.is_finite() || period <= 0.0 {
                                    return Err("Invalid Skate physics period".into());
                                }
                                accumulated = (accumulated + dt).min(period * 8.0);
                                while accumulated >= period {
                                    session.advance()?;
                                    accumulated -= period;
                                }
                                if publish.send(Ok(convert(session.pose()))).is_err() {
                                    break;
                                }
                            }
                        }
                    }
                    Ok(())
                };
                if let Err(error) = run() {
                    let _ = publish.send(Err(error));
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self { jobs, replies })
    }

    /// One request in flight prevents an overloaded renderer from queuing old input.
    pub fn step(&self, input: InputFrame, dt: f32) -> Result<bool, String> {
        match self.jobs.try_send(Job::Step(input, dt)) {
            Ok(()) => Ok(true),
            Err(mpsc::TrySendError::Full(_)) => Ok(false),
            Err(mpsc::TrySendError::Disconnected(_)) => Err("Skate worker stopped".into()),
        }
    }

    pub fn replace_world(&self, collision: Collision) -> Result<bool, String> {
        match self.jobs.try_send(Job::World(collision)) {
            Ok(()) => Ok(true),
            Err(mpsc::TrySendError::Full(_)) => Ok(false),
            Err(mpsc::TrySendError::Disconnected(_)) => Err("Skate worker stopped".into()),
        }
    }

    pub fn latest(&self) -> Result<Option<Pose>, String> {
        let mut latest = None;
        loop {
            match self.replies.try_recv() {
                Ok(Ok(pose)) => latest = Some(pose),
                Ok(Err(error)) => return Err(error),
                Err(mpsc::TryRecvError::Empty) => return Ok(latest),
                Err(mpsc::TryRecvError::Disconnected) => return Err("Skate worker stopped".into()),
            }
        }
    }
}

fn convert(pose: skate_host::bridge::Pose) -> Pose {
    Pose {
        root: pose.root.to_cols_array(),
        bones: pose.bones.iter().map(|m| m.to_cols_array()).collect(),
        names: pose.names,
        camera: pose
            .camera
            .map(|(at, basis, fov)| (at.to_array(), basis.z_axis.to_array(), fov)),
        velocity: pose.velocity.to_array(),
        state: pose.state,
    }
}
