#![feature(div_duration, generators, generator_trait)]
use lazy_static::lazy_static;
use questions::{Question, QuestionClues, QuestionSet, QuestionType};
use rand::Rng;
use regex::Regex;
use sdl2::event::Event;
use sdl2::gfx::primitives::DrawRenderer;
use sdl2::keyboard::Keycode;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::Texture;
use sdl2::surface::Surface;
use sdl2::ttf::Font;
use std::{
    fmt::Debug,
    ops::Generator,
    ops::GeneratorState,
    pin::Pin,
    time::{Duration, Instant},
};

mod questions;
// mod questions2;

const BACKGROUND_GREY: Color = Color::RGB(0x66, 0x66, 0x66);
const BACKGROUND_RED: Color = Color::RGB(0x66, 0x33, 0x33);
const BACKGROUND_BLUE: Color = Color::RGB(0x33, 0x33, 0xcc);
const TILE_TEXT_COLOUR: Color = Color::RGB(0x33, 0x33, 0x33);
const TILE_BACKGROUND_COLOUR: Color = Color::RGB(0x99, 0x99, 0x99);
const PROGRESS_BAR_BACKGROUND_COLOUR: Color = Color::RGB(0x33, 0x33, 0x33);
const PROGRESS_BAR_FOREGROUND_COLOUR: Color = Color::RGB(0x99, 0x99, 0x99);
const PROGRESS_BAR_TEXT_COLOUR: Color = Color::RGB(0xff, 0xff, 0xff);
const RED_SCORE_TILE_COLOUR: Color = Color::RGB(0x99, 0x66, 0x66);
const BLUE_SCORE_TILE_COLOUR: Color = Color::RGB(0x66, 0x66, 0xff);
const SCORE_TILE_TEXT_COLOUR: Color = Color::RGB(0xff, 0xff, 0xff);

const TIME_PER_QUESTION: Duration = Duration::from_secs(45);
const COUNT_IN_TIME: Duration = Duration::from_secs(3);
const COUNTDOWN_TIME: Duration = Duration::from_secs(5);

#[derive(Debug)]
enum BackgroundColour {
    Red,
    Blue,
    Grey,
}

trait QuestionPhase: Debug {
    fn get_points(&self) -> i32;
    fn pass_over(&mut self);
    fn show_answer(&mut self);
    fn next(&mut self);
    fn is_passed_over(&self) -> bool;
    fn is_answer_shown(&self) -> bool;
    fn clues_to_show(&self) -> usize;
    fn is_progress_bar_shown(&self) -> bool;
    fn is_count_in(&self) -> bool;
    fn is_first_team_guess(&self) -> bool;
}

#[derive(Debug)]
enum ConnectionPhase {
    CountIn,
    OneClueShown,
    TwoCluesShown,
    ThreeCluesShown,
    FourCluesShown,
    PassedOver,
    AnswerShown,
}

impl QuestionPhase for ConnectionPhase {
    fn get_points(&self) -> i32 {
        match self {
            ConnectionPhase::CountIn => unreachable!(),
            ConnectionPhase::OneClueShown => 5,
            ConnectionPhase::TwoCluesShown => 3,
            ConnectionPhase::ThreeCluesShown => 2,
            ConnectionPhase::FourCluesShown => 1,
            ConnectionPhase::PassedOver => 1,
            ConnectionPhase::AnswerShown => unreachable!(),
        }
    }

    fn pass_over(&mut self) {
        match self {
            ConnectionPhase::OneClueShown
            | ConnectionPhase::TwoCluesShown
            | ConnectionPhase::ThreeCluesShown
            | ConnectionPhase::FourCluesShown => *self = ConnectionPhase::PassedOver,
            ConnectionPhase::CountIn
            | ConnectionPhase::PassedOver
            | ConnectionPhase::AnswerShown => unreachable!(),
        }
    }

    fn show_answer(&mut self) {
        match self {
            ConnectionPhase::OneClueShown
            | ConnectionPhase::TwoCluesShown
            | ConnectionPhase::ThreeCluesShown
            | ConnectionPhase::FourCluesShown
            | ConnectionPhase::PassedOver => *self = ConnectionPhase::AnswerShown,
            ConnectionPhase::CountIn | ConnectionPhase::AnswerShown => unreachable!(),
        }
    }

    fn next(&mut self) {
        match self {
            ConnectionPhase::CountIn => *self = ConnectionPhase::OneClueShown,
            ConnectionPhase::OneClueShown => *self = ConnectionPhase::TwoCluesShown,
            ConnectionPhase::TwoCluesShown => *self = ConnectionPhase::ThreeCluesShown,
            ConnectionPhase::ThreeCluesShown => *self = ConnectionPhase::FourCluesShown,
            ConnectionPhase::FourCluesShown => *self = ConnectionPhase::FourCluesShown,
            ConnectionPhase::PassedOver | ConnectionPhase::AnswerShown => unreachable!(),
        }
    }

    fn is_passed_over(&self) -> bool {
        if let ConnectionPhase::PassedOver = self {
            true
        } else {
            false
        }
    }

    fn is_answer_shown(&self) -> bool {
        if let ConnectionPhase::AnswerShown = self {
            true
        } else {
            false
        }
    }

    fn clues_to_show(&self) -> usize {
        match self {
            ConnectionPhase::CountIn => 0,
            ConnectionPhase::OneClueShown => 1,
            ConnectionPhase::TwoCluesShown => 2,
            ConnectionPhase::ThreeCluesShown => 3,
            ConnectionPhase::FourCluesShown => 4,
            ConnectionPhase::PassedOver => 4,
            ConnectionPhase::AnswerShown => 4,
        }
    }

    fn is_progress_bar_shown(&self) -> bool {
        match self {
            ConnectionPhase::OneClueShown
            | ConnectionPhase::TwoCluesShown
            | ConnectionPhase::ThreeCluesShown
            | ConnectionPhase::FourCluesShown
            | ConnectionPhase::PassedOver => true,
            ConnectionPhase::CountIn | ConnectionPhase::AnswerShown => false,
        }
    }

    fn is_count_in(&self) -> bool {
        if let ConnectionPhase::CountIn = self {
            true
        } else {
            false
        }
    }

    fn is_first_team_guess(&self) -> bool {
        match self {
            ConnectionPhase::OneClueShown
            | ConnectionPhase::TwoCluesShown
            | ConnectionPhase::ThreeCluesShown
            | ConnectionPhase::FourCluesShown => true,
            ConnectionPhase::CountIn
            | ConnectionPhase::PassedOver
            | ConnectionPhase::AnswerShown => false,
        }
    }
}

#[derive(Debug)]
enum SequencePhase {
    CountIn,
    OneClueShown,
    TwoCluesShown,
    ThreeCluesShown,
    PassedOver,
    AnswerShown,
}

impl QuestionPhase for SequencePhase {
    fn get_points(&self) -> i32 {
        match self {
            SequencePhase::OneClueShown => 5,
            SequencePhase::TwoCluesShown => 3,
            SequencePhase::ThreeCluesShown => 2,
            SequencePhase::PassedOver => 1,
            SequencePhase::CountIn | SequencePhase::AnswerShown => unreachable!(),
        }
    }

    fn pass_over(&mut self) {
        match self {
            SequencePhase::OneClueShown
            | SequencePhase::TwoCluesShown
            | SequencePhase::ThreeCluesShown => *self = SequencePhase::PassedOver,
            SequencePhase::CountIn | SequencePhase::PassedOver | SequencePhase::AnswerShown => {
                unreachable!()
            }
        }
    }

    fn show_answer(&mut self) {
        match self {
            SequencePhase::OneClueShown
            | SequencePhase::TwoCluesShown
            | SequencePhase::ThreeCluesShown
            | SequencePhase::PassedOver => *self = SequencePhase::AnswerShown,
            SequencePhase::CountIn | SequencePhase::AnswerShown => unreachable!(),
        }
    }

    fn next(&mut self) {
        match self {
            SequencePhase::CountIn => *self = SequencePhase::OneClueShown,
            SequencePhase::OneClueShown => *self = SequencePhase::TwoCluesShown,
            SequencePhase::TwoCluesShown => *self = SequencePhase::ThreeCluesShown,
            SequencePhase::ThreeCluesShown => *self = SequencePhase::ThreeCluesShown,
            SequencePhase::PassedOver | SequencePhase::AnswerShown => {
                unreachable!()
            }
        }
    }

    fn is_passed_over(&self) -> bool {
        if let SequencePhase::PassedOver = self {
            true
        } else {
            false
        }
    }

    fn is_answer_shown(&self) -> bool {
        if let SequencePhase::AnswerShown = self {
            true
        } else {
            false
        }
    }

    fn clues_to_show(&self) -> usize {
        match self {
            SequencePhase::CountIn => 0,
            SequencePhase::OneClueShown => 1,
            SequencePhase::TwoCluesShown => 2,
            SequencePhase::ThreeCluesShown => 3,
            SequencePhase::PassedOver => 3,
            SequencePhase::AnswerShown => 4,
        }
    }

    fn is_progress_bar_shown(&self) -> bool {
        match self {
            SequencePhase::OneClueShown
            | SequencePhase::TwoCluesShown
            | SequencePhase::ThreeCluesShown => true,
            SequencePhase::CountIn | SequencePhase::PassedOver | SequencePhase::AnswerShown => {
                false
            }
        }
    }

    fn is_count_in(&self) -> bool {
        if let SequencePhase::CountIn = self {
            true
        } else {
            false
        }
    }

    fn is_first_team_guess(&self) -> bool {
        match self {
            SequencePhase::OneClueShown
            | SequencePhase::TwoCluesShown
            | SequencePhase::ThreeCluesShown => true,
            SequencePhase::CountIn | SequencePhase::PassedOver | SequencePhase::AnswerShown => {
                false
            }
        }
    }
}

#[derive(Debug, Default, Copy, Clone)]
struct KeyboardInput {
    next: bool,
    stop: bool,
    correct: bool,
    incorrect: bool,
}

#[derive(Debug)]
enum QuestionState {
    StartPage,
    TitlePage {
        title: String,
    },
    Question {
        clues: QuestionClues,
        connection: String,
        phase: Box<dyn QuestionPhase>,
        offered_to_red: bool, // question initially given to red team
        started: Instant,
        stopped: Option<Instant>,
    },
    EndPage,
}

#[derive(Debug)]
struct UpdateResult {
    next_question: bool,
    red_points_change: i32,
    blue_points_change: i32,
}

impl UpdateResult {
    pub fn no_change() -> Self {
        Self {
            next_question: false,
            red_points_change: 0,
            blue_points_change: 0,
        }
    }

    pub fn next_question() -> Self {
        Self {
            next_question: true,
            red_points_change: 0,
            blue_points_change: 0,
        }
    }

    pub fn points(points: i32, to_red_team: bool) -> Self {
        if to_red_team {
            Self {
                next_question: false,
                red_points_change: points,
                blue_points_change: 0,
            }
        } else {
            Self {
                next_question: false,
                red_points_change: 0,
                blue_points_change: points,
            }
        }
    }
}

impl QuestionState {
    /// Return value is whether to advance to the next question
    pub fn update(&mut self, input: KeyboardInput) -> UpdateResult {
        match self {
            QuestionState::StartPage | QuestionState::TitlePage { .. } => {
                if input.next {
                    UpdateResult::next_question()
                } else {
                    UpdateResult::no_change()
                }
            }
            QuestionState::Question {
                phase,
                started,
                stopped,
                offered_to_red,
                ..
            } => {
                if phase.is_count_in() {
                    if *started <= Instant::now() {
                        phase.next();
                    }
                    UpdateResult::no_change()
                } else if phase.is_answer_shown() {
                    if input.next {
                        UpdateResult::next_question()
                    } else {
                        UpdateResult::no_change()
                    }
                } else if phase.is_passed_over() {
                    if input.correct {
                        let points = phase.get_points();
                        phase.show_answer();
                        UpdateResult::points(points, !*offered_to_red)
                    } else if input.incorrect {
                        phase.show_answer();
                        UpdateResult::no_change()
                    } else {
                        UpdateResult::no_change()
                    }
                } else if let Some(_) = stopped {
                    // the clock has been stopped
                    if input.correct {
                        let points = phase.get_points();
                        phase.show_answer();
                        UpdateResult::points(points, *offered_to_red)
                    } else if input.incorrect {
                        phase.pass_over();
                        UpdateResult::no_change()
                    } else {
                        UpdateResult::no_change()
                    }
                } else {
                    // clock is still running
                    if Instant::now() - *started >= TIME_PER_QUESTION {
                        // out of time
                        phase.pass_over();
                        UpdateResult::no_change()
                    } else if input.next {
                        phase.next();
                        UpdateResult::no_change()
                    } else if input.stop {
                        *stopped = Some(Instant::now());
                        UpdateResult::no_change()
                    } else {
                        UpdateResult::no_change()
                    }
                }
            }
            QuestionState::EndPage => UpdateResult::no_change(), // no way out!
        }
    }

    pub fn get_background_colour(&self) -> BackgroundColour {
        match self {
            QuestionState::StartPage | QuestionState::TitlePage { .. } | QuestionState::EndPage => {
                BackgroundColour::Grey
            }
            QuestionState::Question {
                phase,
                offered_to_red,
                ..
            } => {
                if phase.is_answer_shown() {
                    BackgroundColour::Grey
                } else if phase.is_passed_over() {
                    if *offered_to_red {
                        BackgroundColour::Blue
                    } else {
                        BackgroundColour::Red
                    }
                } else {
                    if *offered_to_red {
                        BackgroundColour::Red
                    } else {
                        BackgroundColour::Blue
                    }
                }
            }
        }
    }
}

pub fn main() {
    unsafe {
        winapi::um::winuser::SetProcessDPIAware();
    }

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let ttf_context = sdl2::ttf::init().unwrap();

    // questions2::generate_test();

    let questions = match questions::load_question_sets("./io_trivia.txt") {
        Ok(qs) => qs,
        Err(es) => {
            eprintln!("Error(s) loading file");
            for e in es {
                eprintln!("{}", e);
            }
            return;
        }
    };

    let mut question_state_generator = || {
        let mut team_is_red: bool = rand::thread_rng().gen();

        yield QuestionState::StartPage;

        for set in questions {
            yield QuestionState::TitlePage { title: set.title };
            for q in set.questions {
                match q.question_type {
                    QuestionType::Connection => {
                        yield QuestionState::Question {
                            clues: q.clues,
                            connection: q.connection,
                            phase: Box::new(ConnectionPhase::CountIn),
                            offered_to_red: team_is_red,
                            started: Instant::now() + COUNT_IN_TIME,
                            stopped: None,
                        }
                    }
                    QuestionType::Sequence => {
                        yield QuestionState::Question {
                            clues: q.clues,
                            connection: q.connection,
                            phase: Box::new(SequencePhase::CountIn),
                            offered_to_red: team_is_red,
                            started: Instant::now() + COUNT_IN_TIME,
                            stopped: None,
                        }
                    }
                }
                team_is_red = !team_is_red;
            }
        }

        yield QuestionState::EndPage;
    };

    let mut question_state: QuestionState =
        if let GeneratorState::Yielded(x) = Pin::new(&mut question_state_generator).resume(()) {
            x
        } else {
            unreachable!()
        };

    let mut red_points = 0;
    let mut blue_points = 0;

    let window = video_subsystem
        .window("Only Connect Trivia", 1280, 720)
        .position_centered()
        .allow_highdpi()
        // .fullscreen_desktop()
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

    // let mut tile_textures: Vec<Texture> = (0..4)
    //     .map(|_| {
    //         let mut tex = texture_creator
    //             .create_texture_target(
    //                 sdl2::pixels::PixelFormatEnum::ARGB8888,
    //                 metrics.tile_size.0,
    //                 metrics.tile_size.1,
    //             )
    //             .unwrap();
    //         tex.set_blend_mode(sdl2::render::BlendMode::Blend);
    //         tex
    //     })
    //     .collect();
    // let mut answer_texture: Texture = texture_creator
    //     .create_texture_target(
    //         sdl2::pixels::PixelFormatEnum::ARGB8888,
    //         metrics.answer_size.0,
    //         metrics.answer_size.1,
    //     )
    //     .unwrap();
    // answer_texture.set_blend_mode(sdl2::render::BlendMode::Blend);

    let mut event_pump = sdl_context.event_pump().unwrap();
    'running: loop {
        let mut input = KeyboardInput::default();
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => break 'running,
                Event::KeyDown {
                    keycode: Some(Keycode::N),
                    repeat: false,
                    ..
                } => input.next = true,
                Event::KeyDown {
                    keycode: Some(Keycode::S),
                    repeat: false,
                    ..
                } => input.stop = true,
                Event::KeyDown {
                    keycode: Some(Keycode::C),
                    repeat: false,
                    ..
                } => input.correct = true,
                Event::KeyDown {
                    keycode: Some(Keycode::I),
                    repeat: false,
                    ..
                } => input.incorrect = true,
                _ => {}
            }
        }

        let update_result = question_state.update(input);

        blue_points += update_result.blue_points_change;
        red_points += update_result.red_points_change;

        if update_result.next_question {
            match Pin::new(&mut question_state_generator).resume(()) {
                GeneratorState::Yielded(x) => question_state = x,
                GeneratorState::Complete(_) => break 'running,
            }
        }

        // drawing code here
        match question_state.get_background_colour() {
            BackgroundColour::Red => canvas.set_draw_color(BACKGROUND_RED),
            BackgroundColour::Blue => canvas.set_draw_color(BACKGROUND_BLUE),
            BackgroundColour::Grey => canvas.set_draw_color(BACKGROUND_GREY),
        }

        canvas.clear();

        // let rerender_tiles = game_state.update(input);

        // // render each of the clue tiles ahead of time. Hopefully won't cause too much jank...
        // if rerender_tiles {
        match question_state {
            QuestionState::StartPage => {
                let banner_surface = render_text(
                    "Only Connect",
                    &font,
                    metrics.width,
                    metrics.height,
                    metrics.padding,
                    TILE_TEXT_COLOUR,
                )
                .unwrap();

                let banner_texture = texture_creator
                    .create_texture_from_surface(banner_surface)
                    .unwrap();

                canvas.copy(&banner_texture, None, None).unwrap();
            }
            QuestionState::TitlePage { ref title } => {
                let banner_surface = render_text(
                    title,
                    &font,
                    metrics.width,
                    metrics.height,
                    metrics.padding,
                    TILE_TEXT_COLOUR,
                )
                .unwrap();

                let banner_texture = texture_creator
                    .create_texture_from_surface(banner_surface)
                    .unwrap();

                canvas.copy(&banner_texture, None, None).unwrap();
            }
            QuestionState::Question {
                ref clues,
                ref phase,
                ref connection,
                ref started,
                ref stopped,
                ..
            } => {
                for i in 0..phase.clues_to_show() {
                    let dst_rect = metrics.get_tile_dest_rect(i);
                    canvas.set_draw_color(TILE_BACKGROUND_COLOUR);
                    canvas.fill_rect(dst_rect).unwrap();
                    match clues {
                        QuestionClues::TextClues(clues) => {
                            let text_surface = render_text(
                                &clues[i],
                                &font,
                                metrics.tile_size.0,
                                metrics.tile_size.1,
                                metrics.padding,
                                TILE_TEXT_COLOUR,
                            )
                            .unwrap();

                            let text_texture = texture_creator
                                .create_texture_from_surface(text_surface)
                                .unwrap();

                            canvas.copy(&text_texture, None, dst_rect).unwrap();
                        }
                    }
                }
                if phase.is_answer_shown() {
                    let dst_rect = metrics.get_answer_dest_rect();
                    canvas.set_draw_color(TILE_BACKGROUND_COLOUR);
                    canvas.fill_rect(dst_rect).unwrap();
                    let text_surface = render_text(
                        &connection,
                        &font,
                        metrics.answer_size.0,
                        metrics.answer_size.1,
                        metrics.padding,
                        TILE_TEXT_COLOUR,
                    )
                    .unwrap();

                    let text_texture = texture_creator
                        .create_texture_from_surface(text_surface)
                        .unwrap();

                    canvas.copy(&text_texture, None, dst_rect).unwrap();
                }
                if phase.is_progress_bar_shown() {
                    let stop_time = stopped.unwrap_or(Instant::now());
                    let time_elapsed = stop_time - *started;
                    let fraction_time_elapsed = time_elapsed.div_duration_f32(TIME_PER_QUESTION);
                    let progress_bar_fraction = fraction_time_elapsed.clamp(0.0, 1.0);

                    let last_tile_shown_index = phase.clues_to_show() - 1;

                    let background_dst_rect =
                        metrics.get_progress_bar_dest_rect(last_tile_shown_index);
                    let fill_dst_rect = metrics.get_progress_bar_fill_dest_rect(
                        last_tile_shown_index,
                        progress_bar_fraction,
                    );

                    // draw bar background
                    canvas.set_draw_color(PROGRESS_BAR_BACKGROUND_COLOUR);
                    canvas.fill_rect(background_dst_rect).unwrap();
                    // draw bar fill
                    canvas.set_draw_color(PROGRESS_BAR_FOREGROUND_COLOUR);
                    canvas.fill_rect(fill_dst_rect).unwrap();

                    // draw points text overlay
                    let question_points = phase.get_points();
                    let overlay_text = match question_points {
                        1 => "1 point".into(),
                        _ => format!("{} points", question_points),
                    };

                    let text_surface = render_text(
                        &overlay_text,
                        &font,
                        metrics.tile_size.0,
                        metrics.progress_bar_height,
                        metrics.padding,
                        PROGRESS_BAR_TEXT_COLOUR,
                    )
                    .unwrap();

                    let text_texture = texture_creator
                        .create_texture_from_surface(text_surface)
                        .unwrap();

                    canvas
                        .copy(&text_texture, None, background_dst_rect)
                        .unwrap();
                }
                if phase.is_count_in() {
                    if let Some(count_in_time) = started.checked_duration_since(Instant::now()) {
                        let count_in_text = (count_in_time.as_secs_f32().ceil() as u32).to_string();
                        let text_surface = render_text(
                            &count_in_text,
                            &font,
                            metrics.countdown_tile_rect.width(),
                            metrics.countdown_tile_rect.height(),
                            metrics.padding,
                            PROGRESS_BAR_TEXT_COLOUR,
                        )
                        .unwrap();

                        let text_texture = texture_creator
                            .create_texture_from_surface(text_surface)
                            .unwrap();

                        canvas
                            .copy(&text_texture, None, metrics.countdown_tile_rect)
                            .unwrap();
                    }
                } else if let None = stopped {
                    if let Some(remaining_time) = (*started + TIME_PER_QUESTION).checked_duration_since(Instant::now()) {
                        if remaining_time < COUNTDOWN_TIME {
                            let countdown_text = (remaining_time.as_secs_f32().ceil() as u32).to_string();
                            let text_surface = render_text(
                                &countdown_text,
                                &font,
                                metrics.countdown_tile_rect.width(),
                                metrics.countdown_tile_rect.height(),
                                metrics.padding,
                                PROGRESS_BAR_TEXT_COLOUR,
                            )
                            .unwrap();

                            let text_texture = texture_creator
                                .create_texture_from_surface(text_surface)
                                .unwrap();

                            canvas
                                .copy(&text_texture, None, metrics.countdown_tile_rect)
                                .unwrap();
                        }
                    }
                }
            }
            QuestionState::EndPage => {
                let banner_surface = render_text(
                    "Game over",
                    &font,
                    metrics.width,
                    metrics.height,
                    metrics.padding,
                    TILE_TEXT_COLOUR,
                )
                .unwrap();

                let banner_texture = texture_creator
                    .create_texture_from_surface(banner_surface)
                    .unwrap();

                canvas.copy(&banner_texture, None, None).unwrap();
            }
        }

        // render scores on all pages except the start page
        match question_state {
            QuestionState::StartPage => {}
            _ => {
                let score_tiles = [
                    (
                        metrics.left_score_tile_rect,
                        RED_SCORE_TILE_COLOUR,
                        red_points,
                    ),
                    (
                        metrics.right_score_tile_rect,
                        BLUE_SCORE_TILE_COLOUR,
                        blue_points,
                    ),
                ];

                for (rect, colour, points) in score_tiles {
                    canvas.set_draw_color(colour);
                    canvas.fill_rect(rect).unwrap();
                    let points_string = points.to_string();
                    let text_surface = render_text(
                        &points_string,
                        &font,
                        rect.width(),
                        rect.height(),
                        metrics.padding,
                        SCORE_TILE_TEXT_COLOUR,
                    )
                    .unwrap();

                    let text_texture = texture_creator
                        .create_texture_from_surface(text_surface)
                        .unwrap();

                    canvas.copy(&text_texture, None, rect).unwrap();
                }
            }
        }

        canvas.present();
    }
}

// struct GameState {
//     red_score: i32,
//     blue_score: i32,
//     is_red_turn: bool,
//     questions: Vec<QuestionSet>,
//     time_per_question: Duration,
//     phase_state: GamePhaseState,
// }

// impl GameState {
//     /// Should be called each frame. Checks timers and keyboard input
//     pub fn update(&mut self, input: KeyboardInput) {
//         let now = Instant::now();

//         match self.phase_state {
//             GamePhaseState::StartPage => {
//                 if input.next {
//                     if self.questions.is_empty() {
//                         self.phase_state = GamePhaseState::EndPage;
//                     } else {
//                         self.phase_state = GamePhaseState::TitlePage {
//                             current_set: 0
//                         };
//                     }
//                 }
//             }
//             GamePhaseState::TitlePage {
//                 ref mut current_set,
//             } => {
//                 if input.next {
//                     if self.questions[*current_set].questions.is_empty() {
//                         if current_set + 1 < self.questions.len() {

//                         }
//                     }
//                 }
//             },
//             GamePhaseState::Questions {
//                 ref mut current_question,
//                 ref mut question_state,
//             } => {
//                 if question_state.clues_shown < 5 {
//                     question_state.clues_shown += 1;
//                     false
//                 } else if *current_question + 1 < self.questions.len() {
//                     *current_question += 1;
//                     *question_state = QuestionState {
//                         start_time: Instant::now(),
//                         clues_shown: 1,
//                     };
//                     true
//                 } else {
//                     self.phase_state = GamePhaseState::EndPage;
//                     false
//                 }
//             }
//             GamePhaseState::EndPage => false,
//         }
//     }
// }

// enum GamePhaseState {
//     StartPage,
//     TitlePage {
//         current_set: usize,
//     },
//     Questions {
//         current_set: usize,
//         current_question: usize,
//         question_state: QuestionState,
//     },
//     EndPage,
// }

// #[derive(Debug, Copy, Clone)]
// struct QuestionState {
//     start_time: Instant,
//     clues_shown: usize,
// }

#[derive(Debug)]
struct Metrics {
    width: u32,
    height: u32,
    tile_size: (u32, u32),
    tile_0_pos: (u32, u32),
    tile_x_stride: u32,
    padding: u32,
    answer_size: (u32, u32),
    answer_pos: (u32, u32),
    progress_bar_y: i32,
    progress_bar_height: u32,
    left_score_tile_rect: Rect,
    right_score_tile_rect: Rect,
    countdown_tile_rect: Rect,
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

        let progress_bar_height = tile_height / 4;
        let progress_bar_y = (tile_ypos - progress_bar_height - tile_spacing) as i32;

        let score_tile_width = tile_width * 2 / 3;
        let score_tile_height = tile_height * 2 / 3;

        let countdown_tile_width = tile_width / 2;
        let countdown_tile_height = tile_height / 2;

        let countdown_tile_xpos = ((width - countdown_tile_width) / 2) as i32;
        let countdown_tile_ypos = (2 * padding) as i32;

        let left_score_tile_rect = Rect::new(
            margin as i32,
            margin as i32,
            score_tile_width,
            score_tile_height,
        );
        let right_score_tile_rect = Rect::new(
            (width - margin - score_tile_width) as i32,
            margin as i32,
            score_tile_width,
            score_tile_height,
        );

        let countdown_tile_rect = Rect::new(
            countdown_tile_xpos,
            countdown_tile_ypos,
            countdown_tile_width,
            countdown_tile_height,
        );

        Metrics {
            width,
            height,
            tile_size: (tile_width, tile_height),
            tile_0_pos: (margin, tile_ypos),
            tile_x_stride: tile_width + tile_spacing,
            answer_size: (answer_width, answer_height),
            answer_pos: (margin, answer_ypos),
            padding,
            progress_bar_y,
            progress_bar_height,
            left_score_tile_rect,
            right_score_tile_rect,
            countdown_tile_rect,
            text_size,
        }
    }

    fn get_tile_dest_rect(&self, index: usize) -> Rect {
        let x = (self.tile_0_pos.0 + self.tile_x_stride * index as u32) as i32;
        let y = self.tile_0_pos.1 as i32;
        let width = self.tile_size.0;
        let height = self.tile_size.1;
        Rect::new(x, y, width, height)
    }

    fn get_answer_dest_rect(&self) -> Rect {
        Rect::new(
            self.answer_pos.0 as i32,
            self.answer_pos.1 as i32,
            self.answer_size.0,
            self.answer_size.1,
        )
    }

    fn get_progress_bar_dest_rect(&self, index: usize) -> Rect {
        let x = (self.tile_0_pos.0 + self.tile_x_stride * index as u32) as i32;
        let y = self.progress_bar_y;
        let width = self.tile_size.0;
        let height = self.progress_bar_height;
        Rect::new(x, y, width, height)
    }

    fn get_progress_bar_fill_dest_rect(&self, index: usize, fraction: f32) -> Rect {
        let mut rect = self.get_progress_bar_dest_rect(index);
        let new_width = (rect.width() as f32 * fraction).round() as u32;
        rect.set_width(new_width);
        rect
    }
}

// Renders a block of text centred and word-wrapped into a rectangle
fn render_text<'a>(
    text: &'a str,
    font: &Font,
    width: u32,
    height: u32,
    padding: u32,
    colour: Color,
) -> Result<Surface<'a>, String> {
    let text_width = width - 2 * padding;
    let splits = split_text(text, font, text_width);

    let mut output_surface = Surface::new(width, height, sdl2::pixels::PixelFormatEnum::RGBA8888)?;

    let text_height = splits.len() as i32 * font.recommended_line_spacing();
    let y_start = (height as i32 - text_height) / 2;

    for (i, text_line) in splits.into_iter().enumerate() {
        let rendered_line = match font.render(text_line).blended(colour) {
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

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum PrevChar {
    NoChar,
    Space,
    NotSpace,
}

// greedy algorithm
fn split_text<'a>(text: &'a str, font: &Font, width: u32) -> Vec<&'a str> {
    // find indices of word starts and word ends
    // let word_starts = WORD_START_REGEX
    //     .find_iter(text)
    //     .map(|m| m.end() - 1)
    //     .collect::<Vec<_>>();
    // let word_ends = WORD_END_REGEX
    //     .find_iter(text)
    //     .map(|m| m.start() + 1)
    //     .collect::<Vec<_>>();

    let mut word_starts = vec![];
    let mut word_ends = vec![];

    let mut prev = PrevChar::NoChar;

    for (i, c) in text.char_indices() {
        match prev {
            PrevChar::NoChar | PrevChar::Space => {
                if c != ' ' {
                    word_starts.push(i);
                }
            }
            PrevChar::NotSpace => {
                if c == ' ' {
                    word_ends.push(i);
                }
            }
        }

        if c == ' ' {
            prev = PrevChar::Space;
        } else {
            prev = PrevChar::NotSpace
        }
    }

    if prev == PrevChar::NotSpace {
        word_ends.push(text.len());
    }

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
