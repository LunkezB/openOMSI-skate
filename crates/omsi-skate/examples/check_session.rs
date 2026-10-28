//! Exercise the real worker and owned assets without a renderer or a controller.
use omsi_skate::{Collision, InputFrame, Pose, Runtime};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

fn receive(runtime: &Runtime) -> Result<Pose, String> {
    let deadline = Instant::now() + Duration::from_secs(120);
    while Instant::now() < deadline {
        if let Some(pose) = runtime.latest()? {
            return Ok(pose);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Err("Skate worker did not return a pose within 120 seconds".into())
}

fn run() -> Result<(), String> {
    let assets = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("Usage: check_session <assets>")?,
    );
    let world = Collision {
        triangles: vec![
            [
                [-100.0, 0.0, -100.0],
                [-100.0, 0.0, 100.0],
                [100.0, 0.0, 100.0],
            ],
            [
                [-100.0, 0.0, -100.0],
                [100.0, 0.0, 100.0],
                [100.0, 0.0, -100.0],
            ],
        ],
        rails: Vec::new(),
    };
    let runtime = Runtime::start(assets, world, [0.0, 0.2, 0.0], 0.0)?;
    let initial = receive(&runtime)?;
    println!(
        "READY state={} bones={}",
        initial.state,
        initial.bones.len()
    );
    let mut latest = initial;
    for tick in 0..360 {
        let buttons = if (60..240).contains(&tick) { 0x1000 } else { 0 };
        if !runtime.step(
            InputFrame::from_pad(buttons, [0; 2], [0; 2], [0; 2], tick + 1),
            1.0 / 60.0,
        )? {
            return Err("Worker queue was unexpectedly full after receiving its pose".into());
        }
        latest = receive(&runtime)?;
        if !latest
            .root
            .iter()
            .chain(&latest.velocity)
            .all(|v| v.is_finite())
        {
            return Err(format!("Nonfinite pose at tick {tick}"));
        }
        if tick % 60 == 59 {
            println!(
                "TICK {} state={} position={:?} velocity={:?}",
                tick + 1,
                latest.state,
                &latest.root[12..15],
                latest.velocity
            );
        }
    }
    let distance = latest.root[12].hypot(latest.root[14]);
    if distance < 0.1 {
        return Err(format!("Push input did not move the board: {distance} m"));
    }
    if latest.root[13] < -1.0 {
        return Err("Skater fell through the test ground".into());
    }
    println!("PASS 360 ticks, push distance {distance:.3} m");
    let ground_y = latest.root[13];
    let mut peak_y = ground_y;
    for tick in 0..120 {
        let flick = match tick {
            0..24 => -i16::MAX,
            24..32 => i16::MAX,
            _ => 0,
        };
        if !runtime.step(InputFrame::from_pad(0, [0; 2], [0; 2], [0, flick], tick + 361), 1.0 / 60.0)? {
            return Err("Worker queue was full during the Flick-It test".into());
        }
        latest = receive(&runtime)?;
        peak_y = peak_y.max(latest.root[13]);
    }
    let rise = peak_y - ground_y;
    println!("FLICK_IT rise={rise:.3} m final_state={}", latest.state);
    if rise < 0.2 { return Err("Flick-It input did not lift the skater".into()); }
    println!("PASS Flick-It jump");
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("FAIL: {error}");
        std::process::exit(1);
    }
}
