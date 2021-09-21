#![feature(div_duration)]
use lazy_static::lazy_static;
use regex::Regex;
use sdl2::event::Event;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Texture;
use sdl2::surface::Surface;
use sdl2::ttf::Font;
use std::io::{prelude::*, BufReader};
use std::path::Path;
use std::time::{Duration, Instant};

const TEXT_COLOUR: Color = Color::RGB(0x33, 0x33, 0x33);
const TILE_BACKGROUND_COLOUR: Color = Color::RGB(0x99, 0x99, 0xff);

pub fn main() {
    unsafe {
        winapi::um::winuser::SetProcessDPIAware();
    }

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let ttf_context = sdl2::ttf::init().unwrap();

    let questions = match load_questions("./connections.txt") {
        Ok(qs) => qs,
        Err(es) => {
            eprintln!("Error(s) loading file");
            for e in es {
                eprintln!("{}", e);
            }
            return;
        }
    };

    let mut game_state = GameState {
        questions,
        time_per_question: Duration::from_secs(45),
        phase_state: GamePhaseState::StartPage,
    };

    let window = video_subsystem
        .window("Only Connect Trivia", 1280, 720)
        .position_centered()
        .allow_highdpi()
        .fullscreen_desktop()
        .build()
        .unwrap();

    let metrics = Metrics::from_window_dimensions(window.size());

    let font = ttf_context
        .load_font("fonts/Roboto-Regular.ttf", metrics.text_size)
        .unwrap();
    // let (_ddpi, _hdpi, _vdpi) = video_subsystem.display_dpi(0).unwrap();

    let mut canvas = window
        .into_canvas()
        .present_vsync()
        .accelerated()
        .build()
        .unwrap();

    let texture_creator = canvas.texture_creator();

    let mut tile_textures: Vec<Texture> = (0..4)
        .map(|_| {
            texture_creator
                .create_texture_target(
                    sdl2::pixels::PixelFormatEnum::RGBA8888,
                    metrics.tile_size.0,
                    metrics.tile_size.1,
                )
                .unwrap()
        })
        .collect();
    let mut answer_texture: Texture = texture_creator
        .create_texture_target(
            sdl2::pixels::PixelFormatEnum::RGBA8888,
            metrics.answer_size.0,
            metrics.answer_size.1,
        )
        .unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        canvas.set_draw_color(Color::RGB(32, 64, 192));
        canvas.clear();

        let mut rerender_tiles = false;
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(Keycode::Space),
                    repeat: false,
                    ..
                } => rerender_tiles = game_state.advance(),
                _ => {}
            }
        }
        // The rest of the game loop goes here...

        // render each of the clue tiles ahead of time. Hopefully won't cause too much jank...
        if rerender_tiles {
            eprintln!("Rendering");
            match game_state.phase_state {
                GamePhaseState::Questions {
                    current_question, ..
                } => {
                    let question = &game_state.questions[current_question];

                    let textures_vec = tile_textures
                        .iter_mut()
                        .enumerate()
                        .map(|(i, t)| (t, i))
                        .collect::<Vec<_>>();

                    canvas
                        .with_multiple_texture_canvas(textures_vec.iter(), |texture_canvas, i| {
                            let local_texture_creator = texture_canvas.texture_creator();

                            texture_canvas.set_draw_color(TILE_BACKGROUND_COLOUR);
                            texture_canvas.clear();

                            let content_surface = match question.clues {
                                QuestionClues::TextClues(ref clues) => render_text(
                                    &clues[*i],
                                    &font,
                                    metrics.tile_size.0,
                                    metrics.tile_size.1,
                                )
                                .unwrap(),
                            };

                            let content_texture = local_texture_creator
                                .create_texture_from_surface(content_surface)
                                .unwrap();

                            texture_canvas.copy(&content_texture, None, None).unwrap();

                            // no need to present, apparently
                        })
                        .unwrap();
                }
                _ => {}
            }
        }

        match game_state.phase_state {
            GamePhaseState::StartPage => canvas.string(100, 100, "Start", Color::WHITE).unwrap(),
            GamePhaseState::Questions {
                current_question,
                question_state,
            } => {
                canvas.set_draw_color(Color::RGB(0xcc, 0xcc, 0xff));

                let question = &game_state.questions[current_question];
                for i in 0..question_state.clues_shown {
                    if i < 4 {
                        let x = (metrics.tile_0_pos.0 + metrics.tile_x_stride * i as u32) as i32;
                        let y = metrics.tile_0_pos.1 as i32;
                        let width = metrics.tile_size.0;
                        let height = metrics.tile_size.1;
                        let dst_rect = Rect::new(x, y, width, height);

                        canvas.copy(&tile_textures[i], None, dst_rect).unwrap();
                    } else {
                        canvas
                            .fill_rect(Rect::new(
                                metrics.answer_pos.0 as i32,
                                metrics.answer_pos.1 as i32,
                                metrics.answer_size.0,
                                metrics.answer_size.1,
                            ))
                            .unwrap();
                        let text_surface = font
                            .render(&question.connection)
                            .blended_wrapped(
                                Color::RGB(0x33, 0x33, 0x33),
                                metrics.answer_size.0 - 2 * metrics.padding,
                            )
                            .unwrap();

                        let text_texture = texture_creator
                            .create_texture_from_surface(&text_surface)
                            .unwrap();

                        canvas
                            .copy(
                                &text_texture,
                                None,
                                Some(Rect::new(
                                    (metrics.answer_pos.0 + metrics.padding) as i32,
                                    (metrics.answer_pos.1 + metrics.padding) as i32,
                                    text_texture.query().width,
                                    text_texture.query().height,
                                )),
                            )
                            .unwrap();
                    }
                }
            }
            GamePhaseState::EndPage => canvas.string(100, 100, "Game over", Color::WHITE).unwrap(),
        }

        canvas.present();
        //::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}

struct GameState {
    questions: Vec<Question>,
    time_per_question: Duration,
    phase_state: GamePhaseState,
}

impl GameState {
    /// e.g. pressing space
    /// # Returns
    /// True if the tiles need to be rerendered
    /// False otherwise
    pub fn advance(&mut self) -> bool {
        match self.phase_state {
            GamePhaseState::StartPage => {
                self.phase_state = GamePhaseState::Questions {
                    current_question: 0,
                    question_state: QuestionState {
                        start_time: Instant::now(),
                        clues_shown: 1,
                    },
                };
                true
            }
            GamePhaseState::Questions {
                ref mut current_question,
                ref mut question_state,
            } => {
                if question_state.clues_shown < 5 {
                    question_state.clues_shown += 1;
                    false
                } else if *current_question + 1 < self.questions.len() {
                    *current_question += 1;
                    *question_state = QuestionState {
                        start_time: Instant::now(),
                        clues_shown: 1,
                    };
                    true
                } else {
                    self.phase_state = GamePhaseState::EndPage;
                    false
                }
            }
            GamePhaseState::EndPage => false,
        }
    }
}

enum GamePhaseState {
    StartPage,
    Questions {
        current_question: usize,
        question_state: QuestionState,
    },
    EndPage,
}

#[derive(Debug, Copy, Clone)]
struct QuestionState {
    start_time: Instant,
    clues_shown: usize,
}

fn load_questions<P: AsRef<Path>>(path: P) -> Result<Vec<Question>, Vec<String>> {
    let mut questions = Vec::new();
    let mut errors = Vec::new();

    let mut current_question: Option<(QuestionType, String, Vec<String>)> = None;

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            errors.push(e.to_string());
            return Err(errors);
        }
    };

    let bufreader = BufReader::new(file);

    let replace_question = |current_question: &mut Option<(QuestionType, String, Vec<String>)>,
                            new_question,
                            questions: &mut Vec<Question>,
                            errors: &mut Vec<String>| {
        if let Some((question_type, connection, clues)) = current_question.take() {
            if clues.len() == 4 {
                questions.push(Question {
                    question_type,
                    connection,
                    clues: QuestionClues::TextClues(clues),
                });
            } else {
                errors.push(format!(
                    "Incorrect number of prompts for connection: {}",
                    connection
                ));
            }
        }

        *current_question = new_question;
    };

    for line in bufreader.lines() {
        match line {
            Ok(l) => {
                if let Some(q) = l.strip_prefix("sequence: ") {
                    replace_question(
                        &mut current_question,
                        Some((QuestionType::Sequence, q.to_string(), Vec::new())),
                        &mut questions,
                        &mut errors,
                    );
                } else if let Some(q) = l.strip_prefix("connection: ") {
                    replace_question(
                        &mut current_question,
                        Some((QuestionType::Connection, q.to_string(), Vec::new())),
                        &mut questions,
                        &mut errors,
                    );
                } else if let Some(p) = l.strip_prefix("    ") {
                    if let Some((_, _, ref mut clues)) = current_question {
                        clues.push(p.into());
                    } else {
                        errors.push(format!("Clue {} doesn't belong to a question", p));
                    }
                } else {
                    errors.push(format!("{} is neither a question nor a clue", l));
                }
            }
            Err(e) => {
                errors.push(e.to_string());
            }
        }
    }

    // close the last question
    replace_question(&mut current_question, None, &mut questions, &mut errors);

    if !errors.is_empty() {
        Err(errors)
    } else {
        Ok(questions)
    }
}

#[derive(Debug)]
struct Question {
    question_type: QuestionType,
    connection: String,
    clues: QuestionClues,
}

#[derive(Debug)]
enum QuestionType {
    Sequence,
    Connection,
}

#[derive(Debug)]
enum QuestionClues {
    TextClues(Vec<String>),
    // PictureClues([;4]),
    // MusicClues([;4]),
}

#[derive(Debug)]
struct Metrics {
    tile_size: (u32, u32),
    tile_0_pos: (u32, u32),
    tile_x_stride: u32,
    padding: u32,
    answer_size: (u32, u32),
    answer_pos: (u32, u32),
    text_size: u16,
}

impl Metrics {
    fn from_window_dimensions(window_dimensions: (u32, u32)) -> Self {
        let (width, height) = window_dimensions;

        // width = 2 * tile_0_pos.x + 3 * intertile_spacing + 4 * tile_width
        // constrained by all of them being integers
        // tile width is 4 parts, intertile spacing is 1 part, margin is 1 part
        // these formulas give pretty reasonable results

        let tile_width = width * 8 / 37;
        let rem = width - 4 * tile_width;
        let tile_spacing = rem / 5 + (rem - rem / 5) % 2;
        let margin = (rem - tile_spacing * 3) / 2;

        assert_eq!(tile_width * 4 + tile_spacing * 3 + margin * 2, width);

        let tile_height = tile_width * 3 / 4; // might need to compensate for aspect ratio here
        let answer_width = 4 * tile_width + 3 * tile_spacing;
        let answer_height = tile_height / 2;
        let text_size = (answer_height * 2 / 5) as u16;

        let answer_ypos = height - margin - answer_height;
        let tile_ypos = answer_ypos - tile_spacing - tile_height;

        let padding = tile_height / 6;

        Metrics {
            tile_size: (tile_width, tile_height),
            tile_0_pos: (margin, tile_ypos),
            tile_x_stride: tile_width + tile_spacing,
            answer_size: (answer_width, answer_height),
            answer_pos: (margin, answer_ypos),
            text_size,
            padding,
        }
    }
}

// Renders a block of text centred and word-wrapped into a rectangle
fn render_text<'a>(
    text: &'a str,
    font: &Font,
    width: u32,
    height: u32,
) -> Result<Surface<'a>, String> {
    let splits = split_text(text, font, width);

    let mut output_surface = Surface::new(width, height, sdl2::pixels::PixelFormatEnum::RGBA8888)?;

    let text_height = splits.len() as i32 * font.recommended_line_spacing();
    let y_start = (height as i32 - text_height) / 2;

    for (i, text_line) in splits.into_iter().enumerate() {
        let rendered_line = match font.render(text_line).blended(TEXT_COLOUR) {
            Ok(s) => s,
            Err(sdl2::ttf::FontError::InvalidLatin1Text(_)) => unreachable!(),
            Err(sdl2::ttf::FontError::SdlError(s)) => return Err(s),
        };

        // position each line centred
        let dst_rect = Rect::new(
            (width as i32 - rendered_line.width() as i32) / 2,
            y_start + i as i32 * font.recommended_line_spacing(),
            rendered_line.width(),
            rendered_line.height(),
        );

        // blit the rendered line of text into the output surface
        rendered_line.blit(None, &mut output_surface, dst_rect)?;
    }
    Ok(output_surface)
}

lazy_static! {
    static ref WORD_START_REGEX: Regex = Regex::new("(?:^| )[^ ]").unwrap();
    static ref WORD_END_REGEX: Regex = Regex::new("[^ ](?:$| )").unwrap();
}

// greedy algorithm
fn split_text<'a>(text: &'a str, font: &Font, width: u32) -> Vec<&'a str> {
    // find indices of word starts and word ends
    let word_starts = WORD_START_REGEX
        .find_iter(text)
        .map(|m| m.end() - 1)
        .collect::<Vec<_>>();
    let word_ends = WORD_END_REGEX
        .find_iter(text)
        .map(|m| m.start() + 1)
        .collect::<Vec<_>>();

    assert_eq!(word_starts.len(), word_ends.len());

    let mut output_vec = Vec::new();

    let mut i = 0;

    while i < word_starts.len() {
        // find the longest run of words that will fit in the width allowed
        let mut j = i;
        for jx in i..word_ends.len() {
            let slice = &text[word_starts[i]..word_ends[jx]];
            let computed_length = font.size_of(slice).unwrap().0;
            if computed_length <= width {
                j = jx;
            } else {
                break;
            }
        }

        output_vec.push(&text[word_starts[i]..word_ends[j]]);

        i = j + 1;
    }

    output_vec
}
