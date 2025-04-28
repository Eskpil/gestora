mod sway;

use sway::Sway;

use input::event::gesture::{
    GestureEventCoordinates, GestureEventTrait, GestureSwipeBeginEvent, GestureSwipeUpdateEvent,
};
use std::f64::consts::PI;

use input::event::GestureEvent;
use input::event::gesture::GestureSwipeEvent;
use input::{Libinput, LibinputInterface};
use libc::{O_RDONLY, O_RDWR, O_WRONLY};
use std::fs::{File, OpenOptions};
use std::os::unix::{fs::OpenOptionsExt, io::OwnedFd};
use std::path::Path;

struct Interface;

impl LibinputInterface for Interface {
    fn open_restricted(&mut self, path: &Path, flags: i32) -> Result<OwnedFd, i32> {
        OpenOptions::new()
            .custom_flags(flags)
            .read((flags & O_RDONLY != 0) | (flags & O_RDWR != 0))
            .write((flags & O_WRONLY != 0) | (flags & O_RDWR != 0))
            .open(path)
            .map(|file| file.into())
            .map_err(|err| err.raw_os_error().unwrap())
    }
    fn close_restricted(&mut self, fd: OwnedFd) {
        drop(File::from(fd));
    }
}

enum SwipeDir {
    N,
    S,
    W,
    E,
    NE,
    NW,
    SE,
    SW,
}

struct Swipe {
    dir: SwipeDir,
    finger_count: i32,
}

#[derive(Debug, Clone, Copy)]
struct SwipeVector {
    dx: f64,
    dy: f64,
}

impl SwipeVector {
    fn new() -> Self {
        SwipeVector { dx: 0.0, dy: 0.0 }
    }

    fn add_update(&mut self, update: &GestureSwipeUpdateEvent) {
        self.dx += update.dx();
        self.dy += update.dy();
    }

    fn calculate_direction(&self) -> SwipeDir {
        if self.dx == 0.0 && self.dy == 0.0 {
            return SwipeDir::N; // default if no movement
        }

        let angle_rad = self.dy.atan2(self.dx);
        let angle_deg = angle_rad * 180.0 / PI;

        let angle_deg = if angle_deg < 0.0 {
            angle_deg + 360.0
        } else {
            angle_deg
        };

        match angle_deg {
            a if (22.5..67.5).contains(&a) => SwipeDir::NE,
            a if (67.5..112.5).contains(&a) => SwipeDir::N,
            a if (112.5..157.5).contains(&a) => SwipeDir::NW,
            a if (157.5..202.5).contains(&a) => SwipeDir::W,
            a if (202.5..247.5).contains(&a) => SwipeDir::SW,
            a if (247.5..292.5).contains(&a) => SwipeDir::S,
            a if (292.5..337.5).contains(&a) => SwipeDir::SE,
            _ => SwipeDir::E,
        }
    }
}

struct SwipeStateMachine {
    finger_count: i32,
    accumulated_swipe: SwipeVector,
}

impl SwipeStateMachine {
    fn new() -> Self {
        SwipeStateMachine {
            finger_count: 0,
            accumulated_swipe: SwipeVector::new(),
        }
    }

    fn begin(&mut self, begin: GestureSwipeBeginEvent) {
        self.finger_count = begin.finger_count();
        self.accumulated_swipe = SwipeVector::new();
    }

    fn update(&mut self, update: GestureSwipeUpdateEvent) {
        self.accumulated_swipe.add_update(&update);
    }

    fn end(&mut self) -> Option<Swipe> {
        let dir = self.accumulated_swipe.calculate_direction();
        let finger_count = self.finger_count;
        self.finger_count = 0;

        Some(Swipe { finger_count, dir })
    }
}

fn handle_swipe_gesture(
    gesture: GestureSwipeEvent,
    state_machine: &mut SwipeStateMachine,
) -> Option<Swipe> {
    match gesture {
        GestureSwipeEvent::Begin(begin) => {
            state_machine.begin(begin);
            None
        }
        GestureSwipeEvent::Update(update) => {
            state_machine.update(update);
            None
        }
        GestureSwipeEvent::End(_) => state_machine.end(),
        _ => None,
    }
}

fn handle_gesture(gesture: GestureEvent, state_machine: &mut SwipeStateMachine) -> Option<Swipe> {
    match gesture {
        GestureEvent::Swipe(swipe) => handle_swipe_gesture(swipe, state_machine),
        _ => None,
    }
}

fn main() -> Result<(), anyhow::Error> {
    let mut sway = Sway::new()?;

    let mut input = Libinput::new_with_udev(Interface);
    let mut state_machine = SwipeStateMachine::new();

    input.udev_assign_seat("seat0").unwrap();

    loop {
        input.dispatch().unwrap();
        for event in &mut input {
            match event {
                input::Event::Gesture(gesture) => {
                    if let Some(swipe) = handle_gesture(gesture, &mut state_machine) {
                        let direction = swipe.dir;
                        let finger_count = swipe.finger_count;

                        let active_workspace = sway.get_active_workspace()?;

                        if finger_count == 3 {
                            match direction {
                                SwipeDir::W => {
                                    if active_workspace - 1 > 0 {
                                        sway.set_active_workspace(active_workspace - 1)?;
                                    }
                                }
                                SwipeDir::E => {
                                    if active_workspace + 1 <= 10 {
                                        sway.set_active_workspace(active_workspace + 1)?;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}
