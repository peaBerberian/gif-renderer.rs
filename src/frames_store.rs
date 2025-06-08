use std::time::{self, Duration, Instant};

pub(crate) struct FrameChange<T> {
    new_frame: Option<T>,
    delay_before_recheck: Option<Duration>,
}

impl<T> FrameChange<T> {
    pub(crate) fn into_frame_data(self) -> Option<T> {
        self.new_frame
    }
    pub(crate) fn delay_before_recheck(&self) -> Option<Duration> {
        self.delay_before_recheck
    }
}

impl<T> Default for FrameChange<T> {
    fn default() -> Self {
        Self {
            new_frame: None,
            delay_before_recheck: None,
        }
    }
}

/// Object storing GIF frame data and metadata and indicating the current frame that
/// should be displayed depending on which frame it last communicated and when.
pub(crate) struct FramesStore<T> {
    /// Store every frames and the corresponding delays to the next frame, if one.
    frames: Vec<(T, Option<u16>)>,
    /// Time at which the last frame change has been communicated.
    last_frame_change_time: Instant,
    /// Index of the currently-displayed frame in `frames`.
    /// Should always be a valid index.
    curr_frame_idx: usize,
    /// Duration during which the current frame should be displayed before going to the
    /// next.
    curr_frame_duration: Option<u16>,
    /// Set to `true` if `frames` is considered complete. `false` if there may be
    /// supplementary frames communicated in the future.
    last_frame_known: bool,
    /// Remaining loop iterations in the GIF. Initially set to `None`, as it is initially
    /// unknown.
    ///
    /// `0` is a special value indicating infinite looping.
    left_loop_iterations: Option<u16>,
}

impl<T> FramesStore<T> {
    pub(crate) fn new() -> Self {
        Self {
            frames: vec![],
            last_frame_change_time: time::Instant::now(),
            curr_frame_duration: Some(0),
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

    pub(crate) fn check(&mut self) -> FrameChange<&T> {
        let now = time::Instant::now();

        if self.frames.is_empty() {
            // Frame not known yet
            return FrameChange {
                new_frame: None,

                // ~60fps by default while waiting for frames
                delay_before_recheck: Some(Duration::from_millis(16)),
            };
        }

        match self.curr_frame_duration {
            None => FrameChange {
                new_frame: None,
                delay_before_recheck: None,
            },
            Some(delay) => {
                let delay_dur = time::Duration::from_millis(10 * delay as u64);

                let time_since_last_change = now - self.last_frame_change_time;
                if time_since_last_change < delay_dur {
                    // No change for now, just tell the remaining time
                    return FrameChange {
                        new_frame: None,
                        delay_before_recheck: Some(delay_dur - time_since_last_change),
                    };
                }

                if self.curr_frame_idx < self.frames.len() {
                    let duration = self.frames[self.curr_frame_idx].1;
                    let frame = &self.frames[self.curr_frame_idx].0;
                    self.curr_frame_duration = duration;
                    self.curr_frame_idx += 1;
                    self.last_frame_change_time = now;

                    return FrameChange {
                        new_frame: Some(frame),
                        delay_before_recheck: duration.map(|d| Duration::from_millis(d as u64)),
                    };
                }

                if !self.last_frame_known {
                    FrameChange {
                        new_frame: None,

                        // ~60fps by default while waiting for frames
                        delay_before_recheck: Some(Duration::from_millis(16)),
                    }
                } else {
                    match self.left_loop_iterations {
                        /* Nothing to show anymore */
                        None => FrameChange {
                            new_frame: None,
                            delay_before_recheck: None,
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

                            self.curr_frame_duration = self.frames[0].1;
                            self.curr_frame_idx = 1;
                            self.last_frame_change_time = now;
                            let frame = &self.frames[0].0;
                            if let Some(dur) = self.curr_frame_duration {
                                let delay_til_next = Some(Duration::from_millis(dur as u64));
                                FrameChange {
                                    new_frame: Some(frame),
                                    delay_before_recheck: delay_til_next,
                                }
                            } else {
                                FrameChange {
                                    new_frame: Some(frame),
                                    delay_before_recheck: None,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
