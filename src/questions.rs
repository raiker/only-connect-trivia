use std::fs::File;
use std::io::prelude::*;
use std::{io::BufReader, path::Path};

use rand::prelude::*;

pub struct QuestionSet {
    pub title: String,
    pub questions: Vec<Question>,
}

#[derive(Debug)]
pub struct Question {
    pub question_type: QuestionType,
    pub connection: String,
    pub clues: QuestionClues,
}

#[derive(Debug)]
pub enum QuestionType {
    Sequence,
    Connection,
}

#[derive(Debug)]
pub enum QuestionClues {
    TextClues(Vec<String>),
    // PictureClues(Vec<(Surface, String)>),
    // MusicClues([;4]),
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
                } else if let Some(p) = l.strip_prefix("        ") {
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
        Ok(QuestionSet { title, questions })
    }
}
