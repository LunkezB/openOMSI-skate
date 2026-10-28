mod visual;

use crate::{
    App,
    on_foot::{AVATAR_KEY, FootCam, OnFoot, OnFootCx},
};
use glam::{DVec2, DVec3, Mat4, Vec3};
use omsi_skate::{ControllerTransport, Frame, Pose, Runtime};
use std::sync::Arc;
use winit::keyboard::KeyCode;

pub(crate) struct Skating {
    runtime: Runtime,
    frame: Frame,
    pad: ControllerTransport,
    visual: visual::Visual,
    pose: Option<Pose>,
    ground: Arc<omsi_sim::world_data::GroundTiles>,
    lists: Arc<omsi_sim::world_data::TileLists>,
    collision_at: DVec3,
    refresh: f32,
    pending_dt: f32,
    keyboard_packet: u32,
}

impl App {
    pub(crate) fn clear_skate(&mut self) {
        if let Some(f) = self.session.on_foot.as_mut() {
            f.skate_requested = false;
            if let Some(skate) = f.skate.take() {
                if let (Some(r), Some(scene)) = (self.renderer.as_ref(), self.scene.as_mut()) {
                    skate.visual.free(r, scene);
                }
            }
        }
    }

    pub(crate) fn skate_controls(&self) -> bool {
        self.session
            .on_foot
            .as_ref()
            .is_some_and(|f| f.skate.is_some() || f.skate_requested)
    }

    pub(crate) fn toggle_skate(&mut self) {
        if let Some(f) = self.session.on_foot.as_mut() {
            if let Some(skate) = f.skate.take() {
                if let (Some(r), Some(scene)) = (self.renderer.as_ref(), self.scene.as_mut()) {
                    skate.visual.free(r, scene);
                }
                f.vel = DVec2::ZERO;
                f.eye = None;
                f.skate_requested = false;
                self.service_msg = Some((
                    "On foot. G by the driver's place returns to the bus; J skates.".into(),
                    5.0,
                ));
                return;
            }
            if f.skate_requested {
                f.skate_requested = false;
                return;
            }
        }
        if omsi_cfg::flags::OMSI_SKATE_ASSETS.var().is_none() {
            self.service_msg = Some((
                "Set OMSI_SKATE_ASSETS to prepared Skate 3 assets. See docs/SKATE3.md".into(),
                7.0,
            ));
            return;
        }
        if self
            .player
            .as_ref()
            .is_some_and(|p| p.vehicle.physics.velocity_kmh().abs() > 0.5)
            && self.session.on_foot.is_none()
        {
            self.service_msg = Some(("Stop the bus before skating.".into(), 5.0));
            return;
        }
        if self.session.on_foot.is_none() {
            self.get_up();
        }
        self.step_out();
        if let Some(f) = self.session.on_foot.as_mut() {
            if f.seat.is_some() || f.inside.is_some() && f.transit.is_none() {
                self.service_msg = Some((
                    "Walk outside the bus before getting on the board.".into(),
                    5.0,
                ));
                return;
            }
            f.skate_requested = true;
            self.service_msg = Some((
                "Skate: leaving the bus and loading the session...".into(),
                5.0,
            ));
        }
    }
}

pub(crate) fn tick(f: &mut OnFoot, cx: &mut OnFootCx, dt: f32) -> bool {
    if f.skate_requested
        && f.transit.is_none()
        && f.inside.is_none()
        && f.seat.is_none()
        && dt > 0.0
    {
        f.skate_requested = false;
        match start(f, cx) {
            Ok(skate) => {
                f.cam = FootCam::First;
                f.kneel = false;
                f.crouch = 0.0;
                f.lift = 0.0;
                f.vz = 0.0;
                f.skate = Some(skate);
                *cx.view.view = "foot".into();
                *cx.ui.service_msg = Some(("Skate loading. J dismounts. W/Space push, A/D steer, S brake, arrows Flick-It. Controller supported.".into(), 8.0));
            }
            Err(e) => *cx.ui.service_msg = Some((e, 8.0)),
        }
    }
    let Some(mut skate) = f.skate.take() else {
        return false;
    };
    let result = advance(&mut skate, f, cx, dt);
    if let Some(h) = cx.people.humans.as_mut() {
        h.avatar_show(AVATAR_KEY, false);
    }
    match result {
        Ok(()) => f.skate = Some(skate),
        Err(error) => {
            log::error!("Skate: {error}");
            if let (Some(r), Some(scene)) = (cx.gfx.renderer.as_ref(), cx.gfx.scene.as_mut()) {
                skate.visual.free(r, scene);
            }
            f.vel = DVec2::ZERO;
            f.eye = None;
            *cx.ui.service_msg = Some((
                format!("Skate stopped: {error}. Returned to walking."),
                10.0,
            ));
        }
    }
    true
}

fn nearby_vehicles(f: &OnFoot, cx: &OnFootCx) -> Vec<omsi_sim::collision::Obb> {
    crate::on_foot::vehicle_boxes(
        f.pos.truncate(),
        110.0,
        cx.player.player.as_ref(),
        cx.player.placed,
        cx.traffic.traffic.as_ref(),
        cx.net,
    )
}

fn start(f: &OnFoot, cx: &mut OnFootCx) -> Result<Skating, String> {
    let root = omsi_cfg::flags::OMSI_SKATE_ASSETS
        .var()
        .ok_or("OMSI_SKATE_ASSETS is not configured")?;
    let assets = std::path::PathBuf::from(root);
    let visual = visual::Visual::load(&assets.join("private/skater.glb"))?;
    let world = cx.world.world.as_ref().ok_or("No OMSI world loaded")?;
    let data = world.data();
    let frame = Frame { origin: f.pos };
    let geometry = omsi_skate::collision(&data, frame, f.pos, &nearby_vehicles(f, cx))?;
    let runtime = Runtime::start(assets, geometry, [0.0, 0.2, 0.0], Frame::heading(f.heading))?;
    Ok(Skating {
        runtime,
        frame,
        visual,
        pad: ControllerTransport::default(),
        pose: None,
        ground: data.ground.clone(),
        lists: data.lists.clone(),
        collision_at: f.pos,
        refresh: 0.0,
        pending_dt: 0.0,
        keyboard_packet: 0,
    })
}

fn advance(s: &mut Skating, f: &mut OnFoot, cx: &mut OnFootCx, dt: f32) -> Result<(), String> {
    if let Some(pose) = s.runtime.latest()? {
        if s.pose.is_none() {
            log::info!("Skate session ready: {}", pose.state);
            *cx.ui.service_msg = Some((
                "Skate ready. J dismounts; G by the driver's place returns to the bus.".into(),
                7.0,
            ));
        }
        s.pose = Some(pose);
    }
    if let Some(pose) = &s.pose {
        let root = Mat4::from_cols_array(&pose.root);
        f.pos = s.frame.to_world(root.w_axis.truncate().to_array());
        if !f.pos.is_finite() {
            return Err("Skater position is not finite".into());
        }
        let v = pose.velocity;
        f.vel = DVec2::new(v[0] as f64, -v[2] as f64);
        let forward = root.z_axis.truncate();
        f.heading = (forward.x as f64).atan2(-forward.z as f64).to_degrees();
        if let Some(cam) = cx.view.camera.as_mut() {
            if let Some((at, direction, fov)) = pose.camera {
                cam.position = s.frame.to_world(at);
                let d = Vec3::new(direction[0], -direction[2], direction[1]).normalize_or_zero();
                cam.yaw = d.x.atan2(d.y).to_degrees();
                cam.pitch = d.z.clamp(-1.0, 1.0).asin().to_degrees();
                cam.roll = 0.0;
                cam.fov_deg = fov.clamp(20.0, 120.0);
            } else {
                cam.position = f.pos + DVec3::new(0.0, -3.0, 1.7);
                let d = (f.pos + DVec3::Z - cam.position).normalize();
                cam.yaw = d.x.atan2(d.y).to_degrees() as f32;
                cam.pitch = d.z.asin().to_degrees() as f32;
            }
            f.yaw = cam.yaw;
            f.pitch = cam.pitch;
        }
        if let (Some(r), Some(scene)) = (cx.gfx.renderer.as_ref(), cx.gfx.scene.as_mut()) {
            s.visual.draw(r, scene, s.frame.origin, pose)?;
        }
    }
    if dt <= 0.0 {
        return Ok(());
    }
    let world = cx
        .world
        .world
        .as_ref()
        .ok_or("The OMSI world was unloaded")?;
    let data = world.data();
    let tile = omsi_sim::world_data::tile_key(f.pos.x, f.pos.y);
    if !data.ground.terrains.contains_key(&tile) {
        return Ok(());
    }
    if s.pose.is_none() {
        return Ok(());
    }
    s.pending_dt = (s.pending_dt + dt).min(8.0 / 60.0);
    s.refresh += dt;
    let changed = !Arc::ptr_eq(&s.ground, &data.ground)
        || !Arc::ptr_eq(&s.lists, &data.lists)
        || f.pos.distance(s.collision_at) > 16.0;
    if s.refresh > 0.5 && (changed || s.refresh > 2.0) {
        let geometry = omsi_skate::collision(&data, s.frame, f.pos, &nearby_vehicles(f, cx))?;
        if s.runtime.replace_world(geometry)? {
            s.ground = data.ground.clone();
            s.lists = data.lists.clone();
            s.collision_at = f.pos;
            s.refresh = 0.0;
        }
    } else if s.pose.is_some() {
        let input = if cx.input.window_focused || omsi_cfg::flags::OMSI_BACKGROUND.is_set() {
            let pad = s.pad.poll();
            if pad.controller().is_some() {
                pad
            } else {
                s.keyboard_packet = s.keyboard_packet.wrapping_add(1);
                keyboard(&cx.input.keys, s.keyboard_packet)
            }
        } else {
            omsi_skate::InputFrame::neutral()
        };
        if s.runtime.step(input, s.pending_dt)? {
            s.pending_dt = 0.0;
        }
    }
    Ok(())
}

fn keyboard(keys: &hashbrown::HashSet<KeyCode>, packet: u32) -> omsi_skate::InputFrame {
    let held = |key| keys.contains(&key);
    let mut buttons = 0;
    for (down, bit) in [
        (held(KeyCode::KeyW) || held(KeyCode::Space), 0x1000),
        (held(KeyCode::KeyS), 0x2000),
        (
            held(KeyCode::ShiftLeft) || held(KeyCode::ShiftRight),
            0x4000,
        ),
        (held(KeyCode::KeyF), 0x8000),
    ] {
        if down {
            buttons |= bit;
        }
    }
    let axis = |positive, negative| {
        ((i32::from(held(positive)) - i32::from(held(negative))) * i32::from(i16::MAX)) as i16
    };
    omsi_skate::InputFrame::from_pad(
        buttons,
        [
            if held(KeyCode::KeyQ) { 255 } else { 0 },
            if held(KeyCode::KeyE) { 255 } else { 0 },
        ],
        [axis(KeyCode::KeyD, KeyCode::KeyA), 0],
        [
            axis(KeyCode::ArrowRight, KeyCode::ArrowLeft),
            axis(KeyCode::ArrowUp, KeyCode::ArrowDown),
        ],
        packet,
    )
}
