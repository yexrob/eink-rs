use std::time::Duration;

use image::metadata::LoopCount;

pub(super) struct EncodedFrame {
    pub png: Vec<u8>,
    pub delay: Duration,
}

pub(super) struct Animation {
    pub frames: Vec<EncodedFrame>,
    pub loops: LoopCount,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Position {
    pub frame: usize,
    pub at: Duration,
    pub ended: bool,
}

impl Animation {
    pub fn position(&self, time: Duration) -> Position {
        let duration: Duration = self.frames.iter().map(|frame| frame.delay).sum();
        let cycle = time.as_nanos() / duration.as_nanos();
        if let LoopCount::Finite(loops) = self.loops
            && cycle >= u128::from(loops.get())
        {
            return Position {
                frame: self.frames.len() - 1,
                at: duration * loops.get() - self.frames.last().expect("animation frame").delay,
                ended: true,
            };
        }
        let remainder = Duration::from_nanos((time.as_nanos() % duration.as_nanos()) as u64);
        let mut at = time - remainder;
        for (index, frame) in self.frames.iter().enumerate() {
            if time < at + frame.delay {
                return Position {
                    frame: index,
                    at,
                    ended: false,
                };
            }
            at += frame.delay;
        }
        unreachable!("nonempty animation with positive delays")
    }

    pub fn following(&self, position: Position) -> Position {
        self.position(position.at + self.frames[position.frame].delay)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_observes_delays_loops_and_the_final_frame() {
        let mut animation = Animation {
            frames: [100, 40, 60]
                .map(|ms| EncodedFrame {
                    png: Vec::new(),
                    delay: Duration::from_millis(ms),
                })
                .into(),
            loops: LoopCount::Infinite,
        };
        for (ms, frame, at) in [
            (0, 0, 0),
            (99, 0, 0),
            (100, 1, 100),
            (140, 2, 140),
            (200, 0, 200),
            (340, 2, 340),
        ] {
            assert_eq!(
                animation.position(Duration::from_millis(ms)),
                Position {
                    frame,
                    at: Duration::from_millis(at),
                    ended: false,
                }
            );
        }
        animation.loops = LoopCount::Finite(2.try_into().unwrap());
        for ms in [400, 1000] {
            assert_eq!(
                animation.position(Duration::from_millis(ms)),
                Position {
                    frame: 2,
                    at: Duration::from_millis(340),
                    ended: true,
                }
            );
        }
    }
}
