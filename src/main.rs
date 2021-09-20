#![feature(div_duration)]
use sdl2::event::Event;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use std::io::{prelude::*, BufReader};
use std::path::Path;
use std::time::{Duration, Instant};

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

    dbg!(&metrics);

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

    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        canvas.set_draw_color(Color::RGB(32, 64, 192));
        canvas.clear();
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
                } => game_state.advance(),
                _ => {}
            }
        }
        // The rest of the game loop goes here...

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
                        canvas.fill_rect(Rect::new(x, y, width, height)).unwrap();
                        match question.clues {
                            QuestionClues::TextClues(ref clues) => {
                                let text_surface = font
                                    .render(&clues[i])
                                    .blended_wrapped(
                                        Color::RGB(0x33, 0x33, 0x33),
                                        metrics.tile_size.0,
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
                                            x,
                                            y,
                                            text_texture.query().width,
                                            text_texture.query().height,
                                        )),
                                    )
                                    .unwrap();
                            }
                        }
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
                            .blended_wrapped(Color::RGB(0x33, 0x33, 0x33), metrics.answer_size.0)
                            .unwrap();

                        let text_texture = texture_creator
                            .create_texture_from_surface(&text_surface)
                            .unwrap();

                        canvas
                            .copy(
                                &text_texture,
                                None,
                                Some(Rect::new(
                                    metrics.answer_pos.0 as i32,
                                    metrics.answer_pos.1 as i32,
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
    /// EG pressing space
    pub fn advance(&mut self) {
        match self.phase_state {
            GamePhaseState::StartPage => {
                self.phase_state = GamePhaseState::Questions {
                    current_question: 0,
                    question_state: QuestionState {
                        start_time: Instant::now(),
                        clues_shown: 1,
                    },
                };
            }
            GamePhaseState::Questions {
                ref mut current_question,
                ref mut question_state,
            } => {
                if question_state.clues_shown < 5 {
                    question_state.clues_shown += 1;
                } else if *current_question + 1 < self.questions.len() {
                    *current_question += 1;
                    *question_state = QuestionState {
                        start_time: Instant::now(),
                        clues_shown: 1,
                    };
                } else {
                    self.phase_state = GamePhaseState::EndPage
                }
            }
            GamePhaseState::EndPage => {}
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

        let tile_width = width * 4 / 21;
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

        Metrics {
            tile_size: (tile_width, tile_height),
            tile_0_pos: (margin, tile_ypos),
            tile_x_stride: tile_width + tile_spacing,
            answer_size: (answer_width, answer_height),
            answer_pos: (margin, answer_ypos),
            text_size,
        }
    }
}
