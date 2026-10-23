use std::sync::Arc;
use std::time::Duration;

use eink_rs::image::LoadError;
use eink_rs::video::{
    FrameRequest, Video, VideoDecoder, VideoDecoders, VideoFrames, VideoInfo, VideoOptions,
    VideoStatus, use_video_player,
};
use ink_rs::{Element, FlexDirection, Options, Scope, Styled, Text, View, component};

struct Pattern;

impl VideoDecoder for Pattern {
    fn info(&self, _: &str) -> Result<VideoInfo, LoadError> {
        Ok(VideoInfo {
            width: 640,
            height: 360,
            duration: Some(Duration::from_secs(10)),
            fps: Some(24),
        })
    }

    fn frames(&self, _: &str, request: &FrameRequest) -> Result<VideoFrames, LoadError> {
        let request = request.clone();
        let count = (Duration::from_secs(10)
            .saturating_sub(request.start)
            .as_secs_f64()
            * f64::from(request.fps))
        .ceil() as u32;
        Ok(Box::new((0..count).map(move |index| {
            // Radix blue, grass, amber and tomato, step 9.
            let colors = [[0, 144, 255], [70, 167, 88], [255, 197, 61], [229, 77, 46]];
            let phase = (request.start.as_secs_f64() * 64.0
                + f64::from(index) * 64.0 / f64::from(request.fps)) as u32;
            let mut rgb = Vec::with_capacity((request.width * request.height * 3) as usize);
            for _ in 0..request.height {
                for x in 0..request.width {
                    let bar = ((x * 640 / request.width + phase) / 160) as usize % colors.len();
                    rgb.extend_from_slice(&colors[bar]);
                }
            }
            Ok(rgb)
        })))
    }
}

fn root(cx: &mut Scope, props: &(String,)) -> Element {
    let src = &props.0;
    if src == "--pattern" {
        let decoders = cx.use_memo((), || VideoDecoders {
            local: Some(Arc::new(Pattern)),
            network: None,
        });
        cx.provide(decoders);
    }
    component(player, props.clone())
}

fn player(cx: &mut Scope, props: &(String,)) -> Element {
    let src = &props.0;
    let looping = cx.use_state(|| false);
    let player = use_video_player(
        cx,
        VideoOptions {
            src: src.clone(),
            looping: looping.get(),
            ..VideoOptions::default()
        },
    );
    let app = cx.use_app();
    let controls = player.clone();
    cx.use_input(true, move |text, key| {
        if key.left_arrow {
            controls.seek(controls.position().saturating_sub(Duration::from_secs(5)));
        } else if key.right_arrow {
            controls.seek(controls.position().saturating_add(Duration::from_secs(5)));
        } else {
            match text {
                " " if controls.status() == VideoStatus::Playing => controls.pause(),
                " " => controls.play(),
                "l" => looping.update(|value| *value = !*value),
                "q" => app.exit(),
                _ => {}
            }
        }
    });
    let status = match player.status() {
        VideoStatus::Loading => "Loading".into(),
        VideoStatus::Playing => "Playing".into(),
        VideoStatus::Paused => "Paused".into(),
        VideoStatus::Ended => "Ended".into(),
        VideoStatus::Failed(error) => format!("Failed: {error}"),
    };
    let duration = player
        .duration()
        .map_or_else(|| "?".into(), |time| format!("{:.1}s", time.as_secs_f64()));
    View::new()
        .flex_direction(FlexDirection::Column)
        .child(Video {
            player: player.clone(),
            alt: src.clone(),
        })
        .child(Text::new(format!(
            "{status}  {:.1}s / {duration}  loop: {}",
            player.position().as_secs_f64(),
            if looping.get() { "on" } else { "off" }
        )))
        .into()
}

fn main() -> Result<(), ink_rs::Error> {
    let Some(src) = std::env::args().nth(1) else {
        eprintln!("Usage: video <src|--pattern>");
        return Ok(());
    };
    ink_rs::render(component(root, (src,)), Options::default())
        .wait_until_exit()
        .map(|_| ())
}
