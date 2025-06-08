use std::time::{self, Duration, Instant};

pub(crate) struct FrameChange<T> {
    pub(crate) new_frame: Option<T>,
    pub(crate) delay_before_next_frame: Option<Duration>,
}

impl<T> Default for FrameChange<T> {
    fn default() -> Self {
        Self {
            new_frame: None,
            delay_before_next_frame: None,
        }
    }
}

pub(crate) struct FramesStore<T: Clone> {
    // Store every frames and the corresponding delays to the next frame, if one.
    // This will be needed if the GIF has to loop
    frames: Vec<(T, Option<u16>)>,
    last_change_time: Instant,
    curr_frame_delay: Option<u16>,
    curr_frame_idx: usize,
    last_frame_known: bool,
    left_loop_iterations: Option<u16>,
}

impl<T: Clone> FramesStore<T> {
    pub(crate) fn new() -> Self {
        Self {
            frames: vec![],
            last_change_time: time::Instant::now(),
            curr_frame_delay: Some(0),
            curr_frame_idx: 0,
            last_frame_known: false,
            left_loop_iterations: None,
        }
    }

    pub(crate) fn add_frame(&mut self, frame: T, duration: Option<u16>) {
        self.frames.push((frame, duration));
    }

    pub(crate) fn set_loop_iterations(&mut self, iterations: Option<u16>) {
        self.left_loop_iterations = iterations;
    }

    pub(crate) fn end_of_frames(&mut self) {
        self.last_frame_known = true;
    }

    pub(crate) fn check(&mut self) -> FrameChange<T> {
        let now = time::Instant::now();

        if self.frames.is_empty() {
            // Frame not known yet
            return FrameChange {
                new_frame: None,

                // ~60fps by default while waiting for frames
                delay_before_next_frame: Some(Duration::from_millis(16)),
            };
        }

        match self.curr_frame_delay {
            None => FrameChange {
                new_frame: None,
                delay_before_next_frame: None,
            },
            Some(delay) => {
                let delay_dur = time::Duration::from_millis(10 * delay as u64);

                let time_since_last_change = now - self.last_change_time;
                if time_since_last_change < delay_dur {
                    // No change for now, just tell the remaining time
                    return FrameChange {
                        new_frame: None,
                        delay_before_next_frame: Some(delay_dur - time_since_last_change),
                    };
                }

                if self.curr_frame_idx < self.frames.len() {
                    let duration = self.frames[self.curr_frame_idx].1;
                    self.curr_frame_delay = duration;
                    self.curr_frame_idx += 1;
                    self.last_change_time = now;

                    let frame = self.frames[self.curr_frame_idx].0.clone();
                    return FrameChange {
                        new_frame: Some(frame),
                        delay_before_next_frame: duration.map(|d| Duration::from_millis(d as u64)),
                    };
                }

                if !self.last_frame_known {
                    FrameChange {
                        new_frame: None,

                        // ~60fps by default while waiting for frames
                        delay_before_next_frame: Some(Duration::from_millis(16)),
                    }
                } else {
                    match self.left_loop_iterations {
                        /* Nothing to show anymore */
                        None => FrameChange {
                            new_frame: None,
                            delay_before_next_frame: None,
                        },

                        Some(x) => {
                            match x {
                                0 => { /* Infinite looping, do nothing. */ }
                                1 => {
                                    self.left_loop_iterations = None;
                                }
                                x => {
                                    self.left_loop_iterations = Some(x - 1);
                                }
                            };

                            self.curr_frame_delay = self.frames[0].1;
                            self.curr_frame_idx = 1;
                            self.last_change_time = now;
                            let frame = self.frames[0].0.clone();
                            if let Some(dur) = self.curr_frame_delay {
                                let delay_til_next = Some(Duration::from_millis(dur as u64));
                                FrameChange {
                                    new_frame: Some(frame),
                                    delay_before_next_frame: delay_til_next,
                                }
                            } else {
                                FrameChange {
                                    new_frame: Some(frame),
                                    delay_before_next_frame: None,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
