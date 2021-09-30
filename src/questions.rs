use std::fs::File;
use std::io::prelude::*;
use std::{io::BufReader, path::Path};

use lazy_static::lazy_static;
use rand::prelude::*;
use regex::Regex;
use sdl2::image::LoadSurface;
use sdl2::surface::Surface;

lazy_static! {
    static ref PICTURE_CLUE_REGEX: Regex = Regex::new(r"^        picture: (\S+) (\S.+)$").unwrap();
}

pub struct QuestionSet {
    pub title: String,
    pub questions: Vec<Question>,
}

#[derive(Debug)]
pub struct Question {
    pub question_type: QuestionType,
    pub connection: String,
    pub clues: Vec<Clue>,
}

#[derive(Debug)]
pub enum QuestionType {
    Sequence,
    Connection,
}

pub enum Clue {
    TextClue(String),
    PictureClue(Surface<'static>, String),
    // MusicClues([;4]),
}

impl std::fmt::Debug for Clue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TextClue(clue) => f.debug_tuple("TextClue").field(clue).finish(),
            Self::PictureClue(_surface, clue) => f.debug_tuple("PictureClue").field(clue).finish(),
        }
    }
}

pub fn load_question_sets<P: AsRef<Path>>(path: P) -> Result<Vec<QuestionSet>, Vec<String>> {
    let mut errors = vec![];
    let mut outputs = vec![];
    let question_sets_file = File::open(path).map_err(|e| vec![e.to_string()])?;

    let bufreader = BufReader::new(question_sets_file);

    for line in bufreader.lines() {
        match line {
            Ok(l) => {
                if l.trim_start().starts_with("#") {
                    continue;
                }

                if let Some(filename) = l.strip_prefix("include_shuffle: ") {
                    match load_questions(filename) {
                        Ok(mut qs) => {
                            // shuffle the questions
                            qs.questions.shuffle(&mut rand::thread_rng());
                            outputs.push(qs);
                        }
                        Err(mut es) => errors.append(&mut es),
                    }
                } else {
                    errors.push(format!("Unknown command {}", l));
                }
            }
            Err(e) => {
                errors.push(e.to_string());
            }
        }
    }

    if errors.is_empty() {
        Ok(outputs)
    } else {
        Err(errors)
    }
}

pub fn load_questions<P: AsRef<Path>>(path: P) -> Result<QuestionSet, Vec<String>> {
    let mut questions = Vec::new();
    let mut errors = Vec::new();

    let mut current_question: Option<(QuestionType, String, Vec<Clue>)> = None;

    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => {
            errors.push(e.to_string());
            return Err(errors);
        }
    };

    let bufreader = BufReader::new(file);

    let replace_question = |current_question: &mut Option<(QuestionType, String, Vec<Clue>)>,
                            new_question,
                            questions: &mut Vec<Question>,
                            errors: &mut Vec<String>| {
        if let Some((question_type, connection, clues)) = current_question.take() {
            if clues.len() == 4 {
                questions.push(Question {
                    question_type,
                    connection,
                    clues,
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

    let mut line_iter = bufreader.lines();

    let title = if let Some(Ok(t)) = line_iter.next() {
        t
    } else {
        return Err(vec![format!("No lines in file")]);
    };

    for line in line_iter {
        match line {
            Ok(l) => {
                if l.trim_start().starts_with("#") {
                    continue;
                }
                if let Some(q) = l.strip_prefix("    sequence: ") {
                    replace_question(
                        &mut current_question,
                        Some((QuestionType::Sequence, q.to_string(), Vec::new())),
                        &mut questions,
                        &mut errors,
                    );
                } else if let Some(q) = l.strip_prefix("    connection: ") {
                    replace_question(
                        &mut current_question,
                        Some((QuestionType::Connection, q.to_string(), Vec::new())),
                        &mut questions,
                        &mut errors,
                    );
                } else if let Some(captures) = PICTURE_CLUE_REGEX.captures(&l) {
                    let picture_path = captures.get(1).unwrap();
                    let text_clue = captures.get(2).unwrap();

                    // attempt to load the picture
                    if let Ok(image) = Surface::from_file(picture_path.as_str()) {
                        if let Some((_, _, ref mut clues)) = current_question {
                            clues.push(Clue::PictureClue(image, text_clue.as_str().into()));
                        } else {
                            errors.push(format!(
                                "Clue {} doesn't belong to a question",
                                text_clue.as_str()
                            ));
                        }
                    } else {
                        errors.push(format!("Could not load image {}", picture_path.as_str()));
                    }
                } else if let Some(p) = l.strip_prefix("        ") {
                    if let Some((_, _, ref mut clues)) = current_question {
                        clues.push(Clue::TextClue(p.into()));
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
        Ok(QuestionSet { title, questions })
    }
}
